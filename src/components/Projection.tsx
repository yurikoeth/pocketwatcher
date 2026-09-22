import { useEffect, useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money, moneyK } from "../format";
import type { Projection, ProjectionParams } from "../types";
import { LineChart } from "./charts";
import { Badge, Button, Card, ErrorText, Field, NumberInput, Stat } from "./ui";

export default function ProjectionView({ snap, go }: TabProps) {
  const base = snap.projection.params_used;
  const [p, setP] = useState<ProjectionParams>({ ...base });
  const [res, setRes] = useState<Projection>(snap.projection);
  const [error, setError] = useState("");

  useEffect(() => {
    setP({ ...snap.projection.params_used });
    setRes(snap.projection);
  }, [snap]);

  useEffect(() => {
    let cancelled = false;
    const t = setTimeout(async () => {
      try {
        const r = await api.project(p);
        if (!cancelled) {
          setRes(r);
          setError("");
        }
      } catch (e) {
        if (!cancelled) setError(errMsg(e));
      }
    }, 150);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [p]);

  const years = res.params_used.retirement_age - res.params_used.age;
  const targetToday = res.target_nest_egg / Math.pow(1 + res.params_used.inflation_pct / 100, years);
  const set = (k: keyof ProjectionParams) => (n: number) => setP({ ...p, [k]: n });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Retirement projection</h1>
        <Button size="sm" onClick={() => setP({ ...base })}>
          Reset to my numbers
        </Button>
      </div>
      <ErrorText>{error}</ErrorText>

      <Card title="Assumptions" right={<span className="text-xs text-faint">contribution default: {snap.projection_contribution_source}</span>}>
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-7">
          <Field label="Starting balance">
            <NumberInput value={p.starting_balance} onChange={set("starting_balance")} min={0} />
          </Field>
          <Field label="Monthly contribution">
            <NumberInput value={p.monthly_contribution} onChange={set("monthly_contribution")} min={0} step={50} />
          </Field>
          <Field label="Return % / yr">
            <NumberInput value={p.annual_return_pct} onChange={set("annual_return_pct")} step={0.5} />
          </Field>
          <Field label="Inflation % / yr">
            <NumberInput value={p.inflation_pct} onChange={set("inflation_pct")} step={0.5} />
          </Field>
          <Field label="Current age">
            <NumberInput value={p.age} onChange={set("age")} min={0} step={1} />
          </Field>
          <Field label="Retire at">
            <NumberInput value={p.retirement_age} onChange={set("retirement_age")} min={1} step={1} />
          </Field>
          <Field label="Income wanted / yr">
            <NumberInput value={p.retirement_income} onChange={set("retirement_income")} min={0} step={1000} />
          </Field>
        </div>
        <p className="mt-3 text-xs text-faint">
          Persistent defaults live in{" "}
          <button className="text-accent underline" onClick={() => go("settings")}>
            Settings
          </button>
          . Nominal = future dollars; real = today's purchasing power. The target is 25× the desired income (the 4% rule), grown for inflation.
        </p>
      </Card>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat label={`At age ${res.params_used.retirement_age} (today's $)`} value={money(res.final_real)} tone={res.on_track ? "good" : "warn"} sub={`${money(res.final_nominal)} nominal`} />
        <Stat label="Target nest egg (today's $)" value={money(targetToday)} sub={`${money(res.target_nest_egg)} in year ${res.params_used.retirement_age - res.params_used.age + new Date().getFullYear()}`} />
        <Stat label="Safe withdrawal (today's $)" value={`${money(res.safe_withdrawal_real)}/yr`} tone={res.safe_withdrawal_real >= res.params_used.retirement_income ? "good" : "warn"} sub={`vs ${money(res.params_used.retirement_income)}/yr wanted`} />
        <Stat label="Needed contribution" value={`${money(res.required_monthly ?? 0)}/mo`} tone={(res.required_monthly ?? 0) <= res.params_used.monthly_contribution ? "good" : "warn"} sub={res.on_track ? "you're above this" : `${money((res.required_monthly ?? 0) - res.params_used.monthly_contribution)}/mo more`} />
      </div>

      <Card title="Growth by age" right={<Badge tone={res.on_track ? "good" : "warn"}>{res.on_track ? "On track" : "Short of target"}</Badge>}>
        <LineChart
          labels={res.points.map((pt) => String(pt.age))}
          series={[
            { name: "Nominal", values: res.points.map((pt) => pt.nominal) },
            { name: "Real (today's $)", values: res.points.map((pt) => pt.real) },
            { name: "Contributed", values: res.points.map((pt) => pt.contributed + res.params_used.starting_balance), dashed: true },
          ]}
          format={moneyK}
          height={260}
        />
      </Card>
    </div>
  );
}
