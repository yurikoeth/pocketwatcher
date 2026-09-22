import { useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money, todayISO } from "../format";
import { INCOME_FREQUENCIES, type IncomeFrequency, type IncomeInput, type IncomeStatus } from "../types";
import { Badge, Button, Card, DeleteButton, ErrorText, Field, Modal, NumberInput, Stat, Td, Th, inputCls } from "./ui";

const CATS = ["Salary", "Wages", "Freelance", "Business", "Bonus", "Commission", "Benefits", "Child support", "Rental income", "Dividends", "Interest", "Gift", "Other"];
const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const blank = (accountId: number | null): IncomeInput => ({
  id: null,
  name: "",
  amount: 0,
  category: "Salary",
  frequency: "biweekly",
  due_day: 5,
  due_day2: 0,
  due_month: 1,
  anchor: null,
  account_id: accountId,
  variable: false,
  last_received: null,
  note: "",
});

const ord = (d: number) => `${d}${[1, 21, 31].includes(d) ? "st" : [2, 22].includes(d) ? "nd" : [3, 23].includes(d) ? "rd" : "th"}`;

function scheduleText(i: IncomeStatus): string {
  const inc = i.income;
  switch (inc.frequency) {
    case "weekly":
      return `every ${WEEKDAYS[inc.due_day - 1] ?? "?"}`;
    case "biweekly":
      return `every two weeks${inc.anchor ? ` from ${inc.anchor}` : ""}`;
    case "semimonthly":
      return `on the ${ord(inc.due_day)} and ${ord(inc.due_day2)}`;
    case "monthly":
      return `monthly on the ${ord(inc.due_day)}`;
    case "quarterly":
      return `quarterly from ${MONTHS[inc.due_month - 1]} ${inc.due_day}`;
    case "yearly":
      return `yearly on ${MONTHS[inc.due_month - 1]} ${inc.due_day}`;
    default:
      return "irregular";
  }
}

export default function IncomeView({ snap, refresh }: TabProps) {
  const [editing, setEditing] = useState<IncomeInput | null>(null);
  const [logging, setLogging] = useState<IncomeStatus | null>(null);
  const [logDate, setLogDate] = useState(todayISO());
  const [logTx, setLogTx] = useState(true);
  const [logAmount, setLogAmount] = useState(0);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const accName = (id: number | null) => snap.accounts.find((a) => a.id === id)?.name ?? "—";

  const save = async () => {
    if (!editing) return;
    setBusy(true);
    setError("");
    try {
      await api.upsertIncome(editing);
      setEditing(null);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: number) => {
    try {
      await api.deleteIncome(id);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const logReceived = async () => {
    if (!logging) return;
    setBusy(true);
    setError("");
    try {
      await api.logIncomeReceived(logging.income.id, logDate, logTx, logAmount > 0 ? logAmount : null);
      setLogging(null);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const next30 = snap.incomes.filter((i) => i.days_until != null && i.days_until <= 30).reduce((s, i) => s + i.estimate, 0);
  const late = snap.incomes.filter((i) => i.late_days > 0);
  const varying = snap.incomes.filter((i) => i.income.variable).length;
  const surplus = snap.income_monthly - snap.bills_monthly;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Income</h1>
        <Button variant="primary" onClick={() => setEditing(blank(snap.accounts.find((a) => a.kind === "checking")?.id ?? snap.accounts[0]?.id ?? null))}>
          + Add income source
        </Button>
      </div>
      <ErrorText>{error}</ErrorText>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat label="Expected per month" value={money(snap.income_monthly)} sub={varying ? `${varying} source${varying === 1 ? "" : "s"} vary; estimates from recent receipts` : "all sources, monthly equivalent"} />
        <Stat label="Logged this month" value={money(snap.income_this_month)} sub={snap.income_monthly > 0 ? `${Math.round((snap.income_this_month / snap.income_monthly) * 100)}% of expected` : "income transactions this month"} />
        <Stat label="Expected next 30 days" value={money(next30)} sub="next payment from each source" />
        <Stat label="After bills" value={snap.income_monthly > 0 ? money(surplus) : "—"} tone={snap.income_monthly === 0 ? "neutral" : surplus < 0 ? "bad" : "good"} sub={`expected income − ${money(snap.bills_monthly)} bills`} />
      </div>

      {late.length > 0 && (
        <div className="rounded-md border border-warn/40 bg-warn/10 px-3 py-2 text-sm">
          {late.map((i) => `${i.income.name} was expected ${i.late_days} day${i.late_days === 1 ? "" : "s"} ago`).join("; ")}. If it arrived, log it below.
        </div>
      )}

      <Card title="Income sources">
        {snap.incomes.length === 0 ? (
          <div className="text-sm text-dim">Add your paycheck, side income, benefits or anything else that comes in regularly. Tick "amount varies" for tips, commissions or freelance work and log what you actually receive each time.</div>
        ) : (
          <table className="w-full">
            <thead>
              <tr className="border-b border-line">
                <Th>Source</Th>
                <Th>Schedule</Th>
                <Th>Deposits to</Th>
                <Th right>Per payment</Th>
                <Th right>/ month</Th>
                <Th>Status</Th>
                <Th right></Th>
              </tr>
            </thead>
            <tbody>
              {snap.incomes.map((i) => (
                <tr key={i.income.id} className="border-b border-line/60 hover:bg-panel2/40">
                  <Td className="font-medium">
                    {i.income.name}
                    <div className="text-[11px] font-normal text-faint">
                      {i.income.category}
                      {i.income.note && ` · ${i.income.note}`}
                    </div>
                  </Td>
                  <Td className="text-dim">{scheduleText(i)}</Td>
                  <Td className="text-dim">{accName(i.income.account_id)}</Td>
                  <Td right>
                    {i.income.variable ? (
                      <span title={`Estimate from your last ${Math.min(3, i.history.length) || "0"} receipts`}>
                        ~{money(i.estimate)}
                        {i.history.length > 0 && <div className="text-[11px] font-normal text-faint">last {i.history.slice(0, 3).map((p) => money(p.amount)).join(", ")}</div>}
                      </span>
                    ) : (
                      money(i.income.amount)
                    )}
                  </Td>
                  <Td right className="text-dim">
                    {i.income.frequency === "irregular" && i.history.length === 0 ? <span title="Log receipts to estimate">—</span> : money(i.monthly_equivalent)}
                  </Td>
                  <Td>
                    {i.late_days > 0 ? (
                      <Badge tone="warn">{i.late_days}d late</Badge>
                    ) : i.next_date == null ? (
                      <span className="text-xs text-faint">{i.last_amount != null ? `last ${money(i.last_amount)} on ${i.history[0]?.date}` : "no receipts logged"}</span>
                    ) : i.received_this_cycle ? (
                      <span className="text-xs text-faint">
                        received {i.income.last_received} · next {i.next_date}
                      </span>
                    ) : (
                      <Badge tone="neutral">{i.days_until === 0 ? "expected today" : `expected ${i.next_date} (${i.days_until}d)`}</Badge>
                    )}
                  </Td>
                  <Td right>
                    <div className="flex justify-end gap-1">
                      <Button
                        size="sm"
                        variant="primary"
                        onClick={() => {
                          setLogging(i);
                          setLogDate(todayISO());
                          setLogTx(true);
                          setLogAmount(i.income.variable ? i.estimate : i.income.amount);
                        }}
                      >
                        Log received
                      </Button>
                      <Button size="sm" onClick={() => setEditing({ ...i.income })}>
                        Edit
                      </Button>
                      <DeleteButton onConfirm={() => remove(i.income.id)} />
                    </div>
                  </Td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
      <p className="text-xs text-faint">Logging a receipt adds the income transaction, so cash flow, savings rate and the advisor all see it. Sources with varying amounts use the average of your last three receipts as the estimate.</p>

      <Modal open={!!editing} title={editing?.id ? "Edit income source" : "Add income source"} onClose={() => setEditing(null)}>
        {editing && (
          <form
            className="space-y-3"
            noValidate
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <div className="grid grid-cols-3 gap-3">
              <Field label="Name" className="col-span-2">
                <input className={inputCls} autoFocus value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} placeholder="e.g. Paycheck" />
              </Field>
              <Field label={editing.variable ? "Typical amount" : "Amount per payment"} hint={editing.variable ? "estimate; refined from what you log" : undefined}>
                <NumberInput value={editing.amount} onChange={(n) => setEditing({ ...editing, amount: n })} min={0} />
              </Field>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <Field label="How often">
                <select
                  className={inputCls}
                  value={editing.frequency}
                  onChange={(e) => {
                    const f = e.target.value as IncomeFrequency;
                    setEditing({ ...editing, frequency: f, due_day: f === "weekly" ? Math.min(editing.due_day, 7) || 5 : editing.due_day || 1, due_day2: f === "semimonthly" ? editing.due_day2 || 15 : editing.due_day2 });
                  }}
                >
                  {INCOME_FREQUENCIES.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.label}
                    </option>
                  ))}
                </select>
              </Field>
              {editing.frequency === "weekly" && (
                <Field label="Pay day">
                  <select className={inputCls} value={editing.due_day} onChange={(e) => setEditing({ ...editing, due_day: Number(e.target.value) })}>
                    {WEEKDAYS.map((d, i) => (
                      <option key={d} value={i + 1}>
                        {d}
                      </option>
                    ))}
                  </select>
                </Field>
              )}
              {editing.frequency === "biweekly" && (
                <Field label="A recent pay date" className="col-span-2" hint="any pay date; the next ones are every 14 days from it">
                  <input type="date" className={inputCls} value={editing.anchor ?? ""} onChange={(e) => setEditing({ ...editing, anchor: e.target.value || null })} />
                </Field>
              )}
              {(editing.frequency === "monthly" || editing.frequency === "quarterly" || editing.frequency === "yearly" || editing.frequency === "semimonthly") && (
                <Field label={editing.frequency === "semimonthly" ? "First day" : "Day of month"}>
                  <NumberInput value={editing.due_day} onChange={(n) => setEditing({ ...editing, due_day: Math.round(n) })} min={1} step={1} />
                </Field>
              )}
              {editing.frequency === "semimonthly" && (
                <Field label="Second day" hint="31 = last day of the month">
                  <NumberInput value={editing.due_day2} onChange={(n) => setEditing({ ...editing, due_day2: Math.round(n) })} min={1} step={1} />
                </Field>
              )}
              {(editing.frequency === "yearly" || editing.frequency === "quarterly") && (
                <Field label={editing.frequency === "yearly" ? "Month" : "First month of cycle"}>
                  <select className={inputCls} value={editing.due_month} onChange={(e) => setEditing({ ...editing, due_month: Number(e.target.value) })}>
                    {MONTHS.map((m, i) => (
                      <option key={m} value={i + 1}>
                        {m}
                      </option>
                    ))}
                  </select>
                </Field>
              )}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Category" hint="used for the logged transaction">
                <input className={inputCls} list="income-cats" value={editing.category} onChange={(e) => setEditing({ ...editing, category: e.target.value })} />
                <datalist id="income-cats">
                  {CATS.map((c) => (
                    <option key={c} value={c} />
                  ))}
                </datalist>
              </Field>
              <Field label="Deposits to">
                <select className={inputCls} value={editing.account_id ?? ""} onChange={(e) => setEditing({ ...editing, account_id: e.target.value ? Number(e.target.value) : null })}>
                  <option value="">(none)</option>
                  {snap.accounts.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.name}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Last received (optional)" hint="tells FinCompass which cycle you're on">
                <input type="date" className={inputCls} value={editing.last_received ?? ""} onChange={(e) => setEditing({ ...editing, last_received: e.target.value || null })} />
              </Field>
              <Field label="Note">
                <input className={inputCls} value={editing.note} onChange={(e) => setEditing({ ...editing, note: e.target.value })} />
              </Field>
            </div>
            <label className="flex items-center gap-2 text-sm text-dim">
              <input type="checkbox" checked={editing.variable} onChange={(e) => setEditing({ ...editing, variable: e.target.checked })} />
              Amount varies (tips, commission, hourly, freelance). You'll enter the real amount each time you log it.
            </label>
            <ErrorText>{error}</ErrorText>
            <div className="flex justify-end gap-2 pt-1">
              <Button type="button" onClick={() => setEditing(null)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" disabled={busy}>
                Save
              </Button>
            </div>
          </form>
        )}
      </Modal>

      <Modal open={!!logging} title={logging ? `Log ${logging.income.name} received` : ""} onClose={() => setLogging(null)}>
        {logging && (
          <form
            className="space-y-3"
            noValidate
            onSubmit={(e) => {
              e.preventDefault();
              void logReceived();
            }}
          >
            <div className="text-sm text-dim">
              {logging.income.category} · into {accName(logging.income.account_id)}
              {logging.income.variable && logging.history.length > 0 && <> · last {logging.history.slice(0, 3).map((p) => money(p.amount)).join(", ")}</>}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Amount received" hint={logging.income.variable ? `estimate was ${money(logging.estimate)}` : undefined}>
                <NumberInput autoFocus value={logAmount} onChange={setLogAmount} min={0} />
              </Field>
              <Field label="Date">
                <input type="date" className={inputCls} value={logDate} onChange={(e) => setLogDate(e.target.value)} />
              </Field>
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={logTx} onChange={(e) => setLogTx(e.target.checked)} />
              Also add a {money(logAmount)} income transaction
            </label>
            <div className="flex justify-end gap-2 pt-1">
              <Button type="button" onClick={() => setLogging(null)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" disabled={busy}>
                Log received
              </Button>
            </div>
          </form>
        )}
      </Modal>
    </div>
  );
}
