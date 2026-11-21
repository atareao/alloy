import { useEffect, useState } from "react";
import { Button, Card, Flex, Input, Space, Switch, Typography } from "antd";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";
import type { SavingKey, TestingKey } from "../useConfigPage";

interface ValetCardProps {
  configProp: AppConfig | null;
  setConfigProp: (c: AppConfig) => void;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  testing: TestingKey | null;
  setTesting: (key: TestingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card de configuración del canal de notificaciones Valet. */
export default function ValetCard({
  configProp,
  setConfigProp,
  saving,
  setSaving,
  testing,
  setTesting,
  setError,
  showSuccess,
}: ValetCardProps) {
  const [valetUrl, setValetUrl] = useState("");
  const [valetToken, setValetToken] = useState("");
  const [valetEnabled, setValetEnabled] = useState(false);

  // Sync from props when config changes (from App.tsx eager fetch). Tokens are
  // never preloaded from the API (they are secret and no longer returned in
  // clear). The field starts empty; leaving it empty on save means "keep the
  // stored token". A partial config stays editable: enable the switch if the
  // channel is fully configured, if a token is stored, or if any non-secret
  // field is present.
  useEffect(() => {
    if (!configProp) return;
    setValetUrl(configProp.valet_url || "");
    setValetToken("");
    setValetEnabled(
      configProp.valet_configured ||
        configProp.valet_token_set ||
        !!configProp.valet_url,
    );
  }, [configProp]);

  const saveValet = async () => {
    setSaving("valet");
    setError(null);
    try {
      const body: Record<string, string | null> = {
        valet_url: valetUrl || null,
      };
      if (!valetEnabled) {
        body.valet_url = "";
        body.valet_token = "";
      } else if (valetToken.trim()) {
        body.valet_token = valetToken.trim();
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data: AppConfig = await res.json();
        setConfigProp(data);
        setValetUrl(data.valet_url || "");
        setValetToken("");
        showSuccess(
          valetEnabled ? "✅ Valet configurado" : "❌ Valet desactivado",
        );
      } else {
        setError("Error al guardar Valet");
      }
    } catch {
      setError("Error de conexión al guardar Valet");
    }
    setSaving(null);
  };

  const testValet = async () => {
    setTesting("valet");
    setError(null);
    try {
      const res = await apiFetch("/api/notifications/test", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ channel: "valet" }),
      });
      setTesting(null);
      if (res.ok) {
        showSuccess("📤 Mensaje de prueba enviado a Valet");
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
          🪐 Valet
        </Typography.Title>
        <Flex align="center" gap={8}>
          <Switch
            aria-label="Activar Valet"
            checked={valetEnabled}
            onChange={(checked) => setValetEnabled(checked)}
          />
          <Typography.Text>
            {valetEnabled ? "Activado" : "Desactivado"}
          </Typography.Text>
        </Flex>
      </Flex>
      {valetEnabled && !configProp?.valet_configured && (
        <Typography.Text
          type="warning"
          style={{ display: "block", marginBottom: 12 }}
        >
          Faltan campos para completar la configuración
        </Typography.Text>
      )}
      {valetEnabled && (
        <Space direction="vertical" size="middle" style={{ width: "100%" }}>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>URL</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Base URL de Valet (sin /api/notifications)
            </Typography.Text>
            <Input
              aria-label="URL de Valet"
              placeholder="https://valet.example.com"
              value={valetUrl}
              onChange={(e) => setValetUrl(e.target.value)}
            />
          </Space>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>Ingest key</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Ingest key de Valet
            </Typography.Text>
            <Input.Password
              aria-label="Ingest key de Valet"
              placeholder={
                configProp?.valet_configured ? "•••• configurado" : "val_..."
              }
              value={valetToken}
              onChange={(e) => setValetToken(e.target.value)}
            />
          </Space>
        </Space>
      )}
      <Flex justify="flex-end" gap="middle" style={{ marginTop: 16 }}>
        {valetEnabled && (
          <Button
            onClick={testValet}
            loading={testing === "valet"}
            style={{
              borderColor: "var(--ant-color-success)",
              color: "var(--ant-color-success)",
            }}
          >
            📤 Test
          </Button>
        )}
        <Button
          onClick={saveValet}
          loading={saving === "valet"}
          type="primary"
        >
          Guardar Valet
        </Button>
      </Flex>
    </Card>
  );
}
