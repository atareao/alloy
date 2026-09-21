#!/bin/bash
# ⚠️  PRE-FLIGHT OBLIGATORIO — Ejecutar antes de leer o escribir código fuente.
# Uso: source openspec/require-proposal.sh
# Si falla, el agente DEBE detenerse y crear un change proposal primero.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || echo '.')"
OPENSPEC_DIR="$ROOT_DIR/openspec"

# --- Helper: ¿Hay change proposals activos? ---
has_active_proposal() {
  ls "$OPENSPEC_DIR/changes/"*/proposal.md 2>/dev/null | head -1
}

# --- Helper: ¿Hay cambios sin commit en archivos fuente? ---
has_uncommitted_source_changes() {
  git status --porcelain 2>/dev/null | grep -qE '\.(rs|tsx?|jsx?)$'
}

# --- Helper: ¿Hay cambios staged en archivos fuente? ---
has_staged_source_changes() {
  git diff --cached --name-only 2>/dev/null | grep -qE '\.(rs|tsx?|jsx?)$'
}

echo -e "${YELLOW}🔍 [PRE-FLIGHT] Verificando change proposal activo...${NC}"

# --- Check 1: ¿Hay cambios source sin proposal? ---
if has_uncommitted_source_changes || has_staged_source_changes; then
  if ! has_active_proposal >/dev/null 2>&1; then
    echo -e "${RED}❌ [PRE-FLIGHT] VIOLACIÓN: Hay cambios en archivos fuente sin un change proposal activo.${NC}"
    echo -e "${RED}   Crea un change proposal antes de tocar código fuente.${NC}"
    echo -e "${YELLOW}   Ejecuta: openspec change <feature-name>${NC}"
    echo -e "${YELLOW}   Luego completa openspec/changes/<feature-name>/proposal.md y espera aprobación.${NC}"
    return 1 2>/dev/null || exit 1
  fi

  # --- Check 2: Hay proposal pero no está aprobado ---
  for change_dir in "$OPENSPEC_DIR/changes/"*/; do
    if [ -f "${change_dir}proposal.md" ] && [ ! -f "${change_dir}.approved" ]; then
      echo -e "${YELLOW}⚠️  [PRE-FLIGHT] Hay change proposal sin aprobación explícita.${NC}"
      echo -e "${YELLOW}   Pregunta al usuario si aprueba el spec antes de continuar.${NC}"
      echo -e "${YELLOW}   NO escribas código hasta recibir aprobación.${NC}"
    fi
  done
fi

echo -e "${GREEN}✅ [PRE-FLIGHT] Validación superada${NC}"