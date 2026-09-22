import { useEffect, useMemo, useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { moneyC, monthLong, todayISO } from "../format";
import type { Transaction, TransactionInput } from "../types";
import { Button, Card, DeleteButton, ErrorText, Field, Modal, NumberInput, Td, Th, inputCls } from "./ui";

const SUGGESTED = ["Salary", "Bonus", "Rent", "Mortgage", "Groceries", "Dining", "Transport", "Utilities", "Insurance", "Health", "Subscriptions", "Shopping", "Entertainment", "Travel", "Education", "Gifts", "Debt payment", "Transfer", "Other"];

const blank = (accountId: number | null): TransactionInput => ({ id: null, account_id: accountId, date: todayISO(), amount: 0, category: "", note: "" });

export default function Transactions({ snap, refresh }: TabProps) {
  const [rows, setRows] = useState<Transaction[]>([]);
  const [month, setMonth] = useState<string>("all");
  const [q, setQ] = useState("");
  const [editing, setEditing] = useState<TransactionInput | null>(null);
  const [kind, setKind] = useState<"expense" | "income">("expense");
  const [importOpen, setImportOpen] = useState(false);
  const [csv, setCsv] = useState("");
  const [csvAccount, setCsvAccount] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const load = async () => {
    try {
      setRows(await api.transactions());
    } catch (e) {
      setError(errMsg(e));
    }
  };
  useEffect(() => {
    void load();
  }, [snap]);

  const months = useMemo(() => Array.from(new Set(rows.map((r) => r.date.slice(0, 7)))).sort().reverse(), [rows]);
  const categories = useMemo(() => Array.from(new Set([...SUGGESTED, ...rows.map((r) => r.category)])), [rows]);
  const accName = (id: number | null) => snap.accounts.find((a) => a.id === id)?.name ?? "—";

  const filtered = rows.filter((r) => (month === "all" || r.date.startsWith(month)) && (!q || `${r.category} ${r.note} ${accName(r.account_id)}`.toLowerCase().includes(q.toLowerCase())));
  const inc = filtered.filter((r) => r.amount > 0 && r.category.toLowerCase() !== "transfer").reduce((s, r) => s + r.amount, 0);
  const exp = filtered.filter((r) => r.amount < 0 && r.category.toLowerCase() !== "transfer").reduce((s, r) => s - r.amount, 0);

  const openNew = () => {
    setKind("expense");
    setEditing(blank(snap.accounts[0]?.id ?? null));
  };
  const openEdit = (t: Transaction) => {
    setKind(t.amount >= 0 ? "income" : "expense");
    setEditing({ ...t, amount: Math.abs(t.amount) });
  };

  const save = async () => {
    if (!editing) return;
    setBusy(true);
    setError("");
    try {
      const amt = Math.abs(editing.amount) * (kind === "expense" ? -1 : 1);
      await api.upsertTransaction({ ...editing, amount: amt });
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
      await api.deleteTransaction(id);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const doImport = async () => {
    setBusy(true);
    setError("");
    try {
      const n = await api.importCsv(csvAccount, csv);
      setImportOpen(false);
      setCsv("");
      await refresh();
      setError(n === 0 ? "No rows imported." : "");
    } catch (e) {
      setError(errMsg(e));
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const pickFile = (f: File | undefined) => {
    if (!f) return;
    const r = new FileReader();
    r.onload = () => setCsv(String(r.result ?? ""));
    r.readAsText(f);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Transactions</h1>
        <div className="flex gap-2">
          <Button onClick={() => setImportOpen(true)}>Import CSV</Button>
          <Button variant="primary" onClick={openNew}>
            + Add
          </Button>
        </div>
      </div>
      <ErrorText>{error}</ErrorText>

      <Card
        title={
          <span>
            {filtered.length} rows · <span className="text-good">+{moneyC(inc)}</span> · <span className="text-critical">−{moneyC(exp)}</span>
          </span>
        }
        right={
          <>
            <select className={inputCls + " w-44"} value={month} onChange={(e) => setMonth(e.target.value)}>
              <option value="all">All months</option>
              {months.map((m) => (
                <option key={m} value={m}>
                  {monthLong(m)}
                </option>
              ))}
            </select>
            <input className={inputCls + " w-48"} placeholder="Search…" value={q} onChange={(e) => setQ(e.target.value)} />
          </>
        }
      >
        <table className="w-full">
          <thead>
            <tr className="border-b border-line">
              <Th>Date</Th>
              <Th>Category</Th>
              <Th>Note</Th>
              <Th>Account</Th>
              <Th right>Amount</Th>
              <Th right></Th>
            </tr>
          </thead>
          <tbody>
            {filtered.length === 0 && (
              <tr>
                <Td colSpan={6} className="text-dim">
                  No transactions match.
                </Td>
              </tr>
            )}
            {filtered.slice(0, 500).map((t) => (
              <tr key={t.id} className="border-b border-line/60 hover:bg-panel2/40">
                <Td className="num text-dim">{t.date}</Td>
                <Td>{t.category}</Td>
                <Td className="max-w-xs truncate text-dim">{t.note}</Td>
                <Td className="text-dim">{accName(t.account_id)}</Td>
                <Td right className={t.amount >= 0 ? "text-good" : ""}>
                  {t.amount >= 0 ? "+" : "−"}
                  {moneyC(Math.abs(t.amount))}
                </Td>
                <Td right>
                  <div className="flex justify-end gap-1">
                    <Button size="sm" onClick={() => openEdit(t)}>
                      Edit
                    </Button>
                    <DeleteButton onConfirm={() => remove(t.id)} />
                  </div>
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
        {filtered.length > 500 && <div className="pt-2 text-xs text-faint">Showing first 500. Narrow the filter to see more.</div>}
      </Card>

      <Modal open={!!editing} title={editing?.id ? "Edit transaction" : "Add transaction"} onClose={() => setEditing(null)}>
        {editing && (
          <form
            className="space-y-3"
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <div className="flex gap-1 rounded-md bg-panel2 p-1">
              {(["expense", "income"] as const).map((k) => (
                <button type="button" key={k} onClick={() => setKind(k)} className={`flex-1 rounded px-2 py-1 text-sm capitalize ${kind === k ? "bg-bg font-semibold" : "text-dim"}`}>
                  {k}
                </button>
              ))}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Date">
                <input type="date" className={inputCls} value={editing.date} onChange={(e) => setEditing({ ...editing, date: e.target.value })} />
              </Field>
              <Field label="Amount">
                <NumberInput autoFocus value={editing.amount} onChange={(n) => setEditing({ ...editing, amount: n })} min={0} />
              </Field>
            </div>
            <Field label="Category">
              <input className={inputCls} list="cats" value={editing.category} onChange={(e) => setEditing({ ...editing, category: e.target.value })} placeholder="Groceries" />
              <datalist id="cats">
                {categories.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </Field>
            <Field label="Account">
              <select className={inputCls} value={editing.account_id ?? ""} onChange={(e) => setEditing({ ...editing, account_id: e.target.value ? Number(e.target.value) : null })}>
                <option value="">(none)</option>
                {snap.accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Note">
              <input className={inputCls} value={editing.note} onChange={(e) => setEditing({ ...editing, note: e.target.value })} />
            </Field>
            <p className="text-[11px] text-faint">Use category "Transfer" for moves between your own accounts; they're excluded from income and expenses.</p>
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

      <Modal open={importOpen} title="Import CSV" onClose={() => setImportOpen(false)} width="max-w-2xl">
        <div className="space-y-3">
          <p className="text-sm text-dim">
            One row per line: <code className="font-mono text-fg">date,amount,category,note</code>. Dates as YYYY-MM-DD, positive amounts are income, negative are expenses. A header row is skipped automatically.
          </p>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Assign to account">
              <select className={inputCls} value={csvAccount ?? ""} onChange={(e) => setCsvAccount(e.target.value ? Number(e.target.value) : null)}>
                <option value="">(none)</option>
                {snap.accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Or pick a file">
              <input type="file" accept=".csv,.txt" className="text-sm" onChange={(e) => pickFile(e.target.files?.[0])} />
            </Field>
          </div>
          <textarea className={inputCls + " h-48 font-mono text-xs"} value={csv} onChange={(e) => setCsv(e.target.value)} placeholder={"2026-09-01,2650,Salary,Paycheck\n2026-09-03,-123.45,Groceries,Supermarket"} />
          <div className="flex justify-end gap-2">
            <Button onClick={() => setImportOpen(false)}>Cancel</Button>
            <Button variant="primary" disabled={busy || !csv.trim()} onClick={() => void doImport()}>
              Import
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
