# Sessions Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a top-level Sessions page with local title enrichment (Codex `thread_name`, Claude `customTitle`/`aiTitle`), tool/project/date filters (default Today), hide Codex subagent/guardian threads, and move auto-continue controls out of Settings.

**Architecture:** Pure Rust helpers in `session_enrich.rs` load local Codex/Claude title sources and build `display_name` / `project`. `CodexProvider` skips `upsert_session` when `thread_source != "user"`. `get_sessions` loads DB rows then enriches/filters before returning. Frontend adds `Sessions.tsx` with client-side filters; Settings loses the sessions card.

**Tech Stack:** Tauri 2 + Rust (rusqlite, serde_json, walkdir), React 19, Zustand, date-fns (already used by Tokens), existing UI Button/Card/Switch/Input. No new npm deps.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-30-sessions-page-design.md`
- Titles never use model `"unknown"`; fallback is first 8 chars of session id
- Codex Sessions allowlist: `thread_source == "user"` at scan; if `session_index.jsonl` loads, drop Codex ids absent from the index at enrich
- Claude title priority: latest `customTitle` > latest `aiTitle` > short id
- Display: `{project} - {title}` when project present
- Date filter default: **Today**; options Today / 7d / 30d / All
- Tool filter: All / Codex / Claude only (no Cursor)
- Enrich is local filesystem only; never hardcode `C:\Users\...` paths — use `paths::codex_dir()` / `paths::claude_dir()`
- No frontend test runner; verify UI with `npx tsc --noEmit` + manual checks; Rust with `cargo test`
- Docs under `docs/`; do not commit secrets or absolute user paths

## File Structure

| File | Responsibility |
| --- | --- |
| `src-tauri/src/session_enrich.rs` | Title maps, project basename, Claude JSONL title parse, enrich+filter `SessionView` list |
| `src-tauri/src/models.rs` | Add `display_name`, `project` on `SessionView` |
| `src-tauri/src/db.rs` | Construct new fields with placeholders when reading rows (enrich overwrites) |
| `src-tauri/src/providers/codex.rs` | Skip `upsert_session` for non-`user` `thread_source`; avoid storing model `"unknown"` |
| `src-tauri/src/commands.rs` | `get_sessions` calls enrich after DB load |
| `src-tauri/src/lib.rs` | `mod session_enrich;` |
| `src/lib/types.ts` | Mirror `display_name`, `project` |
| `src/pages/Sessions.tsx` | Filters + list + auto-continue UI |
| `src/store.ts` / `src/App.tsx` | `PageId` + nav entry |
| `src/pages/Settings.tsx` | Remove Sessions / auto-continue card |

---

### Task 1: Session enrich helpers + unit tests

**Files:**
- Create: `src-tauri/src/session_enrich.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod session_enrich;`)
- Modify: `src-tauri/src/models.rs` (add `display_name`, `project` on `SessionView`)
- Modify: `src-tauri/src/db.rs` (placeholder values when reading rows)
- Modify: `src-tauri/Cargo.toml` (dev-dep `tempfile` if missing)
- Test: unit tests inside `session_enrich.rs`

**Interfaces:**
- Consumes: `SessionView`, `paths::codex_dir`, `paths::claude_dir`
- Produces:
  - `pub fn project_from_cwd(cwd: Option<&str>) -> Option<String>`
  - `pub fn short_id(id: &str) -> String`
  - `pub fn format_display_name(project: Option<&str>, title: &str) -> String`
  - `pub fn load_codex_thread_names(index_path: &Path) -> Result<HashMap<String, String>>`
  - `pub fn read_claude_session_title(jsonl_path: &Path) -> Option<String>`
  - `pub fn find_claude_session_jsonl(projects_root: &Path, session_id: &str) -> Option<PathBuf>`
  - `pub fn project_from_claude_projects_dir(dir_name: &str) -> Option<String>`
  - `pub fn enrich_sessions(sessions: Vec<SessionView>) -> Vec<SessionView>`

- [ ] **Step 1: Write failing tests for formatting + Codex index parse**

Add `src-tauri/src/session_enrich.rs` with tests first (helpers can be stubbed to fail compile until Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn project_from_cwd_windows_and_posix() {
        assert_eq!(
            project_from_cwd(Some(r"C:\Users\x\Coding\StudySystem")).as_deref(),
            Some("StudySystem")
        );
        assert_eq!(
            project_from_cwd(Some("/home/x/Coding/StudySystem")).as_deref(),
            Some("StudySystem")
        );
        assert_eq!(project_from_cwd(None), None);
    }

    #[test]
    fn format_display_name_with_and_without_project() {
        assert_eq!(
            format_display_name(Some("StudySystem"), "評估 Firebase"),
            "StudySystem - 評估 Firebase"
        );
        assert_eq!(format_display_name(None, "abc"), "abc");
    }

    #[test]
    fn load_codex_thread_names_from_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session_index.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"id":"aaa","thread_name":"評估 Firebase 題庫雲端更新","updated_at":"2026-08-30T00:00:00Z"}}"#
        )
        .unwrap();
        let map = load_codex_thread_names(&path).unwrap();
        assert_eq!(
            map.get("aaa").map(String::as_str),
            Some("評估 Firebase 題庫雲端更新")
        );
    }

    #[test]
    fn claude_custom_title_beats_ai_title() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"ai-title","aiTitle":"AI Name","sessionId":"sid"}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"custom-title","customTitle":"Custom Name","sessionId":"sid"}}"#
        )
        .unwrap();
        assert_eq!(
            read_claude_session_title(&path).as_deref(),
            Some("Custom Name")
        );
    }

    #[test]
    fn enrich_drops_codex_ids_missing_from_index_when_index_ok() {
        let dir = tempfile::tempdir().unwrap();
        let index = dir.path().join("session_index.jsonl");
        std::fs::write(
            &index,
            r#"{"id":"keep-me","thread_name":"Main thread","updated_at":"2026-08-30T00:00:00Z"}
"#,
        )
        .unwrap();

        let prev = std::env::var_os("CODEX_HOME");
        std::env::set_var("CODEX_HOME", dir.path());

        let input = vec![
            SessionView {
                id: "keep-me".into(),
                source: "codex".into(),
                cwd: Some(r"C:\proj\StudySystem".into()),
                model: Some("unknown".into()),
                auto_continue_enabled: false,
                continue_prompt: None,
                target_kind: None,
                target_ref: None,
                last_seen: Some(1),
                display_name: String::new(),
                project: None,
            },
            SessionView {
                id: "drop-me".into(),
                source: "codex".into(),
                cwd: None,
                model: None,
                auto_continue_enabled: false,
                continue_prompt: None,
                target_kind: None,
                target_ref: None,
                last_seen: Some(1),
                display_name: String::new(),
                project: None,
            },
        ];
        let out = enrich_sessions(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "keep-me");
        assert_eq!(out[0].display_name, "StudySystem - Main thread");
        assert!(out[0].model.is_none());

        match prev {
            Some(v) => std::env::set_var("CODEX_HOME", v),
            None => std::env::remove_var("CODEX_HOME"),
        }
    }
}
```

If `tempfile` is not in `Cargo.toml`, add under `[dev-dependencies]`: `tempfile = "3"`.

- [ ] **Step 2: Run tests — expect compile/fail**

Run: `cd src-tauri && cargo test session_enrich -- --nocapture`  
Expected: FAIL (module/functions missing or tests incomplete)

- [ ] **Step 3: Extend `SessionView` then implement helpers**

In `models.rs` add:

```rust
    pub display_name: String,
    pub project: Option<String>,
```

In `db.rs` `get_sessions`, set `display_name: /* id */ row.get::<_, String>(0)?` clone into both `id` and initial `display_name`, and `project: None`.

Then implement the helpers:

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::Value;
use walkdir::WalkDir;

use crate::models::SessionView;
use crate::paths;

pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub fn project_from_cwd(cwd: Option<&str>) -> Option<String> {
    let cwd = cwd?;
    let p = Path::new(cwd);
    p.file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Claude encodes cwd as e.g. `C--Users-wongo-Coding-StudySystem`.
pub fn project_from_claude_projects_dir(dir_name: &str) -> Option<String> {
    dir_name
        .rsplit('-')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub fn format_display_name(project: Option<&str>, title: &str) -> String {
    match project {
        Some(p) if !p.is_empty() => format!("{p} - {title}"),
        _ => title.to_string(),
    }
}

pub fn load_codex_thread_names(index_path: &Path) -> std::io::Result<HashMap<String, String>> {
    let file = File::open(index_path)?;
    let reader = BufReader::new(file);
    let mut map = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let id = v.get("id").and_then(|x| x.as_str());
        let name = v.get("thread_name").and_then(|x| x.as_str());
        if let (Some(id), Some(name)) = (id, name) {
            if !name.is_empty() {
                map.insert(id.to_string(), name.to_string());
            }
        }
    }
    Ok(map)
}

pub fn read_claude_session_title(jsonl_path: &Path) -> Option<String> {
    let file = File::open(jsonl_path).ok()?;
    let reader = BufReader::new(file);
    let mut custom: Option<String> = None;
    let mut ai: Option<String> = None;
    for line in reader.lines().flatten() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match typ {
            "custom-title" => {
                if let Some(t) = v.get("customTitle").and_then(|x| x.as_str()) {
                    if !t.is_empty() {
                        custom = Some(t.to_string());
                    }
                }
            }
            "ai-title" => {
                if let Some(t) = v.get("aiTitle").and_then(|x| x.as_str()) {
                    if !t.is_empty() {
                        ai = Some(t.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    custom.or(ai)
}

pub fn find_claude_session_jsonl(projects_root: &Path, session_id: &str) -> Option<PathBuf> {
    if !projects_root.exists() {
        return None;
    }
    let want = format!("{session_id}.jsonl");
    for entry in WalkDir::new(projects_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(want.as_str()) {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn usable_model(model: &Option<String>) -> Option<String> {
    model.as_ref().and_then(|m| {
        let t = m.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("unknown") {
            None
        } else {
            Some(t.to_string())
        }
    })
}

pub fn enrich_sessions(mut sessions: Vec<SessionView>) -> Vec<SessionView> {
    let index_path = paths::codex_dir().join("session_index.jsonl");
    let codex_names = load_codex_thread_names(&index_path).ok();
    let index_ok = codex_names.is_some();
    let names = codex_names.unwrap_or_default();
    let projects_root = paths::claude_dir().join("projects");

    if index_ok {
        sessions.retain(|s| {
            if s.source != "codex" {
                return true;
            }
            names.contains_key(&s.id)
        });
    }

    for s in &mut sessions {
        s.model = usable_model(&s.model);

        let mut project = project_from_cwd(s.cwd.as_deref());
        let title = if s.source == "codex" {
            names
                .get(&s.id)
                .cloned()
                .unwrap_or_else(|| short_id(&s.id))
        } else if s.source == "claude" {
            if let Some(jsonl) = find_claude_session_jsonl(&projects_root, &s.id) {
                if project.is_none() {
                    if let Some(parent) = jsonl.parent().and_then(|p| p.file_name()) {
                        project = project_from_claude_projects_dir(&parent.to_string_lossy());
                    }
                }
                read_claude_session_title(&jsonl).unwrap_or_else(|| short_id(&s.id))
            } else {
                short_id(&s.id)
            }
        } else {
            short_id(&s.id)
        };

        s.project = project;
        s.display_name = format_display_name(s.project.as_deref(), &title);
    }

    sessions
}
```

Complete the `enrich_drops_codex_ids_missing_from_index_when_index_ok` test by setting `CODEX_HOME` to a temp dir (save/restore env like `paths.rs` tests).

- [ ] **Step 4: Register module**

In `src-tauri/src/lib.rs`, add `mod session_enrich;` near other `mod` lines.

- [ ] **Step 5: Run tests — expect pass**

Run: `cd src-tauri && cargo test session_enrich -- --nocapture`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/session_enrich.rs src-tauri/src/lib.rs src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/Cargo.toml
git commit -m "feat: add local session title enrich helpers"
```

---

### Task 2: Wire `get_sessions` + TypeScript types

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/lib/types.ts`
- Test: `cargo check` / `npx tsc --noEmit`

**Interfaces:**
- Consumes: `session_enrich::enrich_sessions` (Task 1)
- Produces: enriched `SessionView[]` to the frontend

- [ ] **Step 1: Enrich in command**

```rust
#[tauri::command]
pub fn get_sessions(state: State<'_, AppState>) -> Result<Vec<SessionView>, String> {
    let rows = state.db.get_sessions().map_err(|e| e.to_string())?;
    Ok(crate::session_enrich::enrich_sessions(rows))
}
```

- [ ] **Step 2: Extend TypeScript type**

```ts
export interface SessionView {
  id: string;
  source: string;
  cwd: string | null;
  model: string | null;
  auto_continue_enabled: boolean;
  continue_prompt: string | null;
  target_kind: TargetKind | null;
  target_ref: string | null;
  last_seen: number | null;
  display_name: string;
  project: string | null;
}
```

- [ ] **Step 3: Compile**

Run: `cd src-tauri && cargo test session_enrich && cargo check`  
Expected: PASS / compile OK  
Run: `npx tsc --noEmit` (from repo root)  
Expected: OK (Settings may still compile if it only uses fields that remain)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src/lib/types.ts
git commit -m "feat: return enriched sessions from get_sessions"
```

---

### Task 3: Codex scan — skip non-user threads + avoid `unknown` model

**Files:**
- Modify: `src-tauri/src/providers/codex.rs`
- Test: new unit test for thread_source gate helper

**Interfaces:**
- Consumes: `session_meta.payload.thread_source`
- Produces: `upsert_session` only for main `user` threads; model `None` when still unknown

- [ ] **Step 1: Add helper + test**

```rust
fn is_manageable_codex_thread(thread_source: Option<&str>) -> bool {
    match thread_source {
        None => true, // legacy / missing: allow (index filter cleans later)
        Some("user") => true,
        Some(_) => false,
    }
}

#[cfg(test)]
mod thread_source_tests {
    use super::is_manageable_codex_thread;

    #[test]
    fn only_user_or_missing_are_manageable() {
        assert!(is_manageable_codex_thread(Some("user")));
        assert!(is_manageable_codex_thread(None));
        assert!(!is_manageable_codex_thread(Some("subagent")));
        assert!(!is_manageable_codex_thread(Some("guardian_review")));
    }
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri && cargo test only_user_or_missing_are_manageable -- --nocapture`  
Expected: PASS once helper exists

- [ ] **Step 3: Gate `session_meta` upsert**

In the `"session_meta"` arm of `scan_file`:

```rust
"session_meta" => {
    if let Some(id) = payload.get("id").and_then(|v| v.as_str()) {
        session_id = id.to_string();
    }
    if let Some(c) = payload.get("cwd").and_then(|v| v.as_str()) {
        cwd = Some(c.to_string());
    }
    let thread_source = payload
        .get("thread_source")
        .and_then(|v| v.as_str());
    let manageable = is_manageable_codex_thread(thread_source);
    // Track skip for later upserts in this file if desired:
    // e.g. let mut skip_session_row = !manageable;
    if manageable && !session_id.is_empty() {
        let model_ref = if model == "unknown" {
            None
        } else {
            Some(model.as_str())
        };
        let _ = db.upsert_session(
            &session_id,
            "codex",
            cwd.as_deref(),
            model_ref,
            ts,
        );
    }
}
```

Also gate the later `upsert_session` after token events: if this rollout's `thread_source` was non-user, skip session upsert (still allow usage insert if you keep usage for billing — prefer skipping session row only). Simplest approach: keep a `manageable_session: bool` flag set in `session_meta` (default `true` until meta says otherwise).

When calling `upsert_session` after token_count, pass `None` instead of `Some("unknown")` for model.

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers/codex.rs
git commit -m "fix: skip Codex subagent sessions and unknown model labels"
```

---

### Task 4: Sessions page UI + navigation

**Files:**
- Create: `src/pages/Sessions.tsx`
- Modify: `src/store.ts` (`PageId`)
- Modify: `src/App.tsx`
- Modify: `src/pages/Settings.tsx` (remove sessions card + unused imports/state)
- Test: `npx tsc --noEmit`

**Interfaces:**
- Consumes: `api.getSessions`, `api.setSessionAutocontinue`, `api.listInjectionTargets`, `api.testInjection`, `SessionView.display_name` / `project` / `last_seen`
- Produces: Sessions page with filters defaulting date to Today

- [ ] **Step 1: Extend store + nav**

`src/store.ts`:

```ts
export type PageId = "dashboard" | "tokens" | "sessions" | "settings";
```

`src/App.tsx`:

```ts
import Sessions from "./pages/Sessions";

const NAV: { id: PageId; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "tokens", label: "Tokens" },
  { id: "sessions", label: "Sessions" },
  { id: "settings", label: "Settings" },
];

// in main:
{page === "sessions" && <Sessions />}
```

- [ ] **Step 2: Create `src/pages/Sessions.tsx`**

```tsx
import { useEffect, useMemo, useState } from "react";
import { endOfDay, startOfDay, subDays } from "date-fns";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import type { InjectionTarget, SessionView } from "@/lib/types";

type ToolFilter = "all" | "codex" | "claude";
type DateFilter = "today" | "7d" | "30d" | "all";

function inDateFilter(lastSeen: number | null, filter: DateFilter): boolean {
  if (filter === "all") return true;
  if (lastSeen == null) return false;
  const to = Math.floor(endOfDay(new Date()).getTime() / 1000);
  const days = filter === "today" ? 1 : filter === "7d" ? 7 : 30;
  const from = Math.floor(startOfDay(subDays(new Date(), days - 1)).getTime() / 1000);
  return lastSeen >= from && lastSeen <= to;
}

export default function Sessions() {
  const [sessions, setSessions] = useState<SessionView[]>([]);
  const [targets, setTargets] = useState<InjectionTarget[]>([]);
  const [tool, setTool] = useState<ToolFilter>("all");
  const [project, setProject] = useState<string>("all");
  const [date, setDate] = useState<DateFilter>("today");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    const sess = await api.getSessions();
    setSessions(sess);
    try {
      setTargets(await api.listInjectionTargets());
    } catch {
      setTargets([]);
    }
  }

  useEffect(() => {
    refresh().catch((e: unknown) =>
      setError(e instanceof Error ? e.message : String(e)),
    );
  }, []);

  const projects = useMemo(() => {
    const set = new Set<string>();
    for (const s of sessions) {
      if (s.project) set.add(s.project);
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }, [sessions]);

  const filtered = useMemo(() => {
    return sessions.filter((s) => {
      if (tool !== "all" && s.source !== tool) return false;
      if (project !== "all" && s.project !== project) return false;
      if (!inDateFilter(s.last_seen, date)) return false;
      return true;
    });
  }, [sessions, tool, project, date]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Sessions</h1>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Manage discovered sessions and auto-continue.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 items-center">
          {(["all", "codex", "claude"] as const).map((t) => (
            <Button
              key={t}
              size="sm"
              variant={tool === t ? "default" : "outline"}
              onClick={() => setTool(t)}
            >
              {t === "all" ? "All" : t === "codex" ? "Codex" : "Claude Code"}
            </Button>
          ))}
          <select
            className="h-9 max-w-[12rem] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-2 text-xs"
            value={project}
            onChange={(e) => setProject(e.target.value)}
          >
            <option value="all">All projects</option>
            {projects.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
          {(
            [
              ["today", "Today"],
              ["7d", "7d"],
              ["30d", "30d"],
              ["all", "All"],
            ] as const
          ).map(([id, label]) => (
            <Button
              key={id}
              size="sm"
              variant={date === id ? "default" : "outline"}
              onClick={() => setDate(id)}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>

      {error && (
        <p className="text-sm text-[var(--color-destructive)]">{error}</p>
      )}
      {status && (
        <p className="text-sm text-[var(--color-primary)]">{status}</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Sessions / auto-continue</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {sessions.length === 0 ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">
              No sessions discovered yet.
            </p>
          ) : filtered.length === 0 ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">
              No sessions match these filters.
            </p>
          ) : (
            filtered.map((s) => (
              <div
                key={s.id}
                className="flex flex-wrap items-center gap-3 border-b border-[var(--color-border)] py-2 text-sm"
              >
                <div className="min-w-[12rem] grow">
                  <div className="font-medium">{s.display_name}</div>
                  <div className="text-xs text-[var(--color-muted-foreground)]">
                    {[s.source, s.model, s.cwd || s.id]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                </div>
                <Switch
                  checked={s.auto_continue_enabled}
                  onCheckedChange={(enabled) =>
                    void api
                      .setSessionAutocontinue(s.id, enabled)
                      .then(refresh)
                      .catch((e: unknown) =>
                        setError(e instanceof Error ? e.message : String(e)),
                      )
                  }
                />
                <select
                  className="h-9 max-w-[16rem] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-2 text-xs"
                  value={s.target_ref ?? ""}
                  onChange={(e) => {
                    const reference = e.target.value;
                    const t = targets.find((x) => x.reference === reference);
                    if (!t) return;
                    void api
                      .setSessionAutocontinue(
                        s.id,
                        s.auto_continue_enabled,
                        s.continue_prompt,
                        t,
                      )
                      .then(refresh);
                  }}
                >
                  <option value="">Injection target…</option>
                  {targets.map((t) => (
                    <option key={t.reference} value={t.reference}>
                      {t.reference}
                    </option>
                  ))}
                </select>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!s.target_ref}
                  onClick={() => {
                    const t = targets.find((x) => x.reference === s.target_ref);
                    if (!t) return;
                    void api
                      .testInjection(t, "ReCode test message")
                      .then((o) => setStatus(`Test: ${JSON.stringify(o)}`));
                  }}
                >
                  Send test
                </Button>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Remove Sessions card from Settings**

Delete the entire `<Card>…Sessions / auto-continue…</Card>` block from `src/pages/Settings.tsx`. Remove unused `sessions` / `targets` state and related `api.getSessions` / `listInjectionTargets` calls if nothing else needs them. Keep continue prompt / telemetry / Cursor / pricing.

- [ ] **Step 4: Typecheck**

Run: `npx tsc --noEmit`  
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add src/pages/Sessions.tsx src/store.ts src/App.tsx src/pages/Settings.tsx
git commit -m "feat: add Sessions page with filters and move auto-continue"
```

---

### Task 5: Manual acceptance + docs note

**Files:**
- Optional: short note under `docs/` only if something diverged (otherwise skip)

- [ ] **Step 1: Run automated checks**

```bash
cd src-tauri && cargo test
cd .. && npx tsc --noEmit
```

Expected: all pass

- [ ] **Step 2: Manual UI checklist**

1. Top nav shows Sessions between Tokens and Settings  
2. Settings has no session list  
3. Codex rows show `Project - thread_name` (not `codex · unknown`)  
4. Claude rows show `Project - custom/ai title` when JSONL exists  
5. Subagent/guardian Codex threads absent from list  
6. Filters: tool, project, date; **default Today**  
7. Auto-continue switch, target, Send test work  

- [ ] **Step 3: Final commit if any fixups**

```bash
git add -A
git status   # ensure no secrets / absolute user paths
git commit -m "fix: Sessions page acceptance polish"
```

(Only if there are fixup changes.)

---

## Self-review vs spec

| Spec requirement | Task |
| --- | --- |
| Top-level Sessions nav | Task 4 |
| Auto-continue only on Sessions; remove from Settings | Task 4 |
| `display_name` / `project` enrich | Tasks 1–2 |
| Codex `thread_name` from `session_index.jsonl` | Task 1 |
| Claude `customTitle` > `aiTitle` | Task 1 |
| Hide non-`user` + index allowlist | Tasks 1, 3 |
| Filters tool/project/date; default Today | Task 4 |
| No `unknown` as name | Tasks 1, 3 |
| Local-only paths via `paths::*` | Task 1 |
| Tests for title/hide rules | Tasks 1, 3 |
| Manual acceptance | Task 5 |
