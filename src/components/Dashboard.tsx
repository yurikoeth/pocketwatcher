import type { TabProps } from "../App";
import { money, moneyK, monthLabel, pct } from "../format";
import { GroupedBars } from "./charts";
import { Button, Card, Empty, InsightRow, Progress, Stat } from "./ui";

export default function Dashboard({ snap, go }: TabProps) {
  if (!snap.has_data) {
    return (
      <div className="space-y-4">
        <h1 className="text-xl font-semibold">Dashboard</h1>
        <Empty title="Nothing here yet" text="Add your accounts and a few months of transactions, or load the sample data set to explore the app.">
          <Button variant="primary" onClick={() => go("accounts")}>
            Add accounts
          </Button>
          <Button onClick={() => go("settings")}>Load sample data</Button>
        </Empty>
      </div>
    );
  }

  const months = snap.months;
  const topCats = snap.categories.filter((c) => c.spent > 0 || c.limit != null).slice(0, 6);
  const savingsTone = snap.savings_rate == null ? "neutral" : snap.savings_rate < 0 ? "bad" : snap.savings_rate < 10 ? "warn" : snap.savings_rate >= 20 ? "good" : "neutral";
  const emTone = snap.emergency_months == null ? "neutral" : snap.emergency_months < 1 ? "bad" : snap.emergency_months < 3 ? "warn" : "good";
  const dtiTone = snap.dti == null ? "neutral" : snap.dti > 43 ? "bad" : snap.dti > 36 ? "warn" : "good";

  return (
    <div className="space-y-4">
      <div className="flex items-baseline justify-between">
        <h1 className="text-xl font-semibold">Dashboard</h1>
        <span className="text-xs text-faint">as of {snap.today}</span>
      </div>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat label="Net worth" value={money(snap.net_worth)} tone={snap.net_worth < 0 ? "bad" : "neutral"} sub={`${money(snap.assets)} assets − ${money(snap.liabilities)} debt`} />
        <Stat label="Liquid cash" value={money(snap.liquid)} sub="checking, savings, cash" />
        <Stat label="Invested" value={money(snap.invested)} sub="brokerage, retirement, crypto" />
        <Stat label="Debt" value={money(snap.liabilities)} tone={snap.liabilities > 0 ? "warn" : "good"} sub={snap.payoff.months != null ? `debt-free in ${snap.payoff.months} mo at minimums` : snap.liabilities > 0 ? "not paying down at minimums" : "no debt"} />
      </div>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
        <Stat label="Avg income" value={money(snap.avg_income)} sub={`/mo, ${snap.avg_basis}`} />
        <Stat label="Avg expenses" value={money(snap.avg_expenses)} sub="/mo, excl. transfers" />
        <Stat label="Savings rate" value={snap.savings_rate == null ? "—" : pct(snap.savings_rate)} tone={savingsTone} sub={snap.avg_income > 0 ? `${money(snap.avg_income - snap.avg_expenses)}/mo kept` : "needs income data"} />
        <Stat label="Emergency fund" value={snap.emergency_months == null ? "—" : `${snap.emergency_months.toFixed(1)} mo`} tone={emTone} sub="months of expenses in cash" />
        <Stat label="Debt-to-income" value={snap.dti == null ? "—" : pct(snap.dti)} tone={dtiTone} sub="min payments ÷ income" />
      </div>

      <div className="grid gap-4 lg:grid-cols-5">
        <Card title="Insights" className="lg:col-span-3">
          <div className="space-y-2">
            {snap.insights.map((i, idx) => (
              <InsightRow key={idx} {...i} />
            ))}
          </div>
        </Card>

        <div className="space-y-4 lg:col-span-2">
          <Card title="Cash flow, last 12 months">
            <GroupedBars
              labels={months.map((m) => monthLabel(m.month))}
              series={[
                { name: "Income", values: months.map((m) => m.income) },
                { name: "Expenses", values: months.map((m) => m.expenses) },
              ]}
              format={moneyK}
              height={190}
            />
          </Card>

          <Card title="This month's spending" right={<Button size="sm" onClick={() => go("budget")}>Budget</Button>}>
            {topCats.length === 0 ? (
              <div className="text-sm text-dim">No expenses logged this month.</div>
            ) : (
              <div className="space-y-2.5">
                {topCats.map((c) => {
                  const over = c.limit != null && c.spent > c.limit;
                  return (
                    <div key={c.category}>
                      <div className="mb-1 flex items-baseline justify-between text-sm">
                        <span>{c.category}</span>
                        <span className="num text-dim">
                          <span className={over ? "font-semibold text-critical" : "text-fg"}>{money(c.spent)}</span>
                          {c.limit != null && <span> / {money(c.limit)}</span>}
                        </span>
                      </div>
                      <Progress value={c.spent} max={c.limit ?? Math.max(c.spent, c.avg_3mo, 1)} tone={c.limit == null ? "neutral" : over ? "bad" : c.spent / c.limit > 0.85 ? "warn" : "good"} />
                    </div>
                  );
                })}
              </div>
            )}
          </Card>

          <Card title={`Income · ${money(snap.income_monthly)}/mo expected`} right={<Button size="sm" onClick={() => go("income")}>Income</Button>}>
            {snap.incomes.length === 0 ? (
              <div className="text-sm text-dim">No income sources set up.</div>
            ) : (
              <ul className="space-y-1.5 text-sm">
                {snap.incomes.slice(0, 5).map((i) => (
                  <li key={i.income.id} className="flex items-baseline justify-between gap-3">
                    <span className="truncate">{i.income.name}</span>
                    <span className={`num shrink-0 ${i.late_days > 0 ? "text-warn" : i.received_this_cycle ? "text-faint" : "text-dim"}`}>
                      {i.income.variable ? "~" : ""}
                      {money(i.estimate)} ·{" "}
                      {i.late_days > 0
                        ? `${i.late_days}d late`
                        : i.next_date == null
                          ? "irregular"
                          : i.received_this_cycle
                            ? `received, next ${i.next_date.slice(5)}`
                            : i.days_until === 0
                              ? "today"
                              : `in ${i.days_until}d`}
                    </span>
                  </li>
                ))}
                <li className="flex justify-between border-t border-line pt-1.5 text-xs text-faint">
                  <span>Logged this month</span>
                  <span className="num">{money(snap.income_this_month)}</span>
                </li>
              </ul>
            )}
          </Card>

          <Card title={`Upcoming bills · ${money(snap.bills_monthly)}/mo`} right={<Button size="sm" onClick={() => go("bills")}>Bills</Button>}>
            {snap.bills.length === 0 ? (
              <div className="text-sm text-dim">No recurring bills set up.</div>
            ) : (
              <ul className="space-y-1.5 text-sm">
                {snap.bills
                  .filter((b) => b.overdue_days > 0 || b.days_until <= 30)
                  .slice(0, 6)
                  .map((b) => (
                    <li key={b.bill.id} className="flex items-baseline justify-between gap-3">
                      <span className="truncate">
                        {b.bill.name}
                        {b.bill.autopay && <span className="ml-1 text-[10px] text-faint">AUTO</span>}
                      </span>
                      <span className={`num shrink-0 ${b.overdue_days > 0 ? "text-critical" : b.paid_this_cycle ? "text-faint" : b.days_until <= 3 ? "text-warn" : "text-dim"}`}>
                        {b.bill.variable ? "~" : ""}
                        {money(b.estimate)} · {b.overdue_days > 0 ? `${b.overdue_days}d late` : b.paid_this_cycle ? `paid, next ${b.next_due.slice(5)}` : b.days_until === 0 ? "today" : `in ${b.days_until}d`}
                      </span>
                    </li>
                  ))}
              </ul>
            )}
          </Card>

          <Card title="Goals" right={<Button size="sm" onClick={() => go("goals")}>Manage</Button>}>
            {snap.goals.length === 0 ? (
              <div className="text-sm text-dim">No goals yet.</div>
            ) : (
              <div className="space-y-2.5">
                {snap.goals.map((g) => (
                  <div key={g.goal.id}>
                    <div className="mb-1 flex items-baseline justify-between text-sm">
                      <span>{g.goal.name}</span>
                      <span className="num text-dim">
                        {money(g.goal.current_amount)} / {money(g.goal.target_amount)}
                      </span>
                    </div>
                    <Progress value={g.goal.current_amount} max={g.goal.target_amount} tone={g.on_track ? "good" : "warn"} />
                  </div>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
