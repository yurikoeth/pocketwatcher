import { useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money } from "../format";
import { Button, Card, DeleteButton, ErrorText, Field, NumberInput, Progress, Td, Th, inputCls } from "./ui";

export default function BudgetView({ snap, refresh }: TabProps) {
  const [cat, setCat] = useState("");
  const [limit, setLimit] = useState(0);
  const [editing, setEditing] = useState<string | null>(null);
  const [editVal, setEditVal] = useState(0);
  const [error, setError] = useState("");

  const budgeted = snap.categories.filter((c) => c.limit != null);
  const unbudgeted = snap.categories.filter((c) => c.limit == null && (c.spent > 0 || c.avg_3mo > 0));
  const totalLimit = budgeted.reduce((s, c) => s + (c.limit ?? 0), 0);
  const totalSpent = budgeted.reduce((s, c) => s + c.spent, 0);
  const day = new Date().getDate();
  const daysInMonth = new Date(new Date().getFullYear(), new Date().getMonth() + 1, 0).getDate();
  const monthPct = (day / daysInMonth) * 100;

  const add = async (category: string, monthly_limit: number) => {
    setError("");
    try {
      await api.setBudget({ category, monthly_limit });
      setCat("");
      setLimit(0);
      setEditing(null);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };
  const remove = async (category: string) => {
    try {
      await api.deleteBudget(category);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Budget</h1>
      <ErrorText>{error}</ErrorText>

      <Card title="Add a monthly limit">
        <form
          className="flex flex-wrap items-end gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (cat.trim() && limit > 0) void add(cat.trim(), limit);
          }}
        >
          <Field label="Category" className="w-56">
            <input className={inputCls} list="budget-cats" value={cat} onChange={(e) => setCat(e.target.value)} placeholder="Groceries" />
            <datalist id="budget-cats">
              {unbudgeted.map((c) => (
                <option key={c.category} value={c.category} />
              ))}
            </datalist>
          </Field>
          <Field label="Limit / month" className="w-40">
            <NumberInput value={limit} onChange={setLimit} min={0} />
          </Field>
          <Button type="submit" variant="primary" disabled={!cat.trim() || limit <= 0}>
            Set limit
          </Button>
        </form>
      </Card>

      <Card
        title={`This month · ${money(totalSpent)} of ${money(totalLimit)}`}
        right={<span className="text-xs text-faint">{Math.round(monthPct)}% of the month elapsed</span>}
      >
        {budgeted.length === 0 ? (
          <div className="text-sm text-dim">No limits set. Add one above, or pick from the categories you've been spending in below.</div>
        ) : (
          <table className="w-full">
            <thead>
              <tr className="border-b border-line">
                <Th>Category</Th>
                <Th className="w-1/3">Progress</Th>
                <Th right>Spent</Th>
                <Th right>Limit</Th>
                <Th right>Left</Th>
                <Th right>3-mo avg</Th>
                <Th right></Th>
              </tr>
            </thead>
            <tbody>
              {budgeted.map((c) => {
                const lim = c.limit ?? 0;
                const left = lim - c.spent;
                const ratio = lim > 0 ? c.spent / lim : 0;
                const tone = ratio > 1 ? "bad" : ratio * 100 > monthPct + 15 ? "warn" : "good";
                return (
                  <tr key={c.category} className="border-b border-line/60">
                    <Td className="font-medium">{c.category}</Td>
                    <Td>
                      <Progress value={c.spent} max={lim} tone={tone} />
                    </Td>
                    <Td right className={ratio > 1 ? "font-semibold text-critical" : ""}>
                      {money(c.spent)}
                    </Td>
                    <Td right>
                      {editing === c.category ? (
                        <form
                          className="flex justify-end gap-1"
                          onSubmit={(e) => {
                            e.preventDefault();
                            void add(c.category, editVal);
                          }}
                        >
                          <span className="w-28">
                            <NumberInput value={editVal} onChange={setEditVal} min={0} autoFocus />
                          </span>
                          <Button size="sm" type="submit" variant="primary">
                            OK
                          </Button>
                        </form>
                      ) : (
                        <button
                          className="underline decoration-dotted underline-offset-2 hover:text-accent"
                          onClick={() => {
                            setEditing(c.category);
                            setEditVal(lim);
                          }}
                        >
                          {money(lim)}
                        </button>
                      )}
                    </Td>
                    <Td right className={left < 0 ? "text-critical" : "text-good"}>
                      {left < 0 ? `−${money(-left)}` : money(left)}
                    </Td>
                    <Td right className="text-dim">
                      {money(c.avg_3mo)}
                    </Td>
                    <Td right>
                      <DeleteButton onConfirm={() => remove(c.category)} label="Remove" />
                    </Td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </Card>

      {unbudgeted.length > 0 && (
        <Card title="Spending without a limit">
          <table className="w-full">
            <thead>
              <tr className="border-b border-line">
                <Th>Category</Th>
                <Th right>This month</Th>
                <Th right>3-mo avg</Th>
                <Th right></Th>
              </tr>
            </thead>
            <tbody>
              {unbudgeted.map((c) => (
                <tr key={c.category} className="border-b border-line/60">
                  <Td>{c.category}</Td>
                  <Td right>{money(c.spent)}</Td>
                  <Td right className="text-dim">
                    {money(c.avg_3mo)}
                  </Td>
                  <Td right>
                    <Button
                      size="sm"
                      onClick={() => {
                        setCat(c.category);
                        setLimit(Math.ceil(Math.max(c.avg_3mo, c.spent) / 10) * 10);
                        window.scrollTo({ top: 0, behavior: "smooth" });
                      }}
                    >
                      Set limit
                    </Button>
                  </Td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
