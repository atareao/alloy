import { useEffect, useRef, useState } from "react";

/** Clave del recurso que se está guardando (una Card a la vez). */
export type SavingKey =
  | "telegram"
  | "matrix"
  | "valet"
  | "update-check"
  | "default-policy";

/** Canal de notificación que se está probando (`POST /api/notifications/test`). */
export type TestingKey = "telegram" | "matrix" | "valet";

export interface UseConfigPageResult {
  saving: SavingKey | null;
  setSaving: (key: SavingKey | null) => void;
  testing: TestingKey | null;
  setTesting: (key: TestingKey | null) => void;
  error: string | null;
  setError: (msg: string | null) => void;
  success: string | null;
  setSuccess: (msg: string | null) => void;
  activeTab: string;
  setActiveTab: (tab: string) => void;
  /** Muestra un aviso de éxito y lo auto-oculta a los 3 s. */
  showSuccess: (msg: string) => void;
}

/**
 * Estado COMPARTIDO de la página de configuración: el recurso en curso de
 * guardado/test, las alertas de error/éxito y la pestaña activa. Lo consumen
 * `ConfigPage` y, más adelante, sus Cards.
 */
export function useConfigPage(): UseConfigPageResult {
  const [saving, setSaving] = useState<SavingKey | null>(null);
  const [testing, setTesting] = useState<TestingKey | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<string>("notifications");

  // El aviso de éxito se auto-oculta a los 3 s; el timer se limpia al desmontar
  // para no hacer setState después del unmount (y el anterior al re-disparar).
  const successTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (successTimer.current !== null) clearTimeout(successTimer.current);
    },
    [],
  );

  const showSuccess = (msg: string) => {
    setSuccess(msg);
    if (successTimer.current !== null) clearTimeout(successTimer.current);
    successTimer.current = setTimeout(() => setSuccess(null), 3000);
  };

  return {
    saving,
    setSaving,
    testing,
    setTesting,
    error,
    setError,
    success,
    setSuccess,
    activeTab,
    setActiveTab,
    showSuccess,
  };
}
