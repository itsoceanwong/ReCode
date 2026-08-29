# ReCode

Cross-platform (Windows + macOS) **read-only** desktop monitor for vibe-coding agents (Claude Code, Codex, Cursor, ChatGPT/Claude desktop).

## What it does

1. Shows each tool's **5-hour / 7-day** limit usage % and reset time.
2. Tracks **token usage + USD cost** per app and model (local SQLite).
3. Optionally sends a customizable **continue** prompt into a focused app/terminal after a limit resets (human-like paste + Enter).
4. **Never launches agents.** Reads local files / OTLP and injects via OS UI automation.
5. **All data stays on-device.** The only network socket is the localhost OTLP metrics listener.

## Requirements

- Node 20+
- Rust stable + MSVC Build Tools (Windows) or Xcode CLT (macOS)
- For Claude accurate tokens: Claude Code with telemetry env (ReCode can write it)

## Develop

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

## Cursor usage

ReCode reads Cursor's local `state.vscdb` (read-only copy) under
`User/globalStorage`. It extracts `cursorDiskKV` rows:

- `bubbleId:*` → `tokenCount.inputTokens` / `outputTokens` (when non-zero)
- fallback `composerData:*` → context / promptTokenBreakdown totals

Accuracy is **approximate** (badged as such). Auth tokens are never read.
Toggle or force-scan from Settings → Cursor usage. If the DB is missing,
use **Add manual usage row**.

## Portable paths

All data paths resolve at runtime from the current user / OS:

| Data | Resolution |
| --- | --- |
| ReCode DB | Tauri `app_data_dir()` |
| `~/.recode`, Claude, Codex | `HOME` / `USERPROFILE` (+ `CLAUDE_CONFIG_DIR`, `CODEX_HOME`) |
| Cursor `state.vscdb` | `APPDATA` / `LOCALAPPDATA` / XDG / Application Support, or `CURSOR_USER_DATA_DIR` |

No machine-absolute paths (e.g. `C:\Users\…`) are baked into the app.

## Claude telemetry

Settings → Enable telemetry merges OTEL env + statusline hook into `~/.claude/settings.json` (atomic write + `.recode.bak`). If [cc-switch](https://github.com) rewrites that file, ReCode self-heals when telemetry remains enabled.

Default continue prompt: `read the history, continue on the work`.
