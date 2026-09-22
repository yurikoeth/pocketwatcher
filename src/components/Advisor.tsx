import { useEffect, useRef, useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import type { ChatMsg } from "../types";
import { Button, Card, ErrorText, inputCls } from "./ui";

const SUGGESTIONS = [
  "What should I do with my money this month, in priority order?",
  "Should I pay off my credit card or invest the difference?",
  "Is my emergency fund the right size for my situation?",
  "Where is my spending most out of line with my goals?",
  "Am I on track for retirement, and what one change helps most?",
];

/** Tiny markdown subset: paragraphs, bullets, numbered lists, bold, inline code. */
function render(md: string) {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const inline = (s: string) => esc(s).replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>").replace(/`([^`]+)`/g, "<code>$1</code>");
  const lines = md.split(/\r?\n/);
  const out: string[] = [];
  let list: "ul" | "ol" | null = null;
  let para: string[] = [];
  const flushPara = () => {
    if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`);
    para = [];
  };
  const closeList = () => {
    if (list) out.push(`</${list}>`);
    list = null;
  };
  for (const raw of lines) {
    const line = raw.trim();
    const bullet = /^[-*•]\s+(.*)/.exec(line);
    const num = /^\d+[.)]\s+(.*)/.exec(line);
    if (bullet || num) {
      flushPara();
      const kind = bullet ? "ul" : "ol";
      if (list !== kind) {
        closeList();
        list = kind;
        out.push(`<${kind}>`);
      }
      out.push(`<li>${inline((bullet ?? num)![1])}</li>`);
    } else if (line === "") {
      flushPara();
      closeList();
    } else {
      closeList();
      para.push(line.replace(/^#+\s*/, ""));
    }
  }
  flushPara();
  closeList();
  return out.join("");
}

export default function Advisor({ snap, go }: TabProps) {
  const [history, setHistory] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [usage, setUsage] = useState<{ model: string; inTok: number; outTok: number; cached: number } | null>(null);
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.settings().then((s) => setHasKey(!!s.api_key.trim())).catch(() => setHasKey(false));
  }, [snap]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [history, busy]);

  const send = async (text: string) => {
    const q = text.trim();
    if (!q || busy) return;
    const next: ChatMsg[] = [...history, { role: "user", content: q }];
    setHistory(next);
    setInput("");
    setBusy(true);
    setError("");
    try {
      const r = await api.askAdvisor(next);
      setHistory([...next, { role: "assistant", content: r.text }]);
      setUsage({ model: r.model, inTok: r.input_tokens, outTok: r.output_tokens, cached: r.cache_read_tokens });
    } catch (e) {
      setError(errMsg(e));
      setHistory(history);
      setInput(q);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-[calc(100vh-2.5rem)] flex-col gap-3">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Advisor</h1>
        <div className="flex items-center gap-3">
          {usage && (
            <span className="text-[11px] text-faint">
              {usage.model} · {usage.inTok.toLocaleString()} in ({usage.cached.toLocaleString()} cached) / {usage.outTok.toLocaleString()} out
            </span>
          )}
          {history.length > 0 && (
            <Button size="sm" onClick={() => setHistory([])}>
              New conversation
            </Button>
          )}
        </div>
      </div>

      {hasKey === false && (
        <Card>
          <div className="text-sm">
            The advisor uses Claude and needs an Anthropic API key.{" "}
            <button className="text-accent underline" onClick={() => go("settings")}>
              Add one in Settings
            </button>
            . Your full financial snapshot is sent with each question; nothing else leaves this machine.
          </div>
        </Card>
      )}

      <div className="flex-1 space-y-3 overflow-y-auto rounded-lg border border-line bg-panel p-4">
        {history.length === 0 && (
          <div className="space-y-3">
            <p className="text-sm text-dim">Ask anything about your finances. The advisor sees your accounts, cash flow, budgets, goals, debts, projection and the insights on the dashboard.</p>
            <div className="flex flex-wrap gap-2">
              {SUGGESTIONS.map((s) => (
                <button key={s} disabled={!hasKey || busy} onClick={() => void send(s)} className="rounded-full border border-line bg-panel2 px-3 py-1 text-xs text-dim hover:border-faint hover:text-fg disabled:opacity-50">
                  {s}
                </button>
              ))}
            </div>
          </div>
        )}
        {history.map((m, i) => (
          <div key={i} className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}>
            <div className={`max-w-[85%] rounded-lg px-3.5 py-2.5 text-sm ${m.role === "user" ? "bg-accent/20 text-fg" : "bg-panel2 text-dim"}`}>
              {m.role === "user" ? <span className="whitespace-pre-wrap">{m.content}</span> : <div className="prose-chat" dangerouslySetInnerHTML={{ __html: render(m.content) }} />}
            </div>
          </div>
        ))}
        {busy && <div className="text-sm text-faint">Thinking…</div>}
        <ErrorText>{error}</ErrorText>
        <div ref={endRef} />
      </div>

      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void send(input);
        }}
      >
        <textarea
          className={inputCls + " h-16 resize-none"}
          placeholder={hasKey ? "Ask the advisor… (Enter to send, Shift+Enter for a new line)" : "Add an API key in Settings first"}
          value={input}
          disabled={!hasKey || busy}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void send(input);
            }
          }}
        />
        <Button type="submit" variant="primary" disabled={!hasKey || busy || !input.trim()}>
          Send
        </Button>
      </form>
      <p className="text-[11px] text-faint">Educational guidance from a language model, not a licensed financial adviser. Check anything consequential before acting on it.</p>
    </div>
  );
}
