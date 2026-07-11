import { useEffect, useState } from "react";
import { ActionIcon, AppShell, Badge, Button, Container, Group, Title, Tooltip, Tabs } from "@mantine/core";
import type { ContainerInfo } from "./types";
import LoginScreen from "./components/LoginScreen";
import DashboardPage from "./components/DashboardPage";
import ConfigPage from "./components/ConfigPage";
import StacksPage from "./StacksPage";
import HistoryPage from "./HistoryPage";
import AlertsPage from "./AlertsPage";
import SchedulePage from "./SchedulePage";

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
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  const [user, setUser] = useState<UserInfo | null>(null);
  const [containers, setContainers] = useState<ContainerInfo[]>([]);
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

  // Connect to SSE only when authenticated
  useEffect(() => {
    if (!authenticated) return;
    const evtSource = new EventSource("/api/events", { withCredentials: true });
    evtSource.addEventListener("containers", (e) =>
      setContainers(JSON.parse(e.data).containers),
    );
    return () => evtSource.close();
  }, [authenticated]);

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
        <Group justify="space-between" mb="lg">
          <Title order={2}>
            <img src="/favicon.svg" width="28" height="28" style={{ verticalAlign: 'middle', marginRight: 8 }} alt="Alloy" />
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

        <Tabs defaultValue="dashboard">
          <Tabs.List mb="md">
            <Tabs.Tab value="dashboard">📊 Dashboard</Tabs.Tab>
            <Tabs.Tab value="stacks">📦 Stacks</Tabs.Tab>
            <Tabs.Tab value="history">📜 Historial</Tabs.Tab>
            <Tabs.Tab value="alerts">🔔 Alertas</Tabs.Tab>
            <Tabs.Tab value="schedule">⏰ Planif</Tabs.Tab>
            <Tabs.Tab value="config">⚙️ Config</Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="dashboard"><DashboardPage /></Tabs.Panel>
          <Tabs.Panel value="stacks"><StacksPage /></Tabs.Panel>
          <Tabs.Panel value="history"><HistoryPage /></Tabs.Panel>
          <Tabs.Panel value="alerts"><AlertsPage containers={containers} /></Tabs.Panel>
          <Tabs.Panel value="schedule"><SchedulePage containers={containers} /></Tabs.Panel>
          <Tabs.Panel value="config"><ConfigPage /></Tabs.Panel>
        </Tabs>
      </Container>
    </AppShell>
  );
}
