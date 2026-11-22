import { Button, Card, Flex, InputNumber, Select, Space, Switch, Typography } from "antd";
import { apiFetch } from "../api";
import type { SavingKey } from "../useConfigPage";
import type { UseDefaultPolicyResult } from "../useDefaultPolicy";

interface DefaultPolicyCardProps {
  /**
   * Estado de la política por defecto, hoisted a `ConfigPage` para que el GET
   * de arranque se dispare al montar la página (no al montar la Card en su tab).
   */
  defaultPolicy: UseDefaultPolicyResult;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card «📋 Política de actualización por defecto» (pestaña Actualizaciones). */
export default function DefaultPolicyCard({
  defaultPolicy,
  saving,
  setSaving,
  setError,
  showSuccess,
}: DefaultPolicyCardProps) {
  const {
    defAction,
    setDefAction,
    defCleanup,
    setDefCleanup,
    defRollback,
    setDefRollback,
    defMaxStartAttempts,
    setDefMaxStartAttempts,
    defHealthWindow,
    setDefHealthWindow,
  } = defaultPolicy;

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
          // Siempre presentes para no resetear overrides existentes (null = sin override).
          max_start_attempts: defMaxStartAttempts,
          health_window_secs: defHealthWindow,
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
    <Card bordered size="small">
      <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
        📋 Política de actualización por defecto
      </Typography.Title>
      <Typography.Text
        type="secondary"
        style={{ display: "block", marginBottom: 16 }}
      >
        Esta política se aplica a los contenedores que no tengan una política
        individual configurada. Puedes sobrescribirla para cada contenedor desde
        el Dashboard con el botón ⚙️.
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
          <Typography.Text
            type="secondary"
            style={{ fontSize: 12, paddingLeft: 36 }}
          >
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
          <Typography.Text
            type="secondary"
            style={{ fontSize: 12, paddingLeft: 36 }}
          >
            Si el contenedor no arranca, restaura la imagen anterior
          </Typography.Text>
        </Space>
        <Space direction="vertical" size={4} style={{ width: "100%" }}>
          <Typography.Text strong>
            🛡️ Overrides de la guarda (opcional)
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            Vacío = usar el valor global definido en la pestaña Actualizaciones.
          </Typography.Text>
          <Flex gap={8} wrap="wrap">
            <Space direction="vertical" size={2}>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                Máximo de intentos de arranque
              </Typography.Text>
              <InputNumber
                aria-label="Override máximo de intentos de arranque"
                value={defMaxStartAttempts}
                onChange={(v) =>
                  setDefMaxStartAttempts(v == null ? null : Number(v))
                }
                min={1}
                max={10}
                placeholder="Global"
                style={{ width: 140 }}
              />
            </Space>
            <Space direction="vertical" size={2}>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                Ventana de salud (s)
              </Typography.Text>
              <InputNumber
                aria-label="Override ventana de salud (s)"
                value={defHealthWindow}
                onChange={(v) =>
                  setDefHealthWindow(v == null ? null : Number(v))
                }
                min={5}
                max={600}
                placeholder="Global"
                style={{ width: 140 }}
              />
            </Space>
          </Flex>
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
  );
}
