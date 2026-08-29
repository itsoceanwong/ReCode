import { create } from "zustand";

export type PageId = "dashboard" | "tokens" | "settings";

interface AppStore {
  page: PageId;
  setPage: (page: PageId) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  page: "dashboard",
  setPage: (page) => set({ page }),
}));
