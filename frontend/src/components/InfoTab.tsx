import { Space } from "antd";
import type { AppConfig } from "../types";
import AppInfoCard from "./AppInfoCard";
import ThemeCard from "./ThemeCard";
import ExportImportCard from "./ExportImportCard";

interface InfoTabProps {
  configProp: AppConfig | null;
  colorScheme: "dark" | "light";
  setColorScheme: (scheme: "dark" | "light") => void;
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Pestaña «ℹ️ Información»: datos de la app, tema y export/import. */
export default function InfoTab({
  configProp,
  colorScheme,
  setColorScheme,
  setError,
  showSuccess,
}: InfoTabProps) {
  return (
    <Space
      direction="vertical"
      size="middle"
      style={{ width: "100%", paddingTop: 16 }}
    >
      {/* ═══ Versión e información ═══ */}
      <AppInfoCard configProp={configProp} />

      {/* ═══ Tema ═══ */}
      <ThemeCard colorScheme={colorScheme} setColorScheme={setColorScheme} />

      {/* ═══ Export / Import ═══ */}
      <ExportImportCard setError={setError} showSuccess={showSuccess} />
    </Space>
  );
}
