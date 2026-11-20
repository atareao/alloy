import type { BatchProgress } from "./types";

export interface BatchCheckResults {
  total: number;
  updated: number;
  uptodate: number;
  failed: number;
  done: number;
  errors: string[];
}

export interface BatchResults {
  check: BatchCheckResults;
  update: BatchCheckResults;
}

export function mapBatchToResults(bp: BatchProgress): BatchResults {
  return {
    check: {
      total: bp.checked,
      updated: bp.updatable ?? 0,
      uptodate: bp.uptodate ?? 0,
      failed: bp.errors,
      done: bp.checked,
      errors: [],
    },
    update: {
      total: bp.checked,
      updated: bp.updated,
      uptodate: 0,
      failed: bp.failed ?? 0,
      done: bp.updated,
      errors: [],
    },
  };
}