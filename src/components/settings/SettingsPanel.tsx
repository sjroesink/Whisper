import { useEffect, useRef, useState } from "react";
import { useAppStore } from "../../stores/useAppStore";
import { DEFAULT_HOTKEY, getProviders, listInputDevices, saveSettings } from "../../lib/commands";
import { HotkeyDisplay } from "../HotkeyDisplay";
import type { AppSettings, AudioDevice, ProviderId } from "../../types";

export function SettingsPanel() {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const providers = useAppStore((s) => s.providers);
  const setProviders = useAppStore((s) => s.setProviders);

  const [localSettings, setLocalSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const hotkeyRef = useRef<HTMLDivElement>(null);
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

  const handleHotkeyKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();

    // Escape: cancel and blur without changes
    if (e.key === "Escape") {
      setIsRecordingHotkey(false);
      hotkeyRef.current?.blur();
      return;
    }

    // Backspace/Delete: reset to default
    if (e.key === "Backspace" || e.key === "Delete") {
      updateField("hotkey", DEFAULT_HOTKEY);
      setIsRecordingHotkey(false);
      hotkeyRef.current?.blur();
      return;
    }

    // Ignore standalone modifier keys
    if (
      ["Control", "Alt", "Shift", "Meta", "Command"].includes(e.key)
    ) {
      return;
    }

    // Build the hotkey string
    const parts: string[] = [];

    if (e.ctrlKey || e.metaKey) {
      parts.push("CommandOrControl");
    }
    if (e.altKey) {
      parts.push("Alt");
    }
    if (e.shiftKey) {
      parts.push("Shift");
    }

    let key = e.key;
    if (key === " ") {
      key = "Space";
    } else if (key.length === 1) {
      key = key.toUpperCase();
    }

    parts.push(key);

    updateField("hotkey", parts.join("+"));
    setIsRecordingHotkey(false);
    hotkeyRef.current?.blur();
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
        <div
          ref={hotkeyRef}
          tabIndex={0}
          onFocus={() => setIsRecordingHotkey(true)}
          onBlur={() => setIsRecordingHotkey(false)}
          onKeyDown={handleHotkeyKeyDown}
          className={`flex items-center h-10 bg-zinc-800 border rounded-lg px-3 cursor-pointer transition-colors ${
            isRecordingHotkey
              ? "border-blue-500 ring-2 ring-blue-500/30"
              : "border-zinc-700 hover:border-zinc-600"
          }`}
        >
          {isRecordingHotkey ? (
            <span className="text-sm text-zinc-400 animate-pulse">
              Press a key combination...
            </span>
          ) : (
            <HotkeyDisplay hotkey={localSettings.hotkey} size="sm" />
          )}
        </div>
        <p className="text-xs text-zinc-500">
          Click to record a shortcut. Backspace to reset. Escape to cancel.
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
