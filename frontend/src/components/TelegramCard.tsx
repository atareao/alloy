import { useState } from "react";
import { Button, Card, Flex, Input, Space, Switch, Typography } from "antd";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";
import { useConfigSync } from "../useConfigSync";
import type { SavingKey, TestingKey } from "../useConfigPage";

interface TelegramCardProps {
  configProp: AppConfig | null;
  setConfigProp: (c: AppConfig) => void;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  testing: TestingKey | null;
  setTesting: (key: TestingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card de configuración del canal de notificaciones Telegram. */
export default function TelegramCard({
  configProp,
  setConfigProp,
  saving,
  setSaving,
  testing,
  setTesting,
  setError,
  showSuccess,
}: TelegramCardProps) {
  const [tgToken, setTgToken] = useState("");
  const [tgChatId, setTgChatId] = useState("");
  const [tgTopicId, setTgTopicId] = useState("");
  const [tgEnabled, setTgEnabled] = useState(false);

  // Sync from props when config changes (from App.tsx eager fetch). Tokens are
  // never preloaded from the API (they are secret and no longer returned in
  // clear). The field starts empty; leaving it empty on save means "keep the
  // stored token". A partial config stays editable: enable the switch if the
  // channel is fully configured, if a token is stored, or if any non-secret
  // field is present. `useConfigSync` adjusts the state during render.
  useConfigSync(configProp, (config) => {
    setTgToken("");
    setTgChatId(config.telegram_chat_id || "");
    setTgTopicId(config.telegram_topic_id || "");
    setTgEnabled(
      config.telegram_configured ||
        config.telegram_token_set ||
        !!config.telegram_chat_id,
    );
  });

  const saveTelegram = async () => {
    setSaving("telegram");
    setError(null);
    try {
      const body: Record<string, string | null> = {
        telegram_chat_id: tgChatId || null,
        // Empty string means "delete the stored topic"; null would mean
        // "keep current" (field absent) on the backend.
        telegram_topic_id: tgTopicId.trim() || "",
      };
      if (!tgEnabled) {
        // Channel off ⇒ send "" so the backend deletes the stored token.
        body.telegram_token = "";
        body.telegram_chat_id = "";
        body.telegram_topic_id = "";
      } else if (tgToken.trim()) {
        // Only send when a new token was typed; empty (or blank) means
        // "keep current".
        body.telegram_token = tgToken.trim();
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data: AppConfig = await res.json();
        setConfigProp(data);
        setTgToken("");
        setTgChatId(data.telegram_chat_id || "");
        setTgTopicId(data.telegram_topic_id || "");
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

  const testTelegram = async () => {
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
  };

  return (
    <Card bordered size="small">
      <Flex
        justify="space-between"
        align="center"
        style={{ marginBottom: 16 }}
      >
        <Typography.Title level={4} style={{ margin: 0 }}>
          📱 Telegram
        </Typography.Title>
        <Flex align="center" gap={8}>
          <Switch
            aria-label="Activar Telegram"
            checked={tgEnabled}
            onChange={(checked) => setTgEnabled(checked)}
          />
          <Typography.Text>
            {tgEnabled ? "Activado" : "Desactivado"}
          </Typography.Text>
        </Flex>
      </Flex>
      {tgEnabled && !configProp?.telegram_configured && (
        <Typography.Text
          type="warning"
          style={{ display: "block", marginBottom: 12 }}
        >
          Faltan campos para completar la configuración
        </Typography.Text>
      )}
      {tgEnabled && (
        <Space direction="vertical" size="middle" style={{ width: "100%" }}>
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>Token del Bot</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Token que te proporciona @BotFather
            </Typography.Text>
            <Input.Password
              aria-label="Token del bot de Telegram"
              placeholder={
                configProp?.telegram_configured
                  ? "•••• configurado"
                  : "123456:ABC-DEF..."
              }
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
          <Space direction="vertical" size={4} style={{ width: "100%" }}>
            <Typography.Text strong>Topic ID</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              ID del topic del foro (opcional, para supergrupos)
            </Typography.Text>
            <Input
              aria-label="Topic ID (opcional)"
              placeholder="42"
              inputMode="numeric"
              value={tgTopicId}
              onChange={(e) => setTgTopicId(e.target.value.replace(/\D/g, ""))}
            />
          </Space>
        </Space>
      )}
      <Flex justify="flex-end" gap="middle" style={{ marginTop: 16 }}>
        {tgEnabled && (
          <Button
            onClick={testTelegram}
            loading={testing === "telegram"}
            style={{
              borderColor: "var(--ant-color-success)",
              color: "var(--ant-color-success)",
            }}
          >
            📤 Test
          </Button>
        )}
        <Button
          onClick={saveTelegram}
          loading={saving === "telegram"}
          type="primary"
        >
          Guardar Telegram
        </Button>
      </Flex>
    </Card>
  );
}
