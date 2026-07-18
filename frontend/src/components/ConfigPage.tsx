import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Group,
  Paper,
  PasswordInput,
  Stack,
  Text,
  Title,
  TextInput,
  Switch,
  Select,
} from "@mantine/core";
import type {
  AppConfig,
  DefaultUpdatePolicy,
  UpdateCheckConfig,
} from "../types";
import { apiFetch } from "../api";

const CRON_PRESETS = [
  { value: "0 */6 * * *", label: "Cada 6 horas" },
  { value: "0 */12 * * *", label: "Cada 12 horas" },
  { value: "0 0 * * *", label: "Cada día a medianoche" },
  { value: "0 6 * * *", label: "Cada día a las 6:00" },
  { value: "0 0 * * 0", label: "Cada domingo" },
  { value: "0 0 1 * *", label: "Cada 1 del mes" },
  { value: "*/30 * * * *", label: "Cada 30 minutos" },
  { value: "0 */1 * * *", label: "Cada hora" },
];

interface ConfigPageProps {
  config: AppConfig | null;
  setConfig: (c: AppConfig) => void;
  colorScheme: "dark" | "light";
  setColorScheme: (scheme: "dark" | "light") => void;
}

export default function ConfigPage({
  config: configProp,
  setConfig: setConfigProp,
  colorScheme,
  setColorScheme,
}: ConfigPageProps) {
  const [saving, setSaving] = useState<string | null>(null);
  const [testing, setTesting] = useState<string | null>(null);
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

  // Update check cron
  const [ucCron, setUcCron] = useState("0 */6 * * *");
  const [ucEnabled, setUcEnabled] = useState(false);
  const [ucNotify, setUcNotify] = useState(false);

  // Update check config — fetched separately
  const [_checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(
    null,
  );
  useEffect(() => {
    fetch("/api/update-check/config", { credentials: "include" })
      .then((res) => res.json())
      .then((data: UpdateCheckConfig) => {
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
      })
      .catch(() => {});
  }, []);

  // Default update policy
  const [defAction, setDefAction] = useState<string>("pull-restart");
  const [defCleanup, setDefCleanup] = useState(false);
  const [defRollback, setDefRollback] = useState(false);
  useEffect(() => {
    fetch("/api/update-policies/default", { credentials: "include" })
      .then((res) => res.json())
      .then((data: DefaultUpdatePolicy) => {
        setDefAction(data.action);
        setDefCleanup(data.cleanup_old_image);
        setDefRollback(data.rollback_on_failure);
      })
      .catch(() => {});
  }, []);

  // Sync from props when config changes (from App.tsx eager fetch)
  useEffect(() => {
    if (!configProp) return;
    setTgToken(configProp.telegram_token ?? "");
    setTgChatId(configProp.telegram_chat_id || "");
    setTgEnabled(configProp.telegram_configured);
    setMxHomeserver(configProp.matrix_homeserver || "");
    setMxToken(configProp.matrix_token ?? "");
    setMxRoom(configProp.matrix_room || "");
    setMxEnabled(configProp.matrix_configured);
    setAuEnabled(configProp.auto_update_enabled);
    setAuInterval(configProp.auto_update_interval_hours);
  }, [configProp]);

  const showSuccess = (msg: string) => {
    setSuccess(msg);
    setTimeout(() => setSuccess(null), 3000);
  };

  const saveTelegram = async () => {
    setSaving("telegram");
    setError(null);
    try {
      const body: Record<string, string | null> = {
        telegram_token: tgToken || null,
        telegram_chat_id: tgChatId || null,
      };
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
        setConfigProp(data);
        setTgToken(data.telegram_token ?? "");
        setTgChatId(data.telegram_chat_id || "");
        showSuccess(
          tgEnabled ? "✅ Telegram configurado" : "❌ Telegram desactivado",
        );
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
        matrix_token: mxToken || null,
        matrix_room: mxRoom || null,
      };
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
        setConfigProp(data);
        setMxHomeserver(data.matrix_homeserver || "");
        setMxToken(data.matrix_token ?? "");
        setMxRoom(data.matrix_room || "");
        showSuccess(
          mxEnabled ? "✅ Matrix configurado" : "❌ Matrix desactivado",
        );
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
        setConfigProp(data);
        setAuEnabled(data.auto_update_enabled);
        setAuInterval(data.auto_update_interval_hours);
        showSuccess(
          auEnabled ? "✅ Auto-update activado" : "❌ Auto-update desactivado",
        );
      } else {
        setError("Error al guardar Auto-update");
      }
    } catch {
      setError("Error de conexión al guardar Auto-update");
    }
    setSaving(null);
  };

  const saveUpdateCheck = async () => {
    setSaving("update-check");
    setError(null);
    try {
      const res = await apiFetch("/api/update-check/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          cron: ucCron,
          enabled: ucEnabled,
          notify: ucNotify,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
        showSuccess(
          ucEnabled
            ? "✅ Revisión programada activada"
            : "❌ Revisión programada desactivada",
        );
      } else {
        setError("Error al guardar la revisión de actualizaciones");
      }
    } catch {
      setError("Error de conexión");
    }
    setSaving(null);
  };

  const saveDefaultPolicy = async () => {
    setSaving("default-policy");
    setError(null);
    try {
      const res = await apiFetch("/api/update-policies/default", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: defAction,
          cleanup_old_image: defCleanup,
          rollback_on_failure: defRollback,
        }),
      });
      if (res.ok) {
        showSuccess("✅ Política por defecto actualizada");
      } else {
        setError("Error al guardar política por defecto");
      }
    } catch {
      setError("Error de conexión");
    }
    setSaving(null);
  };

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

      {/* ═══ Tema ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between">
          <div>
            <Title order={4}>
              {colorScheme === "dark" ? "🌙" : "☀️"} Tema
            </Title>
            <Text size="sm" c="dimmed">
              {colorScheme === "dark"
                ? "Modo oscuro"
                : "Modo claro"}
            </Text>
          </div>
          <Switch
            checked={colorScheme === "dark"}
            onChange={(e) => {
              const next = e.currentTarget.checked ? "dark" : "light";
              localStorage.setItem("color-scheme", next);
              setColorScheme(next);
            }}
            onLabel="🌙"
            offLabel="☀️"
            size="lg"
          />
        </Group>
      </Paper>

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
            <PasswordInput
              label="Token del Bot"
              description="Token que te proporciona @BotFather"
              placeholder="123456:ABC-DEF..."
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
          {tgEnabled && (
            <Button
              onClick={async () => {
                setTesting("telegram");
                setError(null);
                try {
                  const res = await apiFetch("/api/notifications/test", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ channel: "telegram" }),
                  });
                  setTesting(null);
                  if (res.ok) {
                    showSuccess("📤 Mensaje de prueba enviado a Telegram");
                  } else {
                    const data = await res
                      .json()
                      .catch(() => ({ error: "Error desconocido" }));
                    setError(data.error || `Error HTTP ${res.status}`);
                  }
                } catch {
                  setTesting(null);
                  setError("Error de conexión al enviar test");
                }
              }}
              loading={testing === "telegram"}
              variant="outline"
              color="green"
            >
              📤 Test
            </Button>
          )}
          {tgEnabled && (
            <Button
              onClick={saveTelegram}
              loading={saving === "telegram"}
              color="blue"
            >
              Guardar Telegram
            </Button>
          )}
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
            <PasswordInput
              label="Access Token"
              description="Token de acceso de la cuenta de bot"
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
          {mxEnabled && (
            <Button
              onClick={async () => {
                setTesting("matrix");
                setError(null);
                try {
                  const res = await apiFetch("/api/notifications/test", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ channel: "matrix" }),
                  });
                  setTesting(null);
                  if (res.ok) {
                    showSuccess("📤 Mensaje de prueba enviado a Matrix");
                  } else {
                    const data = await res
                      .json()
                      .catch(() => ({ error: "Error desconocido" }));
                    setError(data.error || `Error HTTP ${res.status}`);
                  }
                } catch {
                  setTesting(null);
                  setError("Error de conexión al enviar test");
                }
              }}
              loading={testing === "matrix"}
              variant="outline"
              color="green"
            >
              📤 Test
            </Button>
          )}
          {mxEnabled && (
            <Button
              onClick={saveMatrix}
              loading={saving === "matrix"}
              color="blue"
            >
              Guardar Matrix
            </Button>
          )}
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

      {/* ═══ Revisión de actualizaciones ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>⏰ Revisión de actualizaciones</Title>
          <Switch
            label={ucEnabled ? "Activada" : "Desactivada"}
            checked={ucEnabled}
            onChange={(e) => setUcEnabled(e.currentTarget.checked)}
            color={ucEnabled ? "green" : "gray"}
          />
        </Group>
        <Text size="sm" c="dimmed" mb="md">
          Programa revisiones periódicas de imágenes. Cuando se detecte una
          actualización pendiente, se marcará el contenedor y se podrá actuar
          desde el Dashboard.
        </Text>
        {ucEnabled && (
          <Stack>
            <Select
              label="Frecuencia"
              data={CRON_PRESETS}
              value={ucCron}
              onChange={(v) => v && setUcCron(v)}
              searchable
            />
            <TextInput
              label="Expresión Cron (personalizada)"
              description="Edita directamente si los presets no se ajustan"
              placeholder="0 */6 * * *"
              value={ucCron}
              onChange={(e) => setUcCron(e.currentTarget.value)}
            />
            <Switch
              label="🔔 Notificar vía Telegram/Matrix"
              checked={ucNotify}
              onChange={(e) => setUcNotify(e.currentTarget.checked)}
            />
          </Stack>
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveUpdateCheck}
            loading={saving === "update-check"}
            color={ucEnabled ? "blue" : "gray"}
          >
            {ucEnabled ? "Guardar revisión" : "Desactivar revisión"}
          </Button>
        </Group>
      </Paper>

      {/* ═══ Política de actualización por defecto ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">
          📋 Política de actualización por defecto
        </Title>
        <Text size="sm" c="dimmed" mb="md">
          Esta política se aplica a los contenedores que no tengan una política
          individual configurada. Puedes sobrescribirla para cada contenedor
          desde el Dashboard con el botón ⚙️.
        </Text>
        <Stack>
          <Select
            label="Acción por defecto"
            data={[
              { value: "none", label: "❌ No hacer nada" },
              { value: "pull", label: "⬇️ Pull imagen" },
              {
                value: "pull-restart",
                label: "🔄 Pull + reiniciar contenedor",
              },
              {
                value: "pull-restart-stack",
                label: "📦 Pull + reiniciar stack",
              },
            ]}
            value={defAction}
            onChange={(v) => v && setDefAction(v)}
          />
          <Switch
            label="🧹 Borrar imagen anterior"
            description="Elimina la imagen anterior después de actualizar"
            checked={defCleanup}
            onChange={(e) => setDefCleanup(e.currentTarget.checked)}
          />
          <Switch
            label="↩️ Rollback si falla"
            description="Si el contenedor no arranca, restaura la imagen anterior"
            checked={defRollback}
            onChange={(e) => setDefRollback(e.currentTarget.checked)}
          />
        </Stack>
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveDefaultPolicy}
            loading={saving === "default-policy"}
            color="blue"
          >
            Guardar política por defecto
          </Button>
        </Group>
      </Paper>

      {/* ═══ Export / Import ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">
          📦 Exportar / Importar configuración
        </Title>
        <Text size="sm" c="dimmed" mb="md">
          Exporta alertas, programaciones y ajustes a un archivo JSON. Puedes
          importarlo después para restaurar la configuración.
        </Text>
        <Group>
          <Button
            variant="filled"
            color="blue"
            onClick={async () => {
              try {
                const res = await apiFetch("/api/admin/export");
                const data = await res.json();
                const blob = new Blob([JSON.stringify(data, null, 2)], {
                  type: "application/json",
                });
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = `alloy-config-${new Date().toISOString().slice(0, 10)}.json`;
                a.click();
                URL.revokeObjectURL(url);
                showSuccess("✅ Configuración exportada");
              } catch {
                setError("Error al exportar configuración");
              }
            }}
          >
            📤 Exportar
          </Button>
          <Button
            variant="outline"
            color="yellow"
            onClick={() => {
              const input = document.createElement("input");
              input.type = "file";
              input.accept = ".json";
              input.onchange = async (e) => {
                const file = (e.target as HTMLInputElement).files?.[0];
                if (!file) return;
                try {
                  const text = await file.text();
                  const data = JSON.parse(text);
                  const res = await apiFetch("/api/admin/import", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                      alerts: data.alerts || [],
                      schedules: data.schedules || [],
                      settings: data.settings || {},
                    }),
                  });
                  if (res.ok) {
                    showSuccess(
                      "✅ Configuración importada. Recarga la página.",
                    );
                    setTimeout(() => window.location.reload(), 1500);
                  } else {
                    const err = await res.text();
                    setError(`Error al importar: ${err}`);
                  }
                } catch {
                  setError("Archivo JSON inválido");
                }
              };
              input.click();
            }}
          >
            📥 Importar
          </Button>
        </Group>
      </Paper>
    </Stack>
  );
}
