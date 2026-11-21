import { useState } from "react";

/**
 * Sincroniza estado local de formulario desde una prop sin `useEffect`:
 * aplica `sync` durante el render cuando la prop cambia de identidad.
 *
 * Patrón "adjusting state when a prop changes" de React: el `setState` durante
 * el render del propio componente está soportado y React re-renderiza antes de
 * commitear, sin pasar por un efecto ni provocar cascadas de renders.
 *
 * Contrato:
 * - `sync` MUST ser idempotente y MUST aplicar setters del MISMO componente
 *   (el hook los invoca durante el render). No debe lanzar efectos externos ni
 *   tocar estado de otros componentes.
 * - `prev` arranca en `null`, así que el primer `configProp` no nulo se trata
 *   como un cambio y dispara `sync` (mismo comportamiento que el `useEffect`
 *   anterior con dependencia `[configProp]`).
 * - En StrictMode el render se invoca dos veces; con setters de valores
 *   primitivos la segunda invocación es no-op.
 */
export function useConfigSync<T>(
  configProp: T | null,
  sync: (config: T) => void,
): void {
  const [prev, setPrev] = useState<T | null>(null);
  if (configProp !== prev) {
    setPrev(configProp);
    if (configProp) sync(configProp);
  }
}
