import { useEffect, useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money, moneyK, monthsToText, pct } from "../format";
import { isLiability, type PayoffPlan } from "../types";
import { LineChart } from "./charts";
import { Card, ErrorText, Field, NumberInput, Stat, Td, Th } from "./ui";

type Strategy = "avalanche" | "snowball";

export default function Debts({ snap, go }: TabProps) {
  const [extra, setExtra] = useState(0);
  const [strategy, setStrategy] = useState<Strategy>("avalanche");
  const [includeMortgage, setIncludeMortgage] = useState(false);
  const [plan, setPlan] = useState<PayoffPlan | null>(null);
  const [alt, setAlt] = useState<PayoffPlan | null>(null);
  const [baseline, setBaseline] = useState<PayoffPlan | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    const t = setTimeout(async () => {
      try {
        const other: Strategy = strategy === "avalanche" ? "snowball" : "avalanche";
        const [p, a, b] = await Promise.all([api.debtPlan(extra, strategy, includeMortgage), api.debtPlan(extra, other, includeMortgage), api.debtPlan(0, "avalanche", includeMortgage)]);
        if (!cancelled) {
          setPlan(p);
          setAlt(a);
          setBaseline(b);
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
  }, [extra, strategy, includeMortgage, snap]);

  const hasDebt = snap.accounts.some((a) => isLiability(a.kind));
  if (!hasDebt) {
    return (
      <div className="space-y-4">
        <h1 className="text-xl font-semibold">Debts</h1>
        <Card>
          <div className="text-sm text-dim">
            No liabilities recorded.{" "}
            <button className="text-accent underline" onClick={() => go("accounts")}>
              Add a credit card or loan
            </button>{" "}
            with its APR and minimum payment to get a payoff plan.
          </div>
        </Card>
      </div>
    );
  }

  const interestSaved = baseline && plan ? baseline.total_interest - plan.total_interest : 0;
  const monthsSaved = baseline?.months != null && plan?.months != null ? baseline.months - plan.months : null;
  const curve = plan?.balance_by_month ?? [];
  const altCurve = alt?.balance_by_month ?? [];
  const len = Math.max(curve.length, altCurve.length);
  const labels = Array.from({ length: len + 1 }, (_, i) => (i === 0 ? "now" : `${i}`));
  const startBal = plan?.debts.reduce((s, d) => s + d.balance, 0) ?? 0;
  const pad = (arr: number[]) => [startBal, ...arr, ...Array(Math.max(0, len - arr.length)).fill(0)];

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Debts</h1>
      <ErrorText>{error}</ErrorText>

      <Card title="Payoff plan">
        <div className="flex flex-wrap items-end gap-4">
          <Field label="Strategy">
            <div className="flex gap-1 rounded-md bg-panel2 p-1">
              {(["avalanche", "snowball"] as Strategy[]).map((s) => (
                <button key={s} onClick={() => setStrategy(s)} className={`rounded px-3 py-1 text-sm capitalize ${strategy === s ? "bg-bg font-semibold" : "text-dim"}`}>
                  {s}
                </button>
              ))}
            </div>
          </Field>
          <Field label="Extra per month, on top of minimums" className="w-52">
            <NumberInput value={extra} onChange={setExtra} min={0} step={25} />
          </Field>
          <label className="flex items-center gap-2 pb-2 text-sm text-dim">
            <input type="checkbox" checked={includeMortgage} onChange={(e) => setIncludeMortgage(e.target.checked)} />
            Include mortgage
          </label>
          <div className="pb-2 text-xs text-faint">
            {strategy === "avalanche" ? "Highest APR first: least total interest." : "Smallest balance first: quickest wins, usually a bit more interest."}
          </div>
        </div>
      </Card>

      {plan && (
        <>
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Stat label="Debt-free in" value={monthsToText(plan.months)} tone={plan.months == null ? "bad" : "neutral"} sub={monthsSaved != null && monthsSaved > 0 ? `${monthsSaved} months sooner than minimums only` : "at this payment level"} />
            <Stat label="Total interest" value={money(plan.total_interest)} tone={plan.months == null ? "bad" : "neutral"} sub={interestSaved > 0.5 ? `saves ${money(interestSaved)} vs minimums` : "vs minimums only"} />
            <Stat label="Monthly outlay" value={money(plan.debts.reduce((s, d) => s + d.min_payment, 0) + plan.extra_monthly)} sub="minimums + extra" />
            <Stat label={`${alt?.strategy ?? "other"} instead`} value={alt ? monthsToText(alt.months) : "—"} sub={alt ? `${money(alt.total_interest)} interest` : ""} />
          </div>

          <div className="grid gap-4 lg:grid-cols-5">
            <Card title="Balance over time" className="lg:col-span-3">
              {len === 0 ? (
                <div className="text-sm text-dim">Nothing to pay off.</div>
              ) : (
                <LineChart labels={labels} series={[{ name: `${plan.strategy} + ${money(plan.extra_monthly)}/mo extra`, values: pad(curve) }, ...(alt ? [{ name: alt.strategy, values: pad(altCurve), dashed: true }] : [])]} format={moneyK} height={230} />
              )}
              <div className="mt-1 text-[11px] text-faint">x-axis is months from now</div>
            </Card>
            <Card title="Order of attack" className="lg:col-span-2">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-line">
                    <Th>Debt</Th>
                    <Th right>Balance</Th>
                    <Th right>APR</Th>
                    <Th right>Paid off</Th>
                    <Th right>Interest</Th>
                  </tr>
                </thead>
                <tbody>
                  {[...plan.debts]
                    .sort((a, b) => (a.payoff_month ?? 9999) - (b.payoff_month ?? 9999))
                    .map((d) => (
                      <tr key={d.account_id} className="border-b border-line/60">
                        <Td className="font-medium">{d.name}</Td>
                        <Td right>{money(d.balance)}</Td>
                        <Td right className={d.apr >= 15 ? "text-critical" : "text-dim"}>
                          {pct(d.apr, 1)}
                        </Td>
                        <Td right className={d.never ? "text-critical" : ""}>
                          {d.never ? "never" : `month ${d.payoff_month}`}
                        </Td>
                        <Td right className="text-dim">
                          {money(d.interest_paid)}
                        </Td>
                      </tr>
                    ))}
                </tbody>
              </table>
              {plan.debts.some((d) => d.never) && <p className="mt-2 text-xs text-critical">A debt marked "never" has a minimum payment below its monthly interest. Raise the payment on the Accounts page.</p>}
            </Card>
          </div>
        </>
      )}
    </div>
  );
}
