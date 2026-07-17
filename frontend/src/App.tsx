import { useEffect, useState, useCallback } from "react";
import { useMediaQuery } from "@mantine/hooks";
import { ActionIcon, AppShell, Badge, Button, Container, Group, Stack, Title, Tooltip, Tabs } from "@mantine/core";
import type { ContainerInfo, UpdateProgress, NotifEvent, HistoryEntry, AppConfig, UpdatePolicy, UpdateCheckConfig } from "./types";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
import NotifToast from "./components/NotifToast";
import HistoryPage from "./HistoryPage";
import CheckConfigPage from "./CheckConfigPage";

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
  const [policies, setPolicies] = useState<UpdatePolicy[]>([]);
  const [checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const api = useCallback(async (path: string) => {
    try { return await (await fetch(path, { credentials: "include" })).json(); }
    catch { return null; }
  }, []);
  useEffect(() => {
    if (!authenticated) return;
    api("/api/history").then((d) => { if (d) setHistory(d); });
    api("/api/update-policies").then((d) => { if (d) setPolicies(d); });
    api("/api/update-check/config").then((d) => { if (d) setCheckConfig(d); });
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
        {isMobile ? (
          <Stack mb="md">
            <Group justify="space-between">
              <Title order={2}>
                <img src="/icon-48x48.png" width="28" height="28" style={{ verticalAlign: 'middle', marginRight: 8 }} alt="Alloy" />
                Alloy
              </Title>
              <Group gap="xs">
                <Tooltip label={colorScheme === "dark" ? "☀️ Modo claro" : "🌙 Modo oscuro"}>
                  <ActionIcon variant="outline" color="gray" onClick={toggleColorScheme} size="lg" aria-label="Toggle color scheme">
                    {colorScheme === "dark" ? "☀️" : "🌙"}
                  </ActionIcon>
                </Tooltip>
                <Button size="xs" variant="outline" color="gray" onClick={logout}>🚪</Button>
              </Group>
            </Group>
            <Group gap="xs" wrap="wrap">
              {user && <Badge size="sm" variant="light" color="gray">{user.name}</Badge>}
              <Badge size="sm" variant="light" color="blue">{containers.length} containers</Badge>
            </Group>
          </Stack>
        ) : (
          <Group justify="space-between" mb="lg">
            <Title order={2}>
              <img src="/icon-48x48.png" width="28" height="28" style={{ verticalAlign: 'middle', marginRight: 8 }} alt="Alloy" />
              Alloy
            </Title>
            <Group>
              {user && <Badge size="lg" variant="light" color="gray">{user.name}</Badge>}
              <Badge size="lg" variant="light" color="blue">{containers.length} containers</Badge>
              <Tooltip label={colorScheme === "dark" ? "☀️ Modo claro" : "🌙 Modo oscuro"}>
                <ActionIcon variant="outline" color="gray" onClick={toggleColorScheme} size="lg" aria-label="Toggle color scheme">
                  {colorScheme === "dark" ? "☀️" : "🌙"}
                </ActionIcon>
              </Tooltip>
              <Button size="xs" variant="outline" color="gray" onClick={logout}>🚪 Salir</Button>
            </Group>
          </Group>
        )}

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

        <Tabs defaultValue="dashboard">
          <Tabs.List mb="md" style={{ overflowX: 'auto', flexWrap: 'nowrap', scrollbarWidth: 'none' }}>
            <Tabs.Tab value="dashboard" style={{ whiteSpace: 'nowrap' }}>📊 Dashboard</Tabs.Tab>
            <Tabs.Tab value="history" style={{ whiteSpace: 'nowrap' }}>📜 Historial</Tabs.Tab>
            <Tabs.Tab value="updates" style={{ whiteSpace: 'nowrap' }}>🔄 Revisiones</Tabs.Tab>
            <Tabs.Tab value="config" style={{ whiteSpace: 'nowrap' }}>⚙️ Config</Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="dashboard">
            <DashboardPage
              containers={containers}
              setContainers={setContainers}
              progress={progress}
              notifications={notifications}
              setNotifications={setNotifications}
              containersLoaded={containersLoaded}
              config={config}
            />
          </Tabs.Panel>
          <Tabs.Panel value="history">
            <HistoryPage history={history} setHistory={setHistory} />
          </Tabs.Panel>
          <Tabs.Panel value="updates">
            <CheckConfigPage
              containers={containers}
              policies={policies}
              setPolicies={setPolicies}
              config={checkConfig}
              setConfig={setCheckConfig}
            />
          </Tabs.Panel>
          <Tabs.Panel value="config">
            <ConfigPage config={config} setConfig={setConfig} />
          </Tabs.Panel>
        </Tabs>
      </Container>
    </AppShell>
  );
}