import { useEffect, useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import type { Settings } from "../types";
import { Button, Card, ErrorText, Field, NumberInput, inputCls } from "./ui";

export default function SettingsView({ snap, refresh, go }: TabProps) {
  const [s, setS] = useState<Settings | null>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [wipeArmed, setWipeArmed] = useState(false);

  useEffect(() => {
    api.settings().then(setS).catch((e) => setError(errMsg(e)));
    api.dataPath().then(setPath).catch(() => {});
  }, []);

  const save = async () => {
    if (!s) return;
    setError("");
    try {
      await api.saveSettings(s);
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const loadSample = async () => {
    setError("");
    try {
      await api.loadSampleData();
      await refresh();
      const fresh = await api.settings();
      setS(fresh);
      go("dashboard");
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const wipe = async () => {
    setError("");
    try {
      await api.wipeData();
      setWipeArmed(false);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const exportJson = async () => {
    try {
      const [accounts, transactions, budgets, goals, settings] = await Promise.all([api.accounts(), api.transactions(), api.budgets(), api.goals(), api.settings()]);
      const data = { exported: new Date().toISOString(), accounts, transactions, budgets, goals, settings: { ...settings, api_key: "" } };
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = `fincompass-export-${snap.today}.json`;
      a.click();
      URL.revokeObjectURL(a.href);
    } catch (e) {
      setError(errMsg(e));
    }
  };

  if (!s) return <div className="text-dim">Loading…</div>;
  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => setS({ ...s, [k]: v });

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Settings</h1>
      <ErrorText>{error}</ErrorText>

      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <Card title="About you">
          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
            <Field label="Current age">
              <NumberInput value={s.age} onChange={(n) => set("age", Math.round(n))} min={0} step={1} />
            </Field>
            <Field label="Retirement age">
              <NumberInput value={s.retirement_age} onChange={(n) => set("retirement_age", Math.round(n))} min={1} step={1} />
            </Field>
            <Field label="Retirement income / yr" hint="today's dollars">
              <NumberInput value={s.retirement_income} onChange={(n) => set("retirement_income", n)} min={0} step={1000} />
            </Field>
            <Field label="Risk profile">
              <select className={inputCls} value={s.risk_profile} onChange={(e) => set("risk_profile", e.target.value as Settings["risk_profile"])}>
                <option value="conservative">Conservative</option>
                <option value="balanced">Balanced</option>
                <option value="aggressive">Aggressive</option>
              </select>
            </Field>
          </div>
        </Card>

        <Card title="Assumptions">
          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
            <Field label="Emergency fund target" hint="months of expenses">
              <NumberInput value={s.emergency_months} onChange={(n) => set("emergency_months", n)} min={0} step={1} />
            </Field>
            <Field label="Expected return % / yr" hint="7% is a common long-run stock assumption">
              <NumberInput value={s.annual_return_pct} onChange={(n) => set("annual_return_pct", n)} step={0.5} />
            </Field>
            <Field label="Inflation % / yr">
              <NumberInput value={s.inflation_pct} onChange={(n) => set("inflation_pct", n)} step={0.5} />
            </Field>
            <Field label="Monthly investment" hint="0 = use average surplus">
              <NumberInput value={s.monthly_investment} onChange={(n) => set("monthly_investment", n)} min={0} step={50} />
            </Field>
            <Field label="Currency code" hint="ISO 4217, e.g. USD, EUR, GBP">
              <input className={inputCls} value={s.currency} maxLength={3} onChange={(e) => set("currency", e.target.value.toUpperCase())} />
            </Field>
          </div>
        </Card>

        <Card title="Claude advisor">
          <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
            <Field label="Anthropic API key" className="md:col-span-2" hint="Stored only in the local database. Leave empty to disable the Advisor tab.">
              <div className="flex gap-2">
                <input className={inputCls + " font-mono"} type={showKey ? "text" : "password"} value={s.api_key} onChange={(e) => set("api_key", e.target.value)} placeholder="sk-ant-…" autoComplete="off" />
                <Button type="button" onClick={() => setShowKey(!showKey)}>
                  {showKey ? "Hide" : "Show"}
                </Button>
              </div>
            </Field>
            <Field label="Model">
              <select className={inputCls} value={s.model} onChange={(e) => set("model", e.target.value)}>
                <option value="claude-opus-5">Claude Opus 5 (default)</option>
                <option value="claude-sonnet-5">Claude Sonnet 5 (cheaper)</option>
                <option value="claude-fable-5-1">Claude Fable 5.1</option>
              </select>
            </Field>
          </div>
        </Card>

        <div className="flex items-center gap-3">
          <Button type="submit" variant="primary">
            Save settings
          </Button>
          {saved && <span className="text-sm text-good">Saved.</span>}
        </div>
      </form>

      <Card title="Data">
        <div className="space-y-3 text-sm">
          <div>
            <span className="text-faint">Database: </span>
            <span className="font-mono text-xs">{path}</span>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => void exportJson()}>Export JSON</Button>
            <Button onClick={() => void loadSample()} disabled={snap.has_data} title={snap.has_data ? "Only available when the database is empty" : ""}>
              Load sample data
            </Button>
            {wipeArmed ? (
              <>
                <Button variant="danger" onClick={() => void wipe()}>
                  Yes, delete all accounts, transactions, budgets and goals
                </Button>
                <Button onClick={() => setWipeArmed(false)}>Cancel</Button>
              </>
            ) : (
              <Button variant="danger" onClick={() => setWipeArmed(true)} disabled={!snap.has_data}>
                Wipe all data
              </Button>
            )}
          </div>
          <p className="text-xs text-faint">Wiping keeps your settings (including the API key). Export first if you want a backup.</p>
        </div>
      </Card>
    </div>
  );
}
