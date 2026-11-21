# Proposal

## Why

Tras #137 el backend valida el cron (400 si inválido), pero el **formulario no valida nada**: el usuario
solo descubre el error tras un viaje al backend (`ConfigPage.tsx:967-971`, `<Input>` plano sin
validación). Además, un cron pegado con **6 campos** (segundos) se rechaza con 400 aunque su intención
es clara.

## What Changes

- **Frontend** (`ConfigPage.tsx`):
  - `normalizeCron(expr)`: trim; si tiene **6 campos**, elimina el de **segundos** ⇒ 5 campos canónicos.
  - `isValidCronClient(expr)`: validación **ligera** (5 campos + charset permitido), **sin dependencias
    nuevas**. El backend sigue siendo la autoridad final.
  - **Error inline** bajo el campo (estado `cronError`).
  - Normaliza **onBlur** y **antes de enviar**; `saveUpdateCheck` **bloquea** el PUT si el cron es
    inválido, mostrando el error inline.
- **Tests** `ConfigPage.test.tsx`: normalización 6→5; inválido ⇒ error inline + PUT **no** disparado.

## Capabilities

### Modified Capabilities

- `updates/update-check`: el formulario valida y normaliza el cron antes de envíar (nuevo comportamiento
  de UI; el contrato del backend no cambia).

### New Capabilities

- Ninguna.

## Impact

- **Frontend**: `ConfigPage.tsx` (+ tests). **Solo frontend**.
- **Backend**: sin cambios (sigue estricto en 5 campos, como en #137).
- **API**: sin cambios.
- **Fuera de alcance**: validación profunda de rangos; nombres de mes/día (formato canónico = numérico).
