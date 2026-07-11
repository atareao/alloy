# Config Page Overhaul — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use inline execution with checkpoints.

**Goal:** Transform the Config page into an interactive settings panel where Telegram, Matrix, and Auto-update can be configured directly from the UI, removing the read-only Docker info and auth sections.

**Architecture:** Persistent mutable settings stored in `data/settings.json` (same pattern as alerts/schedules), served via a new `PUT /api/config` endpoint. Backend workers resolve settings at runtime — UI values overlay startup config. Frontend gets forms with toggle buttons for each service.

**Tech Stack:** Rust/Axum (backend), React/Mantine (frontend), JSON file persistence

## Global Constraints

- Follow existing patterns: `load_json`/`JsonWriter` for persistence, `Arc<Mutex<T>>` for state
- Settings overlay startup config but don't modify `config.yaml` or env vars
- No new dependencies
- Frontend inputs mask sensitive values (tokens) with `type="password"` when appropriate

---

### Task 1: Add Settings types and file constant

**Files:**
- Modify: `backend/src/models.rs`

**Interfaces:**
- Consumes: `FILE_ALERTS`, `FILE_SCHEDULES` patterns
- Produces: `Settings`, `UpdateSettingsReq`, `FILE_SETTINGS`

- [ ] **Step 1: Add constants and structs to models.rs**

Add after `FILE_SCHEDULES` (line ~217):

```rust
pub const FILE_SETTINGS: &str = "data/settings.json";
```

Add new types after `CreateSchedule` struct:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub auto_update_enabled: Option<bool>,
    #[serde(default)]
    pub auto_update_interval_hours: Option<u64>,
    #[serde(default)]
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub matrix_homeserver: Option<String>,
    #[serde(default)]
    pub matrix_token: Option<String>,
    #[serde(default)]
    pub matrix_room: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSettingsReq {
    pub auto_update_enabled: Option<bool>,
    pub auto_update_interval_hours: Option<u64>,
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token: Option<String>,
    pub matrix_room: Option<String>,
}
```

Extend `PublicConfig` (after `allowed_containers`, line ~50):

```rust
    pub telegram_token_set: bool,
    pub telegram_chat_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token_set: bool,
    pub matrix_room: Option<String>,
```

- [ ] **Step 2: Verify compilation**

```bash
cd backend && cargo check 2>&1
```
Expected: clean compile

- [ ] **Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: add Settings types and FILE_SETTINGS constant"
```

---

### Task 2: Add Settings to AppState and load at startup

**Files:**
- Modify: `backend/src/state.rs`
- Modify: `backend/src/main.rs`

**Interfaces:**
- Consumes: `Settings`, `FILE_SETTINGS`, `load_json`
- Produces: `settings` field in `AppState`, `FromRef` impl

- [ ] **Step 1: Add settings field to AppState (state.rs)**

Add after `schedules` field in `AppState`:

```rust
    pub settings: Arc<Mutex<Settings>>,
```

Add `FromRef` impl after the schedules one:

```rust
impl axum::extract::FromRef<AppState> for Arc<Mutex<Settings>> {
    fn from_ref(state: &AppState) -> Self {
        state.settings.clone()
    }
}
```

- [ ] **Step 2: Load settings at startup (main.rs)**

Add import at top:
```rust
use crate::workers::json_writer;
```

After the schedules loading block (around line 136-140), add:

```rust
        // Load mutable settings
        let settings = Arc::new(Mutex::new(
            load_json::<Settings>(FILE_SETTINGS)
                .into_iter()
                .next()
                .unwrap_or_default(),
        ));
```

Add `settings` to the AppState construction:
```rust
        settings,
```

- [ ] **Step 3: Verify compilation**

```bash
cd backend && cargo check 2>&1
```
Expected: clean compile

- [ ] **Step 4: Commit**

```bash
git add backend/src/state.rs backend/src/main.rs
git commit -m "feat: add Settings to AppState with JSON persistence"
```

---

### Task 3: Add PUT /api/config endpoint to update settings

**Files:**
- Modify: `backend/src/containers.rs` (or create handler in a new file)

**Interfaces:**
- Consumes: `Settings`, `UpdateSettingsReq`, `PublicConfig`, `FILE_SETTINGS`, `json_writer`
- Produces: `PUT /api/config` endpoint, updated `config_handler`

- [ ] **Step 1: Add update_settings handler**

Add a new handler function in `containers.rs` after the `health_h` handler:

```rust
async fn update_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(config): State<Config>,
    Json(body): Json<UpdateSettingsReq>,
) -> Json<PublicConfig> {
    let mut s = settings.lock().await;
    if let Some(v) = body.auto_update_enabled {
        s.auto_update_enabled = Some(v);
    }
    if let Some(v) = body.auto_update_interval_hours {
        s.auto_update_interval_hours = Some(v);
    }
    if let Some(v) = body.telegram_token {
        s.telegram_token = Some(v);
    }
    if let Some(v) = body.telegram_chat_id {
        s.telegram_chat_id = Some(v);
    }
    if let Some(v) = body.matrix_homeserver {
        s.matrix_homeserver = Some(v);
    }
    if let Some(v) = body.matrix_token {
        s.matrix_token = Some(v);
    }
    if let Some(v) = body.matrix_room {
        s.matrix_room = Some(v);
    }
    json_writer().save(FILE_SETTINGS, &*s).await;
    drop(s);

    // Return updated config
    config_handler(State(config)).await
}
```

- [ ] **Step 2: Register the route**

Add route to the router chain in `pub fn routes()`:
```rust
        .route("/api/config", get(config_handler).put(update_config_h))
```

Change from `get(config_handler)` to `get(config_handler).put(update_config_h)`.

- [ ] **Step 3: Update config_handler to resolve settings**

Replace the existing `config_handler` with one that resolves settings:

```rust
async fn config_handler(
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<PublicConfig> {
    let s = settings.lock().await;
    Json(PublicConfig {
        oidc_configured: true,
        port: config.port(),
        auto_update_enabled: s.auto_update_enabled.unwrap_or_else(|| config.auto_update()),
        auto_update_interval_hours: s.auto_update_interval_hours.unwrap_or_else(|| config.auto_update_interval()),
        telegram_configured: s.telegram_token.is_some() || config.telegram_token.is_some(),
        telegram_token_set: s.telegram_token.is_some() || config.telegram_token.is_some(),
        telegram_chat_id: s.telegram_chat_id.clone().or_else(|| config.telegram_chat_id.clone()),
        matrix_configured: s.matrix_homeserver.is_some() || config.matrix_homeserver.is_some(),
        matrix_token_set: s.matrix_token.is_some() || config.matrix_token.is_some(),
        matrix_homeserver: s.matrix_homeserver.clone().or_else(|| config.matrix_homeserver.clone()),
        matrix_room: s.matrix_room.clone().or_else(|| config.matrix_room.clone()),
        allowed_containers: config.allowed_containers.clone(),
    })
}
```

Note: Need to also add `State(settings)` to the config_handler signature and add necessary imports.

- [ ] **Step 4: Update workers to use Settings**

In `workers.rs`, modify `auto_update_worker` to accept settings state and use it:

The auto_update_worker currently uses `config.auto_update()` directly. It should check settings first:

In the worker function signature, add `settings: Arc<Mutex<Settings>>`.

Inside the main loop, before checking auto_update:
```rust
let enabled = {
    let s = settings.lock().await;
    s.auto_update_enabled.unwrap_or_else(|| config.auto_update())
};
```

And pass settings from main.rs when spawning the worker.

Similarly, notification functions (`notify_all`, `notify_selected`) should use settings for telegram/matrix credentials instead of config directly.

- [ ] **Step 5: Verify compilation**

```bash
cd backend && cargo check 2>&1
```
Expected: clean compile

- [ ] **Step 6: Run tests to verify no regressions**

```bash
cd backend && cargo test 2>&1
```
Expected: All 44+ tests passing

- [ ] **Step 7: Commit**

```bash
git add backend/src/containers.rs backend/src/workers.rs backend/src/main.rs
git commit -m "feat: add PUT /api/config endpoint and settings-aware config handler"
```

---

### Task 4: Rewrite frontend ConfigPage

**Files:**
- Modify: `frontend/src/components/ConfigPage.tsx`
- Modify: `frontend/src/types.ts`

**Interfaces:**
- Consumes: `AppConfig` from types, `apiFetch` from api, `PUT /api/config`
- Produces: Interactive config form with Telegram, Matrix, Auto-update sections

- [ ] **Step 1: Update AppConfig type (types.ts)**

Replace with full config including settings:

```typescript
export interface AppConfig {
  oidc_configured: boolean
  port: number
  auto_update_enabled: boolean
  auto_update_interval_hours: number
  telegram_configured: boolean
  telegram_token_set: boolean
  telegram_chat_id: string | null
  matrix_configured: boolean
  matrix_token_set: boolean
  matrix_homeserver: string | null
  matrix_room: string | null
  allowed_containers: string[] | null
}
```

- [ ] **Step 2: Rewrite ConfigPage (ConfigPage.tsx)**

```tsx
import { useCallback, useEffect, useState } from "react";
import {
  Button, Group, Loader, Paper, Stack, Text, Title, TextInput, Switch,
  Divider, Alert,
} from "@mantine/core";
import type { AppConfig } from "../types";
import { apiFetch } from "../api";

export default function ConfigPage() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null); // section being saved
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Telegram form
  const [tgToken, setTgToken] = useState("");
  const [tgChatId, setTgChatId] = useState("");
  const [tgEnabled, setTgEnabled] = useState(false);

  // Matrix form
  const [mxHomeserver, setMxHomeserver] = useState("");
  const [mxToken, setMxToken] = useState("");
  const [mxRoom, setMxRoom] = useState("");
  const [mxEnabled, setMxEnabled] = useState(false);

  // Auto-update
  const [auEnabled, setAuEnabled] = useState(false);
  const [auInterval, setAuInterval] = useState(6);

  const loadConfig = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiFetch("/api/config");
      const data: AppConfig = await res.json();
      setConfig(data);
      // Populate form fields
      setTgToken(data.telegram_token_set ? "********" : "");
      setTgChatId(data.telegram_chat_id || "");
      setTgEnabled(data.telegram_configured);
      setMxHomeserver(data.matrix_homeserver || "");
      setMxToken(data.matrix_token_set ? "********" : "");
      setMxRoom(data.matrix_room || "");
      setMxEnabled(data.matrix_configured);
      setAuEnabled(data.auto_update_enabled);
      setAuInterval(data.auto_update_interval_hours);
    } catch {
      setError("No se pudo cargar la configuración");
    }
    setLoading(false);
  }, []);

  useEffect(() => { loadConfig(); }, [loadConfig]);

  const showSuccess = (msg: string) => {
    setSuccess(msg);
    setTimeout(() => setSuccess(null), 3000);
  };

  const saveTelegram = async () => {
    setSaving("telegram");
    setError(null);
    try {
      const body: Record<string, any> = { telegram_chat_id: tgChatId || null };
      if (tgToken && tgToken !== "********") {
        body.telegram_token = tgToken;
      }
      if (!tgEnabled) {
        // Disable: clear stored tokens
        body.telegram_token = "";
        body.telegram_chat_id = "";
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data = await res.json();
        setConfig(data);
        showSuccess(tgEnabled ? "✅ Telegram configurado" : "❌ Telegram desactivado");
      } else {
        setError("Error al guardar Telegram");
      }
    } catch {
      setError("Error de conexión al guardar Telegram");
    }
    setSaving(null);
  };

  const saveMatrix = async () => {
    setSaving("matrix");
    setError(null);
    try {
      const body: Record<string, any> = {
        matrix_homeserver: mxHomeserver || null,
        matrix_room: mxRoom || null,
      };
      if (mxToken && mxToken !== "********") {
        body.matrix_token = mxToken;
      }
      if (!mxEnabled) {
        body.matrix_homeserver = "";
        body.matrix_token = "";
        body.matrix_room = "";
      }
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const data = await res.json();
        setConfig(data);
        showSuccess(mxEnabled ? "✅ Matrix configurado" : "❌ Matrix desactivado");
      } else {
        setError("Error al guardar Matrix");
      }
    } catch {
      setError("Error de conexión al guardar Matrix");
    }
    setSaving(null);
  };

  const saveAutoUpdate = async () => {
    setSaving("auto-update");
    setError(null);
    try {
      const res = await apiFetch("/api/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          auto_update_enabled: auEnabled,
          auto_update_interval_hours: auInterval,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        setConfig(data);
        showSuccess(auEnabled ? "✅ Auto-update activado" : "❌ Auto-update desactivado");
      } else {
        setError("Error al guardar Auto-update");
      }
    } catch {
      setError("Error de conexión al guardar Auto-update");
    }
    setSaving(null);
  };

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>;

  return (
    <Stack>
      {error && (
        <Alert color="red" onClose={() => setError(null)} withCloseButton>
          {error}
        </Alert>
      )}
      {success && (
        <Alert color="green" onClose={() => setSuccess(null)} withCloseButton>
          {success}
        </Alert>
      )}

      {/* ═══ Telegram ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>📱 Telegram</Title>
          <Switch
            label={tgEnabled ? "Activado" : "Desactivado"}
            checked={tgEnabled}
            onChange={(e) => setTgEnabled(e.currentTarget.checked)}
            color={tgEnabled ? "green" : "gray"}
          />
        </Group>
        {tgEnabled && (
          <Stack>
            <TextInput
              label="Token del Bot"
              description="Token que te proporciona @BotFather"
              placeholder="123456:ABC-DEF..."
              type="password"
              value={tgToken}
              onChange={(e) => setTgToken(e.currentTarget.value)}
            />
            <TextInput
              label="Chat ID"
              description="ID del chat o grupo donde recibir notificaciones"
              placeholder="-1001234567890"
              value={tgChatId}
              onChange={(e) => setTgChatId(e.currentTarget.value)}
            />
          </Stack>
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveTelegram}
            loading={saving === "telegram"}
            color={tgEnabled ? "blue" : "gray"}
          >
            {tgEnabled ? "Guardar Telegram" : "Desactivar Telegram"}
          </Button>
        </Group>
      </Paper>

      {/* ═══ Matrix ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>💬 Matrix</Title>
          <Switch
            label={mxEnabled ? "Activado" : "Desactivado"}
            checked={mxEnabled}
            onChange={(e) => setMxEnabled(e.currentTarget.checked)}
            color={mxEnabled ? "green" : "gray"}
          />
        </Group>
        {mxEnabled && (
          <Stack>
            <TextInput
              label="Homeserver"
              description="URL del servidor Matrix (ej: https://matrix.org)"
              placeholder="https://matrix.example.com"
              value={mxHomeserver}
              onChange={(e) => setMxHomeserver(e.currentTarget.value)}
            />
            <TextInput
              label="Access Token"
              description="Token de acceso de la cuenta de bot"
              type="password"
              placeholder="syt_..."
              value={mxToken}
              onChange={(e) => setMxToken(e.currentTarget.value)}
            />
            <TextInput
              label="Room ID"
              description="ID de la sala donde enviar notificaciones"
              placeholder="!roomid:matrix.org"
              value={mxRoom}
              onChange={(e) => setMxRoom(e.currentTarget.value)}
            />
          </Stack>
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveMatrix}
            loading={saving === "matrix"}
            color={mxEnabled ? "blue" : "gray"}
          >
            {mxEnabled ? "Guardar Matrix" : "Desactivar Matrix"}
          </Button>
        </Group>
      </Paper>

      {/* ═══ Auto-update ═══ */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Title order={4}>🤖 Auto-update</Title>
          <Switch
            label={auEnabled ? "Activado" : "Desactivado"}
            checked={auEnabled}
            onChange={(e) => setAuEnabled(e.currentTarget.checked)}
            color={auEnabled ? "green" : "gray"}
          />
        </Group>
        {auEnabled && (
          <TextInput
            label="Intervalo (horas)"
            description="Cada cuántas horas comprobar y actualizar containers"
            type="number"
            min={1}
            max={168}
            value={auInterval}
            onChange={(e) => setAuInterval(Number(e.currentTarget.value))}
          />
        )}
        <Group justify="flex-end" mt="md">
          <Button
            onClick={saveAutoUpdate}
            loading={saving === "auto-update"}
            color={auEnabled ? "blue" : "gray"}
          >
            {auEnabled ? "Guardar Auto-update" : "Desactivar Auto-update"}
          </Button>
        </Group>
      </Paper>
    </Stack>
  );
}
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: clean compile

- [ ] **Step 4: Verify full build**

```bash
cd frontend && npx vite build 2>&1
```
Expected: clean build

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ConfigPage.tsx frontend/src/types.ts
git commit -m "feat: rewrite ConfigPage with Telegram, Matrix and Auto-update controls"
```

---

### Task 5: Update backend workers to use dynamic settings

**Files:**
- Modify: `backend/src/workers.rs`
- Modify: `backend/src/notifications.rs`
- Modify: `backend/src/main.rs`

**Interfaces:**
- Consumes: `Settings` from AppState
- Produces: Workers that respect runtime settings

- [ ] **Step 1: Update auto_update_worker to check settings**

Modify the worker to accept `settings: Arc<Mutex<Settings>>` parameter. Inside the main loop:

```rust
// Before checking interval, resolve enabled state:
let au_enabled = {
    let s = settings.lock().await;
    s.auto_update_enabled.unwrap_or_else(|| config.auto_update())
};
if !au_enabled {
    tokio::time::sleep(Duration::from_secs(60)).await;
    continue;
}
```

- [ ] **Step 2: Update notification functions to use settings**

Modify `notify_all` and `notify_selected` to accept settings and use settings values for telegram/matrix credentials.

In `notifications.rs`, change from reading `config.telegram_token` etc. to checking settings first:

```rust
async fn notify_telegram(settings: &Settings, config: &Config, message: &str) {
    let token = settings.telegram_token.as_deref()
        .or_else(|| config.telegram_token.as_deref());
    let chat_id = settings.telegram_chat_id.as_deref()
        .or_else(|| config.telegram_chat_id.as_deref());
    // ... rest of existing logic
}
```

- [ ] **Step 3: Verify compilation and tests**

```bash
cd backend && cargo check 2>&1 && cargo test 2>&1
```
Expected: clean compile, all tests pass

- [ ] **Step 4: Commit**

```bash
git add backend/src/workers.rs backend/src/notifications.rs
git commit -m "feat: workers resolve Telegram/Matrix/Auto-update from dynamic settings"
```

---

### Task 6: Full integration test and cleanup

**Files:**
- All modified files

- [ ] **Step 1: Verify full backend compiles and tests pass**

```bash
cd backend && cargo clippy -- -D warnings 2>&1
cd backend && cargo test 2>&1
```

- [ ] **Step 2: Verify full frontend build**

```bash
cd frontend && npx tsc --noEmit 2>&1 && npx vite build 2>&1
```

- [ ] **Step 3: Create data/settings.json if missing (startup handles this)**

The `load_json` function already handles missing files gracefully.

- [ ] **Step 4: Final commit**

```bash
git commit --allow-empty -m "chore: integration test pass"
```

---