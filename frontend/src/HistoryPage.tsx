import { useState } from "react";
import { useMediaQuery } from "./useMediaQuery";
import {
  Button,
  Card,
  Divider,
  Flex,
  Modal,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { DeleteOutlined, FileTextOutlined } from "@ant-design/icons";
import { apiFetch } from "./api";

// ═══════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════

interface HistoryEntry {
  container: string;
  image: string;
  old_digest: string;
  new_digest: string;
  timestamp: string;
  status: string;
  duration_ms: number;
}

// ═══════════════════════════════════════════════════════════════
// Page: History (histórico de updates)
// ═══════════════════════════════════════════════════════════════

interface HistoryPageProps {
  history: HistoryEntry[];
  setHistory: (h: HistoryEntry[]) => void;
}

export default function HistoryPage({ history, setHistory }: HistoryPageProps) {
  const isMobile = useMediaQuery("(max-width: 768px)");
  const [clearing, setClearing] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const handleClear = async () => {
    setClearing(true);
    try {
      await apiFetch("/api/history", { method: "DELETE" });
      setHistory([]);
    } catch {
      /* ignore */
    }
    setClearing(false);
    setConfirmClear(false);
  };

  const formatStatus = (status: string): string => {
    const map: Record<string, string> = {
      "update-check-restart": "🔄 update-check",
      "update-check-pull": "📥 update-check",
    };
    return map[status] || status;
  };

  const isSuccess = (status: string) => {
    const s = status.toLowerCase();
    if (
      s === "success" ||
      s === "ok" ||
      s === "done" ||
      s.startsWith("✅") ||
      s.startsWith("✔️") ||
      s.startsWith("🤖 auto-updated") ||
      s.includes("actualizado") ||
      s.includes("descargado") ||
      s.includes("pulled") ||
      s.includes("ya actualizado") ||
      s.includes("updated") ||
      s.includes("update-check")
    ) {
      return true;
    }
    if (
      s === "failed" ||
      s === "error" ||
      s.startsWith("❌") ||
      s.includes("error") ||
      s.includes("falló") ||
      s.includes("err")
    ) {
      return false;
    }
    return false;
  };

  const statusColor = (status: string) => {
    const s = status.toLowerCase();
    if (s === "skipped") return "yellow";
    return isSuccess(status) ? "green" : "red";
  };

  const statusBg = (status: string) => {
    if (status.toLowerCase() === "skipped") return undefined;
    return isSuccess(status)
      ? "var(--ant-color-success-bg)"
      : "var(--ant-color-error-bg)";
  };

  const statusTooltipLabel = (entry: HistoryEntry): string => {
    const parts: string[] = [];
    const s = entry.status;

    // What action was performed
    if (s === "success" || s === "✅ success") {
      parts.push("✅ Actualización completada con éxito");
    } else if (s === "failed" || s === "❌ failed") {
      parts.push("❌ La actualización falló");
    } else if (s.startsWith("update-check-restart")) {
      parts.push("🔄 Comprobación: reinicio necesario");
    } else if (s.startsWith("update-check-pull")) {
      parts.push("📥 Comprobación: nueva imagen disponible");
    } else if (s.startsWith("🤖 auto-updated")) {
      parts.push("🤖 Auto-actualizado correctamente");
    } else if (s.includes("skipped")) {
      parts.push("⏭️ Actualización omitida");
    } else if (s.includes("ya actualizado")) {
      parts.push("✅ Ya estaba actualizado");
    } else if (s.includes("error") || s.includes("falló")) {
      parts.push("❌ Error durante la actualización");
    } else if (s.includes("pulled") || s.includes("descargado")) {
      parts.push("📥 Imagen descargada");
    } else if (s.includes("updated")) {
      parts.push("✅ Actualizado");
    } else {
      parts.push(`📋 ${s}`);
    }

    // Duration
    parts.push(`⏱️ Duración: ${formatDuration(entry.duration_ms)}`);

    // Digest change (only if they differ)
    if (
      entry.old_digest &&
      entry.new_digest &&
      entry.old_digest !== entry.new_digest
    ) {
      parts.push(
        `📦 ${shortDigest(entry.old_digest)} → ${shortDigest(entry.new_digest)}`,
      );
    }

    // Timestamp
    parts.push(`🕐 ${formatDate(entry.timestamp)}`);

    return parts.join("\n");
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  };

  const formatDate = (ts: string) => {
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  };

  const shortDigest = (d: string | undefined) => {
    if (!d) return "-";
    return d.length > 20 ? d.substring(0, 20) + "..." : d;
  };

  const tagColorMap: Record<string, string> = {
    green: "success",
    red: "error",
    yellow: "warning",
  };

  // ── Table columns ────────────────────────────────────────────
  const columns = [
    {
      title: "Container",
      dataIndex: "container",
      key: "container",
      render: (text: string) => (
        <Typography.Text strong>{text}</Typography.Text>
      ),
    },
    {
      title: "Imagen",
      dataIndex: "image",
      key: "image",
      render: (text: string) => (
        <Tooltip title={text}>
          <Typography.Text
            type="secondary"
            style={{
              maxWidth: 250,
              overflow: "hidden",
              textOverflow: "ellipsis",
              display: "inline-block",
              fontSize: 12,
            }}
          >
            {text}
          </Typography.Text>
        </Tooltip>
      ),
    },
    {
      title: "Versión anterior",
      dataIndex: "old_digest",
      key: "old_digest",
      render: (text: string) => (
        <Typography.Text
          type="secondary"
          style={{ fontFamily: "monospace", fontSize: 12 }}
        >
          {shortDigest(text)}
        </Typography.Text>
      ),
    },
    {
      title: "Nueva versión",
      dataIndex: "new_digest",
      key: "new_digest",
      render: (text: string) => (
        <Typography.Text
          type="secondary"
          style={{ fontFamily: "monospace", fontSize: 12 }}
        >
          {shortDigest(text)}
        </Typography.Text>
      ),
    },
    {
      title: "Estado",
      key: "status",
      render: (_: unknown, record: HistoryEntry) => {
        const c = statusColor(record.status);
        return (
          <Tooltip
            title={statusTooltipLabel(record)}
            overlayStyle={{ maxWidth: 320 }}
          >
            <Tag
              color={tagColorMap[c] || "default"}
              style={{ cursor: "pointer" }}
            >
              {formatStatus(record.status)}
            </Tag>
          </Tooltip>
        );
      },
    },
    {
      title: "Duración",
      dataIndex: "duration_ms",
      key: "duration_ms",
      render: (text: number) => (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {formatDuration(text)}
        </Typography.Text>
      ),
    },
    {
      title: "Fecha",
      dataIndex: "timestamp",
      key: "timestamp",
      render: (text: string) => (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {formatDate(text)}
        </Typography.Text>
      ),
    },
  ];

  const dataSource = history.map((entry, i) => ({ ...entry, key: i }));

  // ── Mobile card ─────────────────────────────────────────────
  const renderMobileCard = (entry: HistoryEntry, i: number) => (
    <Card
      key={i}
      size="small"
      bordered
      style={
        entry.status.toLowerCase() !== "skipped"
          ? { background: statusBg(entry.status) }
          : undefined
      }
    >
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <Flex justify="space-between" wrap="nowrap" align="center">
          <Typography.Text strong ellipsis style={{ flex: 1, fontSize: 14 }}>
            {entry.container}
          </Typography.Text>
          <Tooltip
            title={statusTooltipLabel(entry)}
            overlayStyle={{ maxWidth: 320 }}
          >
            <Tag color={tagColorMap[statusColor(entry.status)] || "default"}>
              {formatStatus(entry.status)}
            </Tag>
          </Tooltip>
        </Flex>
        <Divider />
        <Space direction="vertical" size={2} style={{ width: "100%" }}>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Imagen:
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {entry.image}
            </Typography.Text>
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Anterior:
            </Typography.Text>
            <Typography.Text style={{ fontFamily: "monospace", fontSize: 12 }}>
              {shortDigest(entry.old_digest)}
            </Typography.Text>
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Nueva:
            </Typography.Text>
            <Typography.Text style={{ fontFamily: "monospace", fontSize: 12 }}>
              {shortDigest(entry.new_digest)}
            </Typography.Text>
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Duración:
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {formatDuration(entry.duration_ms)}
            </Typography.Text>
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Fecha:
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {formatDate(entry.timestamp)}
            </Typography.Text>
          </Flex>
        </Space>
      </Space>
    </Card>
  );

  return (
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      <Card bordered size="small">
        <Flex justify="space-between" align="center">
          <Typography.Text type="secondary">
            <FileTextOutlined /> Histórico de actualizaciones · {history.length}{" "}
            entradas
          </Typography.Text>
          {history.length > 0 && (
            <Button
              onClick={() => setConfirmClear(true)}
              danger
              type="primary"
              size={isMobile ? "small" : "middle"}
              icon={<DeleteOutlined />}
            >
              Limpiar
            </Button>
          )}
        </Flex>
      </Card>

      <Modal
        open={confirmClear}
        onCancel={() => setConfirmClear(false)}
        title={
          <>
            <DeleteOutlined /> Limpiar historial
          </>
        }
        width={400}
        footer={
          <Flex justify="flex-end" gap={8}>
            <Button onClick={() => setConfirmClear(false)}>Cancelar</Button>
            <Button
              danger
              type="primary"
              onClick={handleClear}
              loading={clearing}
            >
              Eliminar todo
            </Button>
          </Flex>
        }
      >
        <Typography.Text>
          ¿Estás seguro de que deseas eliminar todo el historial de
          actualizaciones? Esta acción no se puede deshacer.
        </Typography.Text>
      </Modal>

      {history.length === 0 ? (
        <Card bordered>
          <Typography.Text
            type="secondary"
            style={{ textAlign: "center", display: "block" }}
          >
            No hay historial de actualizaciones. Cuando se actualice un
            container, aparecerá aquí.
          </Typography.Text>
        </Card>
      ) : isMobile ? (
        <Space direction="vertical" size="small" style={{ width: "100%" }}>
          {history.map((entry, i) => renderMobileCard(entry, i))}
        </Space>
      ) : (
        <Card bordered styles={{ body: { padding: 0 } }}>
          <Table
            columns={columns}
            dataSource={dataSource}
            scroll={{ x: "max-content" }}
            pagination={false}
          />
        </Card>
      )}
    </Space>
  );
}
