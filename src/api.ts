import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AccountInput,
  Bill,
  BillInput,
  Budget,
  ChatMsg,
  ChatReply,
  Goal,
  GoalInput,
  Income,
  IncomeInput,
  PayoffPlan,
  Projection,
  ProjectionParams,
  Settings,
  Snapshot,
  SyncResult,
  Transaction,
  TransactionInput,
} from "./types";

export const api = {
  snapshot: () => invoke<Snapshot>("get_snapshot"),

  accounts: () => invoke<Account[]>("list_accounts"),
  upsertAccount: (account: AccountInput) => invoke<number>("upsert_account", { account }),
  deleteAccount: (id: number) => invoke<void>("delete_account", { id }),
  syncCrypto: () => invoke<SyncResult[]>("sync_crypto"),

  transactions: () => invoke<Transaction[]>("list_transactions"),
  upsertTransaction: (tx: TransactionInput) => invoke<number>("upsert_transaction", { tx }),
  deleteTransaction: (id: number) => invoke<void>("delete_transaction", { id }),
  importCsv: (accountId: number | null, text: string) => invoke<number>("import_csv", { accountId, text }),

  incomes: () => invoke<Income[]>("list_incomes"),
  upsertIncome: (income: IncomeInput) => invoke<number>("upsert_income", { income }),
  logIncomeReceived: (id: number, date: string | null, log: boolean, amount: number | null) => invoke<void>("log_income_received", { id, date, log, amount }),
  deleteIncome: (id: number) => invoke<void>("delete_income", { id }),

  bills: () => invoke<Bill[]>("list_bills"),
  upsertBill: (bill: BillInput) => invoke<number>("upsert_bill", { bill }),
  markBillPaid: (id: number, date: string | null, log: boolean, amount: number | null) => invoke<void>("mark_bill_paid", { id, date, log, amount }),
  deleteBill: (id: number) => invoke<void>("delete_bill", { id }),

  budgets: () => invoke<Budget[]>("list_budgets"),
  setBudget: (budget: Budget) => invoke<void>("set_budget", { budget }),
  deleteBudget: (category: string) => invoke<void>("delete_budget", { category }),

  goals: () => invoke<Goal[]>("list_goals"),
  upsertGoal: (goal: GoalInput) => invoke<number>("upsert_goal", { goal }),
  deleteGoal: (id: number) => invoke<void>("delete_goal", { id }),

  settings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  dataPath: () => invoke<string>("data_path"),

  debtPlan: (extra: number, strategy: "avalanche" | "snowball", includeMortgage: boolean) =>
    invoke<PayoffPlan>("debt_plan", { extra, strategy, includeMortgage }),
  project: (params: ProjectionParams) => invoke<Projection>("project", { params }),
  askAdvisor: (history: ChatMsg[]) => invoke<ChatReply>("ask_advisor", { history }),

  loadSampleData: () => invoke<void>("load_sample_data"),
  wipeData: () => invoke<void>("wipe_data"),
};

export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}
