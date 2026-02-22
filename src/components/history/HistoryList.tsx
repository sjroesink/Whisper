import { useEffect } from "react";
import { useAppStore } from "../../stores/useAppStore";
import { getHistory, clearHistory } from "../../lib/commands";

export function HistoryList() {
  const history = useAppStore((s) => s.history);
  const setHistory = useAppStore((s) => s.setHistory);

  useEffect(() => {
    getHistory().then(setHistory);
  }, []);

  const handleCopy = async (text: string) => {
    await navigator.clipboard.writeText(text);
  };

  const handleClear = async () => {
    await clearHistory();
    setHistory([]);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-6 pb-3">
        <h2 className="text-lg font-semibold text-zinc-100">History</h2>
        {history.length > 0 && (
          <button
            onClick={handleClear}
            className="text-xs text-zinc-500 hover:text-red-400 transition-colors"
          >
            Clear All
          </button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {history.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-zinc-500 text-sm">
            No transcriptions yet
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {history.map((entry) => (
              <div
                key={entry.id}
                className="group bg-zinc-800/60 rounded-lg p-3 hover:bg-zinc-800 transition-colors"
              >
                <p className="text-sm text-zinc-200 leading-relaxed">
                  {entry.text}
                </p>
                <div className="flex items-center justify-between mt-2">
                  <div className="flex items-center gap-2 text-xs text-zinc-500">
                    <span>{entry.provider}</span>
                    <span>&middot;</span>
                    <span>{(entry.duration_ms / 1000).toFixed(1)}s</span>
                    <span>&middot;</span>
                    <span>
                      {new Date(entry.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <button
                    onClick={() => handleCopy(entry.text)}
                    className="opacity-0 group-hover:opacity-100 text-xs text-zinc-400 hover:text-zinc-200 transition-all"
                  >
                    Copy
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
