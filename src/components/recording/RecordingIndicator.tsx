import { useAppStore } from "../../stores/useAppStore";

export function RecordingIndicator() {
  const isRecording = useAppStore((s) => s.isRecording);
  const isTranscribing = useAppStore((s) => s.isTranscribing);

  if (!isRecording && !isTranscribing) return null;

  return (
    <div className="flex items-center gap-3 px-4 py-3 rounded-xl bg-zinc-800/80 backdrop-blur">
      {isRecording && (
        <>
          <span className="relative flex h-3 w-3">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75" />
            <span className="relative inline-flex rounded-full h-3 w-3 bg-red-500" />
          </span>
          <span className="text-sm text-zinc-200 font-medium">Recording...</span>
        </>
      )}
      {isTranscribing && (
        <>
          <svg
            className="animate-spin h-4 w-4 text-blue-400"
            viewBox="0 0 24 24"
            fill="none"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
            />
          </svg>
          <span className="text-sm text-zinc-200 font-medium">
            Transcribing...
          </span>
        </>
      )}
    </div>
  );
}
