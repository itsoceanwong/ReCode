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

### Fix hard-coded light colors

- `body` gradient in `index.css` — drive from tokens / `.dark` rules
- App shell radial gradient using `#eef3f8` — replace with theme-aware values
- Settings `select` using `bg-white` — use `bg-[var(--color-card)]` (or equivalent token)

## Non-goals

- Theme control on the Settings page
- Explicit “follow system again” control in v1
- Animation libraries or non-emoji icons

## Acceptance criteria

1. First launch with empty localStorage follows the OS light/dark setting.
2. Clicking the header button switches immediately; relaunch restores the saved preference.
3. Dashboard, Tokens, and Settings are readable in dark mode with no obvious light “holes” (white cards/inputs left on dark chrome).
4. Button emoji matches the resolved theme.

## Implementation touchpoints (expected)

- `src/index.css` — dark tokens + body background
- `src/main.tsx` — early theme apply
- `src/store.ts` (and/or `src/lib/theme.ts`) — preference + resolve + toggle
- `src/App.tsx` — header toggle button
- `src/pages/Settings.tsx` — replace `bg-white` on select
- Possibly other components if hard-coded light backgrounds appear during implementation
