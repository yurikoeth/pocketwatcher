import { useCallback, useEffect, useState } from "react";
import { api, errMsg } from "./api";
import { money, setCurrency } from "./format";
import type { Snapshot } from "./types";
import Dashboard from "./components/Dashboard";
import Accounts from "./components/Accounts";
import Transactions from "./components/Transactions";
import BudgetView from "./components/Budget";
import Bills from "./components/Bills";
import IncomeView from "./components/Income";
import Goals from "./components/Goals";
import Debts from "./components/Debts";
import ProjectionView from "./components/Projection";
import Advisor from "./components/Advisor";
import SettingsView from "./components/Settings";

export type Tab = "dashboard" | "accounts" | "transactions" | "income" | "bills" | "budget" | "goals" | "debts" | "projection" | "advisor" | "settings";

const TABS: { id: Tab; label: string; glyph: string }[] = [
  { id: "dashboard", label: "Dashboard", glyph: "◫" },
  { id: "accounts", label: "Accounts", glyph: "▤" },
  { id: "transactions", label: "Transactions", glyph: "≡" },
  { id: "income", label: "Income", glyph: "＋" },
  { id: "bills", label: "Bills", glyph: "▣" },
  { id: "budget", label: "Budget", glyph: "◔" },
  { id: "goals", label: "Goals", glyph: "◎" },
  { id: "debts", label: "Debts", glyph: "▽" },
  { id: "projection", label: "Projection", glyph: "↗" },
  { id: "advisor", label: "Advisor", glyph: "✦" },
  { id: "settings", label: "Settings", glyph: "⚙" },
];

export interface TabProps {
  snap: Snapshot;
  refresh: () => Promise<void>;
  go: (t: Tab) => void;
}

export default function App() {
  const [tab, setTab] = useState<Tab>(() => {
    try {
      return (localStorage.getItem("fincompass.tab") as Tab) || "dashboard";
    } catch {
      return "dashboard";
    }
  });
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await api.snapshot();
      setCurrency(s.currency);
      setSnap(s);
      setError(null);
    } catch (e) {
      setError(errMsg(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const go = useCallback((t: Tab) => {
    setTab(t);
    try {
      localStorage.setItem("fincompass.tab", t);
    } catch {
      /* ignore */
    }
  }, []);

  const critical = snap?.insights.filter((i) => i.severity === "critical").length ?? 0;
  const warns = snap?.insights.filter((i) => i.severity === "warn").length ?? 0;

  return (
    <div className="flex h-full">
      <nav className="flex w-52 shrink-0 flex-col border-r border-line bg-panel">
        <div className="border-b border-line px-4 py-3">
          <div className="text-base font-bold tracking-tight">FinCompass</div>
          <div className="text-[11px] text-faint">personal finance advisor</div>
        </div>
        <ul className="flex-1 space-y-0.5 p-2">
          {TABS.map((t) => (
            <li key={t.id}>
              <button
                onClick={() => go(t.id)}
                className={`flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm ${tab === t.id ? "bg-panel2 text-fg" : "text-dim hover:bg-panel2/60 hover:text-fg"}`}
              >
                <span className="w-4 text-center text-faint">{t.glyph}</span>
                <span className="flex-1">{t.label}</span>
                {t.id === "dashboard" && critical > 0 && <span className="rounded-full bg-critical px-1.5 text-[10px] font-bold text-white">{critical}</span>}
                {t.id === "dashboard" && critical === 0 && warns > 0 && <span className="rounded-full bg-warn px-1.5 text-[10px] font-bold text-black">{warns}</span>}
              </button>
            </li>
          ))}
        </ul>
        {snap && (
          <div className="border-t border-line px-4 py-3">
            <div className="text-[11px] tracking-wide text-faint uppercase">Net worth</div>
            <div className={`num text-lg font-semibold ${snap.net_worth < 0 ? "text-critical" : ""}`}>{money(snap.net_worth)}</div>
            <div className="text-[11px] text-faint">{snap.today}</div>
          </div>
        )}
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-6xl p-5">
          {error && <pre className="mb-4 rounded-md border border-critical/40 bg-critical/10 p-3 text-xs text-critical">{error}</pre>}
          {!snap && !error && <div className="p-10 text-center text-dim">Loading…</div>}
          {snap && (
            <>
              {tab === "dashboard" && <Dashboard snap={snap} refresh={refresh} go={go} />}
              {tab === "accounts" && <Accounts snap={snap} refresh={refresh} go={go} />}
              {tab === "transactions" && <Transactions snap={snap} refresh={refresh} go={go} />}
              {tab === "income" && <IncomeView snap={snap} refresh={refresh} go={go} />}
              {tab === "bills" && <Bills snap={snap} refresh={refresh} go={go} />}
              {tab === "budget" && <BudgetView snap={snap} refresh={refresh} go={go} />}
              {tab === "goals" && <Goals snap={snap} refresh={refresh} go={go} />}
              {tab === "debts" && <Debts snap={snap} refresh={refresh} go={go} />}
              {tab === "projection" && <ProjectionView snap={snap} refresh={refresh} go={go} />}
              {tab === "advisor" && <Advisor snap={snap} refresh={refresh} go={go} />}
              {tab === "settings" && <SettingsView snap={snap} refresh={refresh} go={go} />}
            </>
          )}
        </div>
      </main>
    </div>
  );
}
