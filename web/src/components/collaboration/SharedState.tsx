import { useAtom } from "jotai";
import { collaborationSharedStateAtom } from "@/atoms/collaboration";

export function SharedState() {
  const [entries] = useAtom(collaborationSharedStateAtom);

  if (entries.length === 0) {
    return (
      <p className="text-xs text-muted-foreground py-4 text-center">
        No shared state entries.
      </p>
    );
  }

  return (
    <div className="overflow-auto max-h-60">
      <table className="w-full text-xs">
        <thead>
          <tr className="border-b border-border text-muted-foreground">
            <th className="text-left py-1 pr-4 font-medium">Key</th>
            <th className="text-left py-1 font-medium">Value</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.key} className="border-b border-border/50 hover:bg-secondary/30">
              <td className="py-1 pr-4 font-mono text-muted-foreground">{entry.key}</td>
              <td className="py-1">
                <pre className="font-mono text-xs text-foreground whitespace-pre-wrap break-all">
                  {typeof entry.value === "string"
                    ? entry.value
                    : JSON.stringify(entry.value, null, 2)}
                </pre>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
