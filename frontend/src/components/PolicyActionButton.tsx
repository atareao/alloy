import { useState } from "react";
import {
  Button,
  Flex,
  InputNumber,
  Modal,
  Select,
  Space,
  Switch,
  Typography,
} from "antd";
import type { ButtonProps } from "antd";
import { SettingOutlined } from "@ant-design/icons";
import { apiFetch } from "../api";
import type { UpdatePolicy } from "../types";

interface PolicyActionButtonProps {
  containerName: string;
  getPolicy: (name: string) => UpdatePolicy | undefined;
  setPolicies: React.Dispatch<React.SetStateAction<UpdatePolicy[]>>;
  busy: boolean;
  showToast: (message: string, color: string) => void;
  size?: ButtonProps["size"];
}

export default function PolicyActionButton({
  containerName,
  busy,
  showToast,
  getPolicy,
  setPolicies,
  size = "small",
}: PolicyActionButtonProps) {
  const [showPolicyModal, setShowPolicyModal] = useState(false);
  const [editAction, setEditAction] = useState<string>("pull-restart");
  const [editCleanup, setEditCleanup] = useState(false);
  const [editRollback, setEditRollback] = useState(false);
  const [editNotifyEvents, setEditNotifyEvents] = useState(true);
  // Overrides opcionales de la guarda (null = usar el valor global).
  const [editMaxStartAttempts, setEditMaxStartAttempts] = useState<number | null>(
    null,
  );
  const [editHealthWindow, setEditHealthWindow] = useState<number | null>(null);
  const [savingPolicy, setSavingPolicy] = useState(false);

  const policy = getPolicy(containerName);

  const openConfig = () => {
    setEditAction(policy?.action || "pull-restart");
    setEditCleanup(policy?.cleanup_old_image || false);
    setEditRollback(policy?.rollback_on_failure || false);
    setEditNotifyEvents(policy?.notify_events ?? true);
    setEditMaxStartAttempts(policy?.max_start_attempts ?? null);
    setEditHealthWindow(policy?.health_window_secs ?? null);
    setShowPolicyModal(true);
  };

  const savePolicy = async () => {
    setSavingPolicy(true);
    try {
      const res = await apiFetch(
        `/api/update-policies/${encodeURIComponent(containerName)}`,
        {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: editAction,
            cleanup_old_image: editCleanup,
            rollback_on_failure: editRollback,
            notify_events: editNotifyEvents,
            // Siempre presentes para no resetear overrides existentes (null = sin override).
            max_start_attempts: editMaxStartAttempts,
            health_window_secs: editHealthWindow,
          }),
        },
      );
      if (res.ok) {
        const updated: UpdatePolicy = await res.json();
        setPolicies((prev) => {
          const next = prev.filter((p) => p.container !== containerName);
          next.push(updated);
          return next;
        });
        setShowPolicyModal(false);
        showToast(`⚙️ Política de ${containerName} actualizada ✅`, "green");
      } else {
        const err = await res.text().catch(() => "Error desconocido");
        showToast(`⚙️ Error al guardar política: ${err}`, "red");
      }
    } catch {
      showToast(`⚙️ Error al guardar política`, "red");
    }
    setSavingPolicy(false);
  };

  return (
    <>
      <Button
        size={size}
        type="default"
        icon={<SettingOutlined />}
        onClick={openConfig}
        disabled={busy}
      >
        Configurar
      </Button>
      <Modal
        open={showPolicyModal}
        onCancel={() => setShowPolicyModal(false)}
        title={
          <span>
            <SettingOutlined /> Política: {containerName}
          </span>
        }
        width={520}
        footer={null}
      >
        <Space direction="vertical" size="small" style={{ width: "100%" }}>
          <Typography.Text
            type="secondary"
            style={{ fontSize: 14, marginBottom: 8 }}
          >
            Configura qué hacer cuando haya una actualización disponible para
            este contenedor.
          </Typography.Text>
          <div>
            <Typography.Text
              style={{ fontSize: 14, display: "block", marginBottom: 4 }}
            >
              Acción
            </Typography.Text>
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
              value={editAction}
              onChange={(v) => v && setEditAction(v)}
              style={{ width: "100%" }}
            />
          </div>
          <div>
            <Switch
              checked={editCleanup}
              onChange={(checked) => setEditCleanup(checked)}
            />
            <Typography.Text style={{ marginLeft: 8, fontSize: 14 }}>
              🧹 Borrar imagen anterior
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ display: "block", fontSize: 12, marginLeft: 36 }}
            >
              Elimina la imagen anterior después de actualizar
            </Typography.Text>
          </div>
          <div>
            <Switch
              checked={editRollback}
              onChange={(checked) => setEditRollback(checked)}
            />
            <Typography.Text style={{ marginLeft: 8, fontSize: 14 }}>
              ↩️ Rollback si falla
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ display: "block", fontSize: 12, marginLeft: 36 }}
            >
              Si el contenedor no arranca correctamente, restaura la imagen
              anterior
            </Typography.Text>
          </div>
          <div>
            <Switch
              checked={editNotifyEvents}
              onChange={(checked) => setEditNotifyEvents(checked)}
            />
            <Typography.Text style={{ marginLeft: 8, fontSize: 14 }}>
              🔔 Notificar eventos
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ display: "block", fontSize: 12, marginLeft: 36 }}
            >
              Envía notificación (Telegram/Matrix) cuando el contenedor cambie
              de estado
            </Typography.Text>
          </div>
          <div>
            <Typography.Text style={{ fontSize: 14, display: "block", marginBottom: 4 }}>
              🛡️ Overrides de la guarda (opcional)
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ display: "block", fontSize: 12, marginBottom: 8 }}
            >
              Vacío = usar el valor global definido en Configuración.
            </Typography.Text>
            <Space direction="vertical" size={8} style={{ width: "100%" }}>
              <Flex gap={8} wrap="wrap">
                <Space direction="vertical" size={2}>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    Máximo de intentos de arranque
                  </Typography.Text>
                  <InputNumber
                    aria-label="Máximo de intentos de arranque"
                    value={editMaxStartAttempts}
                    onChange={(v) =>
                      setEditMaxStartAttempts(v == null ? null : Number(v))
                    }
                    min={1}
                    max={10}
                    placeholder="Global"
                    style={{ width: 180 }}
                  />
                </Space>
                <Space direction="vertical" size={2}>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    Ventana de salud (s)
                  </Typography.Text>
                  <InputNumber
                    aria-label="Ventana de salud (s)"
                    value={editHealthWindow}
                    onChange={(v) =>
                      setEditHealthWindow(v == null ? null : Number(v))
                    }
                    min={5}
                    max={600}
                    placeholder="Global"
                    style={{ width: 180 }}
                  />
                </Space>
              </Flex>
            </Space>
          </div>
          <Flex justify="flex-end" style={{ marginTop: 16 }}>
            <Button
              onClick={() => setShowPolicyModal(false)}
              style={{ marginRight: 8 }}
            >
              Cancelar
            </Button>
            <Button type="primary" onClick={savePolicy} loading={savingPolicy}>
              Guardar política
            </Button>
          </Flex>
        </Space>
      </Modal>
    </>
  );
}
