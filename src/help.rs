//! All user-facing help text in one place: short tooltips (`tips`) used next
//! to controls, and long-form topics shown on the in-app Help page.

/// Short hover texts shown next to controls.
pub mod tips {
    pub const PRIVACY: &str = "FinTrack never connects to the internet. Everything is stored in the database folder you choose.";

    // Start / database
    pub const NEW_DATABASE: &str =
        "A database is a folder holding fintrack.sqlite and an imports/ archive. Pick where to create it and give it a name.";
    pub const OPEN_DATABASE: &str = "Choose an existing database folder (the one containing fintrack.sqlite).";
    pub const DEFAULT_DATABASE: &str = "The database opened automatically when FinTrack starts.";
    pub const DEMO: &str = "Opens a built-in sample household (18 months of transactions, rules, budgets and a mortgage) in a temporary folder. Explore and change anything; it is deleted when you exit the demo or close FinTrack, and never touches your own databases.";
    pub const DEMO_IMPORT: &str = "Re-imports the bank exports the demo was built from. Every row is already in the database, so you'll see duplicate detection in action.";
    pub const BACKUP: &str = "Writes a consistent snapshot of the whole database to a single .sqlite file you choose.";

    // Import
    pub const IMPORT: &str = "Import CSV exports from your bank. The format and account are detected automatically; overlapping files are de-duplicated.";
    pub const FORMAT_DETECT: &str =
        "Formats are recognised by their header row (or, for header-less files, by column count and parseable values).";
    pub const HEADER_ROW: &str = "The row containing column names. Rows above it (bank preambles) are ignored. Choose 'No header' for files that start directly with data.";
    pub const DATE_FORMAT: &str = "How dates are written in the file. 'Automatic' tries common formats; choose one explicitly if day and month could be confused.";
    pub const AMOUNT_MODE: &str = "Single column: one signed amount. Debit/credit: separate columns for money out and money in.";
    pub const NEGATE: &str = "Tick when the bank exports money out as positive numbers (common for credit cards).";
    pub const DIRECTION_COL: &str =
        "Optional: a column whose text says whether a row is money out (e.g. 'Debit'). Overrides the sign of the amount.";
    pub const SKIP_FILTER: &str = "Optional: skip rows whose chosen column contains this text, e.g. 'Pending' transactions that will later post with a different description.";
    pub const ACCOUNT_COL: &str =
        "Optional: a column with the account or card number. Rows are grouped by it and matched to accounts automatically.";
    pub const MEMO_COLS: &str =
        "Extra description columns (sub-description, memo, details). They are searchable and rules can match them.";
    pub const ACCOUNT_DETECT: &str = "Detected from the account number in the file, the account's filename hint, or the format the account usually uses. Change it if it's wrong — FinTrack learns the account number for next time.";
    pub const DUPLICATES: &str = "A row is a duplicate when the account already holds a transaction with the same date, amount and description. Identical rows within one file (two coffees on the same day) are all kept.";
    pub const UNDO_IMPORT: &str = "Deletes every transaction added by this import. Manual categories and notes on them are lost.";

    // Categorization
    pub const RULES: &str = "Rules map transaction text to a category. They run in priority order (lowest first); the first match wins. Manual overrides always take precedence.";
    pub const RULE_PRIORITY: &str = "Lower numbers run first. Give specific rules (e.g. 'SAVE ON FOODS GAS') a lower number than general ones ('SAVE ON FOODS').";
    pub const RULE_MATCH: &str = "contains / starts with / equals ignore case and extra spaces. 'matches regex' uses a case-insensitive regular expression.";
    pub const RULE_AMOUNT: &str =
        "Optional bounds on the absolute amount, e.g. only categorize Costco purchases over $200 as Household.";
    pub const UNMAPPED: &str =
        "Transactions with no category. Similar descriptions are grouped so a single rule can categorize them all.";
    pub const OVERRIDE: &str = "Choosing a category here sets a manual override for this transaction only. Pick 'Automatic' to go back to the rule result.";
    pub const REAPPLY: &str =
        "Re-runs every rule over every transaction. Only automatic categories change; manual overrides are kept.";

    // Categories / accounts
    pub const CATEGORY_KIND: &str = "Expense and Income categories feed the analytics. Transfer categories (credit-card payments, moving money between your own accounts) are excluded so they aren't counted twice.";
    pub const CATEGORY_DELETE: &str = "Deleting removes the category and its subcategories. Optionally move their transactions, rules and budget to another category first.";
    pub const ACCOUNT_NUMBER: &str = "Full number or at least the last 4 digits. Matched against account-number columns in CSV files (masked numbers like 5191****1234 work).";
    pub const FILENAME_HINT: &str = "Text that appears in this account's export file names, e.g. 'visa' for 'Visa_2026-03.csv'. Used for detection when the file has no account number.";
    pub const OPENING_BALANCE: &str = "Balance just before the first imported transaction. Used to compute balances and net worth. Use a negative number for money owed on credit cards.";

    // Analytics / budgets
    pub const GRANULARITY: &str = "Size of each time bucket: weekly (Monday start), monthly, quarterly or annual.";
    pub const BUCKETING: &str = "What each series represents: top-level categories (Groceries), subcategories (Groceries / Save on Foods), accounts or payees.";
    pub const FLOW: &str =
        "Spending = money out in Expense categories (refunds reduce it). Income = money in. Transfers are always excluded.";
    pub const TOP_N: &str = "Maximum number of series drawn; smaller ones are combined into 'Other'.";
    pub const SAVINGS_RATE: &str = "Net (income − spending) as a share of income.";
    pub const RECURRING: &str =
        "Payees charged at a regular interval (weekly to annual) with a consistent amount, still active recently.";
    pub const BUDGET: &str = "Monthly amounts per category. Quarterly and annual views multiply by 3 and 12. A top-level budget includes all its subcategories.";
    pub const BUDGET_SUGGEST: &str = "Set the budget to your average monthly amount over the last 3 complete months.";

    // Mortgage
    pub const MORTGAGE_RATE: &str = "Nominal annual rate. Canadian fixed-rate mortgages compound semi-annually; FinTrack uses (1 + rate/2)^(2/n) − 1 per payment.";
    pub const MORTGAGE_FREQ: &str = "Accelerated frequencies pay half (bi-weekly) or a quarter (weekly) of the monthly payment, which pays the mortgage off years sooner.";
    pub const MORTGAGE_OVERRIDE: &str = "Leave at 0 to use the calculated payment. Enter your actual payment if your lender's figure differs or you pay extra every time.";
    pub const MORTGAGE_START: &str = "Date the mortgage was funded. The first payment is one payment period later.";
    pub const PREPAYMENT: &str = "Lump-sum payments against principal. They shorten the amortization; the regular payment stays the same. Untick to exclude one from the projection.";
    pub const RATE_CHANGE: &str =
        "From this date the new rate applies (e.g. at renewal) and the payment is recalculated over the remaining amortization.";
    pub const EXTRA_PAYMENT: &str = "What-if: add this amount to every regular payment. Not saved — use it to explore scenarios.";
}

/// A long-form help topic.
pub struct Topic {
    pub title: &'static str,
    pub body: &'static str,
}

/// Topics shown on the Help page, in reading order.
pub const TOPICS: &[Topic] = &[
    Topic {
        title: "Getting started",
        body: "Just looking? Click 'Enter demo mode' on the start screen (or File ▸ Try the demo) to explore a sample household with 18 months of data. The demo lives in a temporary folder and is deleted when you exit it; it never touches your own databases.\n\
1. Create a database (File ▸ New database). A database is a folder on your computer; nothing is ever sent over the network.\n\
2. Import a CSV export from your bank (Import CSV). FinTrack detects the file format and the account. If the account doesn't exist yet you can create it right in the import screen.\n\
3. Categorize. Open Mapping rules ▸ Unmapped to see uncategorized transactions grouped by payee and create rules for them. Rules apply to past and future imports.\n\
4. Explore Dashboard, Analytics and Budgets.\n\
5. Optionally set this database as the default in Settings so it opens automatically.",
    },
    Topic {
        title: "Databases and where data is stored",
        body: "Each database is a folder containing fintrack.sqlite (all your data) and imports/ (a copy of every CSV you imported, if archiving is enabled in Settings). You can keep several databases, e.g. one per household, and switch with File ▸ Open database. The Settings page chooses the default database and whether it opens at startup.\n\
Back up with File ▸ Backup database, or simply copy the folder while FinTrack is closed.\n\
Settings themselves are stored in your user profile (see the path on the Settings page).",
    },
    Topic {
        title: "Importing bank CSV files",
        body: "Use Import CSV ▸ Choose CSV files, or drag files onto the window. Several files can be queued.\n\
Format detection: each saved format has a header signature (or, for header-less files, a column count). Built-in templates cover Scotiabank and BMO exports; bank layouts change occasionally, so every template can be edited.\n\
Unknown formats: FinTrack guesses a mapping and shows the column mapper. Choose the header row, which columns hold the date, description and amount, and optional memo / account number / balance columns. The live preview shows exactly how rows will be read. Save the format and it will be recognised automatically next time.\n\
Account detection: the account is chosen from (1) an account-number column, (2) the account's filename hint, or (3) the only account that uses this format. You can always change it before importing; FinTrack remembers the account number it saw.\n\
Duplicates: rows already present in the account (same date, amount and description) are skipped, so you can import overlapping date ranges safely. Two identical rows inside the same file are both kept, because they are genuinely separate purchases.\n\
Undo: Import history lists every import; Undo removes all transactions it added.",
    },
    Topic {
        title: "Categories",
        body: "Categories have two levels: a top-level category (Groceries, Income, Housing) and optional subcategories (Groceries / Save on Foods, Income / Mayfair). Transactions can be assigned to either level.\n\
Each top-level category has a kind: Expense, Income or Transfer. Subcategories inherit it. Use Transfer for money moving between your own accounts (credit-card payments, savings contributions) so it isn't counted as spending or income.\n\
Manage categories on the Categories page: add, rename, move a subcategory to another parent, or delete (optionally moving its transactions, rules and budget elsewhere first).",
    },
    Topic {
        title: "Mapping rules, unmapped transactions and overrides",
        body: "A rule says: when a transaction's description (or memo) contains / starts with / equals / matches a regex, assign this category. Rules can also be restricted to money in or out, an amount range, or one account.\n\
Rules run in priority order (lowest number first) and the first match wins. Rules are re-applied automatically whenever you add, edit or delete one, and to every new import.\n\
Unmapped: lists transactions without a category, grouped by payee. Click Create rule to make a rule pre-filled with the payee text, or Assign to set a one-off category for that group.\n\
Overrides: on the Transactions page, choosing a category for a transaction sets a manual override that rules never change. Choose 'Automatic' to remove the override. Overridden transactions show a ✋ marker.",
    },
    Topic {
        title: "Transactions",
        body: "Filter by date range, account, category (including 'Uncategorized'), text and manual overrides. Totals for the filtered set are shown above the table.\n\
Select rows with the checkboxes for bulk actions: set category, clear overrides, or delete. Use ✏ to add a note, and ➕ to create a rule from a transaction. Export writes the filtered rows to a CSV file.",
    },
    Topic {
        title: "Analytics",
        body: "Choose a date range, a granularity (weekly, monthly, quarterly, annual) and a bucketing (top-level category, subcategory, account or payee).\n\
Cash flow: income vs spending per period with net and savings rate.\n\
Breakdown: stacked spending (or income) per period by bucket, a ranking for the whole range, and a pivot table with totals and averages. Click a row label to see its transactions.\n\
Trends: one bucket over time with its average.\n\
Payees: the largest payees in the range.\n\
Recurring: detected subscriptions and bills with their monthly cost.\n\
Balances: account balances and net worth over time (set opening balances on the Accounts page for accurate figures).\n\
Transfers are excluded from income and spending. Uncategorized transactions are counted as spending or income by their sign.",
    },
    Topic {
        title: "Budgets",
        body: "Set a monthly amount for any category. Top-level budgets include their subcategories. Switch between monthly, quarterly and annual views (budgets scale by 3 and 12) and step through periods with the arrows.\n\
Progress bars show status with colour and text: on track (under 85%), close (85–100%), over budget. 'Suggest' sets the budget to your average over the last 3 complete months.",
    },
    Topic {
        title: "Mortgage tracker",
        body: "Add a mortgage with its original principal, rate, amortization, start date and payment frequency. FinTrack uses Canadian semi-annual compounding.\n\
Progress shows where the mortgage stands today according to the schedule: payments made, principal repaid, interest paid and balance.\n\
Prepayments: add lump sums with a date and amount. The projection immediately shows the new payoff date, total interest and interest saved compared with the baseline (no prepayments). Untick a prepayment to exclude it and compare scenarios. The 'extra per payment' field explores increasing every payment without saving anything.\n\
Rate changes: add the new rate at each renewal; the payment is recalculated over the remaining amortization.\n\
The chart compares baseline and scenario balances; the schedule can be viewed per year or per payment.",
    },
    Topic {
        title: "Privacy and security",
        body: "FinTrack has no networking code and no networking libraries; it never sends or receives data over the internet. All data stays in your database folder and settings file. The database is not encrypted, so store it somewhere protected (e.g. an encrypted drive) if others use your computer.",
    },
];
