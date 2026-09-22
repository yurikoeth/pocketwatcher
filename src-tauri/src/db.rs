//! SQLite storage. Every row type is also the wire type sent to the frontend.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LIABILITY_KINDS: &[&str] = &["credit_card", "loan", "mortgage", "rent_owed", "taxes_owed", "medical_debt", "personal_debt", "other_liability"];
pub const LIQUID_KINDS: &[&str] = &["checking", "savings", "cash"];
pub const INVESTED_KINDS: &[&str] = &["brokerage", "retirement", "crypto"];

pub fn is_liability(kind: &str) -> bool {
    LIABILITY_KINDS.contains(&kind)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    /// checking | savings | cash | brokerage | retirement | crypto | property | other_asset
    /// | credit_card | loan | mortgage | rent_owed | taxes_owed | medical_debt | personal_debt | other_liability
    pub kind: String,
    /// Always positive. For liabilities this is the amount owed.
    pub balance: f64,
    /// Annual percentage rate (liabilities) or expected yield (assets). Percent, e.g. 22.9.
    pub apr: f64,
    /// Minimum monthly payment for liabilities.
    pub min_payment: f64,
    /// Crypto wallets: solana | ethereum | bitcoin, or empty for manual.
    pub chain: String,
    /// Crypto wallets: public address; empty means the balance is manual.
    pub address: String,
    /// Crypto wallets: native-token quantity from the last sync.
    pub units: f64,
    /// RFC 3339 timestamp of the last successful sync.
    pub last_synced: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInput {
    pub id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub balance: f64,
    #[serde(default)]
    pub apr: f64,
    #[serde(default)]
    pub min_payment: f64,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub address: String,
}

/// One token position inside a crypto wallet. `contract` is "native" for the
/// chain's own coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    pub contract: String,
    pub symbol: String,
    pub name: String,
    pub units: f64,
    /// Spot price in the sync currency; None when no market price was found.
    pub price: Option<f64>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub account_id: Option<i64>,
    /// YYYY-MM-DD
    pub date: String,
    /// Positive = income, negative = expense.
    pub amount: f64,
    pub category: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub id: Option<i64>,
    pub account_id: Option<i64>,
    pub date: String,
    pub amount: f64,
    pub category: String,
    #[serde(default)]
    pub note: String,
}

/// A recurring obligation: rent, utilities, subscriptions, loan payments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bill {
    pub id: i64,
    pub name: String,
    pub amount: f64,
    pub category: String,
    /// weekly | monthly | quarterly | yearly
    pub frequency: String,
    /// Day of month (1-31) for monthly/quarterly/yearly; weekday 1-7 (Mon-Sun) for weekly.
    pub due_day: u32,
    /// Month (1-12) for yearly bills; first month of the cycle for quarterly.
    pub due_month: u32,
    pub account_id: Option<i64>,
    pub autopay: bool,
    /// Amount fluctuates (utilities); `amount` is only an estimate.
    pub variable: bool,
    /// YYYY-MM-DD of the last payment, if known.
    pub last_paid: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillInput {
    pub id: Option<i64>,
    pub name: String,
    pub amount: f64,
    pub category: String,
    pub frequency: String,
    pub due_day: u32,
    #[serde(default = "one")]
    pub due_month: u32,
    pub account_id: Option<i64>,
    #[serde(default)]
    pub autopay: bool,
    #[serde(default)]
    pub variable: bool,
    pub last_paid: Option<String>,
    #[serde(default)]
    pub note: String,
}

fn one() -> u32 {
    1
}

/// A recurring income source: salary, freelance, benefits, side gigs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Income {
    pub id: i64,
    pub name: String,
    /// Typical amount per payment (an estimate when `variable`).
    pub amount: f64,
    pub category: String,
    /// weekly | biweekly | semimonthly | monthly | quarterly | yearly | irregular
    pub frequency: String,
    /// Weekday 1-7 (weekly) or day of month (monthly/quarterly/yearly/semimonthly first day).
    pub due_day: u32,
    /// Second day of month for semimonthly.
    pub due_day2: u32,
    /// Month for yearly; first month of the cycle for quarterly.
    pub due_month: u32,
    /// A known pay date for biweekly schedules (YYYY-MM-DD).
    pub anchor: Option<String>,
    pub account_id: Option<i64>,
    pub variable: bool,
    /// YYYY-MM-DD of the last payment received, if known.
    pub last_received: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeInput {
    pub id: Option<i64>,
    pub name: String,
    pub amount: f64,
    pub category: String,
    pub frequency: String,
    pub due_day: u32,
    #[serde(default)]
    pub due_day2: u32,
    #[serde(default = "one")]
    pub due_month: u32,
    pub anchor: Option<String>,
    pub account_id: Option<i64>,
    #[serde(default)]
    pub variable: bool,
    pub last_received: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub category: String,
    pub monthly_limit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub target_amount: f64,
    pub current_amount: f64,
    /// YYYY-MM-DD, optional
    pub target_date: Option<String>,
    pub monthly_contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalInput {
    pub id: Option<i64>,
    pub name: String,
    pub target_amount: f64,
    #[serde(default)]
    pub current_amount: f64,
    pub target_date: Option<String>,
    #[serde(default)]
    pub monthly_contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub age: u32,
    pub retirement_age: u32,
    /// Desired annual spending in retirement, today's dollars.
    pub retirement_income: f64,
    /// conservative | balanced | aggressive
    pub risk_profile: String,
    pub emergency_months: f64,
    pub annual_return_pct: f64,
    pub inflation_pct: f64,
    /// Extra invested per month on top of what the accounts already hold.
    pub monthly_investment: f64,
    pub currency: String,
    pub api_key: String,
    pub model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            age: 30,
            retirement_age: 65,
            retirement_income: 60_000.0,
            risk_profile: "balanced".into(),
            emergency_months: 6.0,
            annual_return_pct: 7.0,
            inflation_pct: 3.0,
            monthly_investment: 0.0,
            currency: "USD".into(),
            api_key: String::new(),
            model: "claude-opus-5".into(),
        }
    }
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).context("create data dir")?;
        }
        let conn = Connection::open(path).context("open sqlite")?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                balance REAL NOT NULL DEFAULT 0,
                apr REAL NOT NULL DEFAULT 0,
                min_payment REAL NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY,
                account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                note TEXT NOT NULL DEFAULT ''
             );
             CREATE INDEX IF NOT EXISTS tx_date ON transactions(date);
             CREATE TABLE IF NOT EXISTS budgets (
                category TEXT PRIMARY KEY,
                monthly_limit REAL NOT NULL
             );
             CREATE TABLE IF NOT EXISTS goals (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                target_amount REAL NOT NULL,
                current_amount REAL NOT NULL DEFAULT 0,
                target_date TEXT,
                monthly_contribution REAL NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS bills (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                frequency TEXT NOT NULL DEFAULT 'monthly',
                due_day INTEGER NOT NULL DEFAULT 1,
                due_month INTEGER NOT NULL DEFAULT 1,
                account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
                autopay INTEGER NOT NULL DEFAULT 0,
                variable INTEGER NOT NULL DEFAULT 0,
                last_paid TEXT,
                note TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE IF NOT EXISTS incomes (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                frequency TEXT NOT NULL DEFAULT 'monthly',
                due_day INTEGER NOT NULL DEFAULT 1,
                due_day2 INTEGER NOT NULL DEFAULT 0,
                due_month INTEGER NOT NULL DEFAULT 1,
                anchor TEXT,
                account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
                variable INTEGER NOT NULL DEFAULT 0,
                last_received TEXT,
                note TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE IF NOT EXISTS holdings (
                account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                contract TEXT NOT NULL,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                units REAL NOT NULL,
                price REAL,
                value REAL NOT NULL DEFAULT 0,
                PRIMARY KEY (account_id, contract)
             );",
        )?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Additive column migrations for databases created by older builds.
    fn migrate(&self) -> Result<()> {
        let mut st = self.conn.prepare("PRAGMA table_info(accounts)")?;
        let cols: Vec<String> = st.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<_>>()?;
        let add = |name: &str, ddl: &str| -> Result<()> {
            if !cols.iter().any(|c| c == name) {
                self.conn.execute(&format!("ALTER TABLE accounts ADD COLUMN {name} {ddl}"), [])?;
            }
            Ok(())
        };
        add("chain", "TEXT NOT NULL DEFAULT ''")?;
        add("address", "TEXT NOT NULL DEFAULT ''")?;
        add("units", "REAL NOT NULL DEFAULT 0")?;
        add("last_synced", "TEXT")?;
        let mut st = self.conn.prepare("PRAGMA table_info(bills)")?;
        let bcols: Vec<String> = st.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<_>>()?;
        if !bcols.iter().any(|c| c == "variable") {
            self.conn.execute("ALTER TABLE bills ADD COLUMN variable INTEGER NOT NULL DEFAULT 0", [])?;
        }
        Ok(())
    }

    // ---- accounts ----

    fn account_from_row(r: &Row) -> rusqlite::Result<Account> {
        Ok(Account {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
            balance: r.get(3)?,
            apr: r.get(4)?,
            min_payment: r.get(5)?,
            chain: r.get(6)?,
            address: r.get(7)?,
            units: r.get(8)?,
            last_synced: r.get(9)?,
        })
    }

    const ACCOUNT_COLS: &'static str = "id, name, kind, balance, apr, min_payment, chain, address, units, last_synced";

    pub fn accounts(&self) -> Result<Vec<Account>> {
        let mut st = self
            .conn
            .prepare(&format!("SELECT {} FROM accounts ORDER BY kind, name", Self::ACCOUNT_COLS))?;
        let rows = st.query_map([], Self::account_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn upsert_account(&self, a: &AccountInput) -> Result<i64> {
        match a.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE accounts SET name=?1, kind=?2, balance=?3, apr=?4, min_payment=?5, chain=?6, address=?7 WHERE id=?8",
                    params![a.name, a.kind, a.balance, a.apr, a.min_payment, a.chain.trim(), a.address.trim(), id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO accounts (name, kind, balance, apr, min_payment, chain, address) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![a.name, a.kind, a.balance, a.apr, a.min_payment, a.chain.trim(), a.address.trim()],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    /// Replace the token holdings recorded for a wallet.
    pub fn replace_holdings(&mut self, account_id: i64, rows: &[Holding]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM holdings WHERE account_id=?1", params![account_id])?;
        {
            let mut st = tx.prepare("INSERT INTO holdings (account_id, contract, symbol, name, units, price, value) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
            for h in rows {
                st.execute(params![account_id, h.contract, h.symbol, h.name, h.units, h.price, h.value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// All recorded holdings, keyed by account id, largest value first.
    pub fn holdings(&self) -> Result<std::collections::HashMap<i64, Vec<Holding>>> {
        let mut st = self.conn.prepare("SELECT account_id, contract, symbol, name, units, price, value FROM holdings ORDER BY value DESC, symbol")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, Holding { contract: r.get(1)?, symbol: r.get(2)?, name: r.get(3)?, units: r.get(4)?, price: r.get(5)?, value: r.get(6)? }))
        })?;
        let mut out: std::collections::HashMap<i64, Vec<Holding>> = std::collections::HashMap::new();
        for r in rows {
            let (id, h) = r?;
            out.entry(id).or_default().push(h);
        }
        Ok(out)
    }

    /// Record a wallet sync: token quantity, fiat balance and timestamp.
    pub fn set_synced_balance(&self, id: i64, units: f64, balance: f64, when: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET units=?1, balance=?2, last_synced=?3 WHERE id=?4",
            params![units, balance, when, id],
        )?;
        Ok(())
    }

    pub fn delete_account(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- transactions ----

    fn tx_from_row(r: &Row) -> rusqlite::Result<Transaction> {
        Ok(Transaction {
            id: r.get(0)?,
            account_id: r.get(1)?,
            date: r.get(2)?,
            amount: r.get(3)?,
            category: r.get(4)?,
            note: r.get(5)?,
        })
    }

    /// All transactions, newest first.
    pub fn transactions(&self) -> Result<Vec<Transaction>> {
        let mut st = self.conn.prepare(
            "SELECT id, account_id, date, amount, category, note FROM transactions ORDER BY date DESC, id DESC",
        )?;
        let rows = st.query_map([], Self::tx_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn upsert_transaction(&self, t: &TransactionInput) -> Result<i64> {
        match t.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE transactions SET account_id=?1, date=?2, amount=?3, category=?4, note=?5 WHERE id=?6",
                    params![t.account_id, t.date, t.amount, t.category, t.note, id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO transactions (account_id, date, amount, category, note) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![t.account_id, t.date, t.amount, t.category, t.note],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn insert_transactions(&mut self, rows: &[TransactionInput]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        {
            let mut st = tx.prepare(
                "INSERT INTO transactions (account_id, date, amount, category, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for t in rows {
                st.execute(params![t.account_id, t.date, t.amount, t.category, t.note])?;
            }
        }
        tx.commit()?;
        Ok(rows.len())
    }

    pub fn delete_transaction(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM transactions WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- bills ----

    pub fn bills(&self) -> Result<Vec<Bill>> {
        let mut st = self.conn.prepare(
            "SELECT id, name, amount, category, frequency, due_day, due_month, account_id, autopay, last_paid, note, variable FROM bills ORDER BY name",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Bill {
                id: r.get(0)?,
                name: r.get(1)?,
                amount: r.get(2)?,
                category: r.get(3)?,
                frequency: r.get(4)?,
                due_day: r.get::<_, i64>(5)? as u32,
                due_month: r.get::<_, i64>(6)? as u32,
                account_id: r.get(7)?,
                autopay: r.get::<_, i64>(8)? != 0,
                last_paid: r.get(9)?,
                note: r.get(10)?,
                variable: r.get::<_, i64>(11)? != 0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn upsert_bill(&self, b: &BillInput) -> Result<i64> {
        let last_paid = b.last_paid.as_deref().filter(|s| !s.is_empty());
        match b.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE bills SET name=?1, amount=?2, category=?3, frequency=?4, due_day=?5, due_month=?6, account_id=?7, autopay=?8, last_paid=?9, note=?10, variable=?11 WHERE id=?12",
                    params![b.name.trim(), b.amount, b.category.trim(), b.frequency, b.due_day, b.due_month, b.account_id, b.autopay as i64, last_paid, b.note, b.variable as i64, id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO bills (name, amount, category, frequency, due_day, due_month, account_id, autopay, last_paid, note, variable) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![b.name.trim(), b.amount, b.category.trim(), b.frequency, b.due_day, b.due_month, b.account_id, b.autopay as i64, last_paid, b.note, b.variable as i64],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn set_bill_paid(&self, id: i64, date: &str) -> Result<()> {
        self.conn.execute("UPDATE bills SET last_paid=?1 WHERE id=?2", params![date, id])?;
        Ok(())
    }

    pub fn delete_bill(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM bills WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- incomes ----

    pub fn incomes(&self) -> Result<Vec<Income>> {
        let mut st = self.conn.prepare(
            "SELECT id, name, amount, category, frequency, due_day, due_day2, due_month, anchor, account_id, variable, last_received, note FROM incomes ORDER BY name",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Income {
                id: r.get(0)?,
                name: r.get(1)?,
                amount: r.get(2)?,
                category: r.get(3)?,
                frequency: r.get(4)?,
                due_day: r.get::<_, i64>(5)? as u32,
                due_day2: r.get::<_, i64>(6)? as u32,
                due_month: r.get::<_, i64>(7)? as u32,
                anchor: r.get(8)?,
                account_id: r.get(9)?,
                variable: r.get::<_, i64>(10)? != 0,
                last_received: r.get(11)?,
                note: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn upsert_income(&self, i: &IncomeInput) -> Result<i64> {
        let last = i.last_received.as_deref().filter(|s| !s.is_empty());
        let anchor = i.anchor.as_deref().filter(|s| !s.is_empty());
        match i.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE incomes SET name=?1, amount=?2, category=?3, frequency=?4, due_day=?5, due_day2=?6, due_month=?7, anchor=?8, account_id=?9, variable=?10, last_received=?11, note=?12 WHERE id=?13",
                    params![i.name.trim(), i.amount, i.category.trim(), i.frequency, i.due_day, i.due_day2, i.due_month, anchor, i.account_id, i.variable as i64, last, i.note, id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO incomes (name, amount, category, frequency, due_day, due_day2, due_month, anchor, account_id, variable, last_received, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![i.name.trim(), i.amount, i.category.trim(), i.frequency, i.due_day, i.due_day2, i.due_month, anchor, i.account_id, i.variable as i64, last, i.note],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn set_income_received(&self, id: i64, date: &str) -> Result<()> {
        self.conn.execute("UPDATE incomes SET last_received=?1 WHERE id=?2", params![date, id])?;
        Ok(())
    }

    pub fn delete_income(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM incomes WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- budgets ----

    pub fn budgets(&self) -> Result<Vec<Budget>> {
        let mut st = self.conn.prepare("SELECT category, monthly_limit FROM budgets ORDER BY category")?;
        let rows = st.query_map([], |r| {
            Ok(Budget { category: r.get(0)?, monthly_limit: r.get(1)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_budget(&self, b: &Budget) -> Result<()> {
        self.conn.execute(
            "INSERT INTO budgets (category, monthly_limit) VALUES (?1, ?2)
             ON CONFLICT(category) DO UPDATE SET monthly_limit = excluded.monthly_limit",
            params![b.category, b.monthly_limit],
        )?;
        Ok(())
    }

    pub fn delete_budget(&self, category: &str) -> Result<()> {
        self.conn.execute("DELETE FROM budgets WHERE category=?1", params![category])?;
        Ok(())
    }

    // ---- goals ----

    pub fn goals(&self) -> Result<Vec<Goal>> {
        let mut st = self.conn.prepare(
            "SELECT id, name, target_amount, current_amount, target_date, monthly_contribution FROM goals ORDER BY target_date IS NULL, target_date, id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Goal {
                id: r.get(0)?,
                name: r.get(1)?,
                target_amount: r.get(2)?,
                current_amount: r.get(3)?,
                target_date: r.get(4)?,
                monthly_contribution: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn upsert_goal(&self, g: &GoalInput) -> Result<i64> {
        let date = g.target_date.as_deref().filter(|s| !s.is_empty());
        match g.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE goals SET name=?1, target_amount=?2, current_amount=?3, target_date=?4, monthly_contribution=?5 WHERE id=?6",
                    params![g.name, g.target_amount, g.current_amount, date, g.monthly_contribution, id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO goals (name, target_amount, current_amount, target_date, monthly_contribution) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![g.name, g.target_amount, g.current_amount, date, g.monthly_contribution],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn delete_goal(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM goals WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- settings ----

    pub fn settings(&self) -> Result<Settings> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT json FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()?;
        Ok(match json {
            Some(j) => serde_json::from_str(&j).unwrap_or_default(),
            None => Settings::default(),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        let j = serde_json::to_string(s)?;
        self.conn.execute(
            "INSERT INTO settings (id, json) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET json = excluded.json",
            params![j],
        )?;
        Ok(())
    }

    pub fn is_empty(&self) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM accounts) + (SELECT COUNT(*) FROM transactions)",
            [],
            |r| r.get(0),
        )?;
        Ok(n == 0)
    }

    pub fn wipe(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM transactions; DELETE FROM holdings; DELETE FROM bills; DELETE FROM incomes; DELETE FROM accounts; DELETE FROM budgets; DELETE FROM goals;",
        )?;
        Ok(())
    }
}
