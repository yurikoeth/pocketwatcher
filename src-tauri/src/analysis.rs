//! Pure financial calculations: cash flow, budgets, goals, debt payoff,
//! retirement projection, and the rule-based insight engine.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::db::{is_liability, Account, Bill, Budget, Goal, Holding, Income, Settings, Transaction, INVESTED_KINDS, LIQUID_KINDS};

// ---------- wire types ----------

#[derive(Debug, Clone, Serialize)]
pub struct MonthFlow {
    pub month: String,
    pub income: f64,
    pub expenses: f64,
    pub net: f64,
    pub tx_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategorySpend {
    pub category: String,
    pub spent: f64,
    pub limit: Option<f64>,
    pub avg_3mo: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoalStatus {
    pub goal: Goal,
    pub progress_pct: f64,
    pub months_left: Option<i64>,
    pub required_monthly: Option<f64>,
    pub months_to_reach: Option<f64>,
    pub on_track: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebtLine {
    pub account_id: i64,
    pub name: String,
    pub kind: String,
    pub balance: f64,
    pub apr: f64,
    pub min_payment: f64,
    /// Month index (1-based) at which the balance reaches zero.
    pub payoff_month: Option<u32>,
    pub interest_paid: f64,
    pub never: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayoffPlan {
    pub strategy: String,
    pub extra_monthly: f64,
    pub months: Option<u32>,
    pub total_interest: f64,
    pub total_paid: f64,
    pub debts: Vec<DebtLine>,
    /// Remaining total balance at the end of each month.
    pub balance_by_month: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillPayment {
    pub date: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillStatus {
    pub bill: Bill,
    /// What to expect next time: the bill amount, or for variable bills the
    /// average of the last three logged payments.
    pub estimate: f64,
    pub last_amount: Option<f64>,
    /// Most recent logged payments (newest first, up to 6).
    pub history: Vec<BillPayment>,
    /// Amount per month at this frequency, based on `estimate`.
    pub monthly_equivalent: f64,
    /// Next due date (YYYY-MM-DD).
    pub next_due: String,
    pub days_until: i64,
    /// True when a payment has been recorded on or after the most recent due date.
    pub paid_this_cycle: bool,
    /// Days past the most recent due date without a recorded payment (0 if not overdue).
    pub overdue_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncomeStatus {
    pub income: Income,
    /// Expected next payment: the typical amount, or for variable sources the
    /// average of the last three logged receipts.
    pub estimate: f64,
    pub last_amount: Option<f64>,
    /// Most recent logged receipts (newest first, up to 6).
    pub history: Vec<BillPayment>,
    /// Expected amount per month. Irregular sources use the last 90 days of logs.
    pub monthly_equivalent: f64,
    /// Next expected pay date; None for irregular sources.
    pub next_date: Option<String>,
    pub days_until: Option<i64>,
    pub received_this_cycle: bool,
    /// Days past the last expected pay date without a logged receipt.
    pub late_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Insight {
    /// critical | warn | info | good
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjPoint {
    pub year: u32,
    pub age: u32,
    pub nominal: f64,
    pub real: f64,
    pub contributed: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionParams {
    pub starting_balance: f64,
    pub monthly_contribution: f64,
    pub annual_return_pct: f64,
    pub inflation_pct: f64,
    pub age: u32,
    pub retirement_age: u32,
    pub retirement_income: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Projection {
    pub params_used: ProjectionEcho,
    pub points: Vec<ProjPoint>,
    pub final_nominal: f64,
    pub final_real: f64,
    /// 25x desired retirement income, in retirement-year dollars.
    pub target_nest_egg: f64,
    /// 4% of the final balance, in today's dollars.
    pub safe_withdrawal_real: f64,
    pub on_track: bool,
    /// Monthly contribution that would exactly hit the target.
    pub required_monthly: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectionEcho {
    pub starting_balance: f64,
    pub monthly_contribution: f64,
    pub annual_return_pct: f64,
    pub inflation_pct: f64,
    pub age: u32,
    pub retirement_age: u32,
    pub retirement_income: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub today: String,
    pub currency: String,
    pub accounts: Vec<Account>,
    pub net_worth: f64,
    pub assets: f64,
    pub liabilities: f64,
    pub liquid: f64,
    pub invested: f64,
    /// Token positions per crypto account id, from the last wallet sync.
    pub holdings: std::collections::HashMap<i64, Vec<Holding>>,
    pub months: Vec<MonthFlow>,
    pub avg_income: f64,
    pub avg_expenses: f64,
    pub avg_basis: String,
    pub savings_rate: Option<f64>,
    pub emergency_months: Option<f64>,
    pub dti: Option<f64>,
    pub categories: Vec<CategorySpend>,
    pub goals: Vec<GoalStatus>,
    pub bills: Vec<BillStatus>,
    pub incomes: Vec<IncomeStatus>,
    /// Expected income per month from all sources.
    pub income_monthly: f64,
    /// Income actually logged this calendar month (excluding transfers).
    pub income_this_month: f64,
    /// Sum of monthly equivalents of all bills.
    pub bills_monthly: f64,
    /// Bills whose next occurrence falls within the next 14 days.
    pub bills_due_14d: f64,
    pub payoff: PayoffPlan,
    pub projection: Projection,
    pub projection_contribution_source: String,
    pub insights: Vec<Insight>,
    pub tx_count: usize,
    pub has_data: bool,
}

// ---------- helpers ----------

fn is_transfer(cat: &str) -> bool {
    cat.trim().eq_ignore_ascii_case("transfer")
}

fn month_key(d: NaiveDate) -> String {
    format!("{:04}-{:02}", d.year(), d.month())
}

/// Shift a (year, month) pair by `delta` months.
fn add_months(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let total = year * 12 + (month as i32 - 1) + delta;
    (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32)
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

fn months_between(from: NaiveDate, to: NaiveDate) -> i64 {
    (to.year() as i64 - from.year() as i64) * 12 + (to.month() as i64 - from.month() as i64)
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

// ---------- cash flow ----------

pub fn monthly_flows(txs: &[Transaction], today: NaiveDate, months_back: i32) -> Vec<MonthFlow> {
    let mut out = Vec::with_capacity(months_back as usize);
    for i in (0..months_back).rev() {
        let (y, m) = add_months(today.year(), today.month(), -i);
        let key = format!("{:04}-{:02}", y, m);
        let mut income = 0.0;
        let mut expenses = 0.0;
        let mut n = 0usize;
        for t in txs.iter().filter(|t| t.date.starts_with(&key)) {
            if is_transfer(&t.category) {
                continue;
            }
            n += 1;
            if t.amount >= 0.0 {
                income += t.amount;
            } else {
                expenses -= t.amount;
            }
        }
        out.push(MonthFlow { month: key, income: round2(income), expenses: round2(expenses), net: round2(income - expenses), tx_count: n });
    }
    out
}

/// Average income and expenses over the most recent (up to 3) complete months
/// that contain transactions. Falls back to the current month.
fn averages(months: &[MonthFlow]) -> (f64, f64, String) {
    let complete: Vec<&MonthFlow> = months[..months.len().saturating_sub(1)]
        .iter()
        .rev()
        .filter(|m| m.tx_count > 0)
        .take(3)
        .collect();
    if complete.is_empty() {
        let cur = months.last();
        return (
            cur.map(|m| m.income).unwrap_or(0.0),
            cur.map(|m| m.expenses).unwrap_or(0.0),
            "current month only".into(),
        );
    }
    let n = complete.len() as f64;
    let inc = complete.iter().map(|m| m.income).sum::<f64>() / n;
    let exp = complete.iter().map(|m| m.expenses).sum::<f64>() / n;
    (round2(inc), round2(exp), format!("avg of last {} complete month{}", complete.len(), if complete.len() == 1 { "" } else { "s" }))
}

fn category_spend(txs: &[Transaction], budgets: &[Budget], today: NaiveDate) -> Vec<CategorySpend> {
    use std::collections::BTreeMap;
    let cur = month_key(today);
    let mut spent: BTreeMap<String, f64> = BTreeMap::new();
    let mut prev: BTreeMap<String, f64> = BTreeMap::new();
    let mut prev_keys = Vec::new();
    for i in 1..=3 {
        let (y, m) = add_months(today.year(), today.month(), -i);
        prev_keys.push(format!("{:04}-{:02}", y, m));
    }
    let mut prev_months_with_data = std::collections::BTreeSet::new();
    for t in txs {
        if t.amount >= 0.0 || is_transfer(&t.category) {
            continue;
        }
        let cat = t.category.trim().to_string();
        if t.date.starts_with(&cur) {
            *spent.entry(cat).or_insert(0.0) += -t.amount;
        } else if let Some(k) = prev_keys.iter().find(|k| t.date.starts_with(k.as_str())) {
            prev_months_with_data.insert(k.clone());
            *prev.entry(cat).or_insert(0.0) += -t.amount;
        }
    }
    let divisor = prev_months_with_data.len().max(1) as f64;
    let mut cats: std::collections::BTreeSet<String> = spent.keys().cloned().collect();
    for b in budgets {
        cats.insert(b.category.clone());
    }
    for k in prev.keys() {
        cats.insert(k.clone());
    }
    let mut out: Vec<CategorySpend> = cats
        .into_iter()
        .map(|c| CategorySpend {
            spent: round2(*spent.get(&c).unwrap_or(&0.0)),
            limit: budgets.iter().find(|b| b.category == c).map(|b| b.monthly_limit),
            avg_3mo: round2(prev.get(&c).copied().unwrap_or(0.0) / divisor),
            category: c,
        })
        .collect();
    out.sort_by(|a, b| b.spent.partial_cmp(&a.spent).unwrap_or(std::cmp::Ordering::Equal));
    out
}

// ---------- goals ----------

pub fn goal_status(g: &Goal, today: NaiveDate) -> GoalStatus {
    let remaining = (g.target_amount - g.current_amount).max(0.0);
    let progress_pct = if g.target_amount > 0.0 { (g.current_amount / g.target_amount * 100.0).min(100.0) } else { 100.0 };
    let months_left = g.target_date.as_deref().and_then(parse_date).map(|d| months_between(today, d).max(0));
    let required_monthly = months_left.map(|m| if remaining <= 0.0 { 0.0 } else { round2(remaining / (m.max(1) as f64)) });
    let months_to_reach = if remaining <= 0.0 {
        Some(0.0)
    } else if g.monthly_contribution > 0.0 {
        Some((remaining / g.monthly_contribution * 10.0).ceil() / 10.0)
    } else {
        None
    };
    let on_track = remaining <= 0.0
        || match required_monthly {
            Some(req) => g.monthly_contribution + 1e-9 >= req,
            None => g.monthly_contribution > 0.0,
        };
    GoalStatus { goal: g.clone(), progress_pct: round2(progress_pct), months_left, required_monthly, months_to_reach, on_track }
}

// ---------- bills ----------

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = add_months(y, m, 1);
    NaiveDate::from_ymd_opt(ny, nm, 1).unwrap().signed_duration_since(NaiveDate::from_ymd_opt(y, m, 1).unwrap()).num_days() as u32
}

fn clamp_day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d.clamp(1, days_in_month(y, m))).unwrap()
}

/// Months between consecutive due dates (weekly handled separately).
fn period_months(freq: &str) -> i32 {
    match freq {
        "quarterly" => 3,
        "yearly" => 12,
        _ => 1,
    }
}

/// A recurrence rule shared by bills and income sources.
pub struct Schedule<'a> {
    pub frequency: &'a str,
    pub due_day: u32,
    pub due_day2: u32,
    pub due_month: u32,
    pub anchor: Option<NaiveDate>,
}

/// The most recent occurrence on or before `today` and the first one after
/// it. None for irregular schedules.
fn schedule_window(s: &Schedule, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    match s.frequency {
        "irregular" => None,
        "weekly" => {
            // due_day 1 = Monday .. 7 = Sunday
            let want = s.due_day.clamp(1, 7) as i64 - 1;
            let cur = today.weekday().num_days_from_monday() as i64;
            let back = (cur - want).rem_euclid(7);
            let prev = today - chrono::Duration::days(back);
            Some((prev, prev + chrono::Duration::days(7)))
        }
        "biweekly" => {
            let anchor = s.anchor.unwrap_or(today);
            let diff = (today - anchor).num_days();
            let k = diff.div_euclid(14);
            let prev = anchor + chrono::Duration::days(14 * k);
            Some((prev, prev + chrono::Duration::days(14)))
        }
        "semimonthly" => {
            // Two pay days a month (e.g. 1st and 15th, or 15th and last).
            let mut dates = Vec::new();
            for delta in -1..=1 {
                let (y, m) = add_months(today.year(), today.month(), delta);
                dates.push(clamp_day(y, m, s.due_day));
                if s.due_day2 > 0 {
                    dates.push(clamp_day(y, m, s.due_day2));
                }
            }
            dates.sort();
            dates.dedup();
            let prev = *dates.iter().filter(|d| **d <= today).last()?;
            let next = *dates.iter().find(|d| **d > today)?;
            Some((prev, next))
        }
        _ => {
            let step = period_months(s.frequency);
            // Anchor on due_month for yearly/quarterly cycles, start a year back so the
            // first candidate is on or before today, then walk forward.
            let (mut y, mut m) = if step == 1 { (today.year() - 1, today.month()) } else { (today.year() - 1, s.due_month.clamp(1, 12)) };
            let mut prev = clamp_day(y, m, s.due_day);
            loop {
                let (ny, nm) = add_months(y, m, step);
                let next = clamp_day(ny, nm, s.due_day);
                if next > today {
                    return Some((prev, next));
                }
                y = ny;
                m = nm;
                prev = next;
            }
        }
    }
}

fn due_window(b: &Bill, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let s = Schedule { frequency: &b.frequency, due_day: b.due_day, due_day2: 0, due_month: b.due_month, anchor: None };
    // Bills never use irregular/semimonthly/biweekly, so a window always exists.
    schedule_window(&s, today).unwrap_or((today, today + chrono::Duration::days(30)))
}

/// Note stamped on transactions logged from an income receipt.
pub fn income_note(name: &str) -> String {
    format!("Income: {}", name.trim())
}

/// Monthly equivalent multiplier for a recurrence; None for irregular.
fn per_month(frequency: &str) -> Option<f64> {
    Some(match frequency {
        "weekly" => 52.0 / 12.0,
        "biweekly" => 26.0 / 12.0,
        "semimonthly" => 2.0,
        "monthly" => 1.0,
        "quarterly" => 1.0 / 3.0,
        "yearly" => 1.0 / 12.0,
        _ => return None,
    })
}

pub fn income_status(inc: &Income, today: NaiveDate, txs: &[Transaction]) -> IncomeStatus {
    let note = income_note(&inc.name);
    let history: Vec<BillPayment> = txs
        .iter()
        .filter(|t| t.amount > 0.0 && t.note == note)
        .take(6)
        .map(|t| BillPayment { date: t.date.clone(), amount: round2(t.amount) })
        .collect();
    let estimate = if inc.variable && !history.is_empty() {
        let n = history.len().min(3);
        round2(history.iter().take(n).map(|p| p.amount).sum::<f64>() / n as f64)
    } else {
        inc.amount
    };
    let last_amount = history.first().map(|p| p.amount);
    let monthly_equivalent = match per_month(&inc.frequency) {
        Some(k) => estimate * k,
        None => {
            // Irregular: whatever was logged in the last 90 days, per month.
            let cutoff = today - chrono::Duration::days(90);
            txs.iter().filter(|t| t.amount > 0.0 && t.note == note && parse_date(&t.date).map(|d| d > cutoff).unwrap_or(false)).map(|t| t.amount).sum::<f64>() / 3.0
        }
    };
    let anchor = inc.anchor.as_deref().and_then(parse_date).or_else(|| inc.last_received.as_deref().and_then(parse_date));
    let sched = Schedule { frequency: &inc.frequency, due_day: inc.due_day, due_day2: inc.due_day2, due_month: inc.due_month, anchor };
    let last = inc.last_received.as_deref().and_then(parse_date);
    let (next_date, days_until, received_this_cycle, late_days) = match schedule_window(&sched, today) {
        Some((prev, next)) => {
            let received = last.map(|d| d >= prev).unwrap_or(false);
            let late = if !received && last.is_some() && prev < today { (today - prev).num_days() } else { 0 };
            let due = if late > 0 || (!received && prev == today) { prev } else { next };
            (Some(due.format("%Y-%m-%d").to_string()), Some((due - today).num_days()), received, late)
        }
        None => (None, None, false, 0),
    };
    IncomeStatus {
        income: inc.clone(),
        estimate,
        last_amount,
        history,
        monthly_equivalent: round2(monthly_equivalent),
        next_date,
        days_until,
        received_this_cycle,
        late_days,
    }
}

/// Note stamped on transactions logged from a bill payment.
pub fn bill_note(name: &str) -> String {
    format!("Bill: {}", name.trim())
}

pub fn bill_status(b: &Bill, today: NaiveDate, txs: &[Transaction]) -> BillStatus {
    let note = bill_note(&b.name);
    // txs are newest-first already.
    let history: Vec<BillPayment> = txs
        .iter()
        .filter(|t| t.amount < 0.0 && t.note == note)
        .take(6)
        .map(|t| BillPayment { date: t.date.clone(), amount: round2(-t.amount) })
        .collect();
    let estimate = if b.variable && !history.is_empty() {
        let n = history.len().min(3);
        round2(history.iter().take(n).map(|p| p.amount).sum::<f64>() / n as f64)
    } else {
        b.amount
    };
    let last_amount = history.first().map(|p| p.amount);
    let (prev, next) = due_window(b, today);
    let last = b.last_paid.as_deref().and_then(parse_date);
    let paid_this_cycle = last.map(|d| d >= prev).unwrap_or(false);
    // Freshly added bills (no payment history) are shown as due next, not overdue.
    let overdue_days = if !paid_this_cycle && last.is_some() && prev < today { (today - prev).num_days() } else { 0 };
    let due = if overdue_days > 0 || (!paid_this_cycle && prev == today) { prev } else { next };
    let monthly_equivalent = match b.frequency.as_str() {
        "weekly" => estimate * 52.0 / 12.0,
        "quarterly" => estimate / 3.0,
        "yearly" => estimate / 12.0,
        _ => estimate,
    };
    BillStatus {
        bill: b.clone(),
        estimate,
        last_amount,
        history,
        monthly_equivalent: round2(monthly_equivalent),
        next_due: due.format("%Y-%m-%d").to_string(),
        days_until: (due - today).num_days(),
        paid_this_cycle,
        overdue_days,
    }
}

// ---------- debt payoff ----------

/// Simulate paying down `debts` month by month. All minimums keep being paid
/// even after a debt is cleared (the freed money rolls into the target debt),
/// plus `extra` per month. `strategy` is "avalanche" (highest APR first) or
/// "snowball" (smallest balance first).
pub fn payoff_plan(debts: &[Account], extra: f64, strategy: &str) -> PayoffPlan {
    let extra = extra.max(0.0);
    let mut lines: Vec<DebtLine> = debts
        .iter()
        .filter(|a| a.balance > 0.0)
        .map(|a| DebtLine {
            account_id: a.id,
            name: a.name.clone(),
            kind: a.kind.clone(),
            balance: a.balance,
            apr: a.apr,
            min_payment: a.min_payment,
            payoff_month: None,
            interest_paid: 0.0,
            never: false,
        })
        .collect();
    let mut bal: Vec<f64> = lines.iter().map(|l| l.balance).collect();
    let pool_base: f64 = lines.iter().map(|l| l.min_payment).sum::<f64>() + extra;
    let mut total_interest = 0.0;
    let mut total_paid = 0.0;
    let mut by_month = Vec::new();
    let mut months: Option<u32> = None;

    // Priority order is fixed for the whole run.
    let mut order: Vec<usize> = (0..lines.len()).collect();
    match strategy {
        "snowball" => order.sort_by(|&a, &b| bal[a].partial_cmp(&bal[b]).unwrap_or(std::cmp::Ordering::Equal)),
        _ => order.sort_by(|&a, &b| lines[b].apr.partial_cmp(&lines[a].apr).unwrap_or(std::cmp::Ordering::Equal)),
    }

    if !lines.is_empty() {
        for month in 1..=600u32 {
            let before: f64 = bal.iter().sum();
            // accrue
            for (i, l) in lines.iter_mut().enumerate() {
                if bal[i] > 0.0 {
                    let interest = bal[i] * l.apr / 1200.0;
                    bal[i] += interest;
                    l.interest_paid += interest;
                    total_interest += interest;
                }
            }
            // minimums
            let mut pool = pool_base;
            for (i, l) in lines.iter().enumerate() {
                if bal[i] > 0.0 {
                    let pay = l.min_payment.min(bal[i]).min(pool);
                    bal[i] -= pay;
                    pool -= pay;
                    total_paid += pay;
                }
            }
            // everything left goes to the priority debt(s)
            for &i in &order {
                if pool <= 0.0 {
                    break;
                }
                if bal[i] > 0.0 {
                    let pay = bal[i].min(pool);
                    bal[i] -= pay;
                    pool -= pay;
                    total_paid += pay;
                }
            }
            for (i, l) in lines.iter_mut().enumerate() {
                if bal[i] <= 0.005 && l.payoff_month.is_none() {
                    l.payoff_month = Some(month);
                    bal[i] = 0.0;
                }
            }
            let after: f64 = bal.iter().sum();
            by_month.push(round2(after));
            if after <= 0.0 {
                months = Some(month);
                break;
            }
            // Balances growing with nothing paid off means the payments can't
            // cover interest; stop early instead of grinding 600 months.
            if after >= before && month > 12 && lines.iter().all(|l| l.payoff_month.is_none() || l.payoff_month.unwrap() < month - 12) {
                break;
            }
        }
    } else {
        months = Some(0);
    }
    for (i, l) in lines.iter_mut().enumerate() {
        l.never = l.payoff_month.is_none() && bal[i] > 0.0;
        l.interest_paid = round2(l.interest_paid);
    }
    PayoffPlan {
        strategy: strategy.to_string(),
        extra_monthly: extra,
        months,
        total_interest: round2(total_interest),
        total_paid: round2(total_paid),
        debts: lines,
        balance_by_month: by_month,
    }
}

// ---------- retirement projection ----------

pub fn project(p: &ProjectionParams) -> Projection {
    let years = p.retirement_age.saturating_sub(p.age).max(1);
    let r = p.annual_return_pct / 100.0 / 12.0;
    let infl = p.inflation_pct / 100.0;
    let mut bal = p.starting_balance.max(0.0);
    let mut contributed = 0.0;
    let mut points = vec![ProjPoint { year: 0, age: p.age, nominal: round2(bal), real: round2(bal), contributed: 0.0 }];
    for y in 1..=years {
        for _ in 0..12 {
            bal = bal * (1.0 + r) + p.monthly_contribution;
            contributed += p.monthly_contribution;
        }
        let deflator = (1.0 + infl).powi(y as i32);
        points.push(ProjPoint { year: y, age: p.age + y, nominal: round2(bal), real: round2(bal / deflator), contributed: round2(contributed) });
    }
    let deflator = (1.0 + infl).powi(years as i32);
    let final_nominal = bal;
    let final_real = bal / deflator;
    let target_nest_egg = p.retirement_income * 25.0 * deflator;
    let n = (years * 12) as i32;
    let growth = (1.0 + r).powi(n);
    let required_monthly = if r.abs() < 1e-12 {
        Some(((target_nest_egg - p.starting_balance) / n as f64).max(0.0))
    } else {
        Some(((target_nest_egg - p.starting_balance * growth) * r / (growth - 1.0)).max(0.0))
    };
    Projection {
        params_used: ProjectionEcho {
            starting_balance: p.starting_balance,
            monthly_contribution: p.monthly_contribution,
            annual_return_pct: p.annual_return_pct,
            inflation_pct: p.inflation_pct,
            age: p.age,
            retirement_age: p.retirement_age,
            retirement_income: p.retirement_income,
        },
        points,
        final_nominal: round2(final_nominal),
        final_real: round2(final_real),
        target_nest_egg: round2(target_nest_egg),
        safe_withdrawal_real: round2(final_real * 0.04),
        // 0.1% tolerance so contributing exactly `required_monthly` (rounded) counts.
        on_track: final_nominal >= target_nest_egg * 0.999,
        required_monthly: required_monthly.map(round2),
    }
}

// ---------- snapshot + insights ----------

pub fn snapshot(accounts: &[Account], txs: &[Transaction], budgets: &[Budget], goals: &[Goal], bills: &[Bill], incomes: &[Income], settings: &Settings, today: NaiveDate) -> Snapshot {
    let assets: f64 = accounts.iter().filter(|a| !is_liability(&a.kind)).map(|a| a.balance).sum();
    let liabilities: f64 = accounts.iter().filter(|a| is_liability(&a.kind)).map(|a| a.balance).sum();
    let liquid: f64 = accounts.iter().filter(|a| LIQUID_KINDS.contains(&a.kind.as_str())).map(|a| a.balance).sum();
    let invested: f64 = accounts.iter().filter(|a| INVESTED_KINDS.contains(&a.kind.as_str())).map(|a| a.balance).sum();

    let months = monthly_flows(txs, today, 12);
    let (avg_income, avg_expenses, avg_basis) = averages(&months);
    let savings_rate = if avg_income > 0.0 { Some(round2((avg_income - avg_expenses) / avg_income * 100.0)) } else { None };
    let emergency_months = if avg_expenses > 0.0 { Some(round2(liquid / avg_expenses)) } else { None };
    let debt_min: f64 = accounts.iter().filter(|a| is_liability(&a.kind)).map(|a| a.min_payment).sum();
    let dti = if avg_income > 0.0 { Some(round2(debt_min / avg_income * 100.0)) } else { None };

    let categories = category_spend(txs, budgets, today);
    let goal_statuses: Vec<GoalStatus> = goals.iter().map(|g| goal_status(g, today)).collect();
    let mut bill_statuses: Vec<BillStatus> = bills.iter().map(|b| bill_status(b, today, txs)).collect();
    bill_statuses.sort_by_key(|b| (std::cmp::Reverse(b.overdue_days), b.days_until));
    let bills_monthly = round2(bill_statuses.iter().map(|b| b.monthly_equivalent).sum());
    let mut income_statuses: Vec<IncomeStatus> = incomes.iter().map(|i| income_status(i, today, txs)).collect();
    income_statuses.sort_by_key(|i| (std::cmp::Reverse(i.late_days), i.days_until.unwrap_or(i64::MAX)));
    let income_monthly = round2(income_statuses.iter().map(|i| i.monthly_equivalent).sum());
    let income_this_month = months.last().map(|m| m.income).unwrap_or(0.0);
    // Next occurrence within two weeks, whether or not the previous one was paid.
    let bills_due_14d = round2(bill_statuses.iter().filter(|b| b.days_until <= 14).map(|b| b.estimate).sum());

    let plan_debts: Vec<Account> = accounts.iter().filter(|a| is_liability(&a.kind) && a.kind != "mortgage").cloned().collect();
    let payoff = payoff_plan(&plan_debts, 0.0, "avalanche");

    let (contrib, contrib_source) = if settings.monthly_investment > 0.0 {
        (settings.monthly_investment, "settings".to_string())
    } else {
        ((avg_income - avg_expenses).max(0.0), "average monthly surplus".to_string())
    };
    let projection = project(&ProjectionParams {
        starting_balance: invested,
        monthly_contribution: contrib,
        annual_return_pct: settings.annual_return_pct,
        inflation_pct: settings.inflation_pct,
        age: settings.age,
        retirement_age: settings.retirement_age,
        retirement_income: settings.retirement_income,
    });

    let mut snap = Snapshot {
        today: today.format("%Y-%m-%d").to_string(),
        currency: settings.currency.clone(),
        accounts: accounts.to_vec(),
        net_worth: round2(assets - liabilities),
        assets: round2(assets),
        liabilities: round2(liabilities),
        liquid: round2(liquid),
        invested: round2(invested),
        holdings: Default::default(),
        months,
        avg_income,
        avg_expenses,
        avg_basis,
        savings_rate,
        emergency_months,
        dti,
        categories,
        goals: goal_statuses,
        bills: bill_statuses,
        incomes: income_statuses,
        income_monthly,
        income_this_month,
        bills_monthly,
        bills_due_14d,
        payoff,
        projection,
        projection_contribution_source: contrib_source,
        insights: Vec::new(),
        tx_count: txs.len(),
        has_data: !accounts.is_empty() || !txs.is_empty() || !bills.is_empty() || !incomes.is_empty(),
    };
    snap.insights = insights(&snap, settings);
    snap
}

fn is_housing(category: &str, name: &str) -> bool {
    let c = category.trim().to_ascii_lowercase();
    let n = name.trim().to_ascii_lowercase();
    c == "rent" || c == "mortgage" || c == "housing" || n.contains("rent") || n.contains("mortgage")
}

fn money(x: f64) -> String {
    let neg = x < 0.0;
    let x = x.abs();
    let whole = x.round() as i64;
    let s = whole.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{}${}", if neg { "-" } else { "" }, out)
}

fn insights(s: &Snapshot, settings: &Settings) -> Vec<Insight> {
    let mut out: Vec<Insight> = Vec::new();
    let push = |out: &mut Vec<Insight>, sev: &str, title: &str, detail: String, action: &str| {
        out.push(Insight { severity: sev.into(), title: title.into(), detail, action: action.into() });
    };

    if !s.has_data {
        push(&mut out, "info", "Start by adding your accounts", "FinCompass needs balances and a few months of transactions before it can advise you. Load the sample data from Settings to see what a filled-in picture looks like.".into(), "Add accounts, then log income and expenses.");
        return out;
    }

    // 0. Housing and tax arrears outrank everything: they risk the roof, not just interest.
    for a in s.accounts.iter().filter(|a| a.balance > 0.0) {
        match a.kind.as_str() {
            "rent_owed" => {
                let plan = if a.min_payment > 0.0 {
                    format!(" At {}/mo it clears in about {} months.", money(a.min_payment), ((a.balance / a.min_payment).ceil() as i64).max(1))
                } else {
                    " No monthly payment is set, so nothing is chipping away at it.".to_string()
                };
                push(&mut out, "critical", &format!("Back rent: {} owed on {}", money(a.balance), a.name),
                    format!("Unpaid rent is the debt most likely to cost you your home, and it usually carries late fees.{plan}"),
                    "Agree a written repayment plan with the landlord, then enter that monthly amount on the account so it's tracked. Pay this before any credit card extra.");
            }
            "taxes_owed" => {
                push(&mut out, "critical", &format!("Taxes owed: {}", money(a.balance)),
                    "Tax debt accrues penalties plus interest and the collector has powers a bank doesn't.".to_string(),
                    "Set up an official payment plan (they're routinely granted) and enter the monthly amount on the account.");
            }
            _ => {}
        }
    }

    // 1. Debts whose minimum payment doesn't cover interest, or that have no payment plan at all.
    for d in &s.payoff.debts {
        if d.balance <= 0.0 || d.kind == "rent_owed" || d.kind == "taxes_owed" {
            continue;
        }
        let monthly_interest = d.balance * d.apr / 1200.0;
        if d.apr > 0.0 && d.min_payment <= monthly_interest {
            if d.min_payment <= 0.0 {
                push(&mut out, "warn", &format!("{} has no monthly payment set", d.name),
                    format!("It accrues about {}/mo at {:.1}% APR. Without a payment amount FinCompass can't plan its payoff.", money(monthly_interest), d.apr),
                    "Edit the account and enter the minimum monthly payment.");
            } else {
                push(&mut out, "critical", &format!("{} is not being paid down", d.name),
                    format!("The minimum payment ({}/mo) does not cover the interest ({}/mo at {:.1}% APR), so the balance grows every month.", money(d.min_payment), money(monthly_interest), d.apr),
                    "Raise the monthly payment above the interest charge.");
            }
        } else if d.apr <= 0.0 && d.min_payment <= 0.0 {
            push(&mut out, "info", &format!("{} has no payment plan", d.name),
                format!("{} owed at 0% interest with no monthly payment entered, so it stays flat and is left out of the payoff schedule.", money(d.balance)),
                "Edit the account and add the APR (if any) and what you pay each month.");
        }
    }

    // 2. Spending more than earning.
    if s.avg_income > 0.0 {
        let rate = s.savings_rate.unwrap_or(0.0);
        if rate < 0.0 {
            push(&mut out, "critical", "Spending exceeds income", format!("Average expenses ({}/mo) are higher than average income ({}/mo). The gap is {} per month.", money(s.avg_expenses), money(s.avg_income), money(s.avg_expenses - s.avg_income)), "Cut the largest discretionary categories first; look at Budget for where the money goes.");
        } else if rate < 10.0 {
            push(&mut out, "warn", "Low savings rate", format!("You keep {:.0}% of income. Most guidance targets at least 20%.", rate), "Automate a transfer to savings on payday so saving happens before spending.");
        } else if rate < 20.0 {
            push(&mut out, "info", "Savings rate is decent, not yet strong", format!("You keep {:.0}% of income. Pushing this to 20% would add {} a month to savings.", rate, money(s.avg_income * 0.2 - (s.avg_income - s.avg_expenses))), "Direct future raises to savings rather than lifestyle.");
        } else {
            push(&mut out, "good", "Strong savings rate", format!("You keep {:.0}% of income ({}/mo).", rate, money(s.avg_income - s.avg_expenses)), "Keep it up and make sure the surplus is invested, not idle.");
        }
    }

    // 3. High-APR debt.
    let high: Vec<&DebtLine> = s.payoff.debts.iter().filter(|d| d.apr >= 15.0).collect();
    if !high.is_empty() {
        let total: f64 = high.iter().map(|d| d.balance).sum();
        let names: Vec<String> = high.iter().map(|d| format!("{} ({:.1}%)", d.name, d.apr)).collect();
        push(&mut out, if total > s.liquid { "critical" } else { "warn" }, "High-interest debt outranks investing",
            format!("{} across {}. Paying this down is a guaranteed return higher than markets typically deliver.", money(total), names.join(", ")),
            "Beyond any employer match, throw every spare dollar at the highest-APR balance (avalanche). See Debts for the payoff schedule.");
    }

    // 4. Emergency fund.
    match s.emergency_months {
        Some(m) => {
            let target = settings.emergency_months;
            if m < 1.0 {
                push(&mut out, "critical", "No emergency cushion", format!("Liquid cash ({}) covers {:.1} months of expenses.", money(s.liquid), m), "Build at least one month of expenses in savings before extra debt payments or investing.");
            } else if m < 3.0 {
                push(&mut out, "warn", "Thin emergency fund", format!("Liquid cash covers {:.1} months of expenses; target is {:.0} months ({}).", m, target, money(s.avg_expenses * target)), "Add to savings until you reach 3 months, then balance with debt payoff.");
            } else if m < target {
                push(&mut out, "info", "Emergency fund below target", format!("{:.1} months covered, target {:.0} months. Gap: {}.", m, target, money((target - m) * s.avg_expenses)), "Keep topping up; a high-yield savings account keeps it liquid.");
            } else if m > target * 2.0 && high.is_empty() {
                push(&mut out, "info", "Excess idle cash", format!("Liquid cash covers {:.1} months, well beyond the {:.0}-month target. About {} is sitting idle.", m, target, money((m - target) * s.avg_expenses)), "Move the excess into investments or a higher-yield account.");
            } else {
                push(&mut out, "good", "Emergency fund funded", format!("{:.1} months of expenses in liquid cash.", m), "Nothing to do here.");
            }
        }
        None => push(&mut out, "info", "Can't size your emergency fund yet", "Log a few months of expenses so FinCompass can compare them to your liquid cash.".into(), "Add transactions or import a CSV."),
    }

    // 4b. Bills: overdue, cash to cover the next two weeks, fixed-cost load.
    let overdue: Vec<&BillStatus> = s.bills.iter().filter(|b| b.overdue_days > 0).collect();
    if !overdue.is_empty() {
        let parts: Vec<String> = overdue.iter().map(|b| format!("{} ({}, {} days late)", b.bill.name, money(b.estimate), b.overdue_days)).collect();
        push(&mut out, "critical", "Overdue bills", parts.join("; "), "Pay them, or mark them paid in Bills if you already did.");
    }
    if s.bills_due_14d > 0.0 && s.liquid < s.bills_due_14d {
        let mut soon: Vec<&BillStatus> = s.bills.iter().filter(|b| b.days_until <= 14).collect();
        soon.sort_by_key(|b| b.days_until);
        let names: Vec<String> = soon.iter().take(3).map(|b| format!("{} {} in {}d", b.bill.name, money(b.estimate), b.days_until.max(0))).collect();
        let has_housing = soon.iter().any(|b| is_housing(&b.bill.category, &b.bill.name));
        let incoming: Vec<String> = s.incomes.iter().filter(|i| i.days_until.map(|d| d <= 14).unwrap_or(false) && !i.received_this_cycle).map(|i| format!("{} {} in {}d", i.income.name, money(i.estimate), i.days_until.unwrap_or(0))).collect();
        let mut detail = format!("{} of bills fall due in the next 14 days ({}) but liquid cash is {}.", money(s.bills_due_14d), names.join(", "), money(s.liquid));
        if !incoming.is_empty() {
            detail.push_str(&format!(" Expected in: {}.", incoming.join(", ")));
        }
        push(&mut out, "critical", if has_housing { "Rent is due and the cash isn't there" } else { "Not enough cash for upcoming bills" }, detail,
            if has_housing { "Cover rent first: it's the one bill with eviction behind it. Line up the paycheck timing, or ask the landlord for a dated extension in writing before the due date." } else { "Move money into checking before the due dates, or talk to the biller about timing." });
    }

    // 4d. Housing cost as a share of income.
    let housing_monthly: f64 = s.bills.iter().filter(|b| is_housing(&b.bill.category, &b.bill.name)).map(|b| b.monthly_equivalent).sum::<f64>()
        + s.accounts.iter().filter(|a| a.kind == "mortgage").map(|a| a.min_payment).sum::<f64>();
    let income_basis_h = if s.avg_income > 0.0 { s.avg_income } else { s.income_monthly };
    if housing_monthly > 0.0 && income_basis_h > 0.0 {
        let ratio = housing_monthly / income_basis_h * 100.0;
        if ratio > 50.0 {
            push(&mut out, "critical", "Housing takes over half your income", format!("Rent/mortgage is {}/mo, {:.0}% of {}/mo income. The usual ceiling is 30%.", money(housing_monthly), ratio, money(income_basis_h)), "This is the number that decides everything else: a roommate, a cheaper place at lease end, or more income. Everything else is small next to it.");
        } else if ratio > 30.0 {
            push(&mut out, "warn", "Housing is above the 30% guideline", format!("Rent/mortgage is {}/mo, {:.0}% of income.", money(housing_monthly), ratio), "Manageable, but it squeezes saving. Keep other fixed costs lean and revisit at lease renewal.");
        } else {
            push(&mut out, "good", "Housing cost is in range", format!("Rent/mortgage is {}/mo, {:.0}% of income.", money(housing_monthly), ratio), "Nothing to do here.");
        }
    } else if housing_monthly > 0.0 {
        push(&mut out, "info", "Add income to judge your rent", format!("Rent/mortgage is {}/mo. Once income is set up, FinCompass can tell you whether it's sustainable.", money(housing_monthly)), "Add your paycheck on the Income tab.");
    }
    // 4c. Income sources.
    if s.income_monthly > 0.0 && s.bills_monthly > s.income_monthly {
        push(&mut out, "critical", "Bills exceed expected income", format!("Recurring bills are {}/mo against {}/mo of expected income.", money(s.bills_monthly), money(s.income_monthly)), "Something has to give: cut or renegotiate a bill, or add income. Use the Advisor to work through options.");
    }
    for i in s.incomes.iter().filter(|i| i.late_days > 0) {
        push(&mut out, "info", &format!("{} hasn't been logged", i.income.name), format!("Expected around {} ago ({} typical). If it arrived, log it so cash flow stays accurate.", if i.late_days == 1 { "1 day".to_string() } else { format!("{} days", i.late_days) }, money(i.estimate)), "Open Income and click Log received, or chase the payer if it's genuinely late.");
    }

    let income_basis = if s.avg_income > 0.0 { s.avg_income } else { s.income_monthly };
    if s.bills_monthly > 0.0 && income_basis > 0.0 {
        let ratio = s.bills_monthly / income_basis * 100.0;
        if ratio > 60.0 {
            push(&mut out, "warn", "Fixed bills eat most of your income", format!("Recurring bills come to {}/mo, {:.0}% of {} income. Little is left for saving or surprises.", money(s.bills_monthly), ratio, if s.avg_income > 0.0 { "average" } else { "expected" }), "Look for a bill to cut or renegotiate; subscriptions and insurance are the usual candidates.");
        } else {
            push(&mut out, "info", "Fixed-cost load", format!("Recurring bills come to {}/mo, {:.0}% of {} income.", money(s.bills_monthly), ratio, if s.avg_income > 0.0 { "average" } else { "expected" }), "Under 50% leaves room to save; keep new commitments in check.");
        }
    }

    // 5. Budget overspend (current month).
    let over: Vec<&CategorySpend> = s.categories.iter().filter(|c| c.limit.map(|l| c.spent > l).unwrap_or(false)).collect();
    if !over.is_empty() {
        let parts: Vec<String> = over.iter().map(|c| format!("{} {} over", c.category, money(c.spent - c.limit.unwrap()))).collect();
        push(&mut out, "warn", "Over budget this month", parts.join("; "), "Pause spending in those categories for the rest of the month, or revisit the limits if they were unrealistic.");
    }

    // 6. Debt-to-income.
    if let Some(dti) = s.dti {
        if dti > 43.0 {
            push(&mut out, "critical", "Debt payments dominate income", format!("Minimum debt payments take {:.0}% of income (lenders flag anything above 43%).", dti), "Avoid new credit; consider consolidating or refinancing the highest-rate balances.");
        } else if dti > 36.0 {
            push(&mut out, "warn", "Debt-to-income is high", format!("Minimum payments take {:.0}% of income; under 36% is the usual comfort zone.", dti), "Prioritise paying off the smallest or highest-rate loan to free up cash flow.");
        }
    }

    // 7. Goals off track.
    for g in &s.goals {
        if !g.on_track {
            let detail = match (g.required_monthly, g.months_left) {
                (Some(req), Some(m)) => format!("Needs {}/mo for {} months; currently contributing {}/mo.", money(req), m, money(g.goal.monthly_contribution)),
                _ => "No monthly contribution set, so it will never be reached.".to_string(),
            };
            push(&mut out, "warn", &format!("Goal at risk: {}", g.goal.name), detail, "Raise the contribution, push the date, or lower the target.");
        }
    }

    // 8. Retirement.
    let p = &s.projection;
    // Dust (a few cents in a wallet) shouldn't trigger a retirement verdict.
    if s.invested >= 100.0 || p.params_used.monthly_contribution > 0.0 {
        if p.on_track {
            push(&mut out, "good", "Retirement projection on track", format!("At {:.0}% growth you reach about {} (today's dollars) by age {}, above the {} needed for {}/yr.", p.params_used.annual_return_pct, money(p.final_real), p.params_used.retirement_age, money(p.target_nest_egg / (1.0 + p.params_used.inflation_pct / 100.0).powi((p.params_used.retirement_age - p.params_used.age) as i32)), money(p.params_used.retirement_income)), "Revisit the assumptions once a year.");
        } else {
            push(&mut out, "warn", "Retirement projection falls short", format!("Projected {} (today's dollars) at {} vs. a target of {} in retirement-year dollars. Closing the gap takes about {}/mo instead of {}/mo.", money(p.final_real), p.params_used.retirement_age, money(p.target_nest_egg), money(p.required_monthly.unwrap_or(0.0)), money(p.params_used.monthly_contribution)), "Increase retirement contributions, or adjust the retirement age/income target in Settings.");
        }
    } else {
        push(&mut out, "info", "No retirement savings recorded", "Add a retirement or brokerage account, and set a monthly investment in Settings, to see a projection.".into(), "Even small automatic contributions compound meaningfully over decades.");
    }

    // 9. Credit card carrying a balance.
    for a in s.accounts.iter().filter(|a| a.kind == "credit_card" && a.balance > 0.0 && a.apr > 0.0) {
        let yearly = a.balance * a.apr / 100.0;
        push(&mut out, "info", &format!("{} costs about {} a year", a.name, money(yearly)), format!("Carrying {} at {:.1}% APR.", money(a.balance), a.apr), "If cash flow allows, pay it in full each statement to stop the interest.");
    }

    // 10. Net worth.
    if s.net_worth < 0.0 {
        push(&mut out, "info", "Negative net worth", format!("Liabilities ({}) exceed assets ({}). This is common early on and the payoff plan shows the way out.", money(s.liabilities), money(s.assets)), "Track it monthly; the trend matters more than the level.");
    }

    // 11. Budget coverage.
    let uncapped: Vec<&CategorySpend> = s.categories.iter().filter(|c| c.limit.is_none() && c.spent > 0.0).collect();
    if s.categories.iter().any(|c| c.limit.is_some()) && uncapped.len() >= 3 {
        let names: Vec<&str> = uncapped.iter().take(4).map(|c| c.category.as_str()).collect();
        push(&mut out, "info", "Some spending has no budget", format!("{} have spending this month but no limit.", names.join(", ")), "Set limits in Budget so overspend is caught early.");
    }

    let rank = |sev: &str| match sev { "critical" => 0, "warn" => 1, "info" => 2, _ => 3 };
    out.sort_by_key(|i| rank(&i.severity));
    out
}

// ---------- demo data ----------

pub struct Lcg(pub u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / (u32::MAX as f64 / 2.0)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next().min(1.0)
    }
}

/// Sample transactions for the 6 months up to `today`, for a checking account
/// `checking_id` and credit card `card_id`.
pub fn sample_transactions(today: NaiveDate, checking_id: i64, card_id: i64) -> Vec<crate::db::TransactionInput> {
    let mut rng = Lcg(42);
    let mut out = Vec::new();
    let mk = |acc: i64, date: NaiveDate, amount: f64, cat: &str, note: &str| crate::db::TransactionInput {
        id: None,
        account_id: Some(acc),
        date: date.format("%Y-%m-%d").to_string(),
        amount: round2(amount),
        category: cat.into(),
        note: note.into(),
    };
    for i in (0..6).rev() {
        let (y, m) = add_months(today.year(), today.month(), -i);
        let day = |d: u32| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let last_day = if i == 0 { today.day() } else { 28 };
        out.push(mk(checking_id, day(1), 2650.0, "Salary", "Paycheck"));
        if last_day >= 15 {
            out.push(mk(checking_id, day(15), 2650.0, "Salary", "Paycheck"));
        }
        out.push(mk(checking_id, day(1), -1650.0, "Rent", "Apartment"));
        if last_day >= 5 {
            out.push(mk(checking_id, day(5), -rng.range(140.0, 210.0), "Utilities", "Electric + internet"));
            out.push(mk(checking_id, day(5), -128.0, "Insurance", "Auto insurance"));
            out.push(mk(checking_id, day(5), -310.0, "Debt payment", "Car loan"));
            out.push(mk(checking_id, day(5), -210.0, "Debt payment", "Student loan"));
        }
        if last_day >= 10 {
            out.push(mk(card_id, day(10), -47.97, "Subscriptions", "Streaming + music"));
        }
        for d in [3u32, 9, 16, 23] {
            if d <= last_day {
                out.push(mk(card_id, day(d), -rng.range(90.0, 170.0), "Groceries", "Supermarket"));
            }
        }
        for d in [4u32, 12, 19, 26] {
            if d <= last_day {
                out.push(mk(card_id, day(d), -rng.range(18.0, 65.0), "Dining", "Restaurants"));
            }
        }
        for d in [7u32, 21] {
            if d <= last_day {
                out.push(mk(card_id, day(d), -rng.range(40.0, 75.0), "Transport", "Fuel"));
            }
        }
        if last_day >= 14 {
            out.push(mk(card_id, day(14), -rng.range(30.0, 160.0), "Shopping", "Misc"));
        }
        if last_day >= 20 {
            out.push(mk(card_id, day(20), -rng.range(20.0, 90.0), "Entertainment", "Weekend"));
        }
        if i == 3 {
            out.push(mk(checking_id, day(20), 1200.0, "Bonus", "Quarterly bonus"));
            out.push(mk(checking_id, day(22), -640.0, "Health", "Dental"));
        }
        if last_day >= 25 {
            out.push(mk(checking_id, day(25), -400.0, "Transfer", "To savings"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acct(id: i64, name: &str, kind: &str, balance: f64, apr: f64, min_payment: f64) -> Account {
        Account { id, name: name.into(), kind: kind.into(), balance, apr, min_payment, chain: String::new(), address: String::new(), units: 0.0, last_synced: None }
    }

    #[test]
    fn add_months_wraps_years() {
        assert_eq!(add_months(2026, 1, -1), (2025, 12));
        assert_eq!(add_months(2026, 11, 3), (2027, 2));
        assert_eq!(add_months(2026, 9, -12), (2025, 9));
    }

    #[test]
    fn avalanche_targets_highest_apr_and_beats_snowball_on_interest() {
        let debts = vec![
            acct(1, "card", "credit_card", 3000.0, 24.0, 90.0),
            acct(2, "car", "loan", 10000.0, 6.0, 300.0),
        ];
        let av = payoff_plan(&debts, 200.0, "avalanche");
        let sn = payoff_plan(&debts, 200.0, "snowball");
        let card_av = av.debts.iter().find(|d| d.name == "card").unwrap();
        let card_sn = sn.debts.iter().find(|d| d.name == "card").unwrap();
        // Both clear the card first here (it's both smallest and highest APR).
        assert!(card_av.payoff_month.unwrap() <= card_sn.payoff_month.unwrap());
        assert!(av.total_interest <= sn.total_interest + 0.01);
        assert!(av.months.is_some() && av.months.unwrap() < 60);
        assert!(av.balance_by_month.last().copied().unwrap_or(1.0) <= 0.0);
    }

    #[test]
    fn extra_payment_shortens_payoff() {
        let debts = vec![acct(1, "loan", "loan", 5000.0, 10.0, 150.0)];
        let base = payoff_plan(&debts, 0.0, "avalanche");
        let more = payoff_plan(&debts, 100.0, "avalanche");
        assert!(more.months.unwrap() < base.months.unwrap());
        assert!(more.total_interest < base.total_interest);
    }

    #[test]
    fn underwater_minimum_is_flagged_never() {
        let debts = vec![acct(1, "card", "credit_card", 5000.0, 30.0, 50.0)]; // interest 125/mo > 50
        let p = payoff_plan(&debts, 0.0, "avalanche");
        assert!(p.months.is_none());
        assert!(p.debts[0].never);
    }

    #[test]
    fn projection_required_contribution_hits_target() {
        let p = ProjectionParams {
            starting_balance: 20_000.0,
            monthly_contribution: 0.0,
            annual_return_pct: 6.0,
            inflation_pct: 2.0,
            age: 30,
            retirement_age: 60,
            retirement_income: 50_000.0,
        };
        let r = project(&p);
        assert!(!r.on_track);
        let req = r.required_monthly.unwrap();
        let r2 = project(&ProjectionParams { monthly_contribution: req, ..p });
        assert!((r2.final_nominal - r2.target_nest_egg).abs() / r2.target_nest_egg < 0.001);
        assert!(r2.on_track);
        assert_eq!(r.points.len(), 31);
    }

    #[test]
    fn averages_skip_current_month_when_history_exists() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let tx = |date: &str, amount: f64, cat: &str| Transaction { id: 0, account_id: None, date: date.into(), amount, category: cat.into(), note: String::new() };
        let txs = vec![
            tx("2026-08-01", 3000.0, "Salary"),
            tx("2026-08-10", -1000.0, "Rent"),
            tx("2026-07-01", 3000.0, "Salary"),
            tx("2026-07-10", -2000.0, "Rent"),
            tx("2026-07-15", -500.0, "Transfer"),
            tx("2026-09-01", 3000.0, "Salary"),
        ];
        let months = monthly_flows(&txs, today, 12);
        let (inc, exp, basis) = averages(&months);
        assert_eq!(inc, 3000.0);
        assert_eq!(exp, 1500.0);
        assert!(basis.contains("2 complete"));
    }

    #[test]
    fn goal_math() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let g = Goal { id: 1, name: "trip".into(), target_amount: 4000.0, current_amount: 800.0, target_date: Some("2027-06-01".into()), monthly_contribution: 200.0 };
        let s = goal_status(&g, today);
        assert_eq!(s.months_left, Some(9));
        assert_eq!(s.required_monthly, Some(355.56));
        assert!(!s.on_track);
        assert_eq!(s.months_to_reach, Some(16.0));
    }

    #[test]
    fn zero_apr_debt_without_payment_is_not_critical() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let accounts = vec![acct(1, "Paypal", "other_liability", 822.0, 0.0, 0.0)];
        let s = snapshot(&accounts, &[], &[], &[], &[], &[], &Settings::default(), today);
        assert!(!s.insights.iter().any(|i| i.severity == "critical"));
        assert!(s.insights.iter().any(|i| i.title.contains("no payment plan")));
        let hot = vec![acct(2, "Card", "credit_card", 1000.0, 24.0, 0.0)];
        let s2 = snapshot(&hot, &[], &[], &[], &[], &[], &Settings::default(), today);
        assert!(s2.insights.iter().any(|i| i.title.contains("no monthly payment set") && i.severity == "warn"));
    }

    #[test]
    fn bill_due_dates_and_status() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let mk = |freq: &str, day: u32, month: u32, last: Option<&str>| Bill {
            id: 1, name: "Rent".into(), amount: 1500.0, category: "Rent".into(), frequency: freq.into(), due_day: day, due_month: month,
            account_id: None, autopay: false, variable: false, last_paid: last.map(String::from), note: String::new(),
        };
        // Monthly on the 1st, paid this month: next is Oct 1.
        let s = bill_status(&mk("monthly", 1, 1, Some("2026-09-01")), today, &[]);
        assert!(s.paid_this_cycle);
        assert_eq!(s.next_due, "2026-10-01");
        assert_eq!(s.days_until, 9);
        // Monthly on the 1st, last paid in August: overdue 21 days.
        let s = bill_status(&mk("monthly", 1, 1, Some("2026-08-01")), today, &[]);
        assert_eq!(s.overdue_days, 21);
        assert_eq!(s.next_due, "2026-09-01");
        // No history: not overdue, due on the next occurrence.
        let s = bill_status(&mk("monthly", 25, 1, None), today, &[]);
        assert_eq!(s.overdue_days, 0);
        assert_eq!(s.next_due, "2026-09-25");
        // Day 31 clamps to month length.
        let s = bill_status(&mk("monthly", 31, 1, Some("2026-08-31")), today, &[]);
        assert_eq!(s.next_due, "2026-09-30");
        // Yearly in March: next is March 2027.
        let s = bill_status(&mk("yearly", 15, 3, Some("2026-03-15")), today, &[]);
        assert_eq!(s.next_due, "2027-03-15");
        assert_eq!(s.monthly_equivalent, 125.0);
        // Quarterly from January: Jan/Apr/Jul/Oct -> next Oct 1.
        let s = bill_status(&mk("quarterly", 1, 1, Some("2026-07-01")), today, &[]);
        assert_eq!(s.next_due, "2026-10-01");
        // Weekly on Friday: 2026-09-22 is a Tuesday, so next Friday is the 25th.
        let s = bill_status(&mk("weekly", 5, 1, None), today, &[]);
        assert_eq!(s.next_due, "2026-09-25");
    }

    #[test]
    fn variable_bill_estimates_from_recent_payments() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let bill = Bill {
            id: 7, name: "Electric".into(), amount: 150.0, category: "Utilities".into(), frequency: "monthly".into(), due_day: 5, due_month: 1,
            account_id: None, autopay: true, variable: true, last_paid: Some("2026-09-05".into()), note: String::new(),
        };
        let tx = |date: &str, amount: f64, note: &str| Transaction { id: 0, account_id: None, date: date.into(), amount, category: "Utilities".into(), note: note.into() };
        let txs = vec![
            tx("2026-09-05", -210.0, "Bill: Electric"),
            tx("2026-08-05", -180.0, "Bill: Electric"),
            tx("2026-07-05", -120.0, "Bill: Electric"),
            tx("2026-06-05", -90.0, "Bill: Electric"),
            tx("2026-09-06", -40.0, "Bill: Water"),
        ];
        let s = bill_status(&bill, today, &txs);
        assert_eq!(s.estimate, 170.0); // mean of 210, 180, 120
        assert_eq!(s.last_amount, Some(210.0));
        assert_eq!(s.history.len(), 4);
        assert_eq!(s.monthly_equivalent, 170.0);
        // A fixed bill ignores history for its estimate.
        let fixed = Bill { variable: false, ..bill.clone() };
        assert_eq!(bill_status(&fixed, today, &txs).estimate, 150.0);
    }

    #[test]
    fn income_schedules() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap(); // Tuesday
        let mk = |freq: &str, d1: u32, d2: u32, anchor: Option<&str>, last: Option<&str>, variable: bool| Income {
            id: 1, name: "Pay".into(), amount: 2000.0, category: "Salary".into(), frequency: freq.into(), due_day: d1, due_day2: d2, due_month: 1,
            anchor: anchor.map(String::from), account_id: None, variable, last_received: last.map(String::from), note: String::new(),
        };
        // Semimonthly 1st & 15th, paid on the 15th: next is Oct 1.
        let s = income_status(&mk("semimonthly", 1, 15, None, Some("2026-09-15"), false), today, &[]);
        assert!(s.received_this_cycle);
        assert_eq!(s.next_date.as_deref(), Some("2026-10-01"));
        assert_eq!(s.monthly_equivalent, 4000.0);
        // Biweekly anchored on Friday Sep 11: Sep 25 is next, 4000/mo-ish (26/12).
        let s = income_status(&mk("biweekly", 1, 0, Some("2026-09-11"), Some("2026-09-11"), false), today, &[]);
        assert_eq!(s.next_date.as_deref(), Some("2026-09-25"));
        assert_eq!(s.days_until, Some(3));
        assert_eq!(s.monthly_equivalent, round2(2000.0 * 26.0 / 12.0));
        // Biweekly, last pay missed: Sep 11 expected, still not logged -> 11 days late.
        let s = income_status(&mk("biweekly", 1, 0, Some("2026-08-28"), Some("2026-08-28"), false), today, &[]);
        assert_eq!(s.late_days, 11);
        assert_eq!(s.next_date.as_deref(), Some("2026-09-11"));
        // Irregular + variable: estimate from logged receipts, monthly from last 90 days.
        let tx = |date: &str, amount: f64| Transaction { id: 0, account_id: None, date: date.into(), amount, category: "Freelance".into(), note: "Income: Pay".into() };
        let txs = vec![tx("2026-09-10", 900.0), tx("2026-08-02", 600.0), tx("2026-07-20", 300.0), tx("2026-05-01", 5000.0)];
        let s = income_status(&mk("irregular", 1, 0, None, None, true), today, &txs);
        assert_eq!(s.estimate, 600.0);
        assert_eq!(s.monthly_equivalent, 600.0); // (900+600+300)/3, May is outside 90 days
        assert!(s.next_date.is_none());
        assert_eq!(s.late_days, 0);
    }

    #[test]
    fn back_rent_is_the_first_insight() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let accounts = vec![
            acct(1, "Checking", "checking", -38.0, 0.0, 0.0),
            acct(2, "Apple Card", "credit_card", 1494.0, 25.5, 252.0),
            acct(3, "Rent", "rent_owed", 4611.0, 0.0, 0.0),
        ];
        let rent = Bill {
            id: 1, name: "Rent".into(), amount: 919.0, category: "Rent".into(), frequency: "monthly".into(), due_day: 2, due_month: 1,
            account_id: Some(1), autopay: false, variable: false, last_paid: Some("2026-09-02".into()), note: String::new(),
        };
        let s = snapshot(&accounts, &[], &[], &[], &[rent], &[], &Settings::default(), today);
        assert!(s.insights[0].title.starts_with("Back rent"), "first insight was {}", s.insights[0].title);
        assert_eq!(s.insights[0].severity, "critical");
        assert!(s.insights.iter().any(|i| i.title == "Rent is due and the cash isn't there"));
        assert!(!s.insights.iter().any(|i| i.title == "Rent has no payment plan"));
        assert!(s.insights.iter().any(|i| i.title == "Add income to judge your rent"));
    }

    #[test]
    fn sample_data_produces_sane_snapshot() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let accounts = vec![
            acct(1, "chk", "checking", 4000.0, 0.0, 0.0),
            acct(2, "sav", "savings", 9500.0, 4.0, 0.0),
            acct(3, "401k", "retirement", 38000.0, 0.0, 0.0),
            acct(4, "visa", "credit_card", 3400.0, 24.9, 95.0),
        ];
        let txs: Vec<Transaction> = sample_transactions(today, 1, 4)
            .into_iter()
            .enumerate()
            .map(|(i, t)| Transaction { id: i as i64, account_id: t.account_id, date: t.date, amount: t.amount, category: t.category, note: t.note })
            .collect();
        let s = snapshot(&accounts, &txs, &[], &[], &[], &[], &Settings::default(), today);
        assert_eq!(s.net_worth, 4000.0 + 9500.0 + 38000.0 - 3400.0);
        assert!(s.avg_income > 5000.0 && s.avg_income < 6000.0);
        assert!(s.avg_expenses > 3000.0 && s.avg_expenses < 4500.0);
        assert!(s.savings_rate.unwrap() > 0.0);
        assert!(s.insights.iter().any(|i| i.title.contains("High-interest")));
        assert!(s.insights.iter().all(|i| !i.detail.is_empty()));
    }
}
