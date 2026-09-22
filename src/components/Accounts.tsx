import { useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money, pct } from "../format";
import { ASSET_KINDS, CHAINS, KIND_LABEL, LIABILITY_KINDS, isLiability, type Account, type AccountInput, type AccountKind, type Chain, type SyncResult } from "../types";
import { Button, Card, DeleteButton, ErrorText, Field, Modal, NumberInput, Td, Th, inputCls } from "./ui";

const blank = (kind: AccountKind = "checking"): AccountInput => ({ id: null, name: "", kind, balance: 0, apr: 0, min_payment: 0, chain: "", address: "" });

const shortAddr = (a: string) => (a.length > 14 ? `${a.slice(0, 6)}…${a.slice(-4)}` : a);
const fmtUnits = (n: number) => n.toLocaleString(undefined, { maximumFractionDigits: n >= 100 ? 2 : 6 });
const ago = (iso: string | null) => {
  if (!iso) return "never";
  const m = Math.round((Date.now() - new Date(iso).getTime()) / 60000);
  if (m < 1) return "just now";
  if (m < 60) return `${m} min ago`;
  if (m < 60 * 48) return `${Math.round(m / 60)} h ago`;
  return `${Math.round(m / 1440)} d ago`;
};

export default function Accounts({ snap, refresh }: TabProps) {
  const [editing, setEditing] = useState<AccountInput | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<SyncResult[] | null>(null);
  const [open, setOpen] = useState<Record<number, boolean>>({});

  const wallets = snap.accounts.filter((a) => a.kind === "crypto" && a.address);

  const sync = async () => {
    setSyncing(true);
    setError("");
    try {
      const r = await api.syncCrypto();
      setSyncMsg(r);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSyncing(false);
    }
  };

  const save = async () => {
    if (!editing) return;
    setBusy(true);
    setError("");
    try {
      // Liabilities store the amount owed as a positive number; assets may go negative (overdraft).
      await api.upsertAccount({ ...editing, balance: isLiability(editing.kind) ? Math.abs(editing.balance) : editing.balance });
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
      await api.deleteAccount(id);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const assets = snap.accounts.filter((a) => !isLiability(a.kind));
  const debts = snap.accounts.filter((a) => isLiability(a.kind));
  const liab = editing ? isLiability(editing.kind) : false;
  const isWallet = editing?.kind === "crypto";
  const synced = isWallet && !!editing?.address.trim();

  const Table = ({ rows, isDebt }: { rows: Account[]; isDebt: boolean }) => (
    <table className="w-full">
      <thead>
        <tr className="border-b border-line">
          <Th>Name</Th>
          <Th>Type</Th>
          <Th right>{isDebt ? "Owed" : "Balance"}</Th>
          <Th right>{isDebt ? "APR" : "Yield"}</Th>
          {isDebt && <Th right>Min payment</Th>}
          <Th right></Th>
        </tr>
      </thead>
      <tbody>
        {rows.length === 0 && (
          <tr>
            <Td colSpan={6} className="text-dim">
              None yet.
            </Td>
          </tr>
        )}
        {rows.map((a) => (
          <tr key={a.id} className="border-b border-line/60 hover:bg-panel2/40">
            <Td className="font-medium">
              {a.name}
              {a.kind === "crypto" && a.address && (
                <div className="mt-0.5 font-mono text-[11px] font-normal text-faint" title={a.address}>
                  {CHAINS.find((c) => c.id === a.chain)?.label ?? a.chain} · {shortAddr(a.address)} · synced {ago(a.last_synced)}
                  {(snap.holdings[String(a.id)]?.length ?? 0) > 0 && (
                    <button className="ml-2 text-accent hover:underline" onClick={() => setOpen((o) => ({ ...o, [a.id]: !o[a.id] }))}>
                      {open[a.id] ? "hide" : "show"} {snap.holdings[String(a.id)].length} coins
                    </button>
                  )}
                </div>
              )}
              {a.kind === "crypto" && open[a.id] && snap.holdings[String(a.id)] && (
                <table className="mt-2 w-full max-w-xl font-normal">
                  <tbody>
                    {snap.holdings[String(a.id)].map((h) => (
                      <tr key={h.contract} className="text-xs">
                        <td className="py-0.5 pr-3">
                          <span className="text-fg">{h.symbol}</span>
                          {h.name && h.name !== h.symbol && <span className="ml-1 text-faint">{h.name}</span>}
                        </td>
                        <td className="num py-0.5 pr-3 text-right text-dim">{fmtUnits(h.units)}</td>
                        <td className="num py-0.5 pr-3 text-right text-faint">{h.price == null ? "no price" : `@ ${h.price < 1 ? h.price.toPrecision(3) : money(h.price)}`}</td>
                        <td className="num py-0.5 text-right text-fg">{h.price == null ? "—" : money(h.value)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </Td>
            <Td className="text-dim">{KIND_LABEL[a.kind]}</Td>
            <Td right className={isDebt ? "text-critical" : ""}>
              {money(a.balance)}
            </Td>
            <Td right className="text-dim">
              {a.apr ? pct(a.apr, 1) : "—"}
            </Td>
            {isDebt && (
              <Td right className="text-dim">
                {a.min_payment ? money(a.min_payment) : "—"}
              </Td>
            )}
            <Td right>
              <div className="flex justify-end gap-1">
                <Button size="sm" onClick={() => setEditing({ ...a })}>
                  Edit
                </Button>
                <DeleteButton onConfirm={() => remove(a.id)} />
              </div>
            </Td>
          </tr>
        ))}
      </tbody>
    </table>
  );

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Accounts</h1>
        <Button variant="primary" onClick={() => setEditing(blank())}>
          + Add account
        </Button>
      </div>
      <ErrorText>{error}</ErrorText>
      <Card
        title={`Assets · ${money(snap.assets)}`}
        right={
          <>
            {wallets.length > 0 && (
              <Button size="sm" onClick={() => void sync()} disabled={syncing}>
                {syncing ? "Syncing…" : `↻ Sync ${wallets.length} wallet${wallets.length === 1 ? "" : "s"}`}
              </Button>
            )}
            <Button size="sm" onClick={() => setEditing(blank("crypto"))}>
              + Add wallet
            </Button>
            <Button size="sm" onClick={() => setEditing(blank("checking"))}>
              + Add asset
            </Button>
          </>
        }
      >
        <Table rows={assets} isDebt={false} />
        {syncMsg && syncMsg.length > 0 && (
          <div className="mt-3 space-y-1 text-xs">
            {syncMsg.map((r) => (
              <div key={r.id} className={r.error ? "text-critical" : "text-dim"}>
                <span className="text-fg">{r.name}</span>: {r.holdings.length > 0 && `${r.holdings.length} coin${r.holdings.length === 1 ? "" : "s"}, ${r.holdings.filter((h) => h.price == null).length} unpriced, total ${money(r.balance)} ${r.currency}. `}
                {r.error}
              </div>
            ))}
          </div>
        )}
      </Card>
      <Card
        title={`Liabilities · ${money(snap.liabilities)}`}
        right={
          <Button size="sm" onClick={() => setEditing(blank("credit_card"))}>
            + Add liability
          </Button>
        }
      >
        <Table rows={debts} isDebt />
        {debts.length === 0 && <p className="mt-2 text-xs text-faint">Credit cards, loans and mortgages go here. Enter the APR and minimum payment to get a payoff plan on the Debts tab.</p>}
      </Card>
      <p className="text-xs text-faint">Balances are what you enter here; transactions don't change them automatically. Update balances when you reconcile.</p>

      <Modal open={!!editing} title={editing?.id ? "Edit account" : "Add account"} onClose={() => setEditing(null)}>
        {editing && (
          <form
            className="space-y-3"
            noValidate
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <Field label="Name">
              <input className={inputCls} autoFocus value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} placeholder="e.g. Chase Checking" />
            </Field>
            {!editing.id && (
              <div className="flex gap-1 rounded-md bg-panel2 p-1">
                {(["asset", "liability"] as const).map((side) => {
                  const active = liab === (side === "liability");
                  return (
                    <button
                      type="button"
                      key={side}
                      onClick={() => setEditing({ ...editing, kind: side === "liability" ? "credit_card" : "checking" })}
                      className={`flex-1 rounded px-2 py-1 text-sm capitalize ${active ? "bg-bg font-semibold" : "text-dim"}`}
                    >
                      {side}
                    </button>
                  );
                })}
              </div>
            )}
            <Field label={liab ? "Liability type" : "Asset type"}>
              <select className={inputCls} value={editing.kind} onChange={(e) => setEditing({ ...editing, kind: e.target.value as AccountKind })}>
                {(liab ? LIABILITY_KINDS : ASSET_KINDS).map((k) => (
                  <option key={k} value={k}>
                    {KIND_LABEL[k]}
                  </option>
                ))}
              </select>
            </Field>
            {isWallet && (
              <div className="grid grid-cols-3 gap-3">
                <Field label="Chain">
                  <select className={inputCls} value={editing.chain} onChange={(e) => setEditing({ ...editing, chain: e.target.value as Chain | "" })}>
                    <option value="">Manual (no address)</option>
                    {CHAINS.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.label} ({c.symbol})
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Wallet address" className="col-span-2" hint="Public address only. Native-token balance is fetched from a public node and priced via CoinGecko when you click Sync.">
                  <input className={inputCls + " font-mono text-xs"} value={editing.address} onChange={(e) => setEditing({ ...editing, address: e.target.value.trim() })} placeholder={editing.chain === "bitcoin" ? "bc1q…" : editing.chain === "ethereum" ? "0x…" : "wallet address"} spellCheck={false} autoComplete="off" />
                </Field>
              </div>
            )}
            <div className="grid grid-cols-3 gap-3">
              <Field label={liab ? "Amount owed" : synced ? "Balance (set by sync)" : "Balance"} hint={liab ? undefined : synced ? "overwritten on next sync" : "negative = overdrawn"}>
                <NumberInput value={editing.balance} onChange={(n) => setEditing({ ...editing, balance: n })} min={liab ? 0 : undefined} />
              </Field>
              <Field label={liab ? "APR % (0 if none)" : "Yield % (optional)"}>
                <NumberInput value={editing.apr} onChange={(n) => setEditing({ ...editing, apr: n })} min={0} />
              </Field>
              {liab && (
                <Field label="Payment /mo" hint="what you plan to pay each month">
                  <NumberInput value={editing.min_payment} onChange={(n) => setEditing({ ...editing, min_payment: n })} min={0} />
                </Field>
              )}
            </div>
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
    </div>
  );
}
