import { create } from "zustand";
import type {
  AppSettings,
  TranscriptionEntry,
  ProviderInfo,
} from "../types";

interface AppState {
  // Recording state
  isRecording: boolean;
  isTranscribing: boolean;

  // Data
  settings: AppSettings | null;
  history: TranscriptionEntry[];
  providers: ProviderInfo[];
  currentTranscription: string;
  error: string | null;

  // View
  activeView: "home" | "settings" | "history";

  // Actions
  setRecording: (val: boolean) => void;
  setTranscribing: (val: boolean) => void;
  setSettings: (s: AppSettings) => void;
  setHistory: (entries: TranscriptionEntry[]) => void;
  addHistory: (entry: TranscriptionEntry) => void;
  setProviders: (providers: ProviderInfo[]) => void;
  setCurrentTranscription: (text: string) => void;
  setError: (error: string | null) => void;
  setActiveView: (view: "home" | "settings" | "history") => void;
}

export const useAppStore = create<AppState>((set) => ({
  isRecording: false,
  isTranscribing: false,
  settings: null,
  history: [],
  providers: [],
  currentTranscription: "",
  error: null,
  activeView: "home",

  setRecording: (val) => set({ isRecording: val }),
  setTranscribing: (val) => set({ isTranscribing: val }),
  setSettings: (s) => set({ settings: s }),
  setHistory: (entries) => set({ history: entries }),
  addHistory: (entry) =>
    set((state) => ({
      history: [entry, ...state.history].slice(0, 100),
    })),
  setProviders: (providers) => set({ providers }),
  setCurrentTranscription: (text) => set({ currentTranscription: text }),
  setError: (error) => set({ error }),
  setActiveView: (view) => set({ activeView: view }),
}));
