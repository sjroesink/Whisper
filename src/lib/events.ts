import type { TranscriptionResult } from "../types";

export interface EventCallbacks {
  onRecordingStarted: () => void;
  onRecordingStopped: () => void;
  onTranscribing: () => void;
  onTranscriptionComplete: (result: TranscriptionResult) => void;
  onError: (error: string) => void;
}

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

export async function setupEventListeners(
  callbacks: EventCallbacks
): Promise<Array<() => void>> {
  if (!isTauri) {
    return [];
  }

  const { listen } = await import("@tauri-apps/api/event");
  const unlisteners: Array<() => void> = [];

  unlisteners.push(
    await listen("recording-started", () => {
      callbacks.onRecordingStarted();
    })
  );

  unlisteners.push(
    await listen("recording-stopped", () => {
      callbacks.onRecordingStopped();
    })
  );

  unlisteners.push(
    await listen("transcribing", () => {
      callbacks.onTranscribing();
    })
  );

  unlisteners.push(
    await listen<TranscriptionResult>("transcription-complete", (event) => {
      callbacks.onTranscriptionComplete(event.payload);
    })
  );

  unlisteners.push(
    await listen<string>("error", (event) => {
      callbacks.onError(event.payload);
    })
  );

  return unlisteners;
}
