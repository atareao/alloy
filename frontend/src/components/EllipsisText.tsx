import { Typography } from "antd";
import type { CSSProperties } from "react";

export interface EllipsisTextProps {
  /** Full value; the ellipsis is CSS and the tooltip carries the whole text. */
  value: string;
  /** Monospace + small font, for digests. */
  mono?: boolean;
  /** Secondary (muted) text colour. */
  secondary?: boolean;
  /** Bold text, for the container name. */
  strong?: boolean;
  style?: CSSProperties;
}

/**
 * Truncates text with real (CSS/Ant Design) ellipsis and shows the full value in
 * a tooltip.
 *
 * Two things make the ellipsis effective (D4):
 * - `minWidth: 0` lets the node shrink inside a flex container instead of
 *   overflowing the row. It has no effect on a plain table cell.
 * - Inside a table cell (`width: max-content`), what actually triggers the
 *   ellipsis is the absolute width bound the consumer passes via `style`
 *   (e.g. `maxWidth: 260`); without it, Ant Design's own `max-width: 100%`
 *   resolves to the full text width and nothing is truncated.
 *
 * No manual `slice` and no hand-written `"..."`: the whole value stays in the
 * DOM and Ant Design decides whether to ellipsize it.
 */
export default function EllipsisText({
  value,
  mono,
  secondary,
  strong,
  style,
}: EllipsisTextProps) {
  return (
    <Typography.Text
      strong={strong}
      type={secondary ? "secondary" : undefined}
      ellipsis={{ tooltip: value }}
      style={{
        display: "block",
        minWidth: 0,
        fontFamily: mono ? "monospace" : undefined,
        fontSize: mono ? 12 : undefined,
        ...style,
      }}
    >
      {value}
    </Typography.Text>
  );
}
