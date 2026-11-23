import { Tabs } from "antd";
import type { AppConfig } from "../types";
import type { SavingKey } from "../useConfigPage";
import type { UseUpdateCheckConfigResult } from "../useUpdateCheckConfig";
import type { UseDefaultPolicyResult } from "../useDefaultPolicy";
import UpdateCheckCard from "./UpdateCheckCard";
import DefaultPolicyCard from "./DefaultPolicyCard";

interface UpdatesTabProps {
  configProp: AppConfig | null;
  /** Estado elevado desde `ConfigPage` (fetch al montar la página). */
  updateCheck: UseUpdateCheckConfigResult;
  defaultPolicy: UseDefaultPolicyResult;
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Pestaña «⬆️ Actualizaciones»: revisión programada y política por defecto. */
export default function UpdatesTab({
  configProp,
  updateCheck,
  defaultPolicy,
  saving,
  setSaving,
  setError,
  showSuccess,
}: UpdatesTabProps) {
  return (
    <Tabs
      tabPosition="left"
      style={{ width: "100%", paddingTop: 16 }}
      items={[
        {
          key: "revision",
          label: "Revisión",
          children: (
            <UpdateCheckCard
              configProp={configProp}
              updateCheck={updateCheck}
              saving={saving}
              setSaving={setSaving}
              setError={setError}
              showSuccess={showSuccess}
            />
          ),
        },
        {
          key: "policy",
          label: "Política",
          children: (
            <DefaultPolicyCard
              defaultPolicy={defaultPolicy}
              saving={saving}
              setSaving={setSaving}
              setError={setError}
              showSuccess={showSuccess}
            />
          ),
        },
      ]}
    />
  );
}
