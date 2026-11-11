//! All user-facing help text in one place: short tooltips (`tips`) used next
//! to controls, and long-form topics shown on the in-app Help page.
//!
//! Topic bodies separate paragraphs with a blank line (`\n\n`).

/// Short hover texts shown next to controls.
pub mod tips {
    // Import
    pub const IMPORT: &str = "Import CSV exports from your bank. The format and account are \
        detected automatically; overlapping files are de-duplicated.";
    pub const DEMO_IMPORT: &str = "Re-imports the bank exports the demo was built from. Every \
        row is already in the database, so you'll see duplicate detection in action.";
    pub const FORMAT_DETECT: &str = "Formats are recognised by their header row (or, for \
        header-less files, by column count and parseable values).";
    pub const HEADER_ROW: &str = "The row containing column names. Rows above it (bank \
        preambles) are ignored. Choose 'No header row' for files that start with data.";
    pub const DATE_FORMAT: &str = "How dates are written in the file. 'Automatic' tries common \
        formats; choose one explicitly if day and month could be confused.";
    pub const AMOUNT_MODE: &str = "One signed column, or separate debit (money out) and credit \
        (money in) columns.";
    pub const NEGATE: &str = "Tick when the bank exports money out as positive numbers (common \
        for credit cards).";
    pub const DIRECTION_COL: &str = "Optional: a column whose text says whether a row is money \
        out (e.g. 'Debit'). Overrides the sign of the amount.";
    pub const SKIP_FILTER: &str = "Optional: skip rows whose chosen column contains this text, \
        e.g. 'Pending' transactions that later post with a different description.";
    pub const ACCOUNT_COL: &str = "Optional: a column with the account or card number. Rows are \
        grouped by it and matched to accounts automatically.";
    pub const MEMO_COLS: &str = "Extra description columns (sub-description, memo, details). \
        They are searchable and rules can match them.";
    pub const ACCOUNT_DETECT: &str = "Detected from the account number in the file, the \
        account's filename hint, or the format the account usually uses. Change it if it's \
        wrong; FinTrack learns the account number for next time.";
    pub const DUPLICATES: &str = "A row is a duplicate when the account already holds a \
        transaction with the same date, amount and description. Identical rows within one file \
        (two coffees on the same day) are all kept.";
    pub const UNDO_IMPORT: &str = "Deletes every transaction added by this import. Manual \
        categories and notes on them are lost.";

    // Categorization
    pub const RULES: &str = "Rules map transaction text to a category. They run in priority \
        order (lowest first); the first match wins. Manual overrides always take precedence.";
    pub const RULE_PRIORITY: &str = "Lower numbers run first. Give specific rules (e.g. 'SAVE ON \
        FOODS GAS') a lower number than general ones ('SAVE ON FOODS').";
    pub const RULE_MATCH: &str = "contains / starts with / equals ignore case and extra spaces. \
        'matches regex' uses a case-insensitive regular expression.";
    pub const RULE_AMOUNT: &str = "Optional bounds on the absolute amount, e.g. only categorize \
        Costco purchases over $200 as Household.";
    pub const UNMAPPED: &str = "Transactions with no category. Similar descriptions are grouped \
        so a single rule can categorize them all.";
    pub const OVERRIDE: &str = "Choosing a category for a transaction sets a manual override for \
        it alone. Choose 'Automatic' to go back to the rule result.";
    pub const REAPPLY: &str = "Re-runs every rule over every transaction. Only automatic \
        categories change; manual overrides are kept.";

    // Categories / accounts
    pub const CATEGORY_KIND: &str = "Expense and Income categories feed the analytics. Transfer \
        categories (credit-card payments, moving money between your own accounts) are excluded \
        so they aren't counted twice.";
    pub const CATEGORY_DELETE: &str = "Deleting removes the category and its subcategories. \
        Optionally move their transactions, rules and budget to another category first.";
    pub const ACCOUNT_NUMBER: &str = "Full number or at least the last 4 digits. Matched against \
        account-number columns in CSV files (masked numbers like 5191****1234 work).";
    pub const FILENAME_HINT: &str = "Text that appears in this account's export file names, e.g. \
        'visa' for 'Visa_2026-03.csv'. Used when the file has no account number.";
    pub const OPENING_BALANCE: &str = "Balance just before the first imported transaction, used \
        for balances and net worth. Use a negative number for money owed on credit cards.";

    // Analytics / budgets
    pub const GRANULARITY: &str = "Size of each time bucket: weekly (Monday start), monthly, \
        quarterly or annual.";
    pub const BUCKETING: &str = "What each series represents: top-level categories (Groceries), \
        subcategories (Groceries / Save on Foods), accounts or payees.";
    pub const FLOW: &str = "Spending = money out in Expense categories (refunds reduce it). \
        Income = money in. Transfers are always excluded.";
    pub const TOP_N: &str = "Number of series drawn; smaller ones are combined into 'Other'.";
    pub const SAVINGS_RATE: &str = "Net (income − spending) as a share of income.";
    pub const RECURRING: &str = "Payees charged at a regular interval (weekly to annual) with a \
        consistent amount, and still active recently.";
    pub const BUDGET: &str = "Monthly amounts per category. Quarterly and annual views multiply \
        by 3 and 12. A top-level budget includes all its subcategories.";
    pub const BUDGET_SUGGEST: &str = "Set the budget to your average monthly amount over the \
        last 3 complete months.";

    // Mortgage
    pub const MORTGAGE_RATE: &str = "Nominal annual rate. Canadian fixed-rate mortgages compound \
        semi-annually; FinTrack uses (1 + rate/2)^(2/n) − 1 per payment.";
    pub const MORTGAGE_FREQ: &str = "Accelerated frequencies pay half (bi-weekly) or a quarter \
        (weekly) of the monthly payment, which pays the mortgage off years sooner.";
    pub const MORTGAGE_OVERRIDE: &str = "Leave empty to use the calculated payment. Enter your \
        actual payment if your lender's figure differs or you pay extra every time.";
    pub const MORTGAGE_START: &str = "Date the mortgage was funded. The first payment is one \
        payment period later.";
    pub const PREPAYMENT: &str = "Lump-sum payments against principal. They shorten the \
        amortization; the regular payment stays the same. Exclude one to compare scenarios.";
    pub const RATE_CHANGE: &str = "From this date the new rate applies (e.g. at renewal) and the \
        payment is recalculated over the remaining amortization.";
    pub const EXTRA_PAYMENT: &str = "What-if: add this amount to every regular payment. Not \
        saved; use it to explore scenarios.";
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
        body: "FinTrack runs on a server in your home and you use it from a browser. An admin \
            creates an account for each person and decides which households they can open.\n\n\
            1. Open your household from the menu at the top. (Admins can create a demo \
            household to explore first.)\n\n\
            2. Import a CSV export from your bank (Import CSV). FinTrack detects the file format \
            and the account; you can create the account right in the import screen.\n\n\
            3. Categorize. The Categorize page lists uncategorized transactions grouped by \
            payee; create rules for them. Rules apply to past and future imports.\n\n\
            4. Explore the Dashboard, Analytics, Budgets and Mortgage pages.",
    },
    Topic {
        title: "Households, people and permissions",
        body: "A household is a separate set of accounts, transactions, categories, rules, \
            budgets and mortgages. Each person signs in with their own username and password.\n\n\
            Admins manage people and households (Manage users, Manage households and the Access \
            tab) and can open every household. Everyone else sees only the households an admin \
            gives them: View only (dashboards, transactions and reports) or Can edit (also \
            import, categorize, budgets and the mortgage).\n\n\
            Choose your theme, the household that opens when you sign in, and your password on \
            My account.",
    },
    Topic {
        title: "Importing bank CSV files",
        body: "Upload one or more CSV files on the Import CSV page. Several files are imported \
            one after another.\n\n\
            Format detection: each saved format has a header signature (or, for header-less \
            files, a column count). Built-in templates cover Scotiabank and BMO exports; bank \
            layouts change occasionally, so every format can be edited.\n\n\
            Unknown formats: FinTrack guesses a mapping and shows the column mapper. Choose the \
            header row and which columns hold the date, description and amount, plus optional \
            memo, account number and balance columns. Preview shows exactly how rows will be \
            read. Save the format and it's recognised automatically next time.\n\n\
            Account detection: the account is chosen from (1) an account-number column, (2) the \
            account's filename hint, or (3) the only account that uses this format. You can \
            change it before importing; FinTrack remembers the account number it saw.\n\n\
            Duplicates: rows already in the account (same date, amount and description) are \
            skipped, so overlapping date ranges are safe. Two identical rows in the same file \
            are both kept, because they are genuinely separate purchases.\n\n\
            Undo: Import history lists every import; Undo removes all transactions it added.",
    },
    Topic {
        title: "Categories",
        body: "Categories have two levels: a top-level category (Groceries, Income, Housing) and \
            optional subcategories (Groceries / Save on Foods, Income / Mayfair). Transactions \
            can be assigned to either level.\n\n\
            Each top-level category has a kind: Expense, Income or Transfer, and subcategories \
            inherit it. Use Transfer for money moving between your own accounts (credit-card \
            payments, savings contributions) so it isn't counted as spending or income.\n\n\
            On the Categories page you can add, rename, move a subcategory to another parent, \
            or delete (optionally moving its transactions, rules and budget elsewhere first).",
    },
    Topic {
        title: "Mapping rules, unmapped transactions and overrides",
        body: "A rule says: when a transaction's description (or memo) contains / starts with / \
            equals / matches a regex, assign this category. Rules can also be limited to money \
            in or out, an amount range, or one account.\n\n\
            Rules run in priority order (lowest number first) and the first match wins. They \
            are re-applied whenever you add, edit or delete one, and to every new import.\n\n\
            Unmapped lists transactions without a category, grouped by payee. Create rule makes \
            a rule pre-filled with the payee text; Assign once sets a one-off category for that \
            group.\n\n\
            Overrides: on the Transactions page, choosing a category for a transaction sets a \
            manual override that rules never change. Choose 'Automatic' to remove it. \
            Overridden transactions show a ✋ marker.",
    },
    Topic {
        title: "Transactions",
        body: "Filter by date range, account, category (including 'Uncategorized'), text and \
            manual overrides. Totals for the filtered set are shown above the table, and the \
            page address can be bookmarked or shared.\n\n\
            Tick rows for bulk actions: set category, clear overrides, or delete. Details shows \
            everything about a transaction and lets you add a note; + Rule creates a rule from \
            it. Export CSV downloads the filtered rows.",
    },
    Topic {
        title: "Analytics",
        body: "Choose a date range, a period size (weekly, monthly, quarterly, annual) and a \
            grouping (top-level category, subcategory, account or payee).\n\n\
            Cash flow: income vs spending per period with net and savings rate. Breakdown: \
            stacked spending (or income) per period, a ranking for the whole range, and a pivot \
            table; click a name to see its transactions. Trends: one group over time with its \
            average. Payees: the largest payees. Recurring: detected subscriptions and bills. \
            Balances: account balances and net worth over time.\n\n\
            Transfers are excluded from income and spending. Uncategorized transactions count \
            as spending or income by their sign.\n\n\
            Charts: hover a bar or point to see every series at that period; click it to open \
            the transactions behind it; click a legend entry to hide or show that series. \
            “Show as table” under each chart lists the same numbers. The controls update the \
            page as you change them.",
    },
    Topic {
        title: "Budgets",
        body: "Set a monthly amount for any category. Top-level budgets include their \
            subcategories. Switch between monthly, quarterly and annual views (budgets scale by \
            3 and 12) and step through periods with the arrows.\n\n\
            Progress bars show status with colour and text: on track (under 85%), close to \
            limit (85–100%), over budget. Suggest sets the budget to your average over the last \
            3 complete months.",
    },
    Topic {
        title: "Mortgage tracker",
        body: "Add a mortgage with its original principal, rate, amortization, start date and \
            payment frequency. FinTrack uses Canadian semi-annual compounding.\n\n\
            Progress shows where the mortgage stands today according to the schedule: payments \
            made, principal repaid, interest paid and balance.\n\n\
            Prepayments: add lump sums with a date and amount. The projection shows the new \
            payoff date, total interest and interest saved compared with the baseline (no \
            prepayments). Exclude a prepayment to compare scenarios. 'Extra per payment' \
            explores raising every payment without saving anything.\n\n\
            Rate changes: add the new rate at each renewal; the payment is recalculated over \
            the remaining amortization.",
    },
    Topic {
        title: "Privacy and security",
        body: "FinTrack runs entirely on your own server and never contacts the internet: no \
            analytics, no fonts or scripts from other sites. Every page needs a signed-in \
            account, and households are only visible to the people an admin allows.\n\n\
            Passwords are stored as Argon2id hashes. Repeated wrong passwords slow further \
            attempts. Forms only work from FinTrack's own pages, so other websites can't act \
            on your behalf. Use HTTPS (your reverse proxy) so passwords aren't sent in the \
            clear on your network.\n\n\
            The household databases on the server are not encrypted; protect the server and \
            its backups.",
    },
];
