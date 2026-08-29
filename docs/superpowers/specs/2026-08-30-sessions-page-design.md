# Sessions Page Design

Date: 2026-08-30  
Status: Approved (pending implementation)

## Goal

Add a top-level **Sessions** page for managing discovered Codex / Claude Code sessions: readable titles (no more `codex · unknown`), filters (tool / project / date), and auto-continue controls moved out of Settings.

## Decisions

| Topic | Choice |
| --- | --- |
| Approach | Backend local enrich + dedicated Sessions page |
| Navigation | Top nav: Dashboard / Tokens / **Sessions** / Settings |
| Auto-continue | Lives only on Sessions; remove from Settings |
| Title format | `{project} - {session title}` (e.g. `StudySystem - 評估 Firebase 題庫雲端更新`) |
| Codex title | `~/.codex/session_index.jsonl` → `thread_name` by session id |
| Claude title | Latest `customTitle`, else `aiTitle`, from `~/.claude/projects/.../<id>.jsonl` |
| Title fallback | First 8 chars of session id; never show `unknown` as the name |
| Subagents | Always hide Codex subagent / guardian threads |
| Tool filter | All / Codex / Claude Code (no Cursor on this page) |
| Project filter | Dropdown from distinct `project` values + All |
| Date filter | Today / 7d / 30d / All (by `last_seen`) |
| Default date | **Today** |
| Enrich runs | Local only (Tauri reads filesystem); no remote upload |
| Out of scope | Cursor sessions, custom date range picker, full-text search, separate title index DB |

## Architecture

### Data model

Extend `SessionView` (Rust + TypeScript):

- `display_name: string` — primary list title
- `project: string | null` — basename of `cwd` (e.g. `StudySystem`)
- Keep existing: `id`, `source`, `cwd`, `model`, auto-continue fields, `last_seen`

### Enrich pipeline (on `get_sessions`)

1. Load sessions from SQLite as today.
2. Drop / never expose Codex sessions identified as subagent or guardian (`thread_source` / session_meta such as `subagent`, `guardian_review`, or non-`user` main threads). Prefer skipping at scan/upsert when meta is available; also filter at enrich so the Sessions page never lists them.
3. Build Codex title map once from `session_index.jsonl` (`id` → `thread_name`).
4. For each remaining session, set:
   - `project` = last path segment of `cwd` (Windows and POSIX)
   - session title = Codex `thread_name` / Claude `customTitle`→`aiTitle` / short id
   - `display_name` = `"{project} - {title}"` if project present, else `{title}`
5. Do not treat model `"unknown"` as a display name; omit model from subtitle when missing or `"unknown"`.

### Local sources (read-only)

| Tool | Path | Fields |
| --- | --- | --- |
| Codex | `~/.codex/session_index.jsonl` | `id`, `thread_name`, `updated_at` |
| Codex | session rollout `session_meta` | `cwd`, `thread_source` (for hide rules) |
| Claude | `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` | events `custom-title` / `ai-title` |

All enrichment is best-effort: missing files or parse errors fall back per session without failing the whole list.

### Frontend

- New page `src/pages/Sessions.tsx`; register `PageId` `"sessions"` in store + `App.tsx` nav.
- Filter client-side on the enriched list (tool / project / date).
- Date windows use local calendar days (same spirit as Tokens’ day buttons), default **Today**.
- Row actions: auto-continue switch, injection target select, Send test (moved from Settings).
- Empty copy: no data vs no filter matches.

### Settings

Remove the 「Sessions / auto-continue」 card only. Leave continue prompt, telemetry, Cursor, pricing unchanged.

## UI

- Header: title **Sessions**, short subtitle about manage + auto-continue.
- Filter row: Tool buttons, Project `<select>`, Date buttons (Today / 7d / 30d / All).
- List rows: `display_name` primary; subtitle = model (if useful) · path hint · last seen; controls on the right.
- Visual language: match existing Cards / Buttons / Tokens filter chip style; no new design system.

## Error handling

- Index / JSONL unreadable → short-id titles, list still loads.
- Single-session enrich failure → that row falls back; others unaffected.
- Injection / autocontinue API errors → surface status/error text on the Sessions page (same pattern as Settings today).

## Testing

### Rust unit tests (fixtures / temp files)

- Codex: map `thread_name` from sample `session_index.jsonl`.
- Codex: subagent / guardian sessions excluded from Sessions list.
- Claude: `customTitle` wins over `aiTitle`; neither → short id.
- `display_name` formatting with and without project.
- Model `unknown` / empty not used as title.

### Manual acceptance

1. Top nav includes Sessions; Settings has no session list.
2. Codex main sessions show `Project - thread_name`; no `codex · unknown`.
3. Claude sessions show `Project - custom/ai title`.
4. Filters: tool, project, date (default Today) work.
5. Auto-continue toggle, target, Send test work on Sessions.

## Non-goals

- Syncing or uploading session transcripts
- Editing / renaming sessions inside ReCode
- Showing Cursor composer sessions on this page
- Building a durable secondary title index database
