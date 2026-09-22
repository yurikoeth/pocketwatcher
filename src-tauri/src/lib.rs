mod ai;
mod analysis;
mod crypto;
mod db;

use std::sync::Mutex;

use chrono::{Datelike, Local};
use tauri::Manager;

use ai::{ChatMsg, ChatReply};
use analysis::{PayoffPlan, Projection, ProjectionParams, Snapshot};
use crypto::SyncResult;
use db::{Account, AccountInput, Bill, BillInput, Budget, Db, Goal, GoalInput, Holding, Income, IncomeInput, Settings, Transaction, TransactionInput};

pub struct AppState {
    db: Mutex<Db>,
    db_path: String,
}

type State<'a> = tauri::State<'a, AppState>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn build_snapshot(db: &Db) -> Result<Snapshot, String> {
    let accounts = db.accounts().map_err(err)?;
    let txs = db.transactions().map_err(err)?;
    let budgets = db.budgets().map_err(err)?;
    let goals = db.goals().map_err(err)?;
    let settings = db.settings().map_err(err)?;
    let bills = db.bills().map_err(err)?;
    let incomes = db.incomes().map_err(err)?;
    let mut snap = analysis::snapshot(&accounts, &txs, &budgets, &goals, &bills, &incomes, &settings, Local::now().date_naive());
    snap.holdings = db.holdings().map_err(err)?;
    Ok(snap)
}

#[tauri::command]
fn get_snapshot(state: State<'_>) -> Result<Snapshot, String> {
    let db = state.db.lock().map_err(err)?;
    build_snapshot(&db)
}

// ---- accounts ----

#[tauri::command]
fn list_accounts(state: State<'_>) -> Result<Vec<Account>, String> {
    state.db.lock().map_err(err)?.accounts().map_err(err)
}

#[tauri::command]
fn upsert_account(state: State<'_>, account: AccountInput) -> Result<i64, String> {
    if account.name.trim().is_empty() {
        return Err("Account needs a name".into());
    }
    state.db.lock().map_err(err)?.upsert_account(&account).map_err(err)
}

/// Refresh every crypto account that has a wallet address: native balance
/// plus token holdings from public APIs, priced in the settings currency.
#[tauri::command]
async fn sync_crypto(state: State<'_>) -> Result<Vec<SyncResult>, String> {
    let (wallets, currency) = {
        let db = state.db.lock().map_err(err)?;
        let accounts = db.accounts().map_err(err)?;
        let currency = db.settings().map_err(err)?.currency;
        let wallets: Vec<Account> = accounts.into_iter().filter(|a| a.kind == "crypto" && !a.address.trim().is_empty()).collect();
        (wallets, currency)
    };
    if wallets.is_empty() {
        return Ok(vec![]);
    }
    let client = crypto::client()?;
    let prices = crypto::fetch_prices(&client, &currency).await;
    let mut results = Vec::with_capacity(wallets.len());
    for w in &wallets {
        let info = crypto::chain_info(&w.chain);
        let symbol = info.map(|i| i.2).unwrap_or("?").to_string();
        let mut res = SyncResult { id: w.id, name: w.name.clone(), chain: w.chain.clone(), symbol: symbol.clone(), units: w.units, price: 0.0, balance: w.balance, currency: currency.clone(), holdings: vec![], error: None };
        let Some(info) = info else {
            res.error = Some(format!("Pick a chain for this wallet (got '{}')", w.chain));
            results.push(res);
            continue;
        };
        let prices = match &prices {
            Ok(p) => p,
            Err(e) => {
                res.error = Some(e.clone());
                results.push(res);
                continue;
            }
        };
        let units = match crypto::fetch_units(&client, &w.chain, &w.address).await {
            Ok(u) => u,
            Err(e) => {
                res.error = Some(e);
                results.push(res);
                continue;
            }
        };
        let price = *prices.by_id.get(info.1).unwrap_or(&0.0);
        let mut holdings = vec![Holding { contract: "native".into(), symbol: symbol.clone(), name: info.1.to_string(), units, price: Some(price), value: units * price }];
        // Token discovery is best-effort: a failure here keeps the native balance
        // and reports the problem instead of failing the whole wallet.
        match crypto::fetch_token_holdings(&client, &w.chain, &w.address).await {
            Ok(tokens) => {
                for mut t in tokens {
                    if let Some(p) = t.price {
                        t.price = Some(p * prices.usd_ratio);
                        t.value = t.units * p * prices.usd_ratio;
                    }
                    holdings.push(t);
                }
            }
            Err(e) => res.error = Some(format!("native balance synced; token lookup failed: {e}")),
        }
        holdings.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        let total: f64 = holdings.iter().map(|h| h.value).sum();
        res.units = units;
        res.price = price;
        res.balance = (total * 100.0).round() / 100.0;
        res.currency = prices.currency.clone();
        res.holdings = holdings;
        results.push(res);
    }
    let now = chrono::Utc::now().to_rfc3339();
    let mut db = state.db.lock().map_err(err)?;
    for r in results.iter().filter(|r| !r.holdings.is_empty()) {
        db.set_synced_balance(r.id, r.units, r.balance, &now).map_err(err)?;
        db.replace_holdings(r.id, &r.holdings).map_err(err)?;
    }
    Ok(results)
}

#[tauri::command]
fn delete_account(state: State<'_>, id: i64) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_account(id).map_err(err)
}

// ---- transactions ----

#[tauri::command]
fn list_transactions(state: State<'_>) -> Result<Vec<Transaction>, String> {
    state.db.lock().map_err(err)?.transactions().map_err(err)
}

#[tauri::command]
fn upsert_transaction(state: State<'_>, tx: TransactionInput) -> Result<i64, String> {
    validate_tx(&tx)?;
    state.db.lock().map_err(err)?.upsert_transaction(&tx).map_err(err)
}

#[tauri::command]
fn delete_transaction(state: State<'_>, id: i64) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_transaction(id).map_err(err)
}

fn validate_tx(t: &TransactionInput) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(t.date.trim(), "%Y-%m-%d").map_err(|_| format!("Bad date '{}', expected YYYY-MM-DD", t.date))?;
    if !t.amount.is_finite() || t.amount == 0.0 {
        return Err("Amount must be a non-zero number".into());
    }
    if t.category.trim().is_empty() {
        return Err("Category is required".into());
    }
    Ok(())
}

/// Import `date,amount,category,note` rows. A header line is skipped if the
/// first field isn't a date. Amounts: positive income, negative expense.
#[tauri::command]
fn import_csv(state: State<'_>, account_id: Option<i64>, text: String) -> Result<usize, String> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = split_csv(line);
        if fields.len() < 3 {
            errors.push(format!("line {}: expected date,amount,category[,note]", i + 1));
            continue;
        }
        let date = fields[0].trim();
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            if i == 0 {
                continue; // header
            }
            errors.push(format!("line {}: bad date '{}'", i + 1, date));
            continue;
        }
        let amount: f64 = match fields[1].trim().replace(['$', ','], "").parse() {
            Ok(a) => a,
            Err(_) => {
                errors.push(format!("line {}: bad amount '{}'", i + 1, fields[1]));
                continue;
            }
        };
        let tx = TransactionInput {
            id: None,
            account_id,
            date: date.to_string(),
            amount,
            category: fields[2].trim().to_string(),
            note: fields.get(3).map(|s| s.trim().to_string()).unwrap_or_default(),
        };
        if let Err(e) = validate_tx(&tx) {
            errors.push(format!("line {}: {}", i + 1, e));
            continue;
        }
        rows.push(tx);
    }
    if !errors.is_empty() && rows.is_empty() {
        return Err(errors.join("\n"));
    }
    let n = state.db.lock().map_err(err)?.insert_transactions(&rows).map_err(err)?;
    if errors.is_empty() {
        Ok(n)
    } else {
        Err(format!("Imported {} rows; skipped {}:\n{}", n, errors.len(), errors.join("\n")))
    }
}

fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_q && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_q = !in_q,
            ',' if !in_q => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

// ---- bills ----

const FREQUENCIES: &[&str] = &["weekly", "monthly", "quarterly", "yearly"];

#[tauri::command]
fn list_bills(state: State<'_>) -> Result<Vec<Bill>, String> {
    state.db.lock().map_err(err)?.bills().map_err(err)
}

#[tauri::command]
fn upsert_bill(state: State<'_>, bill: BillInput) -> Result<i64, String> {
    if bill.name.trim().is_empty() {
        return Err("Bill needs a name".into());
    }
    if !bill.amount.is_finite() || bill.amount <= 0.0 {
        return Err("Amount must be greater than zero".into());
    }
    if !FREQUENCIES.contains(&bill.frequency.as_str()) {
        return Err(format!("Unknown frequency '{}'", bill.frequency));
    }
    let max_day = if bill.frequency == "weekly" { 7 } else { 31 };
    if bill.due_day < 1 || bill.due_day > max_day {
        return Err(format!("Due day must be between 1 and {max_day}"));
    }
    if bill.due_month < 1 || bill.due_month > 12 {
        return Err("Due month must be between 1 and 12".into());
    }
    if let Some(d) = bill.last_paid.as_deref().filter(|s| !s.is_empty()) {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| format!("Bad date '{d}', expected YYYY-MM-DD"))?;
    }
    let b = BillInput { category: if bill.category.trim().is_empty() { "Bills".into() } else { bill.category.trim().into() }, ..bill };
    state.db.lock().map_err(err)?.upsert_bill(&b).map_err(err)
}

/// Record a bill payment: stamps `last_paid` and, when `log` is set, adds the
/// matching expense transaction so budgets and cash flow see it. `amount`
/// overrides the bill's amount (variable bills such as utilities).
#[tauri::command]
fn mark_bill_paid(state: State<'_>, id: i64, date: Option<String>, log: bool, amount: Option<f64>) -> Result<(), String> {
    let date = date.filter(|d| !d.is_empty()).unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string());
    chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| format!("Bad date '{date}', expected YYYY-MM-DD"))?;
    let db = state.db.lock().map_err(err)?;
    let bill = db.bills().map_err(err)?.into_iter().find(|b| b.id == id).ok_or("Bill not found")?;
    let paid = amount.filter(|a| a.is_finite() && *a > 0.0).unwrap_or(bill.amount);
    if log {
        db.upsert_transaction(&TransactionInput {
            id: None,
            account_id: bill.account_id,
            date: date.clone(),
            amount: -paid.abs(),
            category: bill.category.clone(),
            note: analysis::bill_note(&bill.name),
        })
        .map_err(err)?;
    }
    db.set_bill_paid(id, &date).map_err(err)
}

#[tauri::command]
fn delete_bill(state: State<'_>, id: i64) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_bill(id).map_err(err)
}

// ---- incomes ----

const INCOME_FREQUENCIES: &[&str] = &["weekly", "biweekly", "semimonthly", "monthly", "quarterly", "yearly", "irregular"];

#[tauri::command]
fn list_incomes(state: State<'_>) -> Result<Vec<Income>, String> {
    state.db.lock().map_err(err)?.incomes().map_err(err)
}

#[tauri::command]
fn upsert_income(state: State<'_>, income: IncomeInput) -> Result<i64, String> {
    if income.name.trim().is_empty() {
        return Err("Income source needs a name".into());
    }
    if !income.amount.is_finite() || income.amount < 0.0 {
        return Err("Amount can't be negative".into());
    }
    if !INCOME_FREQUENCIES.contains(&income.frequency.as_str()) {
        return Err(format!("Unknown frequency '{}'", income.frequency));
    }
    let needs_day = !matches!(income.frequency.as_str(), "irregular" | "biweekly");
    let max_day = if income.frequency == "weekly" { 7 } else { 31 };
    if needs_day && (income.due_day < 1 || income.due_day > max_day) {
        return Err(format!("Pay day must be between 1 and {max_day}"));
    }
    if income.frequency == "semimonthly" && (income.due_day2 < 1 || income.due_day2 > 31 || income.due_day2 == income.due_day) {
        return Err("Semimonthly needs two different days of the month".into());
    }
    if income.due_month < 1 || income.due_month > 12 {
        return Err("Month must be between 1 and 12".into());
    }
    for (label, d) in [("Anchor date", income.anchor.as_deref()), ("Last received", income.last_received.as_deref())] {
        if let Some(d) = d.filter(|s| !s.is_empty()) {
            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| format!("{label}: bad date '{d}', expected YYYY-MM-DD"))?;
        }
    }
    if income.frequency == "biweekly" && income.anchor.as_deref().filter(|s| !s.is_empty()).is_none() && income.last_received.as_deref().filter(|s| !s.is_empty()).is_none() {
        return Err("Every-two-weeks needs a known pay date (anchor or last received)".into());
    }
    let i = IncomeInput { category: if income.category.trim().is_empty() { "Income".into() } else { income.category.trim().into() }, ..income };
    state.db.lock().map_err(err)?.upsert_income(&i).map_err(err)
}

/// Record an income receipt: stamps `last_received` and, when `log` is set,
/// adds the matching income transaction. `amount` overrides the typical
/// amount (variable sources).
#[tauri::command]
fn log_income_received(state: State<'_>, id: i64, date: Option<String>, log: bool, amount: Option<f64>) -> Result<(), String> {
    let date = date.filter(|d| !d.is_empty()).unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string());
    chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| format!("Bad date '{date}', expected YYYY-MM-DD"))?;
    let db = state.db.lock().map_err(err)?;
    let inc = db.incomes().map_err(err)?.into_iter().find(|i| i.id == id).ok_or("Income source not found")?;
    let received = amount.filter(|a| a.is_finite() && *a > 0.0).unwrap_or(inc.amount);
    if log {
        if received <= 0.0 {
            return Err("Enter the amount received".into());
        }
        db.upsert_transaction(&TransactionInput {
            id: None,
            account_id: inc.account_id,
            date: date.clone(),
            amount: received.abs(),
            category: inc.category.clone(),
            note: analysis::income_note(&inc.name),
        })
        .map_err(err)?;
    }
    db.set_income_received(id, &date).map_err(err)
}

#[tauri::command]
fn delete_income(state: State<'_>, id: i64) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_income(id).map_err(err)
}

// ---- budgets ----

#[tauri::command]
fn list_budgets(state: State<'_>) -> Result<Vec<Budget>, String> {
    state.db.lock().map_err(err)?.budgets().map_err(err)
}

#[tauri::command]
fn set_budget(state: State<'_>, budget: Budget) -> Result<(), String> {
    if budget.category.trim().is_empty() {
        return Err("Category is required".into());
    }
    let b = Budget { category: budget.category.trim().to_string(), monthly_limit: budget.monthly_limit.max(0.0) };
    state.db.lock().map_err(err)?.set_budget(&b).map_err(err)
}

#[tauri::command]
fn delete_budget(state: State<'_>, category: String) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_budget(&category).map_err(err)
}

// ---- goals ----

#[tauri::command]
fn list_goals(state: State<'_>) -> Result<Vec<Goal>, String> {
    state.db.lock().map_err(err)?.goals().map_err(err)
}

#[tauri::command]
fn upsert_goal(state: State<'_>, goal: GoalInput) -> Result<i64, String> {
    if goal.name.trim().is_empty() {
        return Err("Goal needs a name".into());
    }
    if let Some(d) = goal.target_date.as_deref().filter(|s| !s.is_empty()) {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| format!("Bad date '{d}', expected YYYY-MM-DD"))?;
    }
    state.db.lock().map_err(err)?.upsert_goal(&goal).map_err(err)
}

#[tauri::command]
fn delete_goal(state: State<'_>, id: i64) -> Result<(), String> {
    state.db.lock().map_err(err)?.delete_goal(id).map_err(err)
}

// ---- settings ----

#[tauri::command]
fn get_settings(state: State<'_>) -> Result<Settings, String> {
    state.db.lock().map_err(err)?.settings().map_err(err)
}

#[tauri::command]
fn save_settings(state: State<'_>, settings: Settings) -> Result<(), String> {
    if settings.retirement_age <= settings.age {
        return Err("Retirement age must be greater than current age".into());
    }
    state.db.lock().map_err(err)?.save_settings(&settings).map_err(err)
}

#[tauri::command]
fn data_path(state: State<'_>) -> String {
    state.db_path.clone()
}

// ---- analysis ----

#[tauri::command]
fn debt_plan(state: State<'_>, extra: f64, strategy: String, include_mortgage: bool) -> Result<PayoffPlan, String> {
    let db = state.db.lock().map_err(err)?;
    let accounts = db.accounts().map_err(err)?;
    let debts: Vec<Account> = accounts
        .into_iter()
        .filter(|a| db::is_liability(&a.kind) && (include_mortgage || a.kind != "mortgage"))
        .collect();
    Ok(analysis::payoff_plan(&debts, extra, &strategy))
}

#[tauri::command]
fn project(params: ProjectionParams) -> Result<Projection, String> {
    if params.retirement_age <= params.age {
        return Err("Retirement age must be greater than current age".into());
    }
    Ok(analysis::project(&params))
}

#[tauri::command]
async fn ask_advisor(state: State<'_>, history: Vec<ChatMsg>) -> Result<ChatReply, String> {
    let (key, model, snap) = {
        let db = state.db.lock().map_err(err)?;
        let settings = db.settings().map_err(err)?;
        let snap = build_snapshot(&db)?;
        (settings.api_key, settings.model, snap)
    };
    // Trim the snapshot to what the model needs: drop the per-month payoff curve.
    let mut v = serde_json::to_value(&snap).map_err(err)?;
    if let Some(p) = v.get_mut("payoff") {
        if let Some(o) = p.as_object_mut() {
            o.remove("balance_by_month");
        }
    }
    let json = serde_json::to_string_pretty(&v).map_err(err)?;
    let model = if model.trim().is_empty() { "claude-opus-5".to_string() } else { model };
    ai::ask(&key, &model, &json, &history).await
}

// ---- demo data ----

#[tauri::command]
fn load_sample_data(state: State<'_>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(err)?;
    if !db.is_empty().map_err(err)? {
        return Err("Sample data can only be loaded into an empty database. Use 'Wipe all data' first.".into());
    }
    let mk = |name: &str, kind: &str, balance: f64, apr: f64, min_payment: f64| AccountInput {
        id: None,
        name: name.into(),
        kind: kind.into(),
        balance,
        apr,
        min_payment,
        chain: String::new(),
        address: String::new(),
    };
    let checking = db.upsert_account(&mk("Everyday Checking", "checking", 4_180.0, 0.0, 0.0)).map_err(err)?;
    db.upsert_account(&mk("High-Yield Savings", "savings", 9_500.0, 4.2, 0.0)).map_err(err)?;
    db.upsert_account(&mk("401(k)", "retirement", 38_400.0, 0.0, 0.0)).map_err(err)?;
    db.upsert_account(&mk("Brokerage", "brokerage", 12_650.0, 0.0, 0.0)).map_err(err)?;
    let card = db.upsert_account(&mk("Rewards Visa", "credit_card", 3_420.0, 24.9, 95.0)).map_err(err)?;
    db.upsert_account(&mk("Car Loan", "loan", 11_200.0, 6.4, 310.0)).map_err(err)?;
    db.upsert_account(&mk("Student Loan", "loan", 18_500.0, 5.5, 210.0)).map_err(err)?;

    let today = Local::now().date_naive();
    let txs = analysis::sample_transactions(today, checking, card);
    db.insert_transactions(&txs).map_err(err)?;

    for (c, l) in [("Groceries", 600.0), ("Dining", 250.0), ("Transport", 300.0), ("Entertainment", 150.0), ("Shopping", 200.0), ("Utilities", 250.0), ("Subscriptions", 60.0)] {
        db.set_budget(&Budget { category: c.into(), monthly_limit: l }).map_err(err)?;
    }
    let goal = |name: &str, target: f64, current: f64, date: Option<&str>, monthly: f64| GoalInput {
        id: None,
        name: name.into(),
        target_amount: target,
        current_amount: current,
        target_date: date.map(String::from),
        monthly_contribution: monthly,
    };
    db.upsert_goal(&goal("Emergency fund (6 months)", 21_000.0, 9_500.0, None, 400.0)).map_err(err)?;
    db.upsert_goal(&goal("Japan trip", 4_000.0, 800.0, Some("2027-06-01"), 200.0)).map_err(err)?;
    db.upsert_goal(&goal("House down payment", 60_000.0, 12_650.0, Some("2030-01-01"), 500.0)).map_err(err)?;

    let bill = |name: &str, amount: f64, category: &str, frequency: &str, due_day: u32, due_month: u32, account: Option<i64>, autopay: bool, last_paid: Option<String>| BillInput {
        id: None,
        name: name.into(),
        amount,
        category: category.into(),
        frequency: frequency.into(),
        due_day,
        due_month,
        account_id: account,
        autopay,
        variable: name.starts_with("Electric"),
        last_paid,
        note: String::new(),
    };
    let this_month = |d: u32| today.with_day(d.min(28)).map(|x| x.format("%Y-%m-%d").to_string());
    let paid_if_past = |d: u32| if today.day() >= d { this_month(d) } else { None };
    db.upsert_bill(&bill("Rent", 1_650.0, "Rent", "monthly", 1, 1, Some(checking), false, this_month(1))).map_err(err)?;
    db.upsert_bill(&bill("Electric + internet", 175.0, "Utilities", "monthly", 5, 1, Some(checking), true, paid_if_past(5))).map_err(err)?;
    db.upsert_bill(&bill("Auto insurance", 128.0, "Insurance", "monthly", 5, 1, Some(checking), true, paid_if_past(5))).map_err(err)?;
    db.upsert_bill(&bill("Car loan", 310.0, "Debt payment", "monthly", 5, 1, Some(checking), true, paid_if_past(5))).map_err(err)?;
    db.upsert_bill(&bill("Student loan", 210.0, "Debt payment", "monthly", 5, 1, Some(checking), false, paid_if_past(5))).map_err(err)?;
    db.upsert_bill(&bill("Streaming + music", 47.97, "Subscriptions", "monthly", 10, 1, Some(card), true, paid_if_past(10))).map_err(err)?;
    db.upsert_bill(&bill("Phone", 65.0, "Phone", "monthly", 28, 1, Some(card), true, None)).map_err(err)?;
    db.upsert_bill(&bill("Renter's insurance", 180.0, "Insurance", "yearly", 15, 3, Some(checking), false, Some(format!("{}-03-15", today.year())))).map_err(err)?;

    let income = |name: &str, amount: f64, category: &str, frequency: &str, d1: u32, d2: u32, variable: bool, last: Option<String>| IncomeInput {
        id: None,
        name: name.into(),
        amount,
        category: category.into(),
        frequency: frequency.into(),
        due_day: d1,
        due_day2: d2,
        due_month: 1,
        anchor: None,
        account_id: Some(checking),
        variable,
        last_received: last,
        note: String::new(),
    };
    db.upsert_income(&income("Salary", 2_650.0, "Salary", "semimonthly", 1, 15, false, if today.day() >= 15 { this_month(15) } else { this_month(1) })).map_err(err)?;
    db.upsert_income(&income("Freelance design", 800.0, "Freelance", "irregular", 1, 0, true, None)).map_err(err)?;

    let mut s = db.settings().map_err(err)?;
    if s.monthly_investment == 0.0 {
        s.monthly_investment = 650.0;
        db.save_settings(&s).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn wipe_data(state: State<'_>) -> Result<(), String> {
    state.db.lock().map_err(err)?.wipe().map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let path = dir.join("fincompass.db");
            let db = Db::open(&path)?;
            app.manage(AppState { db: Mutex::new(db), db_path: path.display().to_string() });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            list_accounts,
            upsert_account,
            sync_crypto,
            delete_account,
            list_transactions,
            upsert_transaction,
            delete_transaction,
            import_csv,
            list_incomes,
            upsert_income,
            log_income_received,
            delete_income,
            list_bills,
            upsert_bill,
            mark_bill_paid,
            delete_bill,
            list_budgets,
            set_budget,
            delete_budget,
            list_goals,
            upsert_goal,
            delete_goal,
            get_settings,
            save_settings,
            data_path,
            debt_plan,
            project,
            ask_advisor,
            load_sample_data,
            wipe_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
