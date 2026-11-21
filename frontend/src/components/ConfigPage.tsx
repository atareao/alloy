import { Alert, Space, Tabs } from "antd";
import type { AppConfig } from "../types";
import { useConfigPage } from "../useConfigPage";
import { useUpdateCheckConfig } from "../useUpdateCheckConfig";
import { useDefaultPolicy } from "../useDefaultPolicy";
import NotificationsTab from "./NotificationsTab";
import UpdatesTab from "./UpdatesTab";
import InfoTab from "./InfoTab";

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
  const {
    saving,
    setSaving,
    testing,
    setTesting,
    error,
    setError,
    success,
    setSuccess,
    activeTab,
    setActiveTab,
    showSuccess,
  } = useConfigPage();

  // Estado de las Cards de la pestaña Actualizaciones, elevado a nivel de página
  // para que sus GET de arranque se disparen al montar `ConfigPage` (los paneles
  // de antd `Tabs` se montan en diferido).
  const updateCheck = useUpdateCheckConfig();
  const defaultPolicy = useDefaultPolicy();

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
              <NotificationsTab
                configProp={configProp}
                setConfigProp={setConfigProp}
                saving={saving}
                setSaving={setSaving}
                testing={testing}
                setTesting={setTesting}
                setError={setError}
                showSuccess={showSuccess}
              />
            ),
          },
          {
            key: "updates",
            label: "⬆️ Actualizaciones",
            children: (
              <UpdatesTab
                configProp={configProp}
                updateCheck={updateCheck}
                defaultPolicy={defaultPolicy}
                saving={saving}
                setSaving={setSaving}
                setError={setError}
                showSuccess={showSuccess}
              />
            ),
          },
          {
            key: "info",
            label: "ℹ️ Información",
            children: (
              <InfoTab
                configProp={configProp}
                colorScheme={colorScheme}
                setColorScheme={setColorScheme}
                setError={setError}
                showSuccess={showSuccess}
              />
            ),
          },
        ]}
      />
    </Space>
  );
}
