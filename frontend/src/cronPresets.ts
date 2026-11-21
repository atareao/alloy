// Presets de frecuencia del auto-update (campo "Frecuencia" de ConfigPage).
// Deben ser expresiones de 5 campos válidas para la crate `cron` del backend
// (autoridad final), en particular día-de-semana `1–7` con `1` = domingo.
export const CRON_PRESETS = [
  { value: "0 */6 * * *", label: "Cada 6 horas" },
  { value: "0 */12 * * *", label: "Cada 12 horas" },
  { value: "0 0 * * *", label: "Cada día a medianoche" },
  { value: "0 4 * * *", label: "Cada día a las 4:00" },
  { value: "0 6 * * *", label: "Cada día a las 6:00" },
  { value: "0 0 * * SUN", label: "Cada domingo" },
  { value: "0 0 1 * *", label: "Cada 1 del mes" },
  { value: "*/30 * * * *", label: "Cada 30 minutos" },
  { value: "0 */1 * * *", label: "Cada hora" },
];
