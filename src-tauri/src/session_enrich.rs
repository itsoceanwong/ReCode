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

pub fn enrich_sessions_with_dirs(
    mut sessions: Vec<SessionView>,
    codex_dir: &Path,
    claude_dir: &Path,
) -> Vec<SessionView> {
    let index_path = codex_dir.join("session_index.jsonl");
    let codex_names = load_codex_thread_names(&index_path).ok();
    let index_ok = codex_names.is_some();
    let names = codex_names.unwrap_or_default();
    let projects_root = claude_dir.join("projects");

    // Sessions page is Codex + Claude only (no Cursor).
    sessions.retain(|s| s.source == "codex" || s.source == "claude");

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

pub fn enrich_sessions(sessions: Vec<SessionView>) -> Vec<SessionView> {
    enrich_sessions_with_dirs(sessions, &paths::codex_dir(), &paths::claude_dir())
}

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
    fn find_claude_session_jsonl_by_id() {
        let root = tempfile::tempdir().unwrap();
        let proj = root.path().join("C--Users-x-Coding-StudySystem");
        std::fs::create_dir_all(&proj).unwrap();
        let sid = "889318d1-7972-4d97-bbdd-820fe0be1f60";
        let path = proj.join(format!("{sid}.jsonl"));
        std::fs::write(&path, "{}\n").unwrap();
        assert_eq!(find_claude_session_jsonl(root.path(), sid), Some(path));
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

        let claude_home = tempfile::tempdir().unwrap();

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
        let out = enrich_sessions_with_dirs(input, dir.path(), claude_home.path());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "keep-me");
        assert_eq!(out[0].display_name, "StudySystem - Main thread");
        assert!(out[0].model.is_none());
    }
}
