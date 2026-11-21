import { Button, Card, Flex, Typography } from "antd";
import { apiFetch } from "../api";

interface ExportImportCardProps {
  setError: (msg: string | null) => void;
  showSuccess: (msg: string) => void;
}

/** Card «📦 Exportar / Importar configuración» (pestaña Información). */
export default function ExportImportCard({
  setError,
  showSuccess,
}: ExportImportCardProps) {
  return (
    <Card bordered size="small">
      <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
        📦 Exportar / Importar configuración
      </Typography.Title>
      <Typography.Text
        type="secondary"
        style={{ display: "block", marginBottom: 16 }}
      >
        Exporta alertas, programaciones y ajustes a un archivo JSON. Puedes
        importarlo después para restaurar la configuración.
      </Typography.Text>
      <Typography.Text
        type="warning"
        style={{ display: "block", marginBottom: 16 }}
      >
        ⚠️ Los tokens de notificación (Telegram, Matrix, Valet) y la URL del
        webhook no se exportan por seguridad; deberás reintroducirlos tras
        importar.
      </Typography.Text>
      <Flex gap="middle">
        <Button
          type="primary"
          onClick={async () => {
            try {
              const res = await apiFetch("/api/admin/export");
              const data = await res.json();
              const blob = new Blob([JSON.stringify(data, null, 2)], {
                type: "application/json",
              });
              const url = URL.createObjectURL(blob);
              const a = document.createElement("a");
              a.href = url;
              a.download = `alloy-config-${new Date().toISOString().slice(0, 10)}.json`;
              a.click();
              URL.revokeObjectURL(url);
              showSuccess("✅ Configuración exportada");
            } catch {
              setError("Error al exportar configuración");
            }
          }}
        >
          📤 Exportar
        </Button>
        <Button
          style={{
            borderColor: "var(--ant-color-warning)",
            color: "var(--ant-color-warning)",
          }}
          onClick={() => {
            const input = document.createElement("input");
            input.type = "file";
            input.accept = ".json";
            input.onchange = async (e) => {
              const file = (e.target as HTMLInputElement).files?.[0];
              if (!file) return;
              try {
                const text = await file.text();
                const data = JSON.parse(text);
                const res = await apiFetch("/api/admin/import", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                    alerts: data.alerts || [],
                    schedules: data.schedules || [],
                    settings: data.settings || {},
                  }),
                });
                if (res.ok) {
                  showSuccess(
                    "✅ Configuración importada. Recarga la página.",
                  );
                  setTimeout(() => window.location.reload(), 1500);
                } else {
                  const err = await res.text();
                  setError(`Error al importar: ${err}`);
                }
              } catch {
                setError("Archivo JSON inválido");
              }
            };
            input.click();
          }}
        >
          📥 Importar
        </Button>
      </Flex>
    </Card>
  );
}
