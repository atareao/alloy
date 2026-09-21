import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Flex,
  Input,
  InputNumber,
  Select,
  Space,
  Switch,
  Tabs,
  Tag,
  Typography,
} from "antd";
import type { AppConfig, DefaultUpdatePolicy, UpdateCheckConfig } from "../types";
import { apiFetch } from "../api";

const CRON_PRESETS = [
  { value: "0 */6 * * *", label: "Cada 6 horas" },
  { value: "0 */12 * * *", label: "Cada 12 horas" },
  { value: "0 0 * * *", label: "Cada día a medianoche" },
  { value: "0 4 * * *", label: "Cada día a las 4:00" },
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
  const [activeTab, setActiveTab] = useState<string>("notifications");

  // Telegram
  const [tgToken, setTgToken] = useState("");
  const [tgChatId, setTgChatId] = useState("");
  const [tgEnabled, setTgEnabled] = useState(false);

  // Matrix
  const [mxHomeserver, setMxHomeserver] = useState("");
  const [mxToken, setMxToken] = useState("");
  const [mxRoom, setMxRoom] = useState("");
  const [mxEnabled, setMxEnabled] = useState(false);

  // Update check cron
  const [ucCron, setUcCron] = useState("0 */6 * * *");
  const [ucEnabled, setUcEnabled] = useState(false);
  const [ucNotify, setUcNotify] = useState(false);

  // Update check config — fetched separately
  const [checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(null);

  // Pull timeout
  const [pullTimeout, setPullTimeout] = useState(600);
  useEffect(() => {
    apiFetch("/api/update-check/config")
      .then((res) => res.json())
      .then((data: UpdateCheckConfig) => {
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
        if (data.pull_timeout_secs != null) setPullTimeout(data.pull_timeout_secs);
      })
      .catch(() => {});
  }, []);

  // Default update policy
  const [defAction, setDefAction] = useState<string>("pull-restart");
  const [defCleanup, setDefCleanup] = useState(false);
  const [defRollback, setDefRollback] = useState(false);
  useEffect(() => {
    apiFetch("/api/update-policies/default")
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
          pull_timeout_secs: pullTimeout,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
        if (data.pull_timeout_secs != null) setPullTimeout(data.pull_timeout_secs);
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
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      {error && (
        <Alert
          type="error"
          closable
          onClose={() => setError(null)}
          showIcon
          message={error}
        />
      )}
      {success && (
        <Alert
          type="success"
          closable
          onClose={() => setSuccess(null)}
          showIcon
          message={success}
        />
      )}

      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "notifications",
            label: "🔔 Notificaciones",
            children: (
              <Space direction="vertical" size="middle" style={{ width: "100%", paddingTop: 16 }}>
                {/* ═══ Telegram ═══ */}
                <Card bordered size="small">
                  <Flex justify="space-between" align="center" style={{ marginBottom: 16 }}>
                    <Typography.Title level={4} style={{ margin: 0 }}>
                      📱 Telegram
                    </Typography.Title>
                    <Flex align="center" gap={8}>
                      <Switch
                        checked={tgEnabled}
                        onChange={(checked) => setTgEnabled(checked)}
                      />
                      <Typography.Text>{tgEnabled ? "Activado" : "Desactivado"}</Typography.Text>
                    </Flex>
                  </Flex>
                  {tgEnabled && (
                    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Token del Bot</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          Token que te proporciona @BotFather
                        </Typography.Text>
                        <Input.Password
                          placeholder="123456:ABC-DEF..."
                          value={tgToken}
                          onChange={(e) => setTgToken(e.target.value)}
                        />
                      </Space>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Chat ID</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          ID del chat o grupo donde recibir notificaciones
                        </Typography.Text>
                        <Input
                          placeholder="-1001234567890"
                          value={tgChatId}
                          onChange={(e) => setTgChatId(e.target.value)}
                        />
                      </Space>
                    </Space>
                  )}
                  <Flex justify="flex-end" gap="middle" style={{ marginTop: 16 }}>
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
                        style={{ borderColor: "var(--ant-color-success)", color: "var(--ant-color-success)" }}
                      >
                        📤 Test
                      </Button>
                    )}
                    {tgEnabled && (
                      <Button
                        onClick={saveTelegram}
                        loading={saving === "telegram"}
                        type="primary"
                      >
                        Guardar Telegram
                      </Button>
                    )}
                  </Flex>
                </Card>

                {/* ═══ Matrix ═══ */}
                <Card bordered size="small">
                  <Flex justify="space-between" align="center" style={{ marginBottom: 16 }}>
                    <Typography.Title level={4} style={{ margin: 0 }}>
                      💬 Matrix
                    </Typography.Title>
                    <Flex align="center" gap={8}>
                      <Switch
                        checked={mxEnabled}
                        onChange={(checked) => setMxEnabled(checked)}
                      />
                      <Typography.Text>{mxEnabled ? "Activado" : "Desactivado"}</Typography.Text>
                    </Flex>
                  </Flex>
                  {mxEnabled && (
                    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Homeserver</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          URL del servidor Matrix (ej: https://matrix.org)
                        </Typography.Text>
                        <Input
                          placeholder="https://matrix.example.com"
                          value={mxHomeserver}
                          onChange={(e) => setMxHomeserver(e.target.value)}
                        />
                      </Space>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Access Token</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          Token de acceso de la cuenta de bot
                        </Typography.Text>
                        <Input.Password
                          placeholder="syt_..."
                          value={mxToken}
                          onChange={(e) => setMxToken(e.target.value)}
                        />
                      </Space>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Room ID</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          ID de la sala donde enviar notificaciones
                        </Typography.Text>
                        <Input
                          placeholder="!roomid:matrix.org"
                          value={mxRoom}
                          onChange={(e) => setMxRoom(e.target.value)}
                        />
                      </Space>
                    </Space>
                  )}
                  <Flex justify="flex-end" gap="middle" style={{ marginTop: 16 }}>
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
                        style={{ borderColor: "var(--ant-color-success)", color: "var(--ant-color-success)" }}
                      >
                        📤 Test
                      </Button>
                    )}
                    {mxEnabled && (
                      <Button
                        onClick={saveMatrix}
                        loading={saving === "matrix"}
                        type="primary"
                      >
                        Guardar Matrix
                      </Button>
                    )}
                  </Flex>
                </Card>
              </Space>
            ),
          },
          {
            key: "updates",
            label: "⬆️ Actualizaciones",
            children: (
              <Space direction="vertical" size="middle" style={{ width: "100%", paddingTop: 16 }}>
                {/* ═══ Revisión de actualizaciones ═══ */}
                <Card bordered size="small">
                  <Flex justify="space-between" align="center" style={{ marginBottom: 16 }}>
                    <Typography.Title level={4} style={{ margin: 0 }}>
                      ⏰ Revisión de actualizaciones
                    </Typography.Title>
                    <Flex align="center" gap={8}>
                      <Switch
                        checked={ucEnabled}
                        onChange={(checked) => setUcEnabled(checked)}
                      />
                      <Typography.Text>{ucEnabled ? "Activada" : "Desactivada"}</Typography.Text>
                    </Flex>
                  </Flex>
                  <Typography.Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
                    Programa revisiones periódicas de imágenes. Cuando se detecte una
                    actualización pendiente, se marcará el contenedor y se podrá actuar
                    desde el Dashboard.
                  </Typography.Text>
                  <div style={{ display: "flex", flexDirection: "column", gap: 4, marginBottom: 16 }}>
                    <Typography.Text type="secondary">
                      Zona horaria: {configProp?.timezone || "UTC"}
                    </Typography.Text>
                    {checkConfig?.last_run_at && (
                      <Typography.Text type="secondary">
                        Última revisión: {new Date(checkConfig.last_run_at).toLocaleString([], {
                          year: "numeric", month: "2-digit", day: "2-digit",
                          hour: "2-digit", minute: "2-digit", second: "2-digit",
                          hour12: false,
                        })}
                      </Typography.Text>
                    )}
                    {checkConfig?.next_run_at && (
                      <Typography.Text type="secondary">
                        Próxima revisión: {new Date(checkConfig.next_run_at).toLocaleString([], {
                          year: "numeric", month: "2-digit", day: "2-digit",
                          hour: "2-digit", minute: "2-digit", second: "2-digit",
                          hour12: false,
                        })}
                      </Typography.Text>
                    )}
                  </div>
                  {ucEnabled && (
                    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Frecuencia</Typography.Text>
                        <Select
                          options={CRON_PRESETS}
                          value={ucCron}
                          onChange={(v) => v && setUcCron(v)}
                          showSearch
                          style={{ width: "100%" }}
                        />
                      </Space>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>Expresión Cron (personalizada)</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          Edita directamente si los presets no se ajustan
                        </Typography.Text>
                        <Input
                          placeholder="0 */6 * * *"
                          value={ucCron}
                          onChange={(e) => setUcCron(e.target.value)}
                        />
                      </Space>
                      <Flex align="center" gap={8}>
                        <Switch
                          checked={ucNotify}
                          onChange={(checked) => setUcNotify(checked)}
                        />
                        <Typography.Text>🔔 Notificar vía Telegram/Matrix</Typography.Text>
                      </Flex>
                      <Space direction="vertical" size={4} style={{ width: "100%" }}>
                        <Typography.Text strong>⏱️ Timeout pull (segundos)</Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          Aumentar para imágenes grandes (&gt;500MB) o conexiones lentas. Default: 1800 (30 min)
                        </Typography.Text>
                        <InputNumber
                          value={pullTimeout}
                          onChange={(v) => setPullTimeout(Number(v) || 1800)}
                          min={60}
                          max={7200}
                          step={60}
                          style={{ width: "100%" }}
                        />
                      </Space>
                    </Space>
                  )}
                  <Flex justify="flex-end" style={{ marginTop: 16 }}>
                    <Button
                      onClick={saveUpdateCheck}
                      loading={saving === "update-check"}
                      type={ucEnabled ? "primary" : "default"}
                    >
                      {ucEnabled ? "Guardar revisión" : "Desactivar revisión"}
                    </Button>
                  </Flex>
                </Card>

                {/* ═══ Política de actualización por defecto ═══ */}
                <Card bordered size="small">
                  <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
                    📋 Política de actualización por defecto
                  </Typography.Title>
                  <Typography.Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
                    Esta política se aplica a los contenedores que no tengan una política
                    individual configurada. Puedes sobrescribirla para cada contenedor
                    desde el Dashboard con el botón ⚙️.
                  </Typography.Text>
                  <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                    <Space direction="vertical" size={4} style={{ width: "100%" }}>
                      <Typography.Text strong>Acción por defecto</Typography.Text>
                      <Select
                        options={[
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
                        style={{ width: "100%" }}
                      />
                    </Space>
                    <Space direction="vertical" size={2}>
                      <Flex align="center" gap={8}>
                        <Switch
                          checked={defCleanup}
                          onChange={(checked) => setDefCleanup(checked)}
                        />
                        <Typography.Text>🧹 Borrar imagen anterior</Typography.Text>
                      </Flex>
                      <Typography.Text type="secondary" style={{ fontSize: 12, paddingLeft: 36 }}>
                        Elimina la imagen anterior después de actualizar
                      </Typography.Text>
                    </Space>
                    <Space direction="vertical" size={2}>
                      <Flex align="center" gap={8}>
                        <Switch
                          checked={defRollback}
                          onChange={(checked) => setDefRollback(checked)}
                        />
                        <Typography.Text>↩️ Rollback si falla</Typography.Text>
                      </Flex>
                      <Typography.Text type="secondary" style={{ fontSize: 12, paddingLeft: 36 }}>
                        Si el contenedor no arranca, restaura la imagen anterior
                      </Typography.Text>
                    </Space>
                  </Space>
                  <Flex justify="flex-end" style={{ marginTop: 16 }}>
                    <Button
                      onClick={saveDefaultPolicy}
                      loading={saving === "default-policy"}
                      type="primary"
                    >
                      Guardar política por defecto
                    </Button>
                  </Flex>
                </Card>
              </Space>
            ),
          },
          {
            key: "info",
            label: "ℹ️ Información",
            children: (
              <Space direction="vertical" size="middle" style={{ width: "100%", paddingTop: 16 }}>
                {/* ═══ Versión e información ═══ */}
                <Card bordered size="small">
                  <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
                    ℹ️ Información de la aplicación
                  </Typography.Title>
                  <Space direction="vertical" size="small" style={{ width: "100%" }}>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Versión</Typography.Text>
                      <Tag color="blue">v{configProp?.version || "—"}</Tag>
                    </Flex>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Compilado</Typography.Text>
                      <Typography.Text type="secondary">
                        {configProp?.build_date
                          ? new Date(configProp.build_date).toLocaleString([], {
                              year: "numeric", month: "long", day: "numeric",
                              hour: "2-digit", minute: "2-digit",
                              hour12: false,
                            })
                          : "—"}
                      </Typography.Text>
                    </Flex>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Repositorio</Typography.Text>
                      <Typography.Link
                        href={configProp?.repo_url || "#"}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        {configProp?.repo_url || "—"}
                      </Typography.Link>
                    </Flex>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Zona horaria</Typography.Text>
                      <Typography.Text type="secondary">{configProp?.timezone || "UTC"}</Typography.Text>
                    </Flex>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Puerto</Typography.Text>
                      <Typography.Text type="secondary">{configProp?.port || 3066}</Typography.Text>
                    </Flex>
                    <Flex gap="middle" align="center">
                      <Typography.Text strong style={{ minWidth: 100 }}>Auth</Typography.Text>
                      <Tag color="green">OIDC</Tag>
                    </Flex>
                  </Space>
                </Card>

                {/* ═══ Tema ═══ */}
                <Card bordered size="small">
                  <Flex justify="space-between" align="center">
                    <div>
                      <Typography.Title level={4} style={{ margin: 0 }}>
                        {colorScheme === "dark" ? "🌙" : "☀️"} Tema
                      </Typography.Title>
                      <Typography.Text type="secondary">
                        {colorScheme === "dark" ? "Modo oscuro" : "Modo claro"}
                      </Typography.Text>
                    </div>
                    <Switch
                      checked={colorScheme === "dark"}
                      onChange={(checked) => {
                        const next = checked ? "dark" : "light";
                        localStorage.setItem("color-scheme", next);
                        setColorScheme(next);
                      }}
                      checkedChildren="🌙"
                      unCheckedChildren="☀️"
                    />
                  </Flex>
                </Card>

                {/* ═══ Export / Import ═══ */}
                <Card bordered size="small">
                  <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
                    📦 Exportar / Importar configuración
                  </Typography.Title>
                  <Typography.Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
                    Exporta alertas, programaciones y ajustes a un archivo JSON. Puedes
                    importarlo después para restaurar la configuración.
                  </Typography.Text>
                  <Flex gap="middle">
                    <Button
                      type="primary"
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
                      style={{ borderColor: "var(--ant-color-warning)", color: "var(--ant-color-warning)" }}
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
                  </Flex>
                </Card>
              </Space>
            ),
          },
        ]}
      />
    </Space>
  );
}