import { useState, useCallback, type ComponentPropsWithoutRef } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Copy, Check, ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/lib/utils";

/** Line count threshold for collapsible content */
const COLLAPSE_LINE_THRESHOLD = 50;
/** Lines shown when collapsed */
const COLLAPSED_PREVIEW_LINES = 20;

// ---------------------------------------------------------------------------
// Code block with copy button
// ---------------------------------------------------------------------------

function CodeBlock({ children, className }: { children: string; className?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    void navigator.clipboard.writeText(children).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [children]);

  return (
    <div className="group relative">
      <pre className={cn("max-h-[32rem] overflow-auto rounded-md p-4 text-sm", className)}>
        <code className={className}>{children}</code>
      </pre>
      <button
        type="button"
        onClick={handleCopy}
        className="absolute right-2 top-2 rounded-md bg-muted/80 p-1.5 text-muted-foreground opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
        aria-label="Copy code"
      >
        {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Custom components for react-markdown
// ---------------------------------------------------------------------------

type CodeProps = ComponentPropsWithoutRef<"code"> & { inline?: boolean };

function Code({ className, children, inline, ...rest }: CodeProps) {
  const match = /language-(\w+)/.exec(className ?? "");
  const text = String(children).replace(/\n$/, "");

  // Fenced code block (has language class or is inside <pre>)
  if (!inline && (match ?? text.includes("\n"))) {
    return <CodeBlock className={className}>{text}</CodeBlock>;
  }

  // Inline code
  return (
    <code
      className={cn(
        "rounded bg-muted px-1.5 py-0.5 text-[0.85em] font-mono text-foreground",
        className,
      )}
      {...rest}
    >
      {children}
    </code>
  );
}

function Pre({ children }: ComponentPropsWithoutRef<"pre">) {
  // CodeBlock already wraps <pre>, so just pass children through
  return <>{children}</>;
}

function Table({ children, ...rest }: ComponentPropsWithoutRef<"table">) {
  return (
    <div className="my-2 overflow-x-auto">
      <table className="min-w-full border-collapse text-sm" {...rest}>
        {children}
      </table>
    </div>
  );
}

function Th({ children, ...rest }: ComponentPropsWithoutRef<"th">) {
  return (
    <th className="border border-border bg-muted/50 px-3 py-1.5 text-left font-medium" {...rest}>
      {children}
    </th>
  );
}

function Td({ children, ...rest }: ComponentPropsWithoutRef<"td">) {
  return (
    <td className="border border-border px-3 py-1.5" {...rest}>
      {children}
    </td>
  );
}

const markdownComponents = {
  code: Code,
  pre: Pre,
  table: Table,
  th: Th,
  td: Td,
};

// ---------------------------------------------------------------------------
// Collapsible wrapper for long content
// ---------------------------------------------------------------------------

function CollapsibleContent({ content }: { content: string }) {
  const [expanded, setExpanded] = useState(false);
  const lines = content.split("\n");
  const needsCollapse = lines.length > COLLAPSE_LINE_THRESHOLD;

  if (!needsCollapse) {
    return <MarkdownContent content={content} />;
  }

  const previewContent = expanded
    ? content
    : lines.slice(0, COLLAPSED_PREVIEW_LINES).join("\n");

  return (
    <div>
      <div className={cn(!expanded && "relative")}>
        <MarkdownContent content={previewContent} />
        {!expanded && (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-12 bg-gradient-to-t from-secondary to-transparent" />
        )}
      </div>
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="mt-1 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        {expanded ? (
          <>
            <ChevronUp className="h-3 w-3" /> Show less
          </>
        ) : (
          <>
            <ChevronDown className="h-3 w-3" /> Show more ({lines.length - COLLAPSED_PREVIEW_LINES} more lines)
          </>
        )}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Core markdown renderer
// ---------------------------------------------------------------------------

function MarkdownContent({ content }: { content: string }) {
  return (
    <div className="markdown-body">
      <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={markdownComponents}
      >
        {content}
      </Markdown>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

interface MarkdownRendererProps {
  content: string;
  /** Enable collapsible behavior for long content (default: true) */
  collapsible?: boolean;
}

export function MarkdownRenderer({ content, collapsible = true }: MarkdownRendererProps) {
  if (collapsible) {
    return <CollapsibleContent content={content} />;
  }
  return <MarkdownContent content={content} />;
}
