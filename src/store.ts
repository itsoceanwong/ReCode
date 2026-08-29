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

export type PageId = "dashboard" | "tokens" | "sessions" | "settings";

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
