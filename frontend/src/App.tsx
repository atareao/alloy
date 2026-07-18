import { useEffect, useState, useCallback } from "react";
import { useMediaQuery } from "@mantine/hooks";
import { ActionIcon, AppShell, Badge, Button, Container, Group, Stack, Title, Tooltip } from "@mantine/core";
import type { ContainerInfo, UpdateProgress, NotifEvent, HistoryEntry, AppConfig } from "./types";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
import NotifToast from "./components/NotifToast";
import HistoryPage from "./HistoryPage";

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
  const [notifications, setNotifications] = useState<NotifEvent[]>([]);
  const [progress, setProgress] = useState<Map<string, UpdateProgress>>(new Map());
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

  // Initial eager fetch of containers — avoids waiting for first SSE event
  useEffect(() => {
    if (!authenticated) return;
    fetch("/api/containers", { credentials: "include" })
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
    try { return await (await fetch(path, { credentials: "include" })).json(); }
    catch { return null; }
  }, []);
  useEffect(() => {
    if (!authenticated) return;
    api("/api/history").then((d) => { if (d) setHistory(d); });
    api("/api/config").then((d) => { if (d) setConfig(d); });
  }, [authenticated, api]);

  // Connect to container events SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const evtSource = new EventSource("/api/events", { withCredentials: true });
    evtSource.addEventListener("containers", (e) => {
      setContainers(JSON.parse(e.data).containers);
      setContainersLoaded(true);
    });
    evtSource.onerror = () => setContainersLoaded(true);
    return () => evtSource.close();
  }, [authenticated]);

  // Connect to notifications SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const notifSource = new EventSource("/api/notifications", { withCredentials: true });
    notifSource.addEventListener("notification", (e) => {
      try {
        const notif: NotifEvent = JSON.parse(e.data);
        setNotifications((prev) => [notif, ...prev].slice(0, 50));
      } catch { /* ignore malformed */ }
    });
    return () => notifSource.close();
  }, [authenticated]);

  // Connect to update progress SSE — lives in App so state persists across tab switches
  useEffect(() => {
    if (!authenticated) return;
    const evtSource = new EventSource("/api/updates", { withCredentials: true });
    evtSource.addEventListener("update-progress", (e) => {
      try {
        const data: UpdateProgress = JSON.parse(e.data);
        setProgress((prev) => {
          const next = new Map(prev);
          next.set(data.container, data);
          return next;
        });
        if (data.done) {
          setTimeout(() => setProgress((prev) => { const n = new Map(prev); n.delete(data.container); return n; }), 3000);
        }
      } catch { /* ignore malformed */ }
    });
    return () => evtSource.close();
  }, [authenticated]);

  const dismissNotif = (index: number) => {
    setNotifications((prev) => prev.filter((_, i) => i !== index));
  };

  const logout = () => {
    window.location.href = "/api/auth/logout";
  };

  const [view, setView] = useState<"dashboard" | "history" | "config">("dashboard");

  const toggleColorScheme = () => {
    const next = colorScheme === "dark" ? "light" : "dark";
    localStorage.setItem("color-scheme", next);
    setColorScheme(next);
  };

  if (checking) return null;
  if (!authenticated) return <LoginScreen />;

  return (
    <AppShell padding="md">
      <Container size="lg" py="md">
        <Stack mb="lg" gap="xs">
          <Group justify="space-between" wrap="nowrap">
            <Group gap="md" style={{ flex: 1 }}>
              <Title order={2} style={{ whiteSpace: 'nowrap' }}>
                <img src="/icon-48x48.png" width="28" height="28" style={{ verticalAlign: 'middle', marginRight: 8 }} alt="Alloy" />
                Alloy
              </Title>
              <Group gap="xs" style={{ flex: 1 }} justify="center">
                <Button
                  size={isMobile ? "xs" : "sm"}
                  variant={view === "dashboard" ? "filled" : "light"}
                  color={view === "dashboard" ? "blue" : "gray"}
                  onClick={() => setView("dashboard")}
                >
                  📊 Dashboard
                </Button>
                <Button
                  size={isMobile ? "xs" : "sm"}
                  variant={view === "history" ? "filled" : "light"}
                  color={view === "history" ? "blue" : "gray"}
                  onClick={() => setView("history")}
                >
                  📜 Historial
                </Button>
                <Button
                  size={isMobile ? "xs" : "sm"}
                  variant={view === "config" ? "filled" : "light"}
                  color={view === "config" ? "blue" : "gray"}
                  onClick={() => setView("config")}
                >
                  ⚙️ Config
                </Button>
              </Group>
            </Group>
            <Group gap="xs" wrap="nowrap">
              {user && <Badge size={isMobile ? "sm" : "lg"} variant="light" color="gray">{user.name}</Badge>}
              <Tooltip label={colorScheme === "dark" ? "☀️ Modo claro" : "🌙 Modo oscuro"}>
                <ActionIcon variant="outline" color="gray" onClick={toggleColorScheme} size="lg" aria-label="Toggle color scheme">
                  {colorScheme === "dark" ? "☀️" : "🌙"}
                </ActionIcon>
              </Tooltip>
              <Button
                size={isMobile ? "xs" : "sm"}
                variant="outline"
                color="gray"
                onClick={logout}
              >
                {isMobile ? "🚪" : "🚪 Salir"}
              </Button>
            </Group>
          </Group>
          </Stack>

        {/* Notification toasts */}
        {notifications.length > 0 && (
          <div style={{ position: 'fixed', top: 16, right: 16, zIndex: 1000, maxWidth: 400, width: '100%' }}>
            {notifications.map((notif, i) => (
              <NotifToast key={`${notif.container}-${notif.timestamp}-${i}`} notif={notif} onDismiss={() => dismissNotif(i)} />
            ))}
            {notifications.length > 3 && (
              <div style={{ textAlign: 'center', marginTop: 4 }}>
                <button onClick={() => setNotifications([])} style={{ background: 'rgba(0,0,0,0.7)', color: '#aaa', border: '1px solid #333', borderRadius: 4, padding: '2px 12px', fontSize: 12, cursor: 'pointer' }}>
                  Limpiar todas ({notifications.length})
                </button>
              </div>
            )}
          </div>
        )}

        {view === "dashboard" && (
          <DashboardPage
            containers={containers}
            setContainers={setContainers}
            progress={progress}
            notifications={notifications}
            setNotifications={setNotifications}
            containersLoaded={containersLoaded}
            config={config}
          />
        )}
        {view === "history" && (
          <HistoryPage history={history} setHistory={setHistory} />
        )}
        {view === "config" && (
          <ConfigPage config={config} setConfig={setConfig} />
        )}
      </Container>
    </AppShell>
  );
}