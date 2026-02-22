const isMac = navigator.platform.toUpperCase().includes("MAC");

function formatKeyLabel(key: string): string {
  switch (key) {
    case "CommandOrControl":
      return isMac ? "\u2318" : "Ctrl";
    case "Control":
      return isMac ? "\u2303" : "Ctrl";
    case "Shift":
      return isMac ? "\u21E7" : "Shift";
    case "Alt":
      return isMac ? "\u2325" : "Alt";
    case "Meta":
    case "Command":
      return "\u2318";
    case "Space":
      return "Space";
    default:
      return key;
  }
}

export function HotkeyDisplay({
  hotkey,
  size = "md",
}: {
  hotkey: string;
  size?: "sm" | "md";
}) {
  const keys = hotkey.split("+");

  const keyClass =
    size === "sm"
      ? "px-1.5 py-0.5 text-[11px] min-w-[22px]"
      : "px-2 py-1 text-xs min-w-[28px]";

  return (
    <div className="flex items-center gap-1">
      {keys.map((key, i) => (
        <span
          key={i}
          className={`inline-flex items-center justify-center rounded-md border border-zinc-600 bg-zinc-700/80 text-zinc-300 font-medium shadow-[0_1px_0_1px_rgba(0,0,0,0.3)] ${keyClass}`}
        >
          {formatKeyLabel(key)}
        </span>
      ))}
    </div>
  );
}
