import { useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { apiFetch } from "./api";
import type { UpdateCheckConfig, UpdateDefaults } from "./types";

export interface UseUpdateCheckConfigResult {
  /** Config de la revisión programada (`GET /api/update-check/config`). */
  checkConfig: UpdateCheckConfig | null;
  setCheckConfig: Dispatch<SetStateAction<UpdateCheckConfig | null>>;
  /** Defaults canónicos (`GET /api/update-check/defaults`). */
  defaults: UpdateDefaults | null;
  setDefaults: Dispatch<SetStateAction<UpdateDefaults | null>>;
  ucCron: string;
  setUcCron: Dispatch<SetStateAction<string>>;
  ucEnabled: boolean;
  setUcEnabled: Dispatch<SetStateAction<boolean>>;
  ucNotify: boolean;
  setUcNotify: Dispatch<SetStateAction<boolean>>;
  /** Timeout de pull en segundos. */
  pullTimeout: number;
  setPullTimeout: Dispatch<SetStateAction<number>>;
  /** Intervalo de comprobación en **segundos** (la API lo expone en ms). */
  checkIntervalSecs: number;
  setCheckIntervalSecs: Dispatch<SetStateAction<number>>;
  /** Guarda de arranque (update-rollback-guard) — settings globales. */
  ucMaxStartAttempts: number;
  setUcMaxStartAttempts: Dispatch<SetStateAction<number>>;
  ucStartAttemptInterval: number;
  setUcStartAttemptInterval: Dispatch<SetStateAction<number>>;
  ucHealthWindow: number;
  setUcHealthWindow: Dispatch<SetStateAction<number>>;
  /** Error inline del campo cron, o `null` si es válido. */
  cronError: string | null;
  setCronError: Dispatch<SetStateAction<string | null>>;
}

/**
 * Estado y carga de la revisión programada de actualizaciones:
 * `GET /api/update-check/config` (con conversión ms→s de `check_interval_ms`) y
 * `GET /api/update-check/defaults`. Al guardar, `ConfigPage` envía
 * `check_interval_ms` multiplicado por 1000 (s→ms).
 */
export function useUpdateCheckConfig(): UseUpdateCheckConfigResult {
  const [ucCron, setUcCron] = useState("");
  const [ucEnabled, setUcEnabled] = useState(false);
  const [ucNotify, setUcNotify] = useState(false);
  const [cronError, setCronError] = useState<string | null>(null);

  // Update check config — fetched separately
  const [checkConfig, setCheckConfig] = useState<UpdateCheckConfig | null>(
    null,
  );

  // Defaults canónicos de la revisión (GET /api/update-check/defaults). Se usan
  // para el placeholder del cron, la etiqueta del timeout y el fallback al
  // vaciar `pull_timeout_secs`, evitando literales hardcodeados.
  const [defaults, setDefaults] = useState<UpdateDefaults | null>(null);

  // Pull timeout
  const [pullTimeout, setPullTimeout] = useState(0);

  // Intervalo de comprobación del digest de cada contenedor, en **segundos**
  // (la API lo expone en ms: se divide al cargar y se multiplica al enviar).
  const [checkIntervalSecs, setCheckIntervalSecs] = useState(0);

  // Guarda de arranque (update-rollback-guard) — settings globales.
  const [ucMaxStartAttempts, setUcMaxStartAttempts] = useState(0);
  const [ucStartAttemptInterval, setUcStartAttemptInterval] = useState(0);
  const [ucHealthWindow, setUcHealthWindow] = useState(0);

  useEffect(() => {
    apiFetch("/api/update-check/config")
      .then((res) => res.json())
      .then((data: UpdateCheckConfig) => {
        setCheckConfig(data);
        setUcCron(data.cron);
        setUcEnabled(data.enabled);
        setUcNotify(data.notify);
        if (data.pull_timeout_secs != null)
          setPullTimeout(data.pull_timeout_secs);
        if (data.update_max_start_attempts != null)
          setUcMaxStartAttempts(data.update_max_start_attempts);
        if (data.update_start_attempt_interval_secs != null)
          setUcStartAttemptInterval(data.update_start_attempt_interval_secs);
        if (data.update_health_window_secs != null)
          setUcHealthWindow(data.update_health_window_secs);
        if (data.check_interval_ms != null)
          setCheckIntervalSecs(Math.round(data.check_interval_ms / 1000));
      })
      .catch(() => {});
  }, []);

  // Carga los defaults canónicos desde la API (placeholder/etiqueta/fallback).
  useEffect(() => {
    apiFetch("/api/update-check/defaults")
      .then((res) => res.json())
      .then((data: UpdateDefaults) => setDefaults(data))
      .catch(() => {});
  }, []);

  return {
    checkConfig,
    setCheckConfig,
    defaults,
    setDefaults,
    ucCron,
    setUcCron,
    ucEnabled,
    setUcEnabled,
    ucNotify,
    setUcNotify,
    pullTimeout,
    setPullTimeout,
    checkIntervalSecs,
    setCheckIntervalSecs,
    ucMaxStartAttempts,
    setUcMaxStartAttempts,
    ucStartAttemptInterval,
    setUcStartAttemptInterval,
    ucHealthWindow,
    setUcHealthWindow,
    cronError,
    setCronError,
  };
}
