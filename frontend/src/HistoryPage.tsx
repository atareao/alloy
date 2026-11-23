import { forwardRef, useEffect, useMemo, useState } from "react";
import { useMediaQuery } from "./useMediaQuery";
import {
  Button,
  Card,
  Divider,
  Flex,
  Input,
  Modal,
  Pagination,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { DeleteOutlined, FileTextOutlined } from "@ant-design/icons";
import { apiFetch } from "./api";
import EllipsisText from "./components/EllipsisText";
import {
  buildHistoryQuery,
  DEFAULT_HISTORY_PAGE_SIZE,
  historyRowKey,
  PAGE_SIZE_OPTIONS,
} from "./historyQuery";
import type { HistoryEntry, PagedHistory } from "./types";

// ═══════════════════════════════════════════════════════════════
// Page: History (histórico de updates)
// ═══════════════════════════════════════════════════════════════

/**
 * `<table>` a ancho natural (max-content) con un mínimo del 100%.
 *
 * La tabla se renderiza sin `scroll.x` (que forzaría a rc-table a inyectar una
 * fila oculta `ant-table-measure-row` duplicando el texto de las cabeceras) y el
 * scroll horizontal se delega a un contenedor con `overflow-x: auto`.
 */
const ContentTable = forwardRef<
  HTMLTableElement,
  React.ComponentPropsWithoutRef<"table">
>(({ style, ...rest }, ref) => (
  <table
    ref={ref}
    {...rest}
    style={{ ...style, width: "max-content", minWidth: "100%" }}
  />
));
ContentTable.displayName = "ContentTable";

type StatusLevel = "success" | "warning" | "error" | "default";

/**
 * Nivel de severidad explícito por estado: **fuente única de verdad** de la que
 * se derivan el color del tag, el fondo de la tarjeta móvil y el resto de
 * estados (vía la heurística `isSuccess`). Tenerlo en un único mapa evita que
 * tag y fondo diverjan (p. ej. `rolled-back` con fondo de error pero tag de
 * aviso — ver D5: `rolled-back` ⇒ warning, `failed-stopped` ⇒ error).
 */
const STATUS_LEVEL: Record<string, StatusLevel> = {
  "rolled-back": "warning",
  "failed-stopped": "error",
  skipped: "warning",
};

const TAG_TYPE: Record<StatusLevel, string> = {
  success: "success",
  warning: "warning",
  error: "error",
  default: "default",
};

const STATUS_BG: Record<StatusLevel, string | undefined> = {
  success: "var(--ant-color-success-bg)",
  warning: "var(--ant-color-warning-bg)",
  error: "var(--ant-color-error-bg)",
  default: undefined,
};

/**
 * Vocabulario de estados ofrecido por el filtro (D2): el ya presente en la
 * página (`success`, `error`, `rolled-back`, `failed-stopped` y la familia
 * `update-check-*`). El valor emitido es el estado crudo; la etiqueta es legible.
 */
const STATUS_FILTERS: { value: string; label: string }[] = [
  { value: "success", label: "Éxito" },
  { value: "error", label: "Con error" },
  { value: "rolled-back", label: "Rollback" },
  { value: "failed-stopped", label: "Fallo terminal" },
  { value: "update-check-restart", label: "Reinicio" },
  { value: "update-check-pull", label: "Nueva imagen" },
  { value: "update-check-stack", label: "Stack" },
];

function isSuccess(status: string): boolean {
  const s = status.toLowerCase();
  return (
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
  );
}

function statusLevel(status: string): StatusLevel {
  return (
    STATUS_LEVEL[status.toLowerCase()] ??
    (isSuccess(status) ? "success" : "error")
  );
}

function statusTagType(status: string): string {
  return TAG_TYPE[statusLevel(status)];
}

function statusBg(status: string): string | undefined {
  return STATUS_BG[statusLevel(status)];
}

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

const formatStatus = (status: string): string => {
  const map: Record<string, string> = {
    "update-check-restart": "🔄 update-check",
    "update-check-pull": "📥 update-check",
    "update-check-stack": "📦 update-check",
    "rolled-back": "↩️ Rollback aplicado",
    "failed-stopped": "🛑 Fallo terminal",
  };
  return map[status] || status;
};

function statusTooltipLabel(entry: HistoryEntry): string {
  const parts: string[] = [];
  const s = entry.status;

  // What action was performed
  if (s === "success" || s === "✅ success") {
    parts.push("✅ Actualización completada con éxito");
  } else if (s === "failed" || s === "❌ failed") {
    parts.push("❌ La actualización falló");
  } else if (s === "rolled-back") {
    parts.push("↩️ Rollback aplicado: se restauró la imagen anterior");
  } else if (s === "failed-stopped") {
    parts.push("🛑 Fallo terminal: el contenedor quedó parado");
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
    parts.push(`📦 ${entry.old_digest} → ${entry.new_digest}`);
  }

  // Timestamp
  parts.push(`🕐 ${formatDate(entry.timestamp)}`);

  return parts.join("\n");
}

export interface HistoryPageProps {
  /**
   * Señal de refresco: cambia al terminar un batch para forzar la recarga de la
   * página actual (D3). `App` ya no mantiene el array del histórico.
   */
  refreshToken?: number;
}

export default function HistoryPage({ refreshToken = 0 }: HistoryPageProps) {
  const isMobile = useMediaQuery("(max-width: 768px)");

  // ── Estado propio de la página (D3) ──────────────────────────
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_HISTORY_PAGE_SIZE);
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string[]>([]);
  const [items, setItems] = useState<HistoryEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [clearing, setClearing] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  // Debounce de la búsqueda (300 ms) antes de construir la query; al asentarse,
  // vuelve a la página 1.
  useEffect(() => {
    // No reprogramar en el montaje ni cuando el texto no cambia de verdad:
    // de lo contrario un `setPage(1)` tardío pisaría un cambio de página reciente.
    if (searchQuery === debouncedQuery) return;
    const t = setTimeout(() => {
      setDebouncedQuery(searchQuery);
      setPage(1);
    }, 300);
    return () => clearTimeout(t);
  }, [searchQuery, debouncedQuery]);

  const queryString = useMemo(
    () =>
      buildHistoryQuery({
        page,
        pageSize,
        q: debouncedQuery,
        status: statusFilter,
      }),
    [page, pageSize, debouncedQuery, statusFilter],
  );

  // Hay filtro activo cuando el texto asentado o algún estado están aplicados.
  const hasActiveFilters =
    debouncedQuery.trim() !== "" || statusFilter.length > 0;

  // La página es quien consulta al servidor; reconsulta al cambiar la query o la
  // señal de refresco.
  useEffect(() => {
    let cancelled = false;
    apiFetch(`/api/history?${queryString}`)
      .then((res) => res.json())
      .then((data: PagedHistory) => {
        if (cancelled) return;
        setItems(data.items);
        setTotal(data.total);
      })
      .catch(() => {
        /* ignore */
      });
    return () => {
      cancelled = true;
    };
  }, [queryString, refreshToken]);

  const handleClear = async () => {
    setClearing(true);
    try {
      await apiFetch("/api/history", { method: "DELETE" });
      setItems([]);
      setTotal(0);
      setSearchQuery("");
      setDebouncedQuery("");
      setStatusFilter([]);
      setPage(1);
    } catch {
      /* ignore */
    }
    setClearing(false);
    setConfirmClear(false);
  };

  const toggleStatus = (value: string) => {
    setStatusFilter((prev) =>
      prev.includes(value) ? prev.filter((s) => s !== value) : [...prev, value],
    );
    setPage(1);
  };

  // ── Table columns ────────────────────────────────────────────
  const columns = [
    {
      title: "Container",
      dataIndex: "container",
      key: "container",
      render: (text: string) => (
        <EllipsisText value={text} strong style={{ maxWidth: 260 }} />
      ),
    },
    {
      title: "Imagen",
      dataIndex: "image",
      key: "image",
      render: (text: string) => (
        <EllipsisText
          value={text}
          secondary
          style={{ fontSize: 12, maxWidth: 260 }}
        />
      ),
    },
    {
      title: "Versión",
      key: "version",
      render: (_: unknown, record: HistoryEntry) => (
        <Flex vertical gap={2} style={{ minWidth: 0 }}>
          <EllipsisText
            value={record.new_digest || "-"}
            mono
            style={{ maxWidth: 260 }}
          />
          <EllipsisText
            value={record.old_digest || "-"}
            mono
            secondary
            style={{ maxWidth: 260 }}
          />
        </Flex>
      ),
    },
    {
      title: "Estado",
      key: "status",
      render: (_: unknown, record: HistoryEntry) => {
        return (
          <Tooltip
            title={statusTooltipLabel(record)}
            overlayStyle={{ maxWidth: 320 }}
          >
            <Tag color={statusTagType(record.status)} style={{ cursor: "pointer" }}>
              {formatStatus(record.status)}
            </Tag>
          </Tooltip>
        );
      },
    },
    {
      title: "Intentos",
      dataIndex: "attempts",
      key: "attempts",
      render: (text: number) => (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {text}
        </Typography.Text>
      ),
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

  // ── Mobile card ─────────────────────────────────────────────
  const renderMobileCard = (entry: HistoryEntry) => (
    <Card
      key={historyRowKey(entry)}
      size="small"
      bordered
      style={
        entry.status.toLowerCase() !== "skipped"
          ? { background: statusBg(entry.status) }
          : undefined
      }
    >
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <Flex justify="space-between" wrap="nowrap" align="center" gap={8}>
          <EllipsisText
            strong
            value={entry.container}
            style={{ flex: 1, fontSize: 14 }}
          />
          <Tooltip
            title={statusTooltipLabel(entry)}
            overlayStyle={{ maxWidth: 320 }}
          >
            <Tag color={statusTagType(entry.status)}>
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
            <EllipsisText value={entry.image} style={{ fontSize: 12, flex: 1 }} />
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Versión:
            </Typography.Text>
            <Flex vertical gap={2} style={{ minWidth: 0, flex: 1 }}>
              <EllipsisText value={entry.new_digest || "-"} mono />
              <EllipsisText value={entry.old_digest || "-"} mono secondary />
            </Flex>
          </Flex>
          <Flex gap={4}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Intentos:
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {entry.attempts}
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

  const pagination = (
    <Flex justify="center" style={{ marginTop: 16 }}>
      <Pagination
        current={page}
        pageSize={pageSize}
        total={total}
        pageSizeOptions={PAGE_SIZE_OPTIONS}
        showSizeChanger
        showTotal={(t) => `${t} entradas`}
        size={isMobile ? "small" : "medium"}
        onChange={(nextPage, nextPageSize) => {
          if (nextPageSize !== pageSize) {
            // El cambio de tamaño resetea a la primera página.
            setPageSize(nextPageSize);
            setPage(1);
          } else {
            setPage(nextPage);
          }
        }}
      />
    </Flex>
  );

  return (
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      <Card bordered size="small">
        <Flex vertical gap="small">
          <Flex justify="space-between" align="center">
            <Typography.Text type="secondary">
              <FileTextOutlined /> Histórico de actualizaciones · {total} entradas
            </Typography.Text>
            {total > 0 && (
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
          <Flex gap="middle" wrap="wrap" align="center">
            <Input
              placeholder="Buscar por contenedor, imagen o digest..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              allowClear
              onClear={() => setSearchQuery("")}
              style={{ flex: 1, minWidth: 200 }}
            />
            <Flex gap="small" wrap="wrap" align="center">
              {STATUS_FILTERS.map((s) => {
                const selected = statusFilter.includes(s.value);
                return (
                  <Tag
                    key={s.value}
                    color={selected ? "blue" : undefined}
                    role="button"
                    tabIndex={0}
                    aria-pressed={selected}
                    style={{ cursor: "pointer", marginInlineEnd: 0 }}
                    onClick={() => toggleStatus(s.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        // `Space` desplazaría la página; se anula al activar.
                        e.preventDefault();
                        toggleStatus(s.value);
                      }
                    }}
                  >
                    {s.label}
                  </Tag>
                );
              })}
            </Flex>
          </Flex>
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

      {total === 0 ? (
        <Card bordered>
          <Typography.Text
            type="secondary"
            style={{ textAlign: "center", display: "block" }}
          >
            {hasActiveFilters
              ? "No hay resultados para los filtros aplicados. Ajusta la búsqueda o los estados."
              : "No hay historial de actualizaciones. Cuando se actualice un container, aparecerá aquí."}
          </Typography.Text>
        </Card>
      ) : (
        <>
          {isMobile ? (
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              {items.map((entry) => renderMobileCard(entry))}
            </Space>
          ) : (
            <Card bordered styles={{ body: { padding: 0 } }}>
              <div
                tabIndex={0}
                role="region"
                aria-label="Histórico de actualizaciones (desplazable)"
                style={{ overflowX: "auto" }}
              >
                <Table
                  columns={columns}
                  dataSource={items}
                  rowKey={historyRowKey}
                  components={{ table: ContentTable }}
                  pagination={false}
                />
              </div>
            </Card>
          )}
          {pagination}
        </>
      )}
    </Space>
  );
}
