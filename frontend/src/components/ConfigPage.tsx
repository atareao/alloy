import { useCallback, useEffect, useState } from "react";
import {
  Alert, Button, Group, Loader, Paper, Stack, Text, Title, TextInput, Switch,
} from "@mantine/core";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";

export default function ConfigPage() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Telegram
  const [tgToken, setTgToken] = useState("");
  const [tgChatId, setTgChatId] = useState("");
  const [tgEnabled, setTgEnabled] = useState(false);

  // Matrix
  const [mxHomeserver, setMxHomeserver] = useState("");
  const [mxToken, setMxToken] = useState("");
  const [mxRoom, setMxRoom] = useState("");
  const [mxEnabled, setMxEnabled] = useState(false);

  // Auto-update
  const [auEnabled, setAuEnabled] = useState(false);
  const [auInterval, setAuInterval] = useState(6);

  const loadConfig = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiFetch("/api/config");
      const data: AppConfig = await res.json();
      setConfig(data);
      setTgToken(data.telegram_token_set ? "********" : "");
      setTgChatId(data.telegram_chat_id || "");
      setTgEnabled(data.telegram_configured);
      setMxHomeserver(data.matrix_homeserver || "");
      setMxToken(data.matrix_token_set ? "********" : "");
      setMxRoom(data.matrix_room || "");
      setMxEnabled(data.matrix_configured);
      setAuEnabled(data.auto_update_enabled);
      setAuInterval(data.auto_update_interval_hours);
    } catch {
      setError("No se pudo cargar la configuración");
    }
    setLoading(false);
  }, []);

  useEffect(() => { loadConfig(); }, [loadConfig]);

  const showSuccess = (msg: string) => {
    setSuccess(msg);
    setTimeout(() => setSuccess(null), 3000);
  };

  const saveTelegram = async () => {
    setSaving("telegram");
    setError(null);
    try {
      const body: Record<string, string | null> = { telegram_chat_id: tgChatId || null };
      if (tgToken && tgToken !== "********") {
        body.telegram_token = tgToken;
      }
      if (!tgEnabled) {
        body.telegram_token = "";
        body.telegram_chat_id = "";
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data: AppConfig = await res.json();
        setConfig(data);
        setTgToken(data.telegram_token_set ? "********" : "");
        setTgChatId(data.telegram_chat_id || "");
        showSuccess(tgEnabled ? "✅ Telegram configurado" : "❌ Telegram desactivado");
      } else {
        setError("Error al guardar Telegram");
      }
    } catch {
      setError("Error de conexión al guardar Telegram");
    }
    setSaving(null);
  };

  const saveMatrix = async () => {
    setSaving("matrix");
    setError(null);
    try {
      const body: Record<string, string | null> = {
        matrix_homeserver: mxHomeserver || null,
        matrix_room: mxRoom || null,
      };
      if (mxToken && mxToken !== "********") {
        body.matrix_token = mxToken;
      }
      if (!mxEnabled) {
        body.matrix_homeserver = "";
        body.matrix_token = "";
        body.matrix_room = "";
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data: AppConfig = await res.json();
        setConfig(data);
        setMxHomeserver(data.matrix_homeserver || "");
        setMxToken(data.matrix_token_set ? "********" : "");
        setMxRoom(data.matrix_room || "");
        showSuccess(mxEnabled ? "✅ Matrix configurado" : "❌ Matrix desactivado");
      } else {
        setError("Error al guardar Matrix");
      }
    } catch {
      setError("Error de conexión al guardar Matrix");
    }
    setSaving(null);
  };

  const saveAutoUpdate = async () => {
    setSaving("auto-update");
    setError(null);
    try {
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          auto_update_enabled: auEnabled,
          auto_update_interval_hours: auInterval,
        }),
      });
      if (res.ok) {
        const data: AppConfig = await res.json();
        setConfig(data);
        setAuEnabled(data.auto_update_enabled);
        setAuInterval(data.auto_update_interval_hours);
        showSuccess(auEnabled ? "✅ Auto-update activado" : "❌ Auto-update desactivado");
      } else {
        setError("Error al guardar Auto-update");
      }
    } catch {
      setError("Error de conexión al guardar Auto-update");
    }
    setSaving(null);
  };

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>;

  return (
    <Stack>
      {error && (
        <Alert color="red" onClose={() => setError(null)} withCloseButton>
          {error}
        </Alert>
      )}
      {success && (
        <Alert color="green" onClose={() => setSuccess(null)} withCloseButton>
          {success}
        </Alert>
      )}

      {/* ═══ Telegram ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>📱 Telegram</Title>
          <Switch
            label={tgEnabled ? "Activado" : "Desactivado"}
            checked={tgEnabled}
            onChange={(e) => setTgEnabled(e.currentTarget.checked)}
            color={tgEnabled ? "green" : "gray"}
          />
        </Group>
        {tgEnabled && (
          <Stack>
            <TextInput
              label="Token del Bot"
              description="Token que te proporciona @BotFather"
              placeholder="123456:ABC-DEF..."
              type="password"
              value={tgToken}
              onChange={(e) => setTgToken(e.currentTarget.value)}
            />
            <TextInput
              label="Chat ID"
              description="ID del chat o grupo donde recibir notificaciones"
              placeholder="-1001234567890"
              value={tgChatId}
              onChange={(e) => setTgChatId(e.currentTarget.value)}
            />
          </Stack>
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveTelegram}
            loading={saving === "telegram"}
            color={tgEnabled ? "blue" : "gray"}
          >
            {tgEnabled ? "Guardar Telegram" : "Desactivar Telegram"}
          </Button>
        </Group>
      </Paper>

      {/* ═══ Matrix ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>💬 Matrix</Title>
          <Switch
            label={mxEnabled ? "Activado" : "Desactivado"}
            checked={mxEnabled}
            onChange={(e) => setMxEnabled(e.currentTarget.checked)}
            color={mxEnabled ? "green" : "gray"}
          />
        </Group>
        {mxEnabled && (
          <Stack>
            <TextInput
              label="Homeserver"
              description="URL del servidor Matrix (ej: https://matrix.org)"
              placeholder="https://matrix.example.com"
              value={mxHomeserver}
              onChange={(e) => setMxHomeserver(e.currentTarget.value)}
            />
            <TextInput
              label="Access Token"
              description="Token de acceso de la cuenta de bot"
              type="password"
              placeholder="syt_..."
              value={mxToken}
              onChange={(e) => setMxToken(e.currentTarget.value)}
            />
            <TextInput
              label="Room ID"
              description="ID de la sala donde enviar notificaciones"
              placeholder="!roomid:matrix.org"
              value={mxRoom}
              onChange={(e) => setMxRoom(e.currentTarget.value)}
            />
          </Stack>
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveMatrix}
            loading={saving === "matrix"}
            color={mxEnabled ? "blue" : "gray"}
          >
            {mxEnabled ? "Guardar Matrix" : "Desactivar Matrix"}
          </Button>
        </Group>
      </Paper>

      {/* ═══ Auto-update ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>🤖 Auto-update</Title>
          <Switch
            label={auEnabled ? "Activado" : "Desactivado"}
            checked={auEnabled}
            onChange={(e) => setAuEnabled(e.currentTarget.checked)}
            color={auEnabled ? "green" : "gray"}
          />
        </Group>
        {auEnabled && (
          <TextInput
            label="Intervalo (horas)"
            description="Cada cuántas horas comprobar y actualizar containers"
            type="number"
            min={1}
            max={168}
            value={auInterval}
            onChange={(e) => setAuInterval(Number(e.currentTarget.value))}
          />
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveAutoUpdate}
            loading={saving === "auto-update"}
            color={auEnabled ? "blue" : "gray"}
          >
            {auEnabled ? "Guardar Auto-update" : "Desactivar Auto-update"}
          </Button>
        </Group>
      </Paper>
    </Stack>
  );
}