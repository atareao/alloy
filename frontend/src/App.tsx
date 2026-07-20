import { useEffect, useState, useCallback } from "react";
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
} from "./types";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
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
    try {
      return await (await fetch(path, { credentials: "include" })).json();
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
      setContainers((prev) => {
        const incoming: ContainerInfo[] = JSON.parse(e.data).containers;
        const prevHasUpdate = new Map(prev.map((c) => [c.name, c.has_update]));
        return incoming.map((c) => ({
          ...c,
          has_update: prevHasUpdate.get(c.name) ?? c.has_update,
        }));
      });
      setContainersLoaded(true);
    });
    evtSource.onerror = () => setContainersLoaded(true);
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
      } catch {
        /* ignore malformed */
      }
    });
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
          setTimeout(
            () =>
              setProgress((prev) => {
                const n = new Map(prev);
                n.delete(data.container);
                return n;
              }),
            3000,
          );
        }
      } catch {
        /* ignore malformed */
      }
    });
    return () => evtSource.close();
  }, [authenticated]);

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

        {view === "dashboard" && (
          <DashboardPage
            containers={containers}
            setContainers={setContainers}
            progress={progress}
            containersLoaded={containersLoaded}
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
      </Container>
    </AppShell>
  );
}
