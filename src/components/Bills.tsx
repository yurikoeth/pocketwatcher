import { useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money, todayISO } from "../format";
import { FREQUENCIES, type BillInput, type BillStatus, type Frequency } from "../types";
import { Badge, Button, Card, DeleteButton, ErrorText, Field, Modal, NumberInput, Stat, Td, Th, inputCls } from "./ui";

const CATS = ["Rent", "Mortgage", "Utilities", "Internet", "Phone", "Insurance", "Subscriptions", "Debt payment", "Childcare", "Transport", "Health", "Other"];
const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const blank = (accountId: number | null): BillInput => ({
  id: null,
  name: "",
  amount: 0,
  category: "Utilities",
  frequency: "monthly",
  due_day: 1,
  due_month: 1,
  account_id: accountId,
  autopay: false,
  variable: false,
  last_paid: null,
  note: "",
});

function dueText(b: BillStatus): string {
  const { bill } = b;
  if (bill.frequency === "weekly") return `every ${WEEKDAYS[bill.due_day - 1] ?? "?"}`;
  const day = `${bill.due_day}${[1, 21, 31].includes(bill.due_day) ? "st" : [2, 22].includes(bill.due_day) ? "nd" : [3, 23].includes(bill.due_day) ? "rd" : "th"}`;
  if (bill.frequency === "monthly") return `monthly on the ${day}`;
  if (bill.frequency === "quarterly") return `quarterly from ${MONTHS[bill.due_month - 1]} ${bill.due_day}`;
  return `yearly on ${MONTHS[bill.due_month - 1]} ${bill.due_day}`;
}

export default function Bills({ snap, refresh }: TabProps) {
  const [editing, setEditing] = useState<BillInput | null>(null);
  const [paying, setPaying] = useState<BillStatus | null>(null);
  const [payDate, setPayDate] = useState(todayISO());
  const [payLog, setPayLog] = useState(true);
  const [payAmount, setPayAmount] = useState(0);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const accName = (id: number | null) => snap.accounts.find((a) => a.id === id)?.name ?? "—";

  const save = async () => {
    if (!editing) return;
    setBusy(true);
    setError("");
    try {
      await api.upsertBill(editing);
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
      await api.deleteBill(id);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const pay = async () => {
    if (!paying) return;
    setBusy(true);
    setError("");
    try {
      await api.markBillPaid(paying.bill.id, payDate, payLog, payAmount > 0 ? payAmount : null);
      setPaying(null);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const overdue = snap.bills.filter((b) => b.overdue_days > 0);
  const due30 = snap.bills.filter((b) => b.days_until <= 30).reduce((s, b) => s + b.estimate, 0);
  const autopay = snap.bills.filter((b) => b.bill.autopay).reduce((s, b) => s + b.monthly_equivalent, 0);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Bills</h1>
        <Button
          variant="primary"
          onClick={() => {
            setEditing(blank(snap.accounts.find((a) => a.kind === "checking")?.id ?? snap.accounts[0]?.id ?? null));
          }}
        >
          + Add bill
        </Button>
      </div>
      <ErrorText>{error}</ErrorText>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat label="Recurring per month" value={money(snap.bills_monthly)} sub={snap.avg_income > 0 ? `${Math.round((snap.bills_monthly / snap.avg_income) * 100)}% of avg income` : "all bills, monthly equivalent"} />
        <Stat label="Due next 14 days" value={money(snap.bills_due_14d)} tone={snap.bills_due_14d > snap.liquid ? "bad" : "neutral"} sub={snap.bills_due_14d > snap.liquid ? `more than ${money(snap.liquid)} liquid cash` : `liquid cash ${money(snap.liquid)}`} />
        <Stat label="Due next 30 days" value={money(due30)} sub="next occurrence of each bill" />
        <Stat label="Overdue" value={overdue.length ? money(overdue.reduce((s, b) => s + b.estimate, 0)) : "none"} tone={overdue.length ? "bad" : "good"} sub={overdue.length ? `${overdue.length} bill${overdue.length === 1 ? "" : "s"}` : `${money(autopay)}/mo on autopay`} />
      </div>

      <Card title="All bills">
        {snap.bills.length === 0 ? (
          <div className="text-sm text-dim">Add rent, utilities, subscriptions, insurance and loan payments here. Each bill knows when it's next due; marking it paid logs the expense for you.</div>
        ) : (
          <table className="w-full">
            <thead>
              <tr className="border-b border-line">
                <Th>Bill</Th>
                <Th>Schedule</Th>
                <Th>Pay from</Th>
                <Th right>Amount</Th>
                <Th right>/ month</Th>
                <Th>Status</Th>
                <Th right></Th>
              </tr>
            </thead>
            <tbody>
              {snap.bills.map((b) => (
                <tr key={b.bill.id} className="border-b border-line/60 hover:bg-panel2/40">
                  <Td className="font-medium">
                    {b.bill.name}
                    <div className="text-[11px] font-normal text-faint">
                      {b.bill.category}
                      {b.bill.note && ` · ${b.bill.note}`}
                    </div>
                  </Td>
                  <Td className="text-dim">
                    {dueText(b)}
                    {b.bill.autopay && <Badge>autopay</Badge>}
                  </Td>
                  <Td className="text-dim">{accName(b.bill.account_id)}</Td>
                  <Td right>
                    {b.bill.variable ? (
                      <span title={`Estimate from your last ${Math.min(3, b.history.length) || "0"} payments`}>
                        ~{money(b.estimate)}
                        {b.history.length > 0 && (
                          <div className="text-[11px] font-normal text-faint">
                            last {b.history.slice(0, 3).map((p) => money(p.amount)).join(", ")}
                          </div>
                        )}
                      </span>
                    ) : (
                      money(b.bill.amount)
                    )}
                  </Td>
                  <Td right className="text-dim">
                    {money(b.monthly_equivalent)}
                  </Td>
                  <Td>
                    {b.overdue_days > 0 ? (
                      <Badge tone="bad">{b.overdue_days}d overdue</Badge>
                    ) : b.paid_this_cycle ? (
                      <span className="text-xs text-faint">
                        paid {b.bill.last_paid} · next {b.next_due}
                      </span>
                    ) : (
                      <Badge tone={b.days_until <= 3 ? "warn" : "neutral"}>{b.days_until === 0 ? "due today" : `due ${b.next_due} (${b.days_until}d)`}</Badge>
                    )}
                  </Td>
                  <Td right>
                    <div className="flex justify-end gap-1">
                      {!b.paid_this_cycle && (
                        <Button
                          size="sm"
                          variant="primary"
                          onClick={() => {
                            setPaying(b);
                            setPayDate(todayISO());
                            setPayLog(true);
                            setPayAmount(b.bill.variable ? b.estimate : b.bill.amount);
                          }}
                        >
                          Mark paid
                        </Button>
                      )}
                      <Button size="sm" onClick={() => setEditing({ ...b.bill })}>
                        Edit
                      </Button>
                      <DeleteButton onConfirm={() => remove(b.bill.id)} />
                    </div>
                  </Td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
      <p className="text-xs text-faint">A bill counts as paid for the current cycle once a payment on or after its latest due date is recorded. Bills you just added aren't flagged overdue until their first due date passes.</p>

      <Modal open={!!editing} title={editing?.id ? "Edit bill" : "Add bill"} onClose={() => setEditing(null)}>
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
                <input className={inputCls} autoFocus value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} placeholder="e.g. Electric" />
              </Field>
              <Field label={editing.variable ? "Typical amount" : "Amount"} hint={editing.variable ? "estimate; refined from what you actually pay" : undefined}>
                <NumberInput value={editing.amount} onChange={(n) => setEditing({ ...editing, amount: n })} min={0} />
              </Field>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <Field label="Frequency">
                <select className={inputCls} value={editing.frequency} onChange={(e) => setEditing({ ...editing, frequency: e.target.value as Frequency, due_day: Math.min(editing.due_day, e.target.value === "weekly" ? 7 : 31) })}>
                  {FREQUENCIES.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.label}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label={editing.frequency === "weekly" ? "Weekday" : "Day of month"}>
                {editing.frequency === "weekly" ? (
                  <select className={inputCls} value={editing.due_day} onChange={(e) => setEditing({ ...editing, due_day: Number(e.target.value) })}>
                    {WEEKDAYS.map((d, i) => (
                      <option key={d} value={i + 1}>
                        {d}
                      </option>
                    ))}
                  </select>
                ) : (
                  <NumberInput value={editing.due_day} onChange={(n) => setEditing({ ...editing, due_day: Math.round(n) })} min={1} step={1} />
                )}
              </Field>
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
              <Field label="Category" hint="used for the logged expense and Budget">
                <input className={inputCls} list="bill-cats" value={editing.category} onChange={(e) => setEditing({ ...editing, category: e.target.value })} />
                <datalist id="bill-cats">
                  {CATS.map((c) => (
                    <option key={c} value={c} />
                  ))}
                </datalist>
              </Field>
              <Field label="Paid from">
                <select className={inputCls} value={editing.account_id ?? ""} onChange={(e) => setEditing({ ...editing, account_id: e.target.value ? Number(e.target.value) : null })}>
                  <option value="">(none)</option>
                  {snap.accounts.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.name}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Last paid (optional)" hint="lets FinCompass know which cycle you're on">
                <input type="date" className={inputCls} value={editing.last_paid ?? ""} onChange={(e) => setEditing({ ...editing, last_paid: e.target.value || null })} />
              </Field>
              <Field label="Note">
                <input className={inputCls} value={editing.note} onChange={(e) => setEditing({ ...editing, note: e.target.value })} />
              </Field>
            </div>
            <label className="flex items-center gap-2 text-sm text-dim">
              <input type="checkbox" checked={editing.variable} onChange={(e) => setEditing({ ...editing, variable: e.target.checked })} />
              Amount varies (electric, water, phone overages). You'll enter the real amount each time you mark it paid.
            </label>
            <label className="flex items-center gap-2 text-sm text-dim">
              <input type="checkbox" checked={editing.autopay} onChange={(e) => setEditing({ ...editing, autopay: e.target.checked })} />
              Autopay (paid automatically; you still mark it paid to log the expense)
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

      <Modal open={!!paying} title={paying ? `Mark ${paying.bill.name} paid` : ""} onClose={() => setPaying(null)}>
        {paying && (
          <form
            className="space-y-3"
            noValidate
            onSubmit={(e) => {
              e.preventDefault();
              void pay();
            }}
          >
            <div className="text-sm text-dim">
              {paying.bill.category} · from {accName(paying.bill.account_id)}
              {paying.bill.variable && paying.history.length > 0 && <> · last {paying.history.slice(0, 3).map((p) => money(p.amount)).join(", ")}</>}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <Field label={paying.bill.variable ? "Amount on this bill" : "Amount"} hint={paying.bill.variable ? `estimate was ${money(paying.estimate)}` : undefined}>
                <NumberInput autoFocus={paying.bill.variable} value={payAmount} onChange={setPayAmount} min={0} />
              </Field>
              <Field label="Payment date">
                <input type="date" className={inputCls} value={payDate} onChange={(e) => setPayDate(e.target.value)} />
              </Field>
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={payLog} onChange={(e) => setPayLog(e.target.checked)} />
              Also log a {money(payAmount)} expense transaction
            </label>
            <div className="flex justify-end gap-2 pt-1">
              <Button type="button" onClick={() => setPaying(null)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" disabled={busy}>
                Mark paid
              </Button>
            </div>
          </form>
        )}
      </Modal>
    </div>
  );
}
