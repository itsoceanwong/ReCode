# Blank / white screen on release startup

## Symptom

`recodeV1.exe` opens a window but the UI stays blank, or the process exits right after start.

## Root causes (fixed)

### 1. Setup blocked by Codex `scan_all` (primary hang / white window)

`watcher::start` called `CodexProvider::scan_all` **synchronously** inside Tauri `setup`. With large Codex trees (~4.3GB / 660 rollout files under `~/.codex/sessions` + `archived_sessions`), setup never finished, so the WebView never became a working UI.

**Fix:** run the initial Codex scan on a background thread.

Debug signal: `%APPDATA%\com.recode.app\debug-startup.log` stopped after settings and never reached `watcher started`.

### 2. `tokio::spawn` in scheduler panicked after the hang was fixed

Once setup reached `scheduler::start`, it used `tokio::spawn` off the Tauri setup thread (no current Tokio context) → panic → process exit within ~1s.

**Fix:** use `tauri::async_runtime::spawn` (same as the watcher).

### 3. Absolute Vite asset URLs (defense in depth)

`vite.config.ts` sets `base: "./"` so release HTML references `./assets/...`.

## Verify

```powershell
.\src-tauri\target\release\recodeV1.exe debug
```

Log should quickly include:

```text
[startup] watcher started
[startup] scheduler started
[startup] setup complete
```

App should stay running; Codex scan may finish later in the background.
