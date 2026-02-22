import { useEffect, useState } from "react";
import { useAppStore } from "../../stores/useAppStore";
import {
  getProviders,
  listInputDevices,
  saveSettings,
  getConstmeWhisperStatus,
  downloadConstmeDll,
  downloadConstmeModel,
} from "../../lib/commands";
import type {
  AppSettings,
  AudioDevice,
  ConstmeWhisperStatus,
  ProviderId,
} from "../../types";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function ConstmeWhisperSettings({
  localSettings,
  updateField,
}: {
  localSettings: AppSettings;
  updateField: <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => void;
}) {
  const [status, setStatus] = useState<ConstmeWhisperStatus | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadStatus();

    // Listen for download progress events
    let cleanup: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<{
        item: string;
        downloaded_bytes: number;
        total_bytes: number | null;
        done: boolean;
      }>("download-progress", (event) => {
        const { item, downloaded_bytes, total_bytes, done } = event.payload;
        if (done) {
          setDownloadProgress("");
          loadStatus();
        } else {
          const downloaded = formatBytes(downloaded_bytes);
          const total = total_bytes ? formatBytes(total_bytes) : "?";
          setDownloadProgress(`${item}: ${downloaded} / ${total}`);
        }
      }).then((unlisten) => {
        cleanup = unlisten;
      });
    });

    return () => cleanup?.();
  }, []);

  const loadStatus = () => {
    getConstmeWhisperStatus()
      .then(setStatus)
      .catch(() => {});
  };

  const handleDownloadDll = async () => {
    setDownloading("dll");
    setError(null);
    try {
      const path = await downloadConstmeDll();
      updateField("constme_whisper_dll_path", path);
      loadStatus();
    } catch (e) {
      setError(`DLL download failed: ${e}`);
    }
    setDownloading(null);
  };

  const handleDownloadModel = async (filename: string) => {
    setDownloading(filename);
    setError(null);
    try {
      const path = await downloadConstmeModel(filename);
      updateField("constme_whisper_model_path", path);
      updateField("constme_whisper_model_name", filename);
      loadStatus();
    } catch (e) {
      setError(`Model download failed: ${e}`);
    }
    setDownloading(null);
  };

  return (
    <section className="flex flex-col gap-3">
      <label className="text-sm font-medium text-zinc-400">
        Whisper GPU Setup
      </label>

      {/* DLL Status */}
      <div className="bg-zinc-800/50 rounded-lg p-3 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="text-xs text-zinc-400">Whisper.dll</span>
          {status?.dll_available ? (
            <span className="text-xs text-green-400">Installed</span>
          ) : (
            <button
              onClick={handleDownloadDll}
              disabled={downloading !== null}
              className="text-xs px-2 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white rounded transition-colors"
            >
              {downloading === "dll" ? "Downloading..." : "Download"}
            </button>
          )}
        </div>

        {/* Model Selection */}
        <div className="flex flex-col gap-1.5 mt-1">
          <span className="text-xs text-zinc-400">Model</span>
          {status?.models.map((model) => (
            <div
              key={model.filename}
              className="flex items-center justify-between bg-zinc-900/50 rounded px-2 py-1.5"
            >
              <div className="flex flex-col">
                <span className="text-xs text-zinc-300">{model.name}</span>
                <span className="text-[10px] text-zinc-500">
                  {model.size_description}
                </span>
              </div>
              {model.available ? (
                <button
                  onClick={() => {
                    if (model.path) {
                      updateField("constme_whisper_model_path", model.path);
                      updateField(
                        "constme_whisper_model_name",
                        model.filename
                      );
                    }
                  }}
                  className={`text-xs px-2 py-1 rounded transition-colors ${
                    localSettings.constme_whisper_model_name === model.filename
                      ? "bg-green-600/30 text-green-400 border border-green-600/50"
                      : "bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
                  }`}
                >
                  {localSettings.constme_whisper_model_name === model.filename
                    ? "Selected"
                    : "Select"}
                </button>
              ) : (
                <button
                  onClick={() => handleDownloadModel(model.filename)}
                  disabled={downloading !== null}
                  className="text-xs px-2 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white rounded transition-colors"
                >
                  {downloading === model.filename
                    ? "Downloading..."
                    : "Download"}
                </button>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Download Progress */}
      {downloadProgress && (
        <p className="text-xs text-blue-400">{downloadProgress}</p>
      )}

      {/* Error */}
      {error && <p className="text-xs text-red-400">{error}</p>}

      <p className="text-xs text-zinc-500">
        GPU-accelerated transcription using DirectCompute. Requires a Direct3D
        11 compatible GPU. Medium model recommended for best balance of speed
        and accuracy.
      </p>
    </section>
  );
}

export function SettingsPanel() {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const providers = useAppStore((s) => s.providers);
  const setProviders = useAppStore((s) => s.setProviders);

  const [localSettings, setLocalSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [inputDevices, setInputDevices] = useState<AudioDevice[]>([]);

  useEffect(() => {
    if (settings) {
      setLocalSettings({ ...settings });
    }
  }, [settings]);

  useEffect(() => {
    getProviders().then(setProviders);
    listInputDevices().then(setInputDevices);
  }, []);

  if (!localSettings) return null;

  const updateField = <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => {
    setLocalSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const updateProviderConfig = (
    providerId: string,
    field: string,
    value: string
  ) => {
    setLocalSettings((prev) => {
      if (!prev) return prev;
      const configs = { ...prev.provider_configs };
      configs[providerId] = {
        ...configs[providerId],
        [field]: value || null,
      };
      return { ...prev, provider_configs: configs };
    });
  };

  const handleSave = async () => {
    if (!localSettings) return;
    setSaving(true);
    try {
      await saveSettings(localSettings);
      setSettings(localSettings);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
    setSaving(false);
  };

  const activeProviderConfig =
    localSettings.provider_configs[localSettings.active_provider] || {};

  return (
    <div className="flex flex-col gap-6 p-6 overflow-y-auto h-full">
      <h2 className="text-lg font-semibold text-zinc-100">Settings</h2>

      {/* Provider Selection */}
      <section className="flex flex-col gap-2">
        <label className="text-sm font-medium text-zinc-400">
          Speech-to-Text Provider
        </label>
        <select
          value={localSettings.active_provider}
          onChange={(e) =>
            updateField("active_provider", e.target.value as ProviderId)
          }
          className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {providers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} {!p.available && "(unavailable)"}
            </option>
          ))}
        </select>
      </section>

      {/* Input Device */}
      <section className="flex flex-col gap-2">
        <label className="text-sm font-medium text-zinc-400">
          Input Device
        </label>
        <select
          value={localSettings.input_device || ""}
          onChange={(e) =>
            updateField("input_device", e.target.value || null)
          }
          className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">System Default</option>
          {inputDevices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name}{d.is_default ? " (default)" : ""}
            </option>
          ))}
        </select>
      </section>

      {/* API Key (for cloud providers) */}
      {(localSettings.active_provider === "OpenAiWhisper" ||
        localSettings.active_provider === "GoogleCloud") && (
        <section className="flex flex-col gap-2">
          <label className="text-sm font-medium text-zinc-400">API Key</label>
          <input
            type="password"
            value={activeProviderConfig.api_key || ""}
            onChange={(e) =>
              updateProviderConfig(
                localSettings.active_provider,
                "api_key",
                e.target.value
              )
            }
            placeholder="Enter your API key..."
            className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </section>
      )}

      {/* Model Selection for Local Whisper */}
      {localSettings.active_provider === "LocalWhisper" && (
        <section className="flex flex-col gap-2">
          <label className="text-sm font-medium text-zinc-400">
            Model Path
          </label>
          <input
            type="text"
            value={localSettings.local_whisper_model_path || ""}
            onChange={(e) =>
              updateField("local_whisper_model_path", e.target.value || null)
            }
            placeholder="/path/to/ggml-base.bin"
            className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <p className="text-xs text-zinc-500">
            Path to a whisper.cpp GGML model file (e.g. ggml-base.bin)
          </p>
        </section>
      )}

      {/* Const-me/Whisper GPU Settings */}
      {localSettings.active_provider === "ConstmeWhisper" && (
        <ConstmeWhisperSettings
          localSettings={localSettings}
          updateField={updateField}
        />
      )}

      {/* Interaction Mode */}
      <section className="flex flex-col gap-2">
        <label className="text-sm font-medium text-zinc-400">
          Interaction Mode
        </label>
        <div className="flex gap-2">
          <button
            onClick={() => updateField("interaction_mode", "PushToTalk")}
            className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
              localSettings.interaction_mode === "PushToTalk"
                ? "bg-blue-600 text-white"
                : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
            }`}
          >
            Push to Talk
          </button>
          <button
            onClick={() => updateField("interaction_mode", "Toggle")}
            className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
              localSettings.interaction_mode === "Toggle"
                ? "bg-blue-600 text-white"
                : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
            }`}
          >
            Toggle
          </button>
        </div>
        <p className="text-xs text-zinc-500">
          {localSettings.interaction_mode === "PushToTalk"
            ? "Hold the hotkey to record, release to transcribe"
            : "Press hotkey once to start, press again to stop and transcribe"}
        </p>
      </section>

      {/* Hotkey */}
      <section className="flex flex-col gap-2">
        <label className="text-sm font-medium text-zinc-400">Hotkey</label>
        <input
          type="text"
          value={localSettings.hotkey}
          onChange={(e) => updateField("hotkey", e.target.value)}
          placeholder="CommandOrControl+Shift+Space"
          className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <p className="text-xs text-zinc-500">
          Example: CommandOrControl+Shift+Space
        </p>
      </section>

      {/* Language */}
      <section className="flex flex-col gap-2">
        <label className="text-sm font-medium text-zinc-400">Language</label>
        <select
          value={localSettings.language}
          onChange={(e) => updateField("language", e.target.value)}
          className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="auto">Auto-detect</option>
          <option value="en">English</option>
          <option value="nl">Nederlands</option>
          <option value="de">Deutsch</option>
          <option value="fr">Français</option>
          <option value="es">Español</option>
          <option value="it">Italiano</option>
          <option value="pt">Português</option>
          <option value="ja">Japanese</option>
          <option value="zh">Chinese</option>
          <option value="ko">Korean</option>
        </select>
      </section>

      {/* Toggles */}
      <section className="flex flex-col gap-3">
        <label className="flex items-center justify-between cursor-pointer">
          <span className="text-sm text-zinc-300">Auto-paste after transcription</span>
          <input
            type="checkbox"
            checked={localSettings.auto_paste}
            onChange={(e) => updateField("auto_paste", e.target.checked)}
            className="w-4 h-4 accent-blue-500"
          />
        </label>
      </section>

      {/* Save Button */}
      <button
        onClick={handleSave}
        disabled={saving}
        className="mt-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white text-sm font-medium rounded-lg transition-colors"
      >
        {saving ? "Saving..." : saved ? "Saved!" : "Save Settings"}
      </button>
    </div>
  );
}
