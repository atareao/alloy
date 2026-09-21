#!/bin/bash
# Pre-flight validation: SDD + TDD workflow guard
# Ejecutar como PRIMER PASO antes de cualquier implementación.
# Uso: source openspec/validate-workflow.sh

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || echo '.')"
OPENSPEC_DIR="$ROOT_DIR/openspec"

# --- Helper ---
has_open_changes() {
  ls "$OPENSPEC_DIR/changes/"*/proposal.md 2>/dev/null | head -1
}

has_approved_spec() {
  for change_dir in "$OPENSPEC_DIR/changes/"*/; do
    if [ -f "${change_dir}proposal.md" ] && [ ! -f "${change_dir}.approved" ]; then
      return 1
    fi
  done
  return 0
}

# --- Check 1: ¿Hay cambios de código sin change proposal? ---
echo -e "${YELLOW}🔍 Validando workflow SDD + TDD...${NC}"

UNCOMMITTED=$(git status --porcelain 2>/dev/null | grep -E '\.(rs|tsx?|jsx?)$' | head -5 || true)

if [ -n "$UNCOMMITTED" ] && ! has_open_changes >/dev/null 2>&1; then
  echo -e "${RED}❌ VIOLACIÓN: Hay cambios de código sin un change proposal en openspec/changes/${NC}"
  echo -e "${RED}   Debes crear un change proposal antes de escribir código.${NC}"
  echo -e "${YELLOW}   Ejecuta: openspec change <feature-name>${NC}"
  return 1 2>/dev/null || exit 1
fi

# --- Check 2: ¿Hay change proposal sin aprobación explícita? ---
if has_open_changes >/dev/null 2>&1 && ! has_approved_spec; then
  echo -e "${YELLOW}⚠️  Hay change proposals sin aprobación explícita del usuario.${NC}"
  echo -e "${YELLOW}   Pregunta al usuario: ¿Apruebas el spec en openspec/changes/...?${NC}"
  echo -e "${YELLOW}   NO escribas código hasta recibir aprobación explícita.${NC}"
fi

# --- Check 3: ¿Hay tests para el código nuevo? ---
RECENT_TESTS=$(git diff --name-only HEAD 2>/dev/null | grep -E '\.test\.(ts|tsx|js|jsx)$' | head -3 || true)
RECENT_CODE=$(git diff --name-only HEAD 2>/dev/null | grep -v '\.test\.' | grep -E '\.(tsx?|jsx?|rs)$' | head -3 || true)

if [ -n "$RECENT_CODE" ] && [ -z "$RECENT_TESTS" ]; then
  echo -e "${YELLOW}⚠️  WARNING: Código nuevo detectado sin tests nuevos.${NC}"
  echo -e "${YELLOW}   TDD requiere escribir tests primero (RED) antes de implementar (GREEN).${NC}"
fi

echo -e "${GREEN}✅ Pre-flight validation passed${NC}"