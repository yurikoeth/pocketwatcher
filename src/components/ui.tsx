import React, { useEffect, useRef, useState } from "react";
import type { Severity } from "../types";

export function Card({ title, right, children, className = "" }: { title?: React.ReactNode; right?: React.ReactNode; children: React.ReactNode; className?: string }) {
  return (
    <section className={`rounded-lg border border-line bg-panel ${className}`}>
      {(title || right) && (
        <header className="flex items-center justify-between gap-3 border-b border-line px-4 py-2.5">
          <h2 className="text-sm font-semibold tracking-wide text-dim uppercase">{title}</h2>
          {right && <div className="flex items-center gap-2">{right}</div>}
        </header>
      )}
      <div className="p-4">{children}</div>
    </section>
  );
}

export function Stat({ label, value, sub, tone }: { label: string; value: React.ReactNode; sub?: React.ReactNode; tone?: "good" | "bad" | "warn" | "neutral" }) {
  const color = tone === "good" ? "text-good" : tone === "bad" ? "text-critical" : tone === "warn" ? "text-warn" : "text-fg";
  return (
    <div className="rounded-lg border border-line bg-panel px-4 py-3">
      <div className="text-xs font-medium tracking-wide text-faint uppercase">{label}</div>
      <div className={`num mt-1 text-2xl font-semibold ${color}`}>{value}</div>
      {sub && <div className="mt-0.5 text-xs text-dim">{sub}</div>}
    </div>
  );
}

type BtnProps = React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "ghost" | "danger"; size?: "sm" | "md" };
export function Button({ variant = "ghost", size = "md", className = "", ...rest }: BtnProps) {
  const base = "inline-flex items-center gap-1.5 rounded-md font-medium transition disabled:cursor-not-allowed disabled:opacity-50";
  const sz = size === "sm" ? "px-2.5 py-1 text-xs" : "px-3.5 py-1.5 text-sm";
  const v =
    variant === "primary"
      ? "bg-accent text-white hover:brightness-110"
      : variant === "danger"
        ? "border border-critical/50 text-critical hover:bg-critical/10"
        : "border border-line bg-panel2 text-fg hover:border-faint";
  return <button className={`${base} ${sz} ${v} ${className}`} {...rest} />;
}

export const inputCls =
  "w-full rounded-md border border-line bg-bg px-2.5 py-1.5 text-sm text-fg outline-none placeholder:text-faint focus:border-accent";

export function Field({ label, children, hint, className = "" }: { label: string; children: React.ReactNode; hint?: string; className?: string }) {
  return (
    <label className={`block ${className}`}>
      <span className="mb-1 block text-xs font-medium text-dim">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-[11px] text-faint">{hint}</span>}
    </label>
  );
}

export function NumberInput({ value, onChange, step = "any", min, ...rest }: { value: number; onChange: (n: number) => void; step?: string | number; min?: number } & Omit<React.InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "step" | "min">) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    // keep local text in sync when the parent value changes externally
    if (Number(text) !== value && !(text === "" && value === 0) && !(text === "-" ) && !text.endsWith(".")) setText(String(value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);
  return (
    <input
      type="number"
      inputMode="decimal"
      step={step}
      min={min}
      className={inputCls + " num"}
      value={text}
      onChange={(e) => {
        setText(e.target.value);
        const n = Number(e.target.value);
        if (e.target.value !== "" && Number.isFinite(n)) onChange(n);
        if (e.target.value === "") onChange(0);
      }}
      {...rest}
    />
  );
}

const sevStyle: Record<Severity, { border: string; text: string; label: string; glyph: string }> = {
  critical: { border: "border-l-critical", text: "text-critical", label: "Critical", glyph: "!" },
  warn: { border: "border-l-warn", text: "text-warn", label: "Attention", glyph: "▲" },
  info: { border: "border-l-s1", text: "text-s1", label: "Note", glyph: "i" },
  good: { border: "border-l-good", text: "text-good", label: "Good", glyph: "✓" },
};

export function SeverityBadge({ s }: { s: Severity }) {
  const st = sevStyle[s];
  return (
    <span className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-semibold ${st.text} bg-panel2`}>
      <span aria-hidden>{st.glyph}</span>
      {st.label}
    </span>
  );
}

export function InsightRow({ severity, title, detail, action }: { severity: Severity; title: string; detail: string; action: string }) {
  const st = sevStyle[severity];
  return (
    <div className={`rounded-md border border-line border-l-4 bg-panel2/40 px-3 py-2.5 ${st.border}`}>
      <div className="flex items-center gap-2">
        <SeverityBadge s={severity} />
        <span className="font-semibold">{title}</span>
      </div>
      <p className="mt-1 text-sm text-dim">{detail}</p>
      <p className="mt-1 text-sm">
        <span className="text-faint">Do: </span>
        {action}
      </p>
    </div>
  );
}

export function Badge({ tone = "neutral", children }: { tone?: "good" | "bad" | "warn" | "neutral"; children: React.ReactNode }) {
  const c = tone === "good" ? "text-good" : tone === "bad" ? "text-critical" : tone === "warn" ? "text-warn" : "text-dim";
  return <span className={`rounded bg-panel2 px-1.5 py-0.5 text-[11px] font-semibold ${c}`}>{children}</span>;
}

export function Progress({ value, max, tone = "neutral" }: { value: number; max: number; tone?: "good" | "bad" | "warn" | "neutral" }) {
  const p = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  const c = tone === "good" ? "bg-good" : tone === "bad" ? "bg-critical" : tone === "warn" ? "bg-warn" : "bg-s1";
  return (
    <div className="h-2 w-full overflow-hidden rounded-full bg-panel2" role="progressbar" aria-valuenow={Math.round(p)} aria-valuemin={0} aria-valuemax={100}>
      <div className={`h-full rounded-full ${c}`} style={{ width: `${p}%` }} />
    </div>
  );
}

export function Empty({ title, text, children }: { title: string; text?: string; children?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-line px-6 py-10 text-center">
      <div className="font-semibold">{title}</div>
      {text && <div className="max-w-md text-sm text-dim">{text}</div>}
      {children && <div className="mt-2 flex gap-2">{children}</div>}
    </div>
  );
}

export function Modal({ open, title, onClose, children, width = "max-w-lg" }: { open: boolean; title: string; onClose: () => void; children: React.ReactNode; width?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={ref} role="dialog" aria-modal className={`w-full ${width} rounded-lg border border-line bg-panel shadow-2xl`}>
        <header className="flex items-center justify-between border-b border-line px-4 py-3">
          <h3 className="font-semibold">{title}</h3>
          <button className="rounded px-2 text-dim hover:bg-panel2" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </header>
        <div className="p-4">{children}</div>
      </div>
    </div>
  );
}

export function ErrorText({ children }: { children: React.ReactNode }) {
  if (!children) return null;
  return <pre className="whitespace-pre-wrap rounded-md border border-critical/40 bg-critical/10 px-3 py-2 text-xs text-critical">{children}</pre>;
}

export function Th({ children, right, className = "" }: { children?: React.ReactNode; right?: boolean; className?: string }) {
  return <th className={`px-2 py-1.5 text-[11px] font-semibold tracking-wide text-faint uppercase ${right ? "text-right" : "text-left"} ${className}`}>{children}</th>;
}
export function Td({ children, right, className = "", colSpan }: { children?: React.ReactNode; right?: boolean; className?: string; colSpan?: number }) {
  return (
    <td colSpan={colSpan} className={`px-2 py-1.5 text-sm ${right ? "num text-right" : ""} ${className}`}>
      {children}
    </td>
  );
}

/** Confirm-on-second-click delete button. */
export function DeleteButton({ onConfirm, label = "Delete" }: { onConfirm: () => void; label?: string }) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const t = setTimeout(() => setArmed(false), 3000);
    return () => clearTimeout(t);
  }, [armed]);
  return (
    <Button size="sm" variant={armed ? "danger" : "ghost"} onClick={() => (armed ? onConfirm() : setArmed(true))}>
      {armed ? "Confirm?" : label}
    </Button>
  );
}

export function useWidth<T extends HTMLElement>(): [React.RefObject<T | null>, number] {
  const ref = useRef<T>(null);
  const [w, setW] = useState(600);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) setW(Math.max(200, e.contentRect.width));
    });
    ro.observe(el);
    setW(Math.max(200, el.clientWidth));
    return () => ro.disconnect();
  }, []);
  return [ref, w];
}
