import { useState } from "react";
import { Button, Card, Flex, Input, Space, Switch, Typography } from "antd";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";
import { useConfigSync } from "../useConfigSync";
import type { SavingKey, TestingKey } from "../useConfigPage";

interface MatrixCardProps {
  configProp: AppConfig | null;
  setConfigProp: (c: AppConfig) => void;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  testing: TestingKey | null;
  setTesting: (key: TestingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card de configuración del canal de notificaciones Matrix. */
export default function MatrixCard({
  configProp,
  setConfigProp,
  saving,
  setSaving,
  testing,
  setTesting,
  setError,
  showSuccess,
}: MatrixCardProps) {
  const [mxHomeserver, setMxHomeserver] = useState("");
  const [mxToken, setMxToken] = useState("");
  const [mxRoom, setMxRoom] = useState("");
  const [mxEnabled, setMxEnabled] = useState(false);

  // Sync from props when config changes (from App.tsx eager fetch). Tokens are
  // never preloaded from the API (they are secret and no longer returned in
  // clear). The field starts empty; leaving it empty on save means "keep the
  // stored token". A partial config stays editable: enable the switch if the
  // channel is fully configured, if a token is stored, or if any non-secret
  // field is present. `useConfigSync` adjusts the state during render.
  useConfigSync(configProp, (config) => {
    setMxHomeserver(config.matrix_homeserver || "");
    setMxToken("");
    setMxRoom(config.matrix_room || "");
    setMxEnabled(
      config.matrix_configured ||
        config.matrix_token_set ||
        !!config.matrix_homeserver ||
        !!config.matrix_room,
    );
  });

  const saveMatrix = async () => {
    setSaving("matrix");
    setError(null);
    try {
      const body: Record<string, string | null> = {
        matrix_homeserver: mxHomeserver || null,
        matrix_room: mxRoom || null,
      };
      if (!mxEnabled) {
        body.matrix_homeserver = "";
        body.matrix_token = "";
        body.matrix_room = "";
      } else if (mxToken.trim()) {
        body.matrix_token = mxToken.trim();
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
        setMxToken("");
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

  const testMatrix = async () => {
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
  };

  return (
    <Card bordered size="small">
      <Flex
        justify="space-between"
        align="center"
        style={{ marginBottom: 16 }}
      >
        <Typography.Title level={4} style={{ margin: 0 }}>
          💬 Matrix
        </Typography.Title>
        <Flex align="center" gap={8}>
          <Switch
            aria-label="Activar Matrix"
            checked={mxEnabled}
            onChange={(checked) => setMxEnabled(checked)}
          />
          <Typography.Text>
            {mxEnabled ? "Activado" : "Desactivado"}
          </Typography.Text>
        </Flex>
      </Flex>
      {mxEnabled && !configProp?.matrix_configured && (
        <Typography.Text
          type="warning"
          style={{ display: "block", marginBottom: 12 }}
        >
          Faltan campos para completar la configuración
        </Typography.Text>
      )}
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
              aria-label="Access token de Matrix"
              placeholder={
                configProp?.matrix_configured ? "•••• configurado" : "syt_..."
              }
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
            onClick={testMatrix}
            loading={testing === "matrix"}
            style={{
              borderColor: "var(--ant-color-success)",
              color: "var(--ant-color-success)",
            }}
          >
            📤 Test
          </Button>
        )}
        <Button
          onClick={saveMatrix}
          loading={saving === "matrix"}
          type="primary"
        >
          Guardar Matrix
        </Button>
      </Flex>
    </Card>
  );
}
