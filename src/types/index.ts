export type ProviderId =
  | "OpenAiWhisper"
  | "GoogleCloud"
  | "LocalWhisper"
  | "NativeStt";

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
  auto_paste: boolean;
  show_overlay: boolean;
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
