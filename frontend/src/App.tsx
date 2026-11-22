import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { useMediaQuery } from "./useMediaQuery";
import { useRealtime } from "./useRealtime";
import { buildContainerQuery, DEFAULT_PAGE_SIZE } from "./containerQuery";
import { notification } from "antd";
import { Layout, Button, Typography, Flex, Space } from "antd";
import {
  BarChartOutlined,
  FileTextOutlined,
  SettingOutlined,
  LogoutOutlined,
} from "@ant-design/icons";
import type {
  BatchProgress as BatchProgressType,
  ContainerEvent,
  ContainerInfo,
  ContainerSummary,
  PagedContainers,
  StateResponse,
  NotifEvent,
  UpdateProgress,
  AppConfig,
  UpdateCheckConfig,
} from "./types";
import { apiFetch } from "./api";
import { mapBatchToResults } from "./batchResults";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
import HistoryPage from "./HistoryPage";
import BatchProgress from "./components/BatchProgress";
import NotifToast from "./components/NotifToast";
import SummaryDialog from "./components/SummaryDialog";

const { Text, Title } = Typography;

interface AppProps {
  colorScheme: "dark" | "light";
  setColorScheme: (scheme: "dark" | "light") => void;
}

interface UserInfo {
  sub: string;
  name: string;
  email: string;
}

export default function App({ colorScheme, setColorScheme }: AppProps) {
  const isMobile = useMediaQuery("(max-width: 768px)");
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  const [user, setUser] = useState<UserInfo | null>(null);
  const [containers, setContainers] = useState<ContainerInfo[]>([]);
  const [containersLoaded, setContainersLoaded] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  // Filters live here (not in DashboardPage) so the server query is built in one place.
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [stateFilter, setStateFilter] = useState<string[]>([]);
  const [showPendingUpdates, setShowPendingUpdates] = useState(false);
  const [summary, setSummary] = useState<ContainerSummary>({
    total: 0,
    running: 0,
    stopped: 0,
    paused: 0,
    with_updates: 0,
  });
  const [progress, setProgress] = useState<BatchProgressType>({
    total: 0,
    checked: 0,
    updated: 0,
    errors: 0,
    checking: "",
  });
  const [checking, setChecking] = useState(true);

  // Check auth status on mount
  useEffect(() => {
    fetch("/api/auth/me", { credentials: "include" })
      .then((res) => res.json())
      .then((data) => {
        if (data.authenticated) {
          setAuthenticated(true);
          setUser(data.user);
        } else {
          setAuthenticated(false);
        }
      })
      .catch(() => setAuthenticated(false))
      .finally(() => setChecking(false));
  }, []);

  // Periodic auth check to detect session expiry (every 5 minutes)
  useEffect(() => {
    if (!authenticated) return;
    const interval = setInterval(
      async () => {
        try {
          const res = await fetch("/api/auth/me", { credentials: "include" });
          if (res.status === 401) {
            const body = await res.json();
            if (body.session_expired) {
              window.location.href = "/api/auth/login";
            }
          }
        } catch {
          // Network error — ignore, retry next interval
        }
      },
      5 * 60 * 1000,
    );
    return () => clearInterval(interval);
  }, [authenticated]);

  // Debounce the search box before it reaches the query (300 ms); once the
  // search settles, jump back to the first page.
  useEffect(() => {
    const t = setTimeout(() => {
      setDebouncedQuery(searchQuery);
      setPage(1);
    }, 300);
    return () => clearTimeout(t);
  }, [searchQuery]);

  // Canonical query shared by the REST list and the realtime transports.
  const queryString = useMemo(
    () =>
      buildContainerQuery({
        page,
        pageSize,
        q: debouncedQuery,
        state: stateFilter,
        hasUpdate: showPendingUpdates,
      }),
    [page, pageSize, debouncedQuery, stateFilter, showPendingUpdates],
  );

  // Eager fetch of the requested page — refetches whenever the query changes.
  useEffect(() => {
    if (!authenticated) return;
    let cancelled = false;
    apiFetch(`/api/containers?${queryString}`)
      .then((res) => res.json())
      .then((data: PagedContainers) => {
        if (cancelled) return;
        // The client owns `page`/`pageSize`; the server response is only the data.
        setContainers(data.items);
        setTotal(data.total);
        setContainersLoaded(true);
      })
      .catch(() => {
        if (!cancelled) setContainersLoaded(true);
      });
    return () => {
      cancelled = true;
    };
  }, [authenticated, queryString]);

  // Filter and page-size changes reset the list to the first page, so the
  // query never points at a page that may no longer exist.
  const handleStateFilterChange = useCallback((filter: string[]) => {
    setStateFilter(filter);
    setPage(1);
  }, []);

  const handleShowPendingUpdatesChange = useCallback((value: boolean) => {
    setShowPendingUpdates(value);
    setPage(1);
  }, []);

  const handlePageSizeChange = useCallback((size: number) => {
    setPageSize(size);
    setPage(1);
  }, []);

  // ── Cached data for instant tab switching ────────────────
  const [config, setConfig] = useState<AppConfig | null>(null);
  // Señal de refresco del histórico (D3): la página es dueña de su estado y de su
  // consulta; `App` sólo indica cuándo debe recargar (al terminar un batch).
  const [historyRefresh, setHistoryRefresh] = useState(0);
  const api = useCallback(async (path: string) => {
    try {
      return await (await apiFetch(path)).json();
    } catch {
      return null;
    }
  }, []);
  useEffect(() => {
    if (!authenticated) return;
    api("/api/config").then((d) => {
      if (d) setConfig(d);
    });
  }, [authenticated, api]);

  // ── Batch check/update state (lives in App to survive tab switches) ──
  type CheckAllPhase = "idle" | "active";

  interface CheckAllResults {
    total: number;
    updated: number;
    uptodate: number;
    failed: number;
    done: number;
    errors: string[];
  }

  const [batchPhase, setBatchPhase] = useState<CheckAllPhase>("idle");
  const batchPhaseRef = useRef(batchPhase);
  useEffect(() => {
    batchPhaseRef.current = batchPhase;
  }, [batchPhase]);

  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 });
  const abortControllerRef = useRef<AbortController | null>(null);
  const [checkResults, setCheckResults] = useState<CheckAllResults>({
    total: 0,
    updated: 0,
    uptodate: 0,
    failed: 0,
    done: 0,
    errors: [],
  });
  const [updateResults, setUpdateResults] = useState<CheckAllResults>({
    total: 0,
    updated: 0,
    uptodate: 0,
    done: 0,
    failed: 0,
    errors: [],
  });
  const [showSummary, setShowSummary] = useState(false);
  const [checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(
    null,
  );

  // ── Realtime transport (SSE `/api/stream`, long-poll fallback) ────
  const finishBatch = useCallback(
    (bp: BatchProgressType) => {
      if (batchPhaseRef.current !== "active") return;
      batchPhaseRef.current = "idle";
      setBatchPhase("idle");
      setShowSummary(true);
      const results = mapBatchToResults(bp);
      setCheckResults(results.check);
      setUpdateResults(results.update);
      // Al terminar un batch, señaliza al histórico para que recargue su página.
      setHistoryRefresh((n) => n + 1);
      api("/api/config").then((d) => {
        if (d) setConfig(d);
      });
      const notifMethod = bp.errors > 0 ? "warning" : "success";
      notification[notifMethod]({
        message: "✅ Batch completado",
        description: `${bp.checked} containers · ${bp.updated} ok · ${bp.errors} errores`,
        duration: 8,
      });
    },
    [api],
  );

  const onState = useCallback((state: StateResponse) => {
    setContainers(state.containers);
    setContainersLoaded(true);
    setTotal(state.total);
    setSummary(state.summary);
    setProgress(state.progress);
    setBatchProgress({
      current: state.progress.checked,
      total: state.progress.total,
    });
  }, []);

  // Batch end is detected from the `update-progress` event, not from heuristics.
  const onProgress = useCallback(
    (p: UpdateProgress) => {
      setProgress({
        total: p.total,
        checked: p.checked,
        updated: p.updated,
        errors: p.errors,
        checking: p.container,
      });
      setBatchProgress({ current: p.checked, total: p.total });
      if (p.done) {
        finishBatch({
          total: p.total,
          checked: p.checked,
          updated: p.updated,
          errors: p.errors,
          checking: p.container,
          updatable: p.updatable ?? 0,
          uptodate: p.uptodate ?? 0,
          failed: p.failed ?? 0,
        });
      }
    },
    [finishBatch],
  );

  // ── UI notifications from `notification` events ───────────────────
  interface UiNotif extends NotifEvent {
    _id: string;
  }
  const [notifs, setNotifs] = useState<UiNotif[]>([]);
  const notifTimersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>(
    {},
  );

  const dismissNotif = useCallback((id: string) => {
    setNotifs((prev) => prev.filter((n) => n._id !== id));
    const timer = notifTimersRef.current[id];
    if (timer) {
      clearTimeout(timer);
      delete notifTimersRef.current[id];
    }
  }, []);

  const onNotification = useCallback(
    (n: NotifEvent) => {
      const id = `${n.container}-${n.timestamp}-${Date.now()}-${Math.random()
        .toString(36)
        .slice(2, 7)}`;
      setNotifs((prev) => [...prev, { ...n, _id: id }].slice(-3));
      notifTimersRef.current[id] = setTimeout(() => dismissNotif(id), 8000);
    },
    [dismissNotif],
  );

  // ── Container highlights from `container-event` events ────────────
  const [highlights, setHighlights] = useState<Record<string, boolean>>({});
  const highlightTimersRef = useRef<
    Record<string, ReturnType<typeof setTimeout>>
  >({});

  const onContainerEvent = useCallback((ev: ContainerEvent) => {
    setHighlights((prev) => ({ ...prev, [ev.name]: true }));
    const existing = highlightTimersRef.current[ev.name];
    if (existing) clearTimeout(existing);
    highlightTimersRef.current[ev.name] = setTimeout(() => {
      setHighlights((prev) => {
        const next = { ...prev };
        delete next[ev.name];
        return next;
      });
      delete highlightTimersRef.current[ev.name];
    }, 2000);
  }, []);

  // Clear pending timers on unmount (avoid state updates after teardown).
  useEffect(
    () => () => {
      Object.values(notifTimersRef.current).forEach(clearTimeout);
      Object.values(highlightTimersRef.current).forEach(clearTimeout);
    },
    [],
  );

  useRealtime({
    query: queryString,
    onState,
    onProgress,
    onNotification,
    onContainerEvent,
    onError: useCallback(() => {
      console.error("[REALTIME] transport down");
    }, []),
  });

  // Fetch update check config on mount (for last/next check times)
  const fetchCheckConfig = useCallback(async () => {
    try {
      const res = await apiFetch("/api/update-check/config");
      if (res.ok) {
        setCheckConfig(await res.json());
      }
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    if (!authenticated) return;
    // `fetchCheckConfig` es async: su `setState` corre después del `await`,
    // nunca de forma síncrona dentro del efecto. Falso positivo de
    // react/set-state-in-effect.
    // oxlint-disable-next-line react/set-state-in-effect
    fetchCheckConfig();
  }, [authenticated, fetchCheckConfig]);

  // checkAll: POST /api/check-all
  const checkAll = useCallback(async () => {
    batchPhaseRef.current = "active";
    setBatchPhase("active");
    setCheckResults({
      total: 0,
      updated: 0,
      uptodate: 0,
      failed: 0,
      done: 0,
      errors: [],
    });
    setUpdateResults({
      total: 0,
      updated: 0,
      uptodate: 0,
      done: 0,
      failed: 0,
      errors: [],
    });
    setBatchProgress({ current: 0, total: summary.total });
    setShowSummary(false);
    const controller = new AbortController();
    abortControllerRef.current = controller;
    try {
      const res = await apiFetch("/api/check-all", {
        method: "POST",
        signal: controller.signal,
      });
      if (res.ok) {
        const updated: ContainerInfo[] = await res.json();
        setContainers((prev) =>
          prev.map((c) => updated.find((u) => u.name === c.name) || c),
        );
      }
    } catch (err: any) {
      if (err.name === "AbortError") {
        // Cancelled by user — ignore
        return;
      }
    }
    fetchCheckConfig();
  }, [summary.total, fetchCheckConfig]);

  // Cancel the batch operation
  const cancelBatch = useCallback(async () => {
    // Abort the in-flight fetch
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    // Notify backend
    try {
      await apiFetch("/api/check-all/cancel", { method: "POST" });
    } catch {
      // Ignore if endpoint doesn't exist yet
    }
    // Reset state
    batchPhaseRef.current = "idle";
    setBatchPhase("idle");
    setBatchProgress({ current: 0, total: 0 });
  }, []);

  const logout = () => {
    window.location.href = "/api/auth/logout";
  };

  const [view, setView] = useState<"dashboard" | "history" | "config">(
    "dashboard",
  );

  if (checking) return null;
  if (!authenticated) return <LoginScreen />;

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Layout>
        <Layout.Header
          style={{
            height: 60,
            padding: "0 16px",
            display: "flex",
            alignItems: "center",
            background: "var(--ant-container-bg)",
            borderBottom: "1px solid var(--ant-color-border)",
          }}
        >
          <Flex
            justify="space-between"
            align="center"
            style={{ width: "100%", maxWidth: 1200, margin: "0 auto" }}
          >
            {/* Left: logo + name */}
            <Flex align="center" gap="small">
              <img src="/icon-48x48.png" width="28" height="28" alt="Alloy" />
              <Title level={4} style={{ margin: 0, whiteSpace: "nowrap" }}>
                Alloy
              </Title>
              {user && !isMobile && (
                <Text type="secondary" style={{ marginLeft: 8 }}>
                  {user.name}
                </Text>
              )}
            </Flex>

            {/* Right: navigation buttons */}
            <Space size={isMobile ? 4 : 8}>
              <Button
                type={view === "dashboard" ? "primary" : "text"}
                icon={<BarChartOutlined />}
                onClick={() => setView("dashboard")}
              >
                {!isMobile && "Dashboard"}
              </Button>
              <Button
                type={view === "history" ? "primary" : "text"}
                icon={<FileTextOutlined />}
                onClick={() => setView("history")}
              >
                {!isMobile && "Historial"}
              </Button>
              <Button
                type={view === "config" ? "primary" : "text"}
                icon={<SettingOutlined />}
                onClick={() => setView("config")}
              >
                {!isMobile && "Config"}
              </Button>
              <Button
                type="text"
                icon={<LogoutOutlined />}
                onClick={logout}
                danger
              >
                {!isMobile && "Salir"}
              </Button>
            </Space>
          </Flex>
        </Layout.Header>
        <Layout.Content style={{ padding: 16 }}>
          <div style={{ maxWidth: 1200, margin: "0 auto" }}>
            <BatchProgress
              phase={batchPhase}
              batchProgress={batchProgress}
              progress={progress}
              onCancel={cancelBatch}
            />

            {view === "dashboard" && (
              <DashboardPage
                containers={containers}
                progress={progress}
                containersLoaded={containersLoaded}
                batchPhase={batchPhase}
                checkResults={checkResults}
                updateResults={updateResults}
                showSummary={showSummary}
                setShowSummary={setShowSummary}
                checkConfig={checkConfig}
                onCheckAll={checkAll}
                summary={summary}
                highlights={highlights}
                searchQuery={searchQuery}
                debouncedSearch={debouncedQuery}
                setSearchQuery={setSearchQuery}
                stateFilter={stateFilter}
                setStateFilter={handleStateFilterChange}
                showPendingUpdates={showPendingUpdates}
                setShowPendingUpdates={handleShowPendingUpdatesChange}
                total={total}
                page={page}
                pageSize={pageSize}
                onPageChange={setPage}
                onPageSizeChange={handlePageSizeChange}
              />
            )}
            {view === "history" && (
              <HistoryPage refreshToken={historyRefresh} />
            )}
            {view === "config" && (
              <ConfigPage
                config={config}
                setConfig={setConfig}
                colorScheme={colorScheme}
                setColorScheme={setColorScheme}
              />
            )}

            <SummaryDialog
              opened={showSummary}
              onClose={() => setShowSummary(false)}
              checkResults={checkResults}
              updateResults={updateResults}
              phase={batchPhase}
            />
          </div>
        </Layout.Content>
      </Layout>

      {/* Realtime notification toasts (from `notification` SSE events) */}
      {notifs.length > 0 && (
        <Flex
          vertical
          gap={4}
          style={{
            position: "fixed",
            top: 72,
            right: 16,
            zIndex: 1100,
            width: "min(320px, calc(100vw - 32px))",
            pointerEvents: "none",
          }}
        >
          {notifs.map((n) => (
            <div key={n._id} style={{ pointerEvents: "auto" }}>
              <NotifToast notif={n} onDismiss={() => dismissNotif(n._id)} />
            </div>
          ))}
        </Flex>
      )}
    </Layout>
  );
}
