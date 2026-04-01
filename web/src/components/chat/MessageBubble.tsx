import { useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import type { ChatMsg } from "@/atoms/session";
import { Brain, Copy, Check } from "lucide-react";
import { MarkdownRenderer } from "./MarkdownRenderer";

interface Props {
  message: ChatMsg;
}

export function MessageBubble({ message }: Props) {
  const isUser = message.role === "user";
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(message.content).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [message.content]);

  return (
    <div className={cn("group relative flex w-full", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "relative max-w-[80%] rounded-lg px-4 py-2 text-sm",
          isUser
            ? "bg-primary text-primary-foreground whitespace-pre-wrap"
            : "bg-secondary text-foreground",
        )}
      >
        {/* Copy button — visible on hover */}
        <button
          type="button"
          onClick={handleCopy}
          className="absolute top-1.5 right-1.5 rounded p-1 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-muted/60"
          title="Copy message"
        >
          {copied ? (
            <Check className="h-3.5 w-3.5 text-green-500" />
          ) : (
            <Copy className="h-3.5 w-3.5 text-muted-foreground" />
          )}
        </button>

        {message.thinking && (
          <details className="mb-2">
            <summary className="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground/70">
              <Brain className="h-3 w-3" />
              <span>Thinking ({message.thinking.length} chars)</span>
            </summary>
            <div className="mt-1 max-h-60 overflow-y-auto rounded-md bg-muted/50 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground/70 whitespace-pre-wrap font-mono">
              {message.thinking}
            </div>
          </details>
        )}
        {isUser ? (
          <p className="whitespace-pre-wrap break-words">{message.content}</p>
        ) : (
          <MarkdownRenderer content={message.content} />
        )}
      </div>
    </div>
  );
}
