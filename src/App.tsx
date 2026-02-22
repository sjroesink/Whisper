import { useEffect } from "react";
import { useAppStore } from "./stores/useAppStore";
import { setupEventListeners } from "./lib/events";
import { getSettings, getHistory, getProviders } from "./lib/commands";
import { RecordingIndicator } from "./components/recording/RecordingIndicator";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { HistoryList } from "./components/history/HistoryList";

function App() {
  const setRecording = useAppStore((s) => s.setRecording);
  const setTranscribing = useAppStore((s) => s.setTranscribing);
  const setCurrentTranscription = useAppStore((s) => s.setCurrentTranscription);
  const setError = useAppStore((s) => s.setError);
  const setSettings = useAppStore((s) => s.setSettings);
  const setHistory = useAppStore((s) => s.setHistory);
  const setProviders = useAppStore((s) => s.setProviders);
  const addHistory = useAppStore((s) => s.addHistory);
  const activeView = useAppStore((s) => s.activeView);
  const setActiveView = useAppStore((s) => s.setActiveView);
  const error = useAppStore((s) => s.error);
  const currentTranscription = useAppStore((s) => s.currentTranscription);

  // Initialize app
  useEffect(() => {
    getSettings().then(setSettings);
    getHistory().then(setHistory);
    getProviders().then(setProviders);

    const cleanup = setupEventListeners({
      onRecordingStarted: () => {
        setRecording(true);
        setError(null);
      },
      onRecordingStopped: () => {
        setRecording(false);
      },
      onTranscribing: () => {
        setTranscribing(true);
      },
      onTranscriptionComplete: (result) => {
        setTranscribing(false);
        setCurrentTranscription(result.text);
        addHistory({
          id: crypto.randomUUID(),
          text: result.text,
          provider: result.provider,
          timestamp: new Date().toISOString(),
          duration_ms: result.duration_ms,
          language: result.language,
        });
      },
      onError: (err) => {
        setRecording(false);
        setTranscribing(false);
        setError(err);
      },
    });

    return () => {
      cleanup.then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-zinc-900 text-zinc-100 select-none">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-zinc-800">
        <h1 className="text-base font-semibold tracking-tight">Whisper</h1>
        <RecordingIndicator />
      </header>

      {/* Navigation */}
      <nav className="flex border-b border-zinc-800">
        {(["home", "history", "settings"] as const).map((view) => (
          <button
            key={view}
            onClick={() => setActiveView(view)}
            className={`flex-1 px-4 py-2.5 text-sm font-medium transition-colors ${
              activeView === view
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-zinc-500 hover:text-zinc-300"
            }`}
          >
            {view.charAt(0).toUpperCase() + view.slice(1)}
          </button>
        ))}
      </nav>

      {/* Content */}
      <main className="flex-1 overflow-hidden">
        {activeView === "home" && (
          <div className="flex flex-col items-center justify-center h-full gap-6 px-6">
            {/* Status */}
            <div className="flex flex-col items-center gap-4">
              <div className="w-24 h-24 rounded-full bg-zinc-800 flex items-center justify-center">
                <svg
                  className="w-10 h-10 text-zinc-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={1.5}
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M12 18.75a6 6 0 006-6v-1.5m-6 7.5a6 6 0 01-6-6v-1.5m6 7.5v3.75m-3.75 0h7.5M12 15.75a3 3 0 01-3-3V4.5a3 3 0 116 0v8.25a3 3 0 01-3 3z"
                  />
                </svg>
              </div>

              <div className="text-center">
                <p className="text-sm text-zinc-400">
                  Press your hotkey to start recording
                </p>
                <p className="text-xs text-zinc-600 mt-1">
                  Default: Ctrl+Shift+Space
                </p>
              </div>
            </div>

            {/* Last transcription */}
            {currentTranscription && (
              <div className="w-full max-w-sm bg-zinc-800/60 rounded-lg p-4">
                <p className="text-xs text-zinc-500 mb-2">Last transcription</p>
                <p className="text-sm text-zinc-200 leading-relaxed">
                  {currentTranscription}
                </p>
              </div>
            )}

            {/* Error */}
            {error && (
              <div className="w-full max-w-sm bg-red-950/50 border border-red-800/50 rounded-lg p-3">
                <p className="text-xs text-red-400">{error}</p>
              </div>
            )}
          </div>
        )}

        {activeView === "history" && <HistoryList />}
        {activeView === "settings" && <SettingsPanel />}
      </main>
    </div>
  );
}

export default App;
