# Light / Dark Mode Design

Date: 2026-08-30  
Status: Approved (pending implementation)

## Goal

Add a light/dark UI theme with a header toggle button using sun/moon emoji. Default to the OS preference; after the user toggles, remember the choice.

## Decisions

| Topic | Choice |
| --- | --- |
| Approach | `html.dark` class + CSS variable overrides |
| Default | Follow `prefers-color-scheme` (`system`) |
| Persistence | `localStorage` key `recode-theme` after manual toggle |
| Toggle placement | Header, right of nav |
| Toggle UI | ☀️ when dark (switch to light), 🌙 when light (switch to dark) |
| Out of scope | Settings duplicate control, three-state “back to system” UI, icon libraries |

## Architecture

### Theme values

- Stored preference: `"light" | "dark" | "system"`
- Resolved theme (what the UI actually uses): `"light" | "dark"`
- When preference is `"system"`, resolve via `window.matchMedia("(prefers-color-scheme: dark)")`
- While preference is `"system"`, listen for `change` on that media query and re-apply

### Application

- Toggle the `dark` class on `document.documentElement`
- Set `document.documentElement.style.colorScheme` to `"light"` or `"dark"` so native controls match
- Bootstrap theme **before** first paint (small sync read in `main.tsx` or an inline/early module) to avoid a flash of the wrong theme

### State

- Extend the existing Zustand store (or a small `useTheme` hook backed by the same storage contract) with:
  - `theme` — stored preference
  - `resolvedTheme` — effective light/dark
  - `setTheme` / `toggle` — toggle writes `"light"` or `"dark"` to localStorage (leaves `"system"` once the user has chosen manually)

## UI

- Button in `App.tsx` header, to the right of the page nav
- Label from **resolved** theme:
  - Resolved dark → show ☀️, `aria-label` ≈ “Switch to light mode”
  - Resolved light → show 🌙, `aria-label` ≈ “Switch to dark mode”
- Click: flip between light and dark and persist

## CSS

### Keep

- Existing `@theme` light tokens in `src/index.css` (`--color-background`, `--color-foreground`, cards, borders, primary, etc.)

### Add

- `.dark { ... }` overrides for the same `--color-*` tokens (dark surfaces, readable foreground, muted borders)

### Fix hard-coded light colors (verified against `src/`)

Must fix for acceptance (white surfaces on dark chrome):

- `body` gradient in `index.css` — drive from tokens / `.dark` rules
- App shell radial gradient using `#eef3f8` in `App.tsx` — theme-aware values
- `src/components/ui/input.tsx` — `bg-white` → card/background token
- `src/components/ui/tabs.tsx` — `data-[state=active]:bg-white` → card token
- `src/pages/Settings.tsx` — native `select` `bg-white` → card token

OK to leave (not light-theme holes):

- Switch thumb `bg-white` in `switch.tsx` — intentional contrast knob
- Destructive button `text-white` — on colored fill
- Chart series fills in `UsageChart.tsx` — intentional data colors; optional later: soften grid `#ddd` for dark

### Tailwind v4 note

Project uses Tailwind v4 (`@tailwindcss/vite`, no `tailwind.config`). This design relies on **CSS variable overrides under `.dark`**, not `dark:` utility classes, so no `@custom-variant dark` is required for the core path. Components that already use `var(--color-*)` pick up dark tokens automatically.

## Non-goals

- Theme control on the Settings page
- Explicit “follow system again” control in v1
- Animation libraries or non-emoji icons
- Chart recoloring (optional polish only)

## Acceptance criteria

1. First launch with empty localStorage follows the OS light/dark setting.
2. Clicking the header button switches immediately; relaunch restores the saved preference.
3. Dashboard, Tokens, and Settings are readable in dark mode with no obvious light “holes” (white cards/inputs/tabs left on dark chrome).
4. Button emoji matches the resolved theme.

## Implementation touchpoints (expected)

- `src/index.css` — dark tokens + body background
- `src/main.tsx` (and/or tiny bootstrap before paint) — early theme apply
- `src/store.ts` (and/or `src/lib/theme.ts`) — preference + resolve + toggle
- `src/App.tsx` — header toggle button + shell gradient
- `src/components/ui/input.tsx`, `tabs.tsx` — replace `bg-white`
- `src/pages/Settings.tsx` — replace `bg-white` on select

## Verification (2026-08-30)

| Check | Result |
| --- | --- |
| Architecture matches codebase (CSS vars + Zustand + header) | Pass |
| Decisions internally consistent | Pass |
| No TBD / ambiguous requirements | Pass |
| Hard-coded light surfaces listed completely | Pass (after this update; input/tabs were missing earlier) |
| Scope still single-feature sized | Pass |
