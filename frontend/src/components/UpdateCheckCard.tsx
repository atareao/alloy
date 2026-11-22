import {
  Button,
  Card,
  Flex,
  Input,
  InputNumber,
  Select,
  Space,
  Switch,
  Typography,
} from "antd";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";
import { cronErrorFor, normalizeCron } from "../cronValidation";
import type { SavingKey } from "../useConfigPage";
import type { UseUpdateCheckConfigResult } from "../useUpdateCheckConfig";
// Los presets viven fuera de este archivo para no romper fast-refresh (un
// componente solo debe exportar componentes).
import { CRON_PRESETS } from "../cronPresets";

interface UpdateCheckCardProps {
  configProp: AppConfig | null;
  /**
   * Estado de la revisión, hoisted a `ConfigPage` para que los GET de arranque
   * se disparen al montar la página (no al montar la Card en su tab).
   */
  updateCheck: UseUpdateCheckConfigResult;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card «⏰ Revisión de actualizaciones» (pestaña Actualizaciones). */
export default function UpdateCheckCard({
  configProp,
  updateCheck,
  saving,
  setSaving,
  setError,
  showSuccess,
}: UpdateCheckCardProps) {
  const {
    checkConfig,
    setCheckConfig,
    defaults,
    ucCron,
    setUcCron,
    ucEnabled,
    setUcEnabled,
    ucNotify,
    setUcNotify,
    pullTimeout,
    setPullTimeout,
    checkIntervalSecs,
    setCheckIntervalSecs,
    ucMaxStartAttempts,
    setUcMaxStartAttempts,
    ucStartAttemptInterval,
    setUcStartAttemptInterval,
    ucHealthWindow,
    setUcHealthWindow,
    cronError,
    setCronError,
  } = updateCheck;

  const saveUpdateCheck = async () => {
    // Validación/normalización en cliente: pre-chequeo antes del PUT.
    // El backend sigue siendo la autoridad final.
    const normalizedCron = normalizeCron(ucCron);
    const clientError = cronErrorFor(normalizedCron);
    if (clientError) {
      setCronError(clientError);
      return;
    }
    if (normalizedCron !== ucCron) setUcCron(normalizedCron);
    setCronError(null);
    setSaving("update-check");
    setError(null);
    try {
      const res = await apiFetch("/api/update-check/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          cron: normalizedCron,
          enabled: ucEnabled,
          notify: ucNotify,
          pull_timeout_secs: pullTimeout,
          update_max_start_attempts: ucMaxStartAttempts,
          update_start_attempt_interval_secs: ucStartAttemptInterval,
          update_health_window_secs: ucHealthWindow,
          check_interval_ms: checkIntervalSecs * 1000,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
        if (data.pull_timeout_secs != null)
          setPullTimeout(data.pull_timeout_secs);
        if (data.update_max_start_attempts != null)
          setUcMaxStartAttempts(data.update_max_start_attempts);
        if (data.update_start_attempt_interval_secs != null)
          setUcStartAttemptInterval(data.update_start_attempt_interval_secs);
        if (data.update_health_window_secs != null)
          setUcHealthWindow(data.update_health_window_secs);
        if (data.check_interval_ms != null)
          setCheckIntervalSecs(Math.round(data.check_interval_ms / 1000));
        showSuccess(
          ucEnabled
            ? "✅ Revisión programada activada"
            : "❌ Revisión programada desactivada",
        );
      } else {
        const data = await res
          .json()
          .catch(() => ({ error: "Error desconocido" }));
        setError(data.error || `Error HTTP ${res.status}`);
      }
    } catch {
      setError("Error de conexión");
    }
    setSaving(null);
  };

  return (
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
          <Typography.Text>
            {ucEnabled ? "Activada" : "Desactivada"}
          </Typography.Text>
        </Flex>
      </Flex>
      <Typography.Text
        type="secondary"
        style={{ display: "block", marginBottom: 16 }}
      >
        Programa revisiones periódicas de imágenes. Cuando se detecte una
        actualización pendiente, se marcará el contenedor y se podrá actuar desde
        el Dashboard.
      </Typography.Text>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 4,
          marginBottom: 16,
        }}
      >
        <Typography.Text type="secondary">
          Zona horaria: {configProp?.timezone || "UTC"}
        </Typography.Text>
        {checkConfig?.last_run_at && (
          <Typography.Text type="secondary">
            Última revisión:{" "}
            {new Date(checkConfig.last_run_at).toLocaleString([], {
              year: "numeric",
              month: "2-digit",
              day: "2-digit",
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
              hour12: false,
            })}
          </Typography.Text>
        )}
        {checkConfig?.next_run_at && (
          <Typography.Text type="secondary">
            Próxima revisión:{" "}
            {new Date(checkConfig.next_run_at).toLocaleString([], {
              year: "numeric",
              month: "2-digit",
              day: "2-digit",
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
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
            <Typography.Text strong>
              Expresión Cron (personalizada)
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Edita directamente si los presets no se ajustan
            </Typography.Text>
            <Input
              placeholder={defaults?.cron ?? ""}
              value={ucCron}
              status={cronError ? "error" : undefined}
              aria-invalid={!!cronError}
              aria-describedby={
                cronError ? "update-check-cron-error" : undefined
              }
              onChange={(e) => {
                // Patrón "touched": no introducimos error mientras el
                // usuario escribe si el campo estaba limpio; si ya
                // había error, lo recalculamos para limpiarlo al
                // corregir.
                const v = e.target.value;
                setUcCron(v);
                if (cronError) setCronError(cronErrorFor(v));
              }}
              onBlur={() => {
                const normalized = normalizeCron(ucCron);
                if (normalized !== ucCron) setUcCron(normalized);
                setCronError(cronErrorFor(normalized));
              }}
            />
          </Space>
          <Flex align="center" gap={8}>
            <Switch
              checked={ucNotify}
              onChange={(checked) => setUcNotify(checked)}
            />
            <Typography.Text>
              🔔 Notificar vía Telegram/Matrix
            </Typography.Text>
          </Flex>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>
              ⏱️ Timeout pull (segundos)
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Aumentar para imágenes grandes (&gt;500MB) o conexiones lentas.
              {defaults
                ? ` Default: ${defaults.pull_timeout_secs} (${Math.round(
                    defaults.pull_timeout_secs / 60,
                  )} min)`
                : ""}
            </Typography.Text>
            <InputNumber
              value={pullTimeout}
              onChange={(v) =>
                setPullTimeout((prev) =>
                  v == null ? (defaults?.pull_timeout_secs ?? prev) : Number(v),
                )
              }
              min={60}
              max={7200}
              step={60}
              style={{ width: "100%" }}
            />
          </Space>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>
              🔍 Intervalo de comprobación (s)
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Tiempo entre verificaciones de digest de cada contenedor.
              {defaults
                ? ` Default: ${Math.round(defaults.check_interval_ms / 1000)} s`
                : ""}
            </Typography.Text>
            <InputNumber
              aria-label="Intervalo de comprobación (s)"
              value={checkIntervalSecs}
              onChange={(v) =>
                setCheckIntervalSecs((prev) =>
                  v == null
                    ? defaults?.check_interval_ms
                      ? Math.round(defaults.check_interval_ms / 1000)
                      : prev
                    : Number(v),
                )
              }
              min={1}
              precision={0}
              style={{ width: 140 }}
            />
          </Space>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>🛡️ Guarda de arranque</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Tras aplicar una actualización se verifica que el contenedor
              arranque bien; si no, se reintenta y, si finalmente falla, se hace
              rollback a la imagen anterior.
            </Typography.Text>
            <Flex gap={8} wrap="wrap">
              <Space direction="vertical" size={2}>
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  Máximo de intentos de arranque
                </Typography.Text>
                <InputNumber
                  aria-label="Máximo de intentos de arranque"
                  value={ucMaxStartAttempts}
                  onChange={(v) =>
                    setUcMaxStartAttempts((prev) =>
                      v == null ? (defaults?.update_max_start_attempts ?? prev) : Number(v),
                    )
                  }
                  min={1}
                  max={10}
                  style={{ width: 140 }}
                />
              </Space>
              <Space direction="vertical" size={2}>
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  Intervalo entre intentos (s)
                </Typography.Text>
                <InputNumber
                  aria-label="Intervalo entre intentos (s)"
                  value={ucStartAttemptInterval}
                  onChange={(v) =>
                    setUcStartAttemptInterval((prev) =>
                      v == null
                        ? (defaults?.update_start_attempt_interval_secs ?? prev)
                        : Number(v),
                    )
                  }
                  min={1}
                  max={120}
                  style={{ width: 140 }}
                />
              </Space>
              <Space direction="vertical" size={2}>
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  Ventana de salud (s)
                </Typography.Text>
                <InputNumber
                  aria-label="Ventana de salud (s)"
                  value={ucHealthWindow}
                  onChange={(v) =>
                    setUcHealthWindow((prev) =>
                      v == null
                        ? (defaults?.update_health_window_secs ?? prev)
                        : Number(v),
                    )
                  }
                  min={5}
                  max={600}
                  style={{ width: 140 }}
                />
              </Space>
            </Flex>
          </Space>
        </Space>
      )}
      {cronError && (
        <Typography.Text
          id="update-check-cron-error"
          type="danger"
          style={{ fontSize: 12, display: "block" }}
          role="alert"
        >
          {cronError}
        </Typography.Text>
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
  );
}
