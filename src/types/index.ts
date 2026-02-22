export type ProviderId =
  | "OpenAiWhisper"
  | "GoogleCloud"
  | "LocalWhisper"
  | "NativeStt"
  | "ConstmeWhisper";

export interface ProviderConfig {
  api_key: string | null;
  model: string | null;
  language: string | null;
  endpoint: string | null;
}

export type InteractionMode = "PushToTalk" | "Toggle";

export interface AppSettings {
  active_provider: ProviderId;
  interaction_mode: InteractionMode;
  hotkey: string;
  language: string;
  provider_configs: Record<string, ProviderConfig>;
  local_whisper_model_path: string | null;
  constme_whisper_dll_path: string | null;
  constme_whisper_model_path: string | null;
  constme_whisper_model_name: string | null;
  auto_paste: boolean;
  show_overlay: boolean;
  input_device: string | null;
}

export interface AudioDevice {
  name: string;
  is_default: boolean;
}

export interface TranscriptionResult {
  text: string;
  provider: ProviderId;
  duration_ms: number;
  language: string | null;
}

export interface TranscriptionEntry {
  id: string;
  text: string;
  provider: ProviderId;
  timestamp: string;
  duration_ms: number;
  language: string | null;
}

export interface ProviderInfo {
  id: ProviderId;
  name: string;
  available: boolean;
}

export interface ConstmeModelStatus {
  name: string;
  filename: string;
  size_description: string;
  available: boolean;
  path: string | null;
}

export interface ConstmeWhisperStatus {
  dll_available: boolean;
  dll_path: string | null;
  models: ConstmeModelStatus[];
}

export interface DownloadProgress {
  item: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  done: boolean;
  error: string | null;
}
