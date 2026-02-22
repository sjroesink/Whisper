import type {
  AppSettings,
  AudioDevice,
  ConstmeWhisperStatus,
  ProviderInfo,
  TranscriptionEntry,
} from "../types";

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) throw new Error("Not running in Tauri");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const DEFAULT_HOTKEY = "CommandOrControl+Shift+Space";

const defaultSettings: AppSettings = {
  active_provider: "OpenAiWhisper",
  interaction_mode: "Toggle",
  hotkey: DEFAULT_HOTKEY,
  language: "auto",
  provider_configs: {},
  local_whisper_model_path: null,
  constme_whisper_dll_path: null,
  constme_whisper_model_path: null,
  constme_whisper_model_name: null,
  auto_paste: true,
  show_overlay: true,
  input_device: null,
};

const defaultProviders: ProviderInfo[] = [
  { id: "OpenAiWhisper", name: "OpenAI Whisper", available: true },
  { id: "GoogleCloud", name: "Google Cloud Speech-to-Text", available: true },
  { id: "LocalWhisper", name: "Local Whisper (whisper.cpp)", available: false },
  { id: "NativeStt", name: "Native OS Speech-to-Text", available: true },
  { id: "ConstmeWhisper", name: "Whisper GPU (DirectCompute)", available: false },
];

export async function startRecording(): Promise<void> {
  return tauriInvoke("start_recording");
}

export async function stopRecordingAndTranscribe(): Promise<string> {
  return tauriInvoke("stop_recording_and_transcribe");
}

export async function getRecordingState(): Promise<boolean> {
  return tauriInvoke<boolean>("get_recording_state").catch(() => false);
}

export async function getSettings(): Promise<AppSettings> {
  return tauriInvoke<AppSettings>("get_settings").catch(() => defaultSettings);
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return tauriInvoke("save_settings", { settings });
}

export async function getHistory(): Promise<TranscriptionEntry[]> {
  return tauriInvoke<TranscriptionEntry[]>("get_history").catch(() => []);
}

export async function clearHistory(): Promise<void> {
  return tauriInvoke("clear_history");
}

export async function getProviders(): Promise<ProviderInfo[]> {
  return tauriInvoke<ProviderInfo[]>("get_providers").catch(() => defaultProviders);
}

export async function listInputDevices(): Promise<AudioDevice[]> {
  return tauriInvoke<AudioDevice[]>("list_input_devices").catch(() => []);
}

export async function getConstmeWhisperStatus(): Promise<ConstmeWhisperStatus> {
  return tauriInvoke<ConstmeWhisperStatus>("get_constme_whisper_status");
}

export async function downloadConstmeDll(): Promise<string> {
  return tauriInvoke<string>("download_constme_dll");
}

export async function downloadConstmeModel(modelFilename: string): Promise<string> {
  return tauriInvoke<string>("download_constme_model", { modelFilename });
}
