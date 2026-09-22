export type AccountKind =
  | "checking"
  | "savings"
  | "cash"
  | "brokerage"
  | "retirement"
  | "crypto"
  | "property"
  | "other_asset"
  | "credit_card"
  | "loan"
  | "mortgage"
  | "rent_owed"
  | "taxes_owed"
  | "medical_debt"
  | "personal_debt"
  | "other_liability";

export const ASSET_KINDS: AccountKind[] = ["checking", "savings", "cash", "brokerage", "retirement", "crypto", "property", "other_asset"];
export const LIABILITY_KINDS: AccountKind[] = ["credit_card", "loan", "mortgage", "rent_owed", "taxes_owed", "medical_debt", "personal_debt", "other_liability"];

export const KIND_LABEL: Record<AccountKind, string> = {
  checking: "Checking",
  savings: "Savings",
  cash: "Cash",
  brokerage: "Brokerage",
  retirement: "Retirement",
  crypto: "Crypto",
  property: "Property",
  other_asset: "Other asset",
  credit_card: "Credit card",
  loan: "Loan",
  mortgage: "Mortgage",
  rent_owed: "Rent owed",
  taxes_owed: "Taxes owed",
  medical_debt: "Medical bill",
  personal_debt: "Owed to a person",
  other_liability: "Other liability",
};

export const isLiability = (k: string) => (LIABILITY_KINDS as string[]).includes(k);

export interface Account {
  id: number;
  name: string;
  kind: AccountKind;
  balance: number;
  apr: number;
  min_payment: number;
  chain: Chain | "";
  address: string;
  units: number;
  last_synced: string | null;
}

export type Chain = "solana" | "ethereum" | "bitcoin";
export const CHAINS: { id: Chain; label: string; symbol: string }[] = [
  { id: "solana", label: "Solana", symbol: "SOL" },
  { id: "ethereum", label: "Ethereum", symbol: "ETH" },
  { id: "bitcoin", label: "Bitcoin", symbol: "BTC" },
];

export interface AccountInput {
  id?: number | null;
  name: string;
  kind: AccountKind;
  balance: number;
  apr: number;
  min_payment: number;
  chain: Chain | "";
  address: string;
}

export interface Holding {
  contract: string;
  symbol: string;
  name: string;
  units: number;
  price: number | null;
  value: number;
}

export interface SyncResult {
  id: number;
  name: string;
  chain: string;
  symbol: string;
  units: number;
  price: number;
  balance: number;
  currency: string;
  holdings: Holding[];
  error: string | null;
}

export interface Transaction {
  id: number;
  account_id: number | null;
  date: string;
  amount: number;
  category: string;
  note: string;
}

export interface TransactionInput {
  id?: number | null;
  account_id: number | null;
  date: string;
  amount: number;
  category: string;
  note: string;
}

export interface Budget {
  category: string;
  monthly_limit: number;
}

export type Frequency = "weekly" | "monthly" | "quarterly" | "yearly";
export const FREQUENCIES: { id: Frequency; label: string }[] = [
  { id: "weekly", label: "Weekly" },
  { id: "monthly", label: "Monthly" },
  { id: "quarterly", label: "Quarterly" },
  { id: "yearly", label: "Yearly" },
];

export interface Bill {
  id: number;
  name: string;
  amount: number;
  category: string;
  frequency: Frequency;
  due_day: number;
  due_month: number;
  account_id: number | null;
  autopay: boolean;
  variable: boolean;
  last_paid: string | null;
  note: string;
}

export interface BillInput {
  id?: number | null;
  name: string;
  amount: number;
  category: string;
  frequency: Frequency;
  due_day: number;
  due_month: number;
  account_id: number | null;
  autopay: boolean;
  variable: boolean;
  last_paid: string | null;
  note: string;
}

export type IncomeFrequency = "weekly" | "biweekly" | "semimonthly" | "monthly" | "quarterly" | "yearly" | "irregular";
export const INCOME_FREQUENCIES: { id: IncomeFrequency; label: string }[] = [
  { id: "weekly", label: "Weekly" },
  { id: "biweekly", label: "Every two weeks" },
  { id: "semimonthly", label: "Twice a month" },
  { id: "monthly", label: "Monthly" },
  { id: "quarterly", label: "Quarterly" },
  { id: "yearly", label: "Yearly" },
  { id: "irregular", label: "Irregular" },
];

export interface Income {
  id: number;
  name: string;
  amount: number;
  category: string;
  frequency: IncomeFrequency;
  due_day: number;
  due_day2: number;
  due_month: number;
  anchor: string | null;
  account_id: number | null;
  variable: boolean;
  last_received: string | null;
  note: string;
}

export interface IncomeInput {
  id?: number | null;
  name: string;
  amount: number;
  category: string;
  frequency: IncomeFrequency;
  due_day: number;
  due_day2: number;
  due_month: number;
  anchor: string | null;
  account_id: number | null;
  variable: boolean;
  last_received: string | null;
  note: string;
}

export interface IncomeStatus {
  income: Income;
  estimate: number;
  last_amount: number | null;
  history: BillPayment[];
  monthly_equivalent: number;
  next_date: string | null;
  days_until: number | null;
  received_this_cycle: boolean;
  late_days: number;
}

export interface BillPayment {
  date: string;
  amount: number;
}

export interface BillStatus {
  bill: Bill;
  estimate: number;
  last_amount: number | null;
  history: BillPayment[];
  monthly_equivalent: number;
  next_due: string;
  days_until: number;
  paid_this_cycle: boolean;
  overdue_days: number;
}

export interface Goal {
  id: number;
  name: string;
  target_amount: number;
  current_amount: number;
  target_date: string | null;
  monthly_contribution: number;
}

export interface GoalInput {
  id?: number | null;
  name: string;
  target_amount: number;
  current_amount: number;
  target_date: string | null;
  monthly_contribution: number;
}

export interface Settings {
  age: number;
  retirement_age: number;
  retirement_income: number;
  risk_profile: "conservative" | "balanced" | "aggressive";
  emergency_months: number;
  annual_return_pct: number;
  inflation_pct: number;
  monthly_investment: number;
  currency: string;
  api_key: string;
  model: string;
}

export interface MonthFlow {
  month: string;
  income: number;
  expenses: number;
  net: number;
  tx_count: number;
}

export interface CategorySpend {
  category: string;
  spent: number;
  limit: number | null;
  avg_3mo: number;
}

export interface GoalStatus {
  goal: Goal;
  progress_pct: number;
  months_left: number | null;
  required_monthly: number | null;
  months_to_reach: number | null;
  on_track: boolean;
}

export interface DebtLine {
  account_id: number;
  name: string;
  kind: string;
  balance: number;
  apr: number;
  min_payment: number;
  payoff_month: number | null;
  interest_paid: number;
  never: boolean;
}

export interface PayoffPlan {
  strategy: string;
  extra_monthly: number;
  months: number | null;
  total_interest: number;
  total_paid: number;
  debts: DebtLine[];
  balance_by_month: number[];
}

export type Severity = "critical" | "warn" | "info" | "good";

export interface Insight {
  severity: Severity;
  title: string;
  detail: string;
  action: string;
}

export interface ProjPoint {
  year: number;
  age: number;
  nominal: number;
  real: number;
  contributed: number;
}

export interface ProjectionParams {
  starting_balance: number;
  monthly_contribution: number;
  annual_return_pct: number;
  inflation_pct: number;
  age: number;
  retirement_age: number;
  retirement_income: number;
}

export interface Projection {
  params_used: ProjectionParams;
  points: ProjPoint[];
  final_nominal: number;
  final_real: number;
  target_nest_egg: number;
  safe_withdrawal_real: number;
  on_track: boolean;
  required_monthly: number | null;
}

export interface Snapshot {
  today: string;
  currency: string;
  accounts: Account[];
  net_worth: number;
  assets: number;
  liabilities: number;
  liquid: number;
  invested: number;
  /** Token positions per crypto account id, from the last sync. */
  holdings: Record<string, Holding[]>;
  months: MonthFlow[];
  avg_income: number;
  avg_expenses: number;
  avg_basis: string;
  savings_rate: number | null;
  emergency_months: number | null;
  dti: number | null;
  categories: CategorySpend[];
  goals: GoalStatus[];
  bills: BillStatus[];
  incomes: IncomeStatus[];
  income_monthly: number;
  income_this_month: number;
  bills_monthly: number;
  bills_due_14d: number;
  payoff: PayoffPlan;
  projection: Projection;
  projection_contribution_source: string;
  insights: Insight[];
  tx_count: number;
  has_data: boolean;
}

export interface ChatMsg {
  role: "user" | "assistant";
  content: string;
}

export interface ChatReply {
  text: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
}
