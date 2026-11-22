import { Space } from "antd";
import type { AppConfig } from "../types";
import type { SavingKey, TestingKey } from "../useConfigPage";
import TelegramCard from "./TelegramCard";
import MatrixCard from "./MatrixCard";
import ValetCard from "./ValetCard";

interface NotificationsTabProps {
  configProp: AppConfig | null;
  setConfigProp: (c: AppConfig) => void;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  testing: TestingKey | null;
  setTesting: (key: TestingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Pestaña «🔔 Notificaciones»: canales Telegram, Matrix y Valet. */
export default function NotificationsTab({
  configProp,
  setConfigProp,
  saving,
  setSaving,
  testing,
  setTesting,
  setError,
  showSuccess,
}: NotificationsTabProps) {
  return (
    <Space
      direction="vertical"
      size="middle"
      style={{ width: "100%", paddingTop: 16 }}
    >
      {/* ═══ Telegram ═══ */}
      <TelegramCard
        configProp={configProp}
        setConfigProp={setConfigProp}
        saving={saving}
        setSaving={setSaving}
        testing={testing}
        setTesting={setTesting}
        setError={setError}
        showSuccess={showSuccess}
      />

      {/* ═══ Matrix ═══ */}
      <MatrixCard
        configProp={configProp}
        setConfigProp={setConfigProp}
        saving={saving}
        setSaving={setSaving}
        testing={testing}
        setTesting={setTesting}
        setError={setError}
        showSuccess={showSuccess}
      />

      {/* ═══ Valet ═══ */}
      <ValetCard
        configProp={configProp}
        setConfigProp={setConfigProp}
        saving={saving}
        setSaving={setSaving}
        testing={testing}
        setTesting={setTesting}
        setError={setError}
        showSuccess={showSuccess}
      />
    </Space>
  );
}
