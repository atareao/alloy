// Pre-chequeo del cron en el cliente (el backend es la autoridad final, vía la
// crate `cron`). Valida, sin dependencias, cada campo canónico
// (min hora día-del-mes mes día-de-semana):
//  - listas `,` (sin elementos vacíos);
//  - comodines `*`/`*/n`, valores `a`/`a/n` y rangos `a-b`/`a-b/n`;
//  - pasos enteros con `1 <= n <= máximo` del campo;
//  - rangos con ambos extremos del mismo tipo y no invertidos (`a <= b`);
//  - valores numéricos dentro del rango `[min, max]` de cada campo;
//  - `?` solo en día-del-mes/día-de-semana y como único elemento del campo;
//  - nombres alfabéticos (≥3 letras) solo en mes/día-de-semana; la validación
//    de nombres es deliberadamente permisiva (superset de los alias de la crate).
// Día-de-semana usa el rango 1–7 (crate: 1 = domingo; `0` es inválido).
interface CronFieldSpec {
  min: number;
  max: number;
  allowsQuestion: boolean;
  allowsNames: boolean;
}

// Orden canónico: minuto, hora, día-del-mes, mes, día-de-semana.
const CRON_FIELDS: readonly CronFieldSpec[] = [
  { min: 0, max: 59, allowsQuestion: false, allowsNames: false },
  { min: 0, max: 23, allowsQuestion: false, allowsNames: false },
  { min: 1, max: 31, allowsQuestion: true, allowsNames: false },
  { min: 1, max: 12, allowsQuestion: false, allowsNames: true },
  { min: 1, max: 7, allowsQuestion: true, allowsNames: true },
];

const NUMERIC_TOKEN = /^\d+$/;
// Nombre deliberadamente permisivo: cualquier token alfabético ASCII de ≥3 letras.
const NAME_TOKEN = /^[A-Za-z]{3,}$/;

function isNumericToken(token: string): boolean {
  return NUMERIC_TOKEN.test(token);
}

function isNameToken(token: string): boolean {
  return NAME_TOKEN.test(token);
}

/** `true` si el token numérico cae dentro del rango `[min, max]` del campo. */
function isInRange(token: string, spec: CronFieldSpec): boolean {
  const value = Number(token);
  return value >= spec.min && value <= spec.max;
}

/**
 * Valida la base de un elemento (sin paso): `*`, un valor `a` o un rango `a-b`.
 * En un rango, ambos extremos deben ser del mismo tipo: si son numéricos deben
 * estar en rango y no invertidos; si son nombres (solo mes/día-de-semana)
 * deben ser alfabéticos de ≥3 letras (el orden no se valida).
 */
function isValidCronBase(base: string, spec: CronFieldSpec): boolean {
  if (base === "*") return true;

  if (base.includes("-")) {
    const bounds = base.split("-");
    if (bounds.length !== 2 || bounds[0] === "" || bounds[1] === "")
      return false;
    const [from, to] = bounds;
    const fromNumeric = isNumericToken(from);
    const toNumeric = isNumericToken(to);

    if (fromNumeric && toNumeric) {
      return (
        isInRange(from, spec) && isInRange(to, spec) && Number(from) <= Number(to)
      );
    }
    if (!fromNumeric && !toNumeric) {
      return spec.allowsNames && isNameToken(from) && isNameToken(to);
    }
    return false;
  }

  if (isNumericToken(base)) return isInRange(base, spec);
  return spec.allowsNames && isNameToken(base);
}

/**
 * Valida un elemento: `*`, un valor, un rango, o cualquiera de ellos con paso
 * `/n` (paso entero con `1 <= n <= máximo` del campo).
 */
function isValidCronElement(element: string, spec: CronFieldSpec): boolean {
  const parts = element.split("/");
  if (parts.length > 2) return false;
  const [base, step] = parts;
  if (step !== undefined) {
    if (!isNumericToken(step) || Number(step) < 1 || Number(step) > spec.max) {
      return false;
    }
  }
  return isValidCronBase(base, spec);
}

/**
 * Valida un campo completo: lista de elementos separados por `,` (sin vacíos).
 * `?` solo es válido si el campo lo admite y es el único elemento del campo.
 */
function isValidCronField(field: string, spec: CronFieldSpec): boolean {
  if (field.length === 0) return false;
  const elements = field.split(",");
  if (elements.some((element) => element === "")) return false;
  if (elements.includes("?")) {
    return spec.allowsQuestion && elements.length === 1;
  }
  return elements.every((element) => isValidCronElement(element, spec));
}

/**
 * Normaliza una expresión cron: `trim`; si trae **6 campos** (formato con
 * segundos) elimina el primero (segundos) y devuelve los 5 canónicos.
 */
export function normalizeCron(expr: string): string {
  const trimmed = expr.trim();
  const fields = trimmed.split(/\s+/);
  if (fields.length === 6) {
    return fields.slice(1).join(" ");
  }
  return trimmed;
}

/**
 * Validación del cron en cliente: normaliza primero, exige **exactamente 5
 * campos** y valida rangos, listas, pasos (1..máximo) y rangos no invertidos
 * por campo. Es un pre-chequeo; el backend valida de verdad.
 */
export function isValidCronClient(expr: string): boolean {
  const normalized = normalizeCron(expr);
  if (!normalized) return false;
  const fields = normalized.split(/\s+/);
  if (fields.length !== 5) return false;
  return fields.every((field, i) => isValidCronField(field, CRON_FIELDS[i]));
}

/** Mensaje de error inline para el campo cron, o `null` si es válido. */
export function cronErrorFor(raw: string): string | null {
  if (raw.trim() === "") return "La expresión cron es obligatoria";
  const fieldCount = normalizeCron(raw).split(/\s+/).length;
  if (fieldCount !== 5)
    return "Expresión cron inválida (se esperan 5 campos: min hora día mes semana)";
  if (!isValidCronClient(raw))
    return "Expresión cron inválida (revisa los valores y rangos de cada campo)";
  return null;
}
