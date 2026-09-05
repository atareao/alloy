import { useEffect, useState, useCallback, useRef } from "react";
import { useMediaQuery } from "@mantine/hooks";
import { showNotification } from "@mantine/notifications";
import {
  AppShell,
  Button,
  Container,
  Group,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import type {
  ContainerInfo,
  UpdateProgress,
  NotifEvent,
  HistoryEntry,
  AppConfig,
  UpdateCheckConfig,
} from "./types";
import { apiFetch } from "./api";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
import HistoryPage from "./HistoryPage";
import BatchProgress from "./components/BatchProgress";
import SummaryDialog from "./components/SummaryDialog";

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
  const [progress, setProgress] = useState<Map<string, UpdateProgress>>(
    new Map(),
  );
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
    const interval = setInterval(async () => {
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
    }, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [authenticated]);

  // Initial eager fetch of containers — avoids waiting for first SSE event
  useEffect(() => {
    if (!authenticated) return;
    apiFetch("/api/containers")
      .then((res) => res.json())
      .then((data) => {
        setContainers(data);
        setContainersLoaded(true);
      })
      .catch(() => setContainersLoaded(true));
  }, [authenticated]);

  // ── Cached data for instant tab switching ────────────────
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const api = useCallback(async (path: string) => {
    try {
      return await (await apiFetch(path)).json();
    } catch {
      return null;
    }
  }, []);
  useEffect(() => {
    if (!authenticated) return;
    api("/api/history").then((d) => {
      if (d) setHistory(d);
    });
    api("/api/config").then((d) => {
      if (d) setConfig(d);
    });
  }, [authenticated, api]);

  // Connect to container events SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const evtSource = new EventSource("/api/events", { withCredentials: true });
    evtSource.addEventListener("containers", (e) => {
      const incoming: ContainerInfo[] = JSON.parse(e.data).containers;
      setContainers(incoming);
      setContainersLoaded(true);
    });
    evtSource.onerror = () => {
      // SSE onerror fires for transient errors too (timeout, reconnect, etc.)
      // The browser will auto-reconnect. Only redirect if we detect session expiry.
      // Check by making a lightweight fetch to /api/auth/me
      fetch("/api/auth/me", { credentials: "include" }).then((res) => {
        if (res.status === 401) {
          window.location.href = "/api/auth/login";
        }
      }).catch(() => {
        // Network error — ignore, SSE will reconnect
      });
    };
    return () => evtSource.close();
  }, [authenticated]);

  // Connect to notifications SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const notifSource = new EventSource("/api/notifications", {
      withCredentials: true,
    });
    notifSource.addEventListener("notification", (e) => {
      try {
        const notif: NotifEvent = JSON.parse(e.data);
        showNotification({
          title: notif.container,
          message: notif.status,
          color: "blue",
          autoClose: 5000,
        });
      } catch (err) {
        console.error("SSE update-progress parse error:", err, "raw:", e.data);
      }
    });
    notifSource.onerror = () => {
      // SSE onerror fires for transient errors too (timeout, reconnect, etc.)
      // The browser will auto-reconnect. Only redirect if we detect session expiry.
      fetch("/api/auth/me", { credentials: "include" }).then((res) => {
        if (res.status === 401) {
          window.location.href = "/api/auth/login";
        }
      }).catch(() => {
        // Network error — ignore, SSE will reconnect
      });
    };
    return () => notifSource.close();
  }, [authenticated]);

  // Connect to update progress SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const evtSource = new EventSource("/api/updates", {
      withCredentials: true,
    });
    evtSource.addEventListener("update-progress", (e) => {
      try {
        const data: UpdateProgress = JSON.parse(e.data);
        if (typeof console !== "undefined") {
          console.log("SSE update-progress:", data);
        }
        setProgress((prev) => {
          const next = new Map(prev);
          next.set(data.container, data);
          return next;
        });
        if (data.done) {
          // Re-fetch history so the new entry appears immediately
          api("/api/history").then((d) => {
            if (d) setHistory(d);
          });
          // Also re-fetch config to pick up any changes
          api("/api/config").then((d) => {
            if (d) setConfig(d);
          });
        }
      } catch (err) {
        console.error("SSE update-progress parse error:", err, "raw:", e.data);
      }
    });
    evtSource.onerror = () => {
      // SSE onerror fires for transient errors too (timeout, reconnect, etc.)
      // The browser will auto-reconnect. Only redirect if we detect session expiry.
      fetch("/api/auth/me", { credentials: "include" }).then((res) => {
        if (res.status === 401) {
          window.location.href = "/api/auth/login";
        }
      }).catch(() => {
        // Network error — ignore, SSE will reconnect
      });
    };
    return () => evtSource.close();
  }, [authenticated, api]);

  const clearProgress = useCallback(() => {
    setProgress(new Map());
  }, []);

  // ── Batch check/update state (lives in App to survive tab switches) ──
  type CheckAllPhase = "idle" | "checking" | "updating";

  interface CheckAllResults {
    total: number;
    updated: number;
    uptodate: number;
    failed: number;
    done: number;
    errors: string[];
  }

  const [batchPhase, setBatchPhase] = useState<CheckAllPhase>("idle");
  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 });
  const [batchCurrentItem, setBatchCurrentItem] = useState("");
  const cancelBatchRef = useRef(false);
  const pendingTotalRef = useRef(0);
  const [checkResults, setCheckResults] = useState<CheckAllResults>({
    total: 0, updated: 0, uptodate: 0, failed: 0, done: 0, errors: [],
  });
  const [updateResults, setUpdateResults] = useState<CheckAllResults>({
    total: 0, updated: 0, uptodate: 0, done: 0, failed: 0, errors: [],
  });
  const [showSummary, setShowSummary] = useState(false);
  const [checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(null);

  // Fetch update check config on mount (for last/next check times)
  const fetchCheckConfig = useCallback(async () => {
    try {
      const res = await apiFetch("/api/update-check/config");
      if (res.ok) {
        setCheckConfig(await res.json());
      }
    } catch {/* ignore */}
  }, []);

  useEffect(() => {
    if (!authenticated) return;
    fetchCheckConfig();
  }, [authenticated, fetchCheckConfig]);

  // Detect in-progress updates on reconnect (e.g. after logout/login)
  // If SSE delivers progress entries and batchPhase is idle, infer a batch is running
  useEffect(() => {
    if (batchPhase !== "idle") return;
    let hasActive = false;
    progress.forEach((p) => {
      if (!p.done) hasActive = true;
    });
    if (hasActive) {
      setBatchPhase("updating");
      // Don't set pendingTotalRef — we don't know the total yet.
      // The completion check below falls back to "all progress entries done".
      setBatchProgress({ current: 0, total: 0 });
      setBatchCurrentItem("⬆️ Retomando...");
    }
  }, [progress, batchPhase]);

  // Monitor progress map to advance batch progress bar and update live counters
  useEffect(() => {
    if (batchPhase === "idle") return;
    let doneCount = 0;
    let currentItem = "";
    let liveDone = 0;
    let liveFailed = 0;
    progress.forEach((p) => {
      if (batchPhase === "updating") {
        const isUpdate =
          p.status.startsWith("🔄") ||
          p.status.startsWith("✅ actualizado") ||
          p.status.startsWith("✅ pulled") ||
          p.status.startsWith("✅ stack") ||
          p.status.startsWith("❌") ||
          p.status.startsWith("⚠️") ||
          p.status.startsWith("📥") ||
          p.status.startsWith("✅ Updated");
        if (!isUpdate) return;
      }
      if (p.done) {
        doneCount++;
        if (p.error || p.status.startsWith("❌") || p.status.startsWith("⚠️")) {
          liveFailed++;
        } else {
          liveDone++;
        }
      } else if (currentItem === "") currentItem = p.container;
    });
    setBatchProgress((prev) =>
      doneCount !== prev.current ? { ...prev, current: doneCount } : prev,
    );
    // Update live counters for the updating phase
    if (batchPhase === "updating") {
      setUpdateResults((prev) => {
        if (prev.done !== liveDone || prev.failed !== liveFailed) {
          return { ...prev, done: liveDone, failed: liveFailed };
        }
        return prev;
      });
    }
    if (currentItem) setBatchCurrentItem(currentItem);
    if (
      batchPhase === "updating" &&
      doneCount > 0 &&
      (doneCount >= pendingTotalRef.current ||
        // Recovery mode: no pendingTotalRef set — complete when all progress entries are done
        (pendingTotalRef.current === 0 &&
          progress.size > 0 &&
          Array.from(progress.values()).every((p) => p.done)))
    ) {
      setTimeout(() => {
        setBatchPhase("idle");
        setShowSummary(true);
        // Count results from live progress map
        let done = 0, failed = 0;
        progress.forEach((p) => {
          if (p.done) {
            if (p.error || p.status.startsWith("❌") || p.status.startsWith("⚠️")) {
              failed++;
            } else {
              done++;
            }
          }
        });
        const total = done + failed;
        if (total > 0) {
          showNotification({
            title: "✅ Batch completado",
            message: failed > 0
              ? `${total} containers · ${done} ok · ${failed} errores`
              : `${total} containers actualizados correctamente`,
            color: failed > 0 ? "yellow" : "green",
            autoClose: 8000,
          });
        }
      }, 1500);
    }
  }, [progress, batchPhase]);

  // checkAll: POST /api/check-all, then auto-update containers with pending updates
  const checkAll = useCallback(async () => {
    cancelBatchRef.current = false;
    clearProgress();
    setBatchPhase("checking");
    setCheckResults({ total: 0, updated: 0, uptodate: 0, failed: 0, done: 0, errors: [] });
    setUpdateResults({ total: 0, updated: 0, uptodate: 0, done: 0, failed: 0, errors: [] });
    setBatchProgress({ current: 0, total: containers.length });
    setBatchCurrentItem("🔍 Verificando...");
    setShowSummary(false);
    let updatedCount = 0;
    let uptodateCount = 0;
    let failedCount = 0;
    const errors: string[] = [];
    try {
      const res = await apiFetch("/api/check-all", { method: "POST" });
      if (res.ok) {
        const updated: ContainerInfo[] = await res.json();
        setContainers((prev) =>
          prev.map((c) => updated.find((u) => u.name === c.name) || c),
        );
        updatedCount = updated.filter((c) => c.has_update).length;
        uptodateCount = updated.filter((c) => !c.has_update).length;
      } else {
        failedCount = containers.length;
        errors.push(`HTTP ${res.status}`);
      }
    } catch (e: any) {
      failedCount = containers.length;
      errors.push(`${e.message || "unknown error"}`);
    }
    setCheckResults({ total: containers.length, updated: updatedCount, uptodate: uptodateCount, failed: failedCount, done: 0, errors });
    setBatchProgress({ current: containers.length, total: containers.length });
    setBatchCurrentItem("");
    fetchCheckConfig();
    if (updatedCount > 0) {
      setBatchPhase("updating");
      pendingTotalRef.current = updatedCount;
      setBatchProgress({ current: 0, total: updatedCount });
      setBatchCurrentItem("⬆️ Aplicando políticas...");
    } else {
      setTimeout(() => {
        setBatchPhase("idle");
        setShowSummary(true);
      }, 500);
    }
  }, [containers, clearProgress, setContainers, fetchCheckConfig]);

  const logout = () => {
    window.location.href = "/api/auth/logout";
  };

  const [view, setView] = useState<"dashboard" | "history" | "config">(
    "dashboard",
  );

  if (checking) return null;
  if (!authenticated) return <LoginScreen />;

  return (
    <AppShell padding="md">
      <Container size="lg" py="md">
        <Stack mb="lg" gap="xs">
          <Group justify="space-between" wrap="nowrap">
            <Group gap="md" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
              <div style={{ flexShrink: 0 }}>
                <Title order={2} style={{ whiteSpace: "nowrap" }}>
                  <img
                    src="/icon-48x48.png"
                    width="28"
                    height="28"
                    style={{ verticalAlign: "middle", marginRight: 8 }}
                    alt="Alloy"
                  />
                  Alloy
                </Title>
                {user && (
                  <Text size="sm" c="dimmed" ml={36}>
                    {user.name}
                  </Text>
                )}
              </div>
              <Group gap={isMobile ? 4 : "xs"} wrap="nowrap" style={{ flex: 1 }} justify="center">
                <Button
                  size="sm"
                  variant={view === "dashboard" ? "filled" : "light"}
                  color={view === "dashboard" ? "blue" : "gray"}
                  onClick={() => setView("dashboard")}
                >
                  {isMobile ? "📊" : "📊 Dashboard"}
                </Button>
                <Button
                  size="sm"
                  variant={view === "history" ? "filled" : "light"}
                  color={view === "history" ? "blue" : "gray"}
                  onClick={() => setView("history")}
                >
                  {isMobile ? "📜" : "📜 Historial"}
                </Button>
                <Button
                  size="sm"
                  variant={view === "config" ? "filled" : "light"}
                  color={view === "config" ? "blue" : "gray"}
                  onClick={() => setView("config")}
                >
                  {isMobile ? "⚙️" : "⚙️ Config"}
                </Button>
                <Button
                  size="sm"
                  variant="light"
                  color="gray"
                  onClick={logout}
                >
                  {isMobile ? "🚪" : "🚪 Salir"}
                </Button>
              </Group>
            </Group>
          </Group>
        </Stack>

        <BatchProgress
          phase={batchPhase}
          batchProgress={batchProgress}
          batchCurrentItem={batchCurrentItem}
          checkResults={checkResults}
          updateResults={updateResults}
          progress={progress}
          onCancel={() => { cancelBatchRef.current = true; }}
        />

        {view === "dashboard" && (
          <DashboardPage
            containers={containers}
            setContainers={setContainers}
            progress={progress}
            containersLoaded={containersLoaded}
            batchPhase={batchPhase}
            checkResults={checkResults}
            updateResults={updateResults}
            showSummary={showSummary}
            setShowSummary={setShowSummary}
            checkConfig={checkConfig}
            onCheckAll={checkAll}
          />
        )}
        {view === "history" && (
          <HistoryPage history={history} setHistory={setHistory} />
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
      </Container>
    </AppShell>
  );
}
