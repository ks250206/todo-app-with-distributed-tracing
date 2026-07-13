import { create } from "zustand";

export type TodoFilter = "all" | "open" | "done";
export type AuthMode = "login" | "register";

type UiState = {
  filter: TodoFilter;
  authMode: AuthMode;
  setFilter: (filter: TodoFilter) => void;
  setAuthMode: (mode: AuthMode) => void;
};

export const useUiStore = create<UiState>((set) => ({
  filter: "all",
  authMode: "login",
  setFilter: (filter) => set({ filter }),
  setAuthMode: (authMode) => set({ authMode }),
}));
