import { useState } from "react";
import type { TabProps } from "../App";
import { api, errMsg } from "../api";
import { money } from "../format";
import type { GoalInput } from "../types";
import { Badge, Button, Card, DeleteButton, ErrorText, Field, Modal, NumberInput, Progress, inputCls } from "./ui";

const blank = (): GoalInput => ({ id: null, name: "", target_amount: 0, current_amount: 0, target_date: null, monthly_contribution: 0 });

export default function Goals({ snap, refresh }: TabProps) {
  const [editing, setEditing] = useState<GoalInput | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const save = async () => {
    if (!editing) return;
    setBusy(true);
    setError("");
    try {
      await api.upsertGoal(editing);
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
      await api.deleteGoal(id);
      await refresh();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  const totalMonthly = snap.goals.reduce((s, g) => s + g.goal.monthly_contribution, 0);
  const surplus = snap.avg_income - snap.avg_expenses;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Goals</h1>
        <Button variant="primary" onClick={() => setEditing(blank())}>
          + Add goal
        </Button>
      </div>
      <ErrorText>{error}</ErrorText>

      {snap.goals.length > 0 && (
        <div className="text-sm text-dim">
          Committed contributions: <span className="num text-fg">{money(totalMonthly)}/mo</span>
          {snap.avg_income > 0 && (
            <>
              {" "}
              · average monthly surplus: <span className={`num ${surplus >= totalMonthly ? "text-good" : "text-warn"}`}>{money(surplus)}</span>
              {surplus < totalMonthly && <span className="text-warn"> (contributions exceed surplus)</span>}
            </>
          )}
        </div>
      )}

      {snap.goals.length === 0 ? (
        <Card>
          <div className="text-sm text-dim">No goals yet. A goal is a target amount, optionally with a date, and what you put toward it each month.</div>
        </Card>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {snap.goals.map((g) => {
            const remaining = Math.max(0, g.goal.target_amount - g.goal.current_amount);
            return (
              <Card
                key={g.goal.id}
                title={g.goal.name}
                right={
                  <>
                    <Badge tone={g.on_track ? "good" : "warn"}>{remaining <= 0 ? "Reached" : g.on_track ? "On track" : "At risk"}</Badge>
                    <Button size="sm" onClick={() => setEditing({ ...g.goal })}>
                      Edit
                    </Button>
                    <DeleteButton onConfirm={() => remove(g.goal.id)} />
                  </>
                }
              >
                <div className="mb-1 flex items-baseline justify-between">
                  <span className="num text-2xl font-semibold">{money(g.goal.current_amount)}</span>
                  <span className="num text-dim">of {money(g.goal.target_amount)}</span>
                </div>
                <Progress value={g.goal.current_amount} max={g.goal.target_amount} tone={g.on_track ? "good" : "warn"} />
                <dl className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
                  <dt className="text-faint">Progress</dt>
                  <dd className="num text-right">{g.progress_pct.toFixed(0)}%</dd>
                  <dt className="text-faint">Contributing</dt>
                  <dd className="num text-right">{money(g.goal.monthly_contribution)}/mo</dd>
                  {g.goal.target_date && (
                    <>
                      <dt className="text-faint">Target date</dt>
                      <dd className="num text-right">
                        {g.goal.target_date} <span className="text-faint">({g.months_left} mo)</span>
                      </dd>
                      <dt className="text-faint">Needed</dt>
                      <dd className={`num text-right ${!g.on_track ? "text-warn" : ""}`}>{money(g.required_monthly ?? 0)}/mo</dd>
                    </>
                  )}
                  <dt className="text-faint">At current pace</dt>
                  <dd className="num text-right">{g.months_to_reach == null ? "never" : g.months_to_reach === 0 ? "reached" : `${g.months_to_reach} mo`}</dd>
                </dl>
              </Card>
            );
          })}
        </div>
      )}

      <Modal open={!!editing} title={editing?.id ? "Edit goal" : "Add goal"} onClose={() => setEditing(null)}>
        {editing && (
          <form
            className="space-y-3"
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <Field label="Name">
              <input className={inputCls} autoFocus value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} placeholder="e.g. House down payment" />
            </Field>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Target amount">
                <NumberInput value={editing.target_amount} onChange={(n) => setEditing({ ...editing, target_amount: n })} min={0} />
              </Field>
              <Field label="Saved so far">
                <NumberInput value={editing.current_amount} onChange={(n) => setEditing({ ...editing, current_amount: n })} min={0} />
              </Field>
              <Field label="Target date (optional)">
                <input type="date" className={inputCls} value={editing.target_date ?? ""} onChange={(e) => setEditing({ ...editing, target_date: e.target.value || null })} />
              </Field>
              <Field label="Monthly contribution">
                <NumberInput value={editing.monthly_contribution} onChange={(n) => setEditing({ ...editing, monthly_contribution: n })} min={0} />
              </Field>
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
