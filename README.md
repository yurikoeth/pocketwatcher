# FinCompass

Personal financial advisory desktop app. Local-only: every number lives in a
SQLite file on your machine. Built with Tauri 2, Rust, React 19 and Tailwind 4.

## What it does

- **Accounts** – assets (checking, savings, cash, brokerage, retirement,
  crypto, property) and liabilities (credit card, loan, mortgage, rent owed,
  taxes owed, medical bill, owed to a person, other). Net worth, liquid cash
  and invested totals roll up from these.
- **Crypto wallets** – a Crypto account can carry a chain (Solana, Ethereum,
  Bitcoin) and a public address. *Sync wallets* fetches the native-token
  balance from public nodes (Solana mainnet RPC / publicnode, Ethereum
  drpc / cloudflare / publicnode / llamarpc, blockstream / mempool.space) and prices
  it via CoinGecko in your settings currency. Tokens are included too:
  ERC-20 balances and prices come from Blockscout, SPL token accounts from
  the Solana RPC priced through DexScreener. Each wallet row can expand to
  show every coin; tokens without a market price are listed but valued at 0.
- **Transactions** – income and expenses by category, with CSV import
  (`date,amount,category,note`). Category `Transfer` is excluded from
  income/expense math.
- **Income** – recurring sources (salary, freelance, benefits) with a
  schedule: weekly, every two weeks (anchored on a known pay date), twice a
  month, monthly, quarterly, yearly or irregular. Shows expected monthly
  income, next pay dates, what's been logged this month, and flags a pay
  date that passed without a receipt. *Log received* adds the income
  transaction; sources flagged *varies* take the real amount each time and
  estimate from the last three receipts (irregular ones from the last 90
  days). Insights compare expected income with recurring bills.
- **Bills** – recurring obligations (rent, utilities, subscriptions, loan
  payments) with amount, frequency (weekly/monthly/quarterly/yearly), due
  day, paying account and autopay flag. Shows next due date, overdue status
  and the monthly-equivalent total; *Mark paid* stamps the payment and logs
  the expense transaction. Bills flagged *variable* (electric, water) take
  the real amount at payment time, and their estimate becomes the average of
  the last three logged payments. Insights flag overdue bills, bills due in the next
  14 days that exceed liquid cash, and a high fixed-cost ratio.
- **Budget** – monthly limits per category, spend vs limit vs 3-month average,
  pace against the elapsed month.
- **Goals** – target amount, optional date, monthly contribution; shows the
  required monthly amount and whether you're on track.
- **Debts** – avalanche vs snowball payoff simulation with an adjustable extra
  payment; months to debt-free, total interest, per-debt payoff order, balance
  curve. Flags debts whose minimum doesn't cover interest.
- **Projection** – retirement growth in nominal and today's dollars against a
  25× (4% rule) target, plus the contribution needed to hit it.
- **Dashboard insights** – a rule engine ranks what to fix first: back rent
  and taxes owed (always first), underwater minimums, negative savings rate,
  high-APR debt, emergency fund size, bills due soon vs cash (names the
  bills, calls out rent), overdue bills, housing cost vs income (30%/50%),
  bills vs expected income, budget overspend, debt-to-income, goals at risk,
  retirement gap, idle cash.
- **Advisor** – optional chat with Claude (`claude-opus-5` by default). Your
  snapshot is sent as context with each question. Needs an Anthropic API key
  in Settings; the tab is disabled without one.

## Run

```
npm install
npm run tauri dev
```

Build installers with `npm run tauri build`. Rust unit tests for the
analysis engine: `cd src-tauri && cargo test`.

## Data

Database: `%APPDATA%\dev.mangin.fincompass\fincompass.db`. Settings → Data
has *Export JSON*, *Load sample data* (empty DB only) and *Wipe all data*.
The API key is stored in that same database, in plain text.

## Notes

- Account balances are entered by hand; transactions don't move them.
- The advisor is a language model giving educational guidance, not a
  licensed adviser.
