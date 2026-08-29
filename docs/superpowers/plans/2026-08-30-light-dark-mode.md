# Light / Dark Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add light/dark UI theming with a header ☀️/🌙 toggle; default to OS preference and persist after a manual choice.

**Architecture:** Apply a `dark` class on `document.documentElement` and override the existing `--color-*` CSS variables. Pure helpers in `src/lib/theme.ts` resolve preference → light/dark and sync the DOM; Zustand holds preference + resolved theme for React; header button toggles light ↔ dark.

**Tech Stack:** React 19, Zustand 5, Tailwind CSS v4 (`@import "tailwindcss"` + `@theme` in `src/index.css`), Vite, Tauri 2. No new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-30-light-dark-mode-design.md`
- Storage key: `recode-theme` with values `"light" | "dark" | "system"`
- Toggle UI: emoji only (☀️ when resolved dark, 🌙 when resolved light) — no icon library
- No Settings-page theme control; no “back to system” third control in v1
- Do not introduce a frontend test runner (repo has none); verify with `npx tsc --noEmit` and manual UI checks
- Reply / commit messages: follow repo style; docs under `docs/`
- Do not commit secrets, local env configs, or absolute user paths

## File Structure

| File | Responsibility |
| --- | --- |
| `src/lib/theme.ts` | Types, storage read/write, resolve preference, apply DOM class/`colorScheme`, bootstrap |
| `src/store.ts` | Zustand: `theme`, `resolvedTheme`, `setTheme`, `toggleTheme`, media-query sync |
| `src/main.tsx` | Call `bootstrapTheme()` before React render |
| `src/index.css` | Keep light `@theme` tokens; add `.dark` overrides; theme-aware `body` background |
| `src/App.tsx` | Header toggle button; theme-aware shell gradient |
| `src/components/ui/input.tsx` | Replace `bg-white` with card token |
| `src/components/ui/tabs.tsx` | Replace active `bg-white` with card token |
| `src/pages/Settings.tsx` | Replace select `bg-white` with card token |

---

### Task 1: Theme helpers (`src/lib/theme.ts`)

**Files:**
- Create: `src/lib/theme.ts`
- Modify: none yet
- Test: manual — Node one-liner after create (see Step 4)

**Interfaces:**
- Consumes: `window` / `document` / `localStorage` (browser)
- Produces:
  - `export type ThemePreference = "light" | "dark" | "system"`
  - `export type ResolvedTheme = "light" | "dark"`
  - `export const THEME_STORAGE_KEY = "recode-theme"`
  - `export function readStoredTheme(): ThemePreference`
  - `export function writeStoredTheme(theme: ThemePreference): void`
  - `export function getSystemTheme(): ResolvedTheme`
  - `export function resolveTheme(preference: ThemePreference): ResolvedTheme`
  - `export function applyThemeToDocument(resolved: ResolvedTheme): void`
  - `export function bootstrapTheme(): { preference: ThemePreference; resolved: ResolvedTheme }`

- [ ] **Step 1: Create `src/lib/theme.ts` with the full module**

```ts
export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export const THEME_STORAGE_KEY = "recode-theme";

export function readStoredTheme(): ThemePreference {
  try {
    const raw = localStorage.getItem(THEME_STORAGE_KEY);
    if (raw === "light" || raw === "dark" || raw === "system") return raw;
  } catch {
    // ignore (private mode / unavailable)
  }
  return "system";
}

export function writeStoredTheme(theme: ThemePreference): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // ignore
  }
}

export function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === "system") return getSystemTheme();
  return preference;
}

export function applyThemeToDocument(resolved: ResolvedTheme): void {
  const root = document.documentElement;
  root.classList.toggle("dark", resolved === "dark");
  root.style.colorScheme = resolved;
}

/** Sync read + apply before first paint. */
export function bootstrapTheme(): {
  preference: ThemePreference;
  resolved: ResolvedTheme;
} {
  const preference = readStoredTheme();
  const resolved = resolveTheme(preference);
  applyThemeToDocument(resolved);
  return { preference, resolved };
}
```

- [ ] **Step 2: Smoke-check `resolveTheme` logic in Node (no DOM)**

Run from repo root (PowerShell):

```powershell
node --input-type=module -e "const resolve=(p,sys)=>p==='system'?sys:p; console.assert(resolve('light','dark')==='light'); console.assert(resolve('dark','light')==='dark'); console.assert(resolve('system','dark')==='dark'); console.assert(resolve('system','light')==='light'); console.log('ok')"
```

Expected: prints `ok` (exit 0).

- [ ] **Step 3: Commit**

```powershell
git add src/lib/theme.ts
git commit -m "Add theme preference helpers for light/dark mode."
```

---

### Task 2: Dark CSS tokens + body background

**Files:**
- Modify: `src/index.css`
- Test: visual / DevTools after later tasks; for now ensure CSS parses (dev server)

**Interfaces:**
- Consumes: existing `@theme` light tokens
- Produces: `.dark` overrides for the same `--color-*` names; body uses token-friendly backgrounds

- [ ] **Step 1: Replace `src/index.css` contents with light tokens + dark overrides**

Keep the existing light `@theme` block values. Add `.dark` overrides and theme-aware body:

```css
@import "tailwindcss";

@theme {
  --color-background: oklch(0.99 0.005 85);
  --color-foreground: oklch(0.22 0.02 250);
  --color-card: oklch(1 0 0);
  --color-card-foreground: oklch(0.22 0.02 250);
  --color-muted: oklch(0.96 0.01 85);
  --color-muted-foreground: oklch(0.48 0.02 250);
  --color-border: oklch(0.9 0.01 85);
  --color-primary: oklch(0.42 0.08 230);
  --color-primary-foreground: oklch(0.99 0 0);
  --color-accent: oklch(0.94 0.02 230);
  --color-accent-foreground: oklch(0.28 0.05 230);
  --color-destructive: oklch(0.55 0.2 25);
  --color-ring: oklch(0.42 0.08 230);
  --radius-sm: 0.375rem;
  --radius-md: 0.5rem;
  --radius-lg: 0.75rem;
}

.dark {
  --color-background: oklch(0.18 0.02 250);
  --color-foreground: oklch(0.93 0.01 85);
  --color-card: oklch(0.22 0.02 250);
  --color-card-foreground: oklch(0.93 0.01 85);
  --color-muted: oklch(0.26 0.02 250);
  --color-muted-foreground: oklch(0.72 0.02 250);
  --color-border: oklch(0.32 0.02 250);
  --color-primary: oklch(0.72 0.08 230);
  --color-primary-foreground: oklch(0.18 0.02 250);
  --color-accent: oklch(0.28 0.03 230);
  --color-accent-foreground: oklch(0.9 0.02 230);
  --color-destructive: oklch(0.65 0.18 25);
  --color-ring: oklch(0.72 0.08 230);
}

* {
  border-color: var(--color-border);
}

body {
  margin: 0;
  min-height: 100vh;
  background: linear-gradient(
    165deg,
    color-mix(in oklch, var(--color-background) 70%, var(--color-accent)),
    var(--color-background)
  );
  color: var(--color-foreground);
  font-family: "Segoe UI", "Helvetica Neue", sans-serif;
  -webkit-font-smoothing: antialiased;
}

#root {
  min-height: 100vh;
}
```

- [ ] **Step 2: Commit**

```powershell
git add src/index.css
git commit -m "Add dark theme CSS variable overrides."
```

---

### Task 3: Wire theme into Zustand + bootstrap in `main.tsx`

**Files:**
- Modify: `src/store.ts`
- Modify: `src/main.tsx`
- Test: `npx tsc --noEmit`

**Interfaces:**
- Consumes: all exports from Task 1 `src/lib/theme.ts`
- Produces (Zustand `useAppStore`):
  - `theme: ThemePreference`
  - `resolvedTheme: ResolvedTheme`
  - `setTheme: (theme: ThemePreference) => void`
  - `toggleTheme: () => void`
  - (internal) media-query listener when preference is `system`

- [ ] **Step 1: Replace `src/store.ts` with page + theme state**

`main.tsx` calls `bootstrapTheme()` once before render. The store only reads storage for React state (same key) and listens for OS changes while preference is `system`.

```ts
import { create } from "zustand";
import {
  applyThemeToDocument,
  getSystemTheme,
  readStoredTheme,
  resolveTheme,
  type ResolvedTheme,
  type ThemePreference,
  writeStoredTheme,
} from "./lib/theme";

export type PageId = "dashboard" | "tokens" | "settings";

const initialPreference = readStoredTheme();
const initialResolved = resolveTheme(initialPreference);

interface AppStore {
  page: PageId;
  setPage: (page: PageId) => void;
  theme: ThemePreference;
  resolvedTheme: ResolvedTheme;
  setTheme: (theme: ThemePreference) => void;
  toggleTheme: () => void;
}

export const useAppStore = create<AppStore>((set, get) => {
  if (typeof window !== "undefined" && window.matchMedia) {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", () => {
      if (get().theme !== "system") return;
      const resolved = getSystemTheme();
      applyThemeToDocument(resolved);
      set({ resolvedTheme: resolved });
    });
  }

  return {
    page: "dashboard",
    setPage: (page) => set({ page }),
    theme: initialPreference,
    resolvedTheme: initialResolved,
    setTheme: (theme) => {
      writeStoredTheme(theme);
      const resolved = resolveTheme(theme);
      applyThemeToDocument(resolved);
      set({ theme, resolvedTheme: resolved });
    },
    toggleTheme: () => {
      const next: ThemePreference =
        get().resolvedTheme === "dark" ? "light" : "dark";
      get().setTheme(next);
    },
  };
});
```

- [ ] **Step 2: Update `src/main.tsx` to bootstrap before render**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { bootstrapTheme } from "./lib/theme";
import "./index.css";

bootstrapTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

`bootstrapTheme` sets the DOM class before paint; the store re-reads the same `localStorage` value for React state.

- [ ] **Step 3: Typecheck**

```powershell
npx tsc --noEmit
```

Expected: exit 0, no errors.

- [ ] **Step 4: Commit**

```powershell
git add src/store.ts src/main.tsx
git commit -m "Wire theme preference into store and bootstrap on load."
```

---

### Task 4: Header toggle + shell gradient

**Files:**
- Modify: `src/App.tsx`
- Test: manual UI (Step 3)

**Interfaces:**
- Consumes: `useAppStore` → `resolvedTheme`, `toggleTheme`
- Produces: header button with ☀️/🌙

- [ ] **Step 1: Update `src/App.tsx`**

Replace the file with:

```tsx
import { useAppStore, type PageId } from "./store";
import Dashboard from "./pages/Dashboard";
import Tokens from "./pages/Tokens";
import { SettingsPage } from "./pages/Settings";
import { cn } from "./lib/utils";

const NAV: { id: PageId; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "tokens", label: "Tokens" },
  { id: "settings", label: "Settings" },
];

function App() {
  const page = useAppStore((s) => s.page);
  const setPage = useAppStore((s) => s.setPage);
  const resolvedTheme = useAppStore((s) => s.resolvedTheme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);
  const isDark = resolvedTheme === "dark";

  return (
    <div className="flex h-full min-h-screen flex-col bg-[radial-gradient(ellipse_at_top,_color-mix(in_oklch,var(--color-accent)_55%,var(--color-background))_0%,_var(--color-background)_55%)]">
      <header className="border-b border-[var(--color-border)] bg-[var(--color-card)]/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-3">
          <div className="flex items-baseline gap-3">
            <span className="text-lg font-semibold tracking-tight text-[var(--color-primary)]">
              ReCode
            </span>
            <span className="text-xs text-[var(--color-muted-foreground)]">
              local agent monitor
            </span>
          </div>
          <div className="flex items-center gap-2">
            <nav className="flex gap-1">
              {NAV.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setPage(item.id)}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-sm transition-colors",
                    page === item.id
                      ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                      : "text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)]",
                  )}
                >
                  {item.label}
                </button>
              ))}
            </nav>
            <button
              type="button"
              onClick={() => toggleTheme()}
              aria-label={isDark ? "Switch to light mode" : "Switch to dark mode"}
              title={isDark ? "Switch to light mode" : "Switch to dark mode"}
              className="rounded-md px-2.5 py-1.5 text-base leading-none transition-colors hover:bg-[var(--color-muted)]"
            >
              {isDark ? "☀️" : "🌙"}
            </button>
          </div>
        </div>
      </header>
      <main className="mx-auto w-full max-w-6xl flex-1 px-6 py-6">
        {page === "dashboard" && <Dashboard />}
        {page === "tokens" && <Tokens />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}

export default App;
```

- [ ] **Step 2: Typecheck**

```powershell
npx tsc --noEmit
```

Expected: exit 0.

- [ ] **Step 3: Manual check**

Run `npm run tauri dev` (or `npm run dev` if web-only is enough). Confirm:

1. Header shows 🌙 or ☀️ matching OS (with empty `localStorage` key `recode-theme`).
2. Click toggles emoji and page colors immediately.
3. DevTools → `<html>` has class `dark` when dark; `localStorage.recode-theme` is `light` or `dark` after click.

- [ ] **Step 4: Commit**

```powershell
git add src/App.tsx
git commit -m "Add header sun/moon theme toggle."
```

---

### Task 5: Remove hard-coded white surfaces

**Files:**
- Modify: `src/components/ui/input.tsx`
- Modify: `src/components/ui/tabs.tsx`
- Modify: `src/pages/Settings.tsx` (select `className` only)
- Test: visual dark mode on Dashboard / Tokens / Settings

**Interfaces:**
- Consumes: `--color-card`
- Produces: no `bg-white` on input, active tab, or Settings select

- [ ] **Step 1: In `src/components/ui/input.tsx`, change `bg-white` → `bg-[var(--color-card)]`**

The `className` string on the input should contain:

```ts
"flex h-9 w-full rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-[var(--color-muted-foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-ring)] disabled:cursor-not-allowed disabled:opacity-50"
```

- [ ] **Step 2: In `src/components/ui/tabs.tsx`, change `data-[state=active]:bg-white` → `data-[state=active]:bg-[var(--color-card)]`**

Relevant fragment of the TabsTrigger classes:

```ts
"data-[state=active]:bg-[var(--color-card)] data-[state=active]:text-[var(--color-foreground)] data-[state=active]:shadow-sm"
```

- [ ] **Step 3: In `src/pages/Settings.tsx`, change the native select `bg-white` → `bg-[var(--color-card)]`**

```tsx
className="h-9 max-w-[16rem] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-2 text-xs"
```

- [ ] **Step 4: Manual acceptance**

With dark mode on, open Dashboard, Tokens, and Settings. Confirm no large white input/tab/select holes. Switch thumb may stay white (allowed).

- [ ] **Step 5: Typecheck + commit**

```powershell
npx tsc --noEmit
git add src/components/ui/input.tsx src/components/ui/tabs.tsx src/pages/Settings.tsx
git commit -m "Replace hard-coded white surfaces with theme tokens."
```

---

### Task 6: Final acceptance pass

**Files:** none (verification only)

- [ ] **Step 1: Clear preference and confirm system default**

In DevTools console:

```js
localStorage.removeItem("recode-theme");
location.reload();
```

Expected: theme matches OS; `document.documentElement.classList.contains("dark")` equals OS dark.

- [ ] **Step 2: Persist check**

Toggle once, reload. Expected: same theme; `localStorage.getItem("recode-theme")` is `"light"` or `"dark"`.

- [ ] **Step 3: Emoji check**

Dark → ☀️; light → 🌙. `aria-label` matches the *target* mode.

- [ ] **Step 4: Build typecheck**

```powershell
npx tsc --noEmit
```

Expected: exit 0.

If all pass, no further commit required unless fixes were made during this task — then commit those fixes with a message like `Fix dark mode acceptance issues.`

---

## Spec coverage (self-review)

| Spec requirement | Task |
| --- | --- |
| `html.dark` + CSS var overrides | 1–2 |
| Default `system` + localStorage after toggle | 1, 3, 4 |
| Header ☀️/🌙 toggle | 4 |
| Bootstrap before paint | 3 |
| Fix body / App gradient / input / tabs / Settings select | 2, 4, 5 |
| No Settings theme control / no 3-state UI | — (omitted by design) |
| Acceptance criteria 1–4 | 6 |

## Placeholder / consistency check

- Types `ThemePreference` / `ResolvedTheme` / `THEME_STORAGE_KEY` consistent across tasks.
- `toggleTheme` always writes `"light"` or `"dark"` via `setTheme` (leaves `system`).
- No TBD steps.
