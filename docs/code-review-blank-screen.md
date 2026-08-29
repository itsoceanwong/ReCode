# Code review: blank screen on startup

**Fixed point:** `main` (`9d57895`, same as `HEAD`)  
**Scope:** uncommitted working tree (`git diff main`)  
**Date:** 2026-08-29

## Root cause (runtime + config)

Release WebView stayed blank because Vite emitted **absolute** asset URLs (`/assets/index-….js`). Under Tauri’s custom protocol those scripts/styles never load, so React never mounts. Backend setup still succeeds (DB + `%APPDATA%\com.recode.app\debug-startup.log`).

**Fix in source:** `vite.config.ts` → `base: "./"` so `dist` uses `./assets/...`.  
**Must also:** rebuild frontend **and** re-embed into `recodeV1.exe` (cargo/tauri build). A stale exe keeps the old absolute paths.

---

## Standards

### Hard violations（documented）

**`src-tauri/src/lib.rs` — nesting > 4**  
Standard: `AGENTS.md`「嵌套不超过 4 层」。  
Cursor scan thread 與 OTLP `spawn` 內：`spawn` → `match` → `Ok` → `if debug*` → `if let Ok(mut f)` 寫檔，深度 ≥ 5。setup 本體的 `dlog` 路徑尚可，這兩段超標。

### Documented standards — no breach in this diff

- Prefer edit over rewrite：皆為局部增補，非整檔重寫。  
- Dev docs under `\docs`：本 diff 未新增開發文件。  
- No local env / `.claude` / `.codex` / API keys / machine-absolute home paths：所列檔案未見。

### Judgement calls（Fowler baseline）

**Duplicated Code — `lib.rs`**  
setup 有 `dlog`，但背景執行緒又各寫一遍 `eprintln` + append file（cursor scan 與 OTLP 各一份）。應共用可跨執行緒的 log helper。

**Data Clumps — `lib.rs`**  
`debug` + `debug_log_path`（及 clone）一起傳進多個 closure；可收成小 struct／logger。

**Mysterious Name — `lib.rs`**  
`dlog` 過簡；`debug_log_path_scan` / `_otlp` 冗長，反映共用抽象不足。

**Divergent Change — `lib.rs` `run`/`setup`**  
同一 setup 再塞除錯檔、DevTools、非阻塞 scan；變更理由增多。

### Clean

`index.html`、`vite.config.ts`（`base: "./"`）、`Cargo.toml`、`tauri.conf.json`、`main.rs`（`win_debug_console`）對齊除錯目標；無路徑／金鑰問題。

---

## Spec

### (a) Missing / partial

- **Rebuild + re-embed (partial / unverified in this tree).** Spec: *"Then rebuild frontend **and** the Rust binary (assets are embedded at compile time)."* Source fix is present (`base: "./"`); `dist/index.html` correctly has `./assets/...`. The diff cannot prove `recodeV1.exe` was rebuilt after that dist — binary may still be stale. End-to-end blank-screen fix is only complete after a release rebuild that re-embeds current `dist`.

### (b) Scope creep

- Spec root cause/fix is Vite `base` + rebuild. Diff also changes **`index.html`**: favicon `/vite.svg` → `/favicon.png`, and injects `"Loading ReCode..."` into `#root` — not asked by the blank-screen doc or the debug/cursor-scan scope.
- Spec does not ask for binary renaming. Diff adds **`mainBinaryName: "recodeV1"`** in `tauri.conf.json`.

### (c) Looks done but wrong / shaky

- **Blank-screen root cause in source: correct.** Spec: *"`vite.config.ts` must set `base: \"./\"` so `dist/index.html` references `./assets/...`"*. Implemented; built HTML matches Verify. No evidence the asset-path diagnosis was mis-fixed.
- **Debug CLI / DevTools: matches brief.** Present in `main.rs` / `lib.rs` + `devtools` feature.
- **Non-blocking cursor scan: matches brief.** Scan moved to `std::thread::spawn`.

**Verdict:** Asset URL root cause is correctly addressed in config + current `dist`. Remaining Spec gap is operational: confirm release binary was rebuilt so embedded frontend is the fixed `dist`, not a pre-`base` embed.

---

## Summary

| Axis | Findings | Worst |
|------|----------|--------|
| **Standards** | 1 hard (nesting) + 4 smells | Nesting > 4 in `lib.rs` debug append paths |
| **Spec** | 1 partial (re-embed) + 2 scope creep | Stale exe may still ship absolute `/assets/` |

Sub-agents: [Standards](b162990d-affe-4ab8-8d14-ba1860cef562), [Spec](c0380ea0-319f-4599-9934-692a1fa48293)
