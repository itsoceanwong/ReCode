# Debug console (Windows release)

Release builds of `recodeV1.exe` use the Windows GUI subsystem, so they have **no console**. Passing a bare `debug` argument used to do nothing — the flag was never parsed, and `eprintln!` had nowhere to go.

## Usage

From PowerShell or cmd:

```text
.\src-tauri\target\release\recodeV1.exe debug
```

or:

```text
.\src-tauri\target\release\recodeV1.exe --debug
```

You should see lines like:

```text
ReCode: debug console enabled (arg: debug|--debug)
[startup] app_data_dir = ...
[startup] db_path      = ...
[startup] watcher started
[startup] otlp listening on port ...
[startup] scheduler started; setup complete
```

## Behavior

1. Detect `debug` or `--debug` in CLI args.
2. Attach to the parent terminal (`AttachConsole`), or open a new console window (`AllocConsole`) if there is no parent.
3. Reconnect CRT stdio so Rust `eprintln!` works.
4. Set `RECODE_DEBUG=1` for the process; `lib.rs` setup prints startup diagnostics when that env is set.
5. The same lines are also written to `%APPDATA%\com.recode.app\debug-startup.log` (Tauri `app_data_dir` on Windows is Roaming).
6. WebView DevTools open automatically so you can inspect blank-screen / JS errors.

Dev builds (`cargo tauri dev` / debug assertions) already have a console; the flag still enables the extra `[startup]` lines.
