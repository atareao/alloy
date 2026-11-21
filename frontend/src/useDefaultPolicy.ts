import { useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { apiFetch } from "./api";
import type { DefaultUpdatePolicy } from "./types";

export interface UseDefaultPolicyResult {
  /** Acción por defecto de la política global. */
  defAction: string;
  setDefAction: Dispatch<SetStateAction<string>>;
  defCleanup: boolean;
  setDefCleanup: Dispatch<SetStateAction<boolean>>;
  defRollback: boolean;
  setDefRollback: Dispatch<SetStateAction<boolean>>;
  /** Override del nº máximo de intentos de arranque (`null` = sin override). */
  defMaxStartAttempts: number | null;
  setDefMaxStartAttempts: Dispatch<SetStateAction<number | null>>;
  /** Override de la ventana de salud en segundos (`null` = sin override). */
  defHealthWindow: number | null;
  setDefHealthWindow: Dispatch<SetStateAction<number | null>>;
}

/**
 * Estado y carga de la política de actualización por defecto
 * (`GET /api/update-policies/default`). Se invoca a nivel de página para que el
 * fetch se dispare al montar `ConfigPage`, no al montar la Card (los paneles de
 * antd `Tabs` se montan en diferido). Al guardar, `DefaultPolicyCard` envía el
 * `PUT /api/update-policies/default` con `max_start_attempts`/`health_window_secs`
 * siempre presentes (`null` = sin override).
 */
export function useDefaultPolicy(): UseDefaultPolicyResult {
  const [defAction, setDefAction] = useState<string>("pull-restart");
  const [defCleanup, setDefCleanup] = useState(false);
  const [defRollback, setDefRollback] = useState(false);
  // Overrides opcionales de la guarda para la política por defecto.
  // `null` = sin override (se usa el valor global).
  const [defMaxStartAttempts, setDefMaxStartAttempts] = useState<number | null>(
    null,
  );
  const [defHealthWindow, setDefHealthWindow] = useState<number | null>(null);

  useEffect(() => {
    apiFetch("/api/update-policies/default")
      .then((res) => res.json())
      .then((data: DefaultUpdatePolicy) => {
        setDefAction(data.action);
        setDefCleanup(data.cleanup_old_image);
        setDefRollback(data.rollback_on_failure);
        setDefMaxStartAttempts(data.max_start_attempts ?? null);
        setDefHealthWindow(data.health_window_secs ?? null);
      })
      .catch(() => {});
  }, []);

  return {
    defAction,
    setDefAction,
    defCleanup,
    setDefCleanup,
    defRollback,
    setDefRollback,
    defMaxStartAttempts,
    setDefMaxStartAttempts,
    defHealthWindow,
    setDefHealthWindow,
  };
}
