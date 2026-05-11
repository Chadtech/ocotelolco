use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct Analysis {
    pub source_files: Vec<PathBuf>,
    pub transaction_source_files: Vec<PathBuf>,
    pub income_source_files: Vec<PathBuf>,
    pub summaries: Vec<TickerSummary>,
    pub transaction_count: usize,
    pub income_count: usize,
    pub unattributed_income_count: usize,
    pub ignored_non_trade_count: usize,
    pub matched_sell_count: usize,
    pub unmatched_sell_count: usize,
}

#[derive(Clone, Debug)]
pub struct TickerSummary {
    pub symbol: String,
    pub buy_quantity: f64,
    pub buy_cost: f64,
    pub sell_quantity: f64,
    pub sell_proceeds: f64,
    pub matched_quantity: f64,
    pub matched_cost: f64,
    pub matched_proceeds: f64,
    pub realized_gain: f64,
    pub income: f64,
    pub unmatched_sell_quantity: f64,
    pub open_quantity: f64,
    pub open_cost: f64,
}

impl TickerSummary {
    pub fn total_gain(&self) -> f64 {
        self.realized_gain + self.income
    }

    pub fn realized_return(&self) -> Option<f64> {
        if self.matched_cost > 0.0 {
            Some(self.total_gain() / self.matched_cost)
        } else {
            None
        }
    }

    pub fn is_closed_performance(&self) -> bool {
        self.matched_quantity > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Buy,
    Sell,
}

#[derive(Clone, Debug)]
struct Transaction {
    row_index: usize,
    date_key: u32,
    action: Action,
    symbol: String,
    quantity: f64,
    amount: f64,
}

#[derive(Clone, Debug)]
struct Lot {
    quantity: f64,
    unit_cost: f64,
}

#[derive(Default)]
struct TickerAccumulator {
    buy_quantity: f64,
    buy_cost: f64,
    sell_quantity: f64,
    sell_proceeds: f64,
    matched_quantity: f64,
    matched_cost: f64,
    matched_proceeds: f64,
    realized_gain: f64,
    income: f64,
    unmatched_sell_quantity: f64,
}

struct IncomeParseResult {
    records: Vec<IncomeRecord>,
}

#[derive(Clone, Debug)]
struct IncomeRecord {
    date_key: u32,
    symbol: String,
    activity: IncomeActivity,
    amount: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum IncomeActivity {
    CashDividend,
    QualifiedDividend,
    PriorYearNonQualifiedDividend,
    BankInterest,
}

enum CsvFileKind {
    Transactions,
    InvestmentIncome,
}

pub fn analyze_default_data_dir() -> io::Result<Analysis> {
    analyze_data_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("imports/schwab"))
}

pub fn print_cli_report() -> io::Result<()> {
    let analysis = analyze_default_data_dir()?;
    let report = render_report(&analysis);
    print!("{report}");

    let output_path = write_default_report(&report)?;
    println!("Wrote report to {}", output_path.display());

    Ok(())
}

fn write_default_report(report: &str) -> io::Result<PathBuf> {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("outputs");
    fs::create_dir_all(&output_dir)?;

    let output_path = output_dir.join(format!("schwab-analysis-{}.txt", unix_timestamp()?));
    fs::write(&output_path, report)?;
    Ok(output_path)
}

fn unix_timestamp() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(io::Error::other)
}

fn render_report(analysis: &Analysis) -> String {
    let mut realized = analysis
        .summaries
        .iter()
        .filter(|summary| summary.is_closed_performance())
        .collect::<Vec<_>>();
    realized.sort_by(|left, right| {
        right
            .realized_return()
            .unwrap_or(0.0)
            .total_cmp(&left.realized_return().unwrap_or(0.0))
            .then(left.symbol.cmp(&right.symbol))
    });
    let realized_count = realized.len();
    let profitable_count = realized
        .iter()
        .filter(|summary| summary.total_gain() >= 0.0)
        .count();
    let losing_count = realized_count - profitable_count;
    let open_only_count = analysis
        .summaries
        .iter()
        .filter(|summary| !summary.is_closed_performance() && summary.open_quantity > 0.0)
        .count();
    let total_buy_cost = analysis
        .summaries
        .iter()
        .map(|summary| summary.buy_cost)
        .sum::<f64>();
    let total_sell_proceeds = analysis
        .summaries
        .iter()
        .map(|summary| summary.sell_proceeds)
        .sum::<f64>();
    let total_buy_quantity = analysis
        .summaries
        .iter()
        .map(|summary| summary.buy_quantity)
        .sum::<f64>();
    let total_sell_quantity = analysis
        .summaries
        .iter()
        .map(|summary| summary.sell_quantity)
        .sum::<f64>();
    let total_open_cost = analysis
        .summaries
        .iter()
        .map(|summary| summary.open_cost)
        .sum::<f64>();
    let total_unmatched_sell_quantity = analysis
        .summaries
        .iter()
        .map(|summary| summary.unmatched_sell_quantity)
        .sum::<f64>();
    let total_income = analysis
        .summaries
        .iter()
        .map(|summary| summary.income)
        .sum::<f64>();
    let _matched_proceeds = analysis
        .summaries
        .iter()
        .map(|summary| summary.matched_proceeds)
        .sum::<f64>();

    let mut report = String::new();
    push_report_line(
        &mut report,
        format_args!(
        "Analyzed {} trade transaction(s) from {} transaction CSV file(s), {} total CSV file(s)",
        analysis.transaction_count,
        analysis.transaction_source_files.len(),
        analysis.source_files.len()
        ),
    );
    push_report_line(
        &mut report,
        format_args!(
        "Included {} unique income row(s) from {} transaction CSV file(s) and {} income CSV file(s), attributed income: {}",
        analysis.income_count,
        analysis.transaction_source_files.len(),
        analysis.income_source_files.len(),
        money(total_income)
        ),
    );
    if analysis.unattributed_income_count > 0 {
        push_report_line(
            &mut report,
            format_args!(
                "Skipped {} income row(s) without a ticker symbol",
                analysis.unattributed_income_count
            ),
        );
    }
    push_report_line(
        &mut report,
        format_args!(
            "Ignored {} non-buy/sell row(s)",
            analysis.ignored_non_trade_count
        ),
    );
    push_report_line(
        &mut report,
        format_args!(
            "Bought {} share(s)/unit(s) for {} | Sold {} share(s)/unit(s) for {}",
            quantity(total_buy_quantity),
            money(total_buy_cost),
            quantity(total_sell_quantity),
            money(total_sell_proceeds)
        ),
    );
    push_report_line(
        &mut report,
        format_args!(
            "Matched sells: {} | Unmatched sells needing earlier basis: {}",
            analysis.matched_sell_count, analysis.unmatched_sell_count
        ),
    );
    push_report_line(
        &mut report,
        format_args!(
            "Open cost from in-period buys: {} | Unmatched sell quantity: {}",
            money(total_open_cost),
            quantity(total_unmatched_sell_quantity)
        ),
    );
    push_report_line(
        &mut report,
        format_args!(
        "Realized tickers: {realized_count} ({profitable_count} profitable, {losing_count} losing) | Open-only tickers: {open_only_count}"
        ),
    );
    report.push('\n');
    push_report_line(
        &mut report,
        format_args!(
            "{:<8} {:>12} {:>12} {:>12}",
            "Ticker", "Return", "P/L", "Income"
        ),
    );

    for summary in realized {
        push_report_line(
            &mut report,
            format_args!(
                "{:<8} {:>12} {:>12} {:>12}",
                summary.symbol,
                summary
                    .realized_return()
                    .map(percent)
                    .unwrap_or_else(|| "n/a".to_string()),
                money(summary.total_gain()),
                money(summary.income)
            ),
        );
    }

    report
}

fn push_report_line(report: &mut String, arguments: std::fmt::Arguments<'_>) {
    use std::fmt::Write as _;

    report
        .write_fmt(arguments)
        .expect("writing to a String should not fail");
    report.push('\n');
}

pub fn analyze_data_dir(path: impl AsRef<Path>) -> io::Result<Analysis> {
    let mut source_files = csv_files(path.as_ref())?;
    source_files.sort();

    let mut transactions = Vec::new();
    let mut income_records = Vec::new();
    let mut transaction_source_files = Vec::new();
    let mut income_source_files = Vec::new();
    let mut ignored_non_trade_count = 0;
    for source_file in &source_files {
        match classify_csv_file(source_file)? {
            CsvFileKind::Transactions => {
                let TransactionsParseResult {
                    transactions: mut source_transactions,
                    income_records: mut source_income_records,
                    ignored_non_trade_count: source_ignored_non_trade_count,
                } = read_transactions(source_file)?;
                transactions.append(&mut source_transactions);
                income_records.append(&mut source_income_records);
                ignored_non_trade_count += source_ignored_non_trade_count;
                transaction_source_files.push(source_file.clone());
            }
            CsvFileKind::InvestmentIncome => {
                let income = read_investment_income(source_file)?;
                income_records.extend(income.records);
                income_source_files.push(source_file.clone());
            }
        }
    }
    let IncomeAggregation {
        income_by_symbol,
        income_count,
        unattributed_income_count,
    } = aggregate_income(income_records);

    Ok(analyze_transactions(
        source_files,
        transaction_source_files,
        income_source_files,
        transactions,
        income_by_symbol,
        income_count,
        unattributed_income_count,
        ignored_non_trade_count,
    ))
}

fn analyze_transactions(
    source_files: Vec<PathBuf>,
    transaction_source_files: Vec<PathBuf>,
    income_source_files: Vec<PathBuf>,
    mut transactions: Vec<Transaction>,
    income_by_symbol: HashMap<String, f64>,
    income_count: usize,
    unattributed_income_count: usize,
    ignored_non_trade_count: usize,
) -> Analysis {
    transactions.sort_by_key(|transaction| (transaction.date_key, transaction.row_index));

    let transaction_count = transactions.len();
    let mut lots_by_symbol: HashMap<String, VecDeque<Lot>> = HashMap::new();
    let mut summaries_by_symbol: HashMap<String, TickerAccumulator> = HashMap::new();
    let mut matched_sell_count = 0;
    let mut unmatched_sell_count = 0;

    for transaction in transactions {
        let summary = summaries_by_symbol
            .entry(transaction.symbol.clone())
            .or_default();

        match transaction.action {
            Action::Buy => {
                let cost = -transaction.amount;

                summary.buy_quantity += transaction.quantity;
                summary.buy_cost += cost;
                lots_by_symbol
                    .entry(transaction.symbol)
                    .or_default()
                    .push_back(Lot {
                        quantity: transaction.quantity,
                        unit_cost: cost / transaction.quantity,
                    });
            }
            Action::Sell => {
                let proceeds = transaction.amount;

                summary.sell_quantity += transaction.quantity;
                summary.sell_proceeds += proceeds;

                let mut remaining = transaction.quantity;
                let unit_proceeds = proceeds / transaction.quantity;
                let lots = lots_by_symbol.entry(transaction.symbol).or_default();
                let mut matched_any = false;

                while remaining > 0.000_000_1 {
                    let Some(lot) = lots.front_mut() else {
                        break;
                    };

                    let matched_quantity = remaining.min(lot.quantity);
                    let matched_cost = matched_quantity * lot.unit_cost;
                    let matched_proceeds = matched_quantity * unit_proceeds;

                    summary.matched_quantity += matched_quantity;
                    summary.matched_cost += matched_cost;
                    summary.matched_proceeds += matched_proceeds;
                    summary.realized_gain += matched_proceeds - matched_cost;
                    matched_any = true;

                    lot.quantity -= matched_quantity;
                    remaining -= matched_quantity;
                    if lot.quantity <= 0.000_000_1 {
                        lots.pop_front();
                    }
                }

                if matched_any {
                    matched_sell_count += 1;
                }
                if remaining > 0.000_000_1 {
                    summary.unmatched_sell_quantity += remaining;
                    unmatched_sell_count += 1;
                }
            }
        }
    }

    for (symbol, income) in income_by_symbol {
        summaries_by_symbol.entry(symbol).or_default().income += income;
    }

    let mut summaries = summaries_by_symbol
        .into_iter()
        .map(|(symbol, summary)| {
            let lots = lots_by_symbol.remove(&symbol).unwrap_or_default();
            let open_quantity = lots.iter().map(|lot| lot.quantity).sum();
            let open_cost = lots
                .iter()
                .map(|lot| lot.quantity * lot.unit_cost)
                .sum::<f64>();

            TickerSummary {
                symbol,
                buy_quantity: summary.buy_quantity,
                buy_cost: summary.buy_cost,
                sell_quantity: summary.sell_quantity,
                sell_proceeds: summary.sell_proceeds,
                matched_quantity: summary.matched_quantity,
                matched_cost: summary.matched_cost,
                matched_proceeds: summary.matched_proceeds,
                realized_gain: summary.realized_gain,
                income: summary.income,
                unmatched_sell_quantity: summary.unmatched_sell_quantity,
                open_quantity,
                open_cost,
            }
        })
        .collect::<Vec<_>>();

    summaries.sort_by(|left, right| {
        right
            .realized_gain
            .total_cmp(&left.realized_gain)
            .then(left.symbol.cmp(&right.symbol))
    });

    Analysis {
        source_files,
        transaction_source_files,
        income_source_files,
        summaries,
        transaction_count,
        income_count,
        unattributed_income_count,
        ignored_non_trade_count,
        matched_sell_count,
        unmatched_sell_count,
    }
}

fn csv_files(import_dir: &Path) -> io::Result<Vec<PathBuf>> {
    if !import_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_csv_files(import_dir, &mut files)?;
    Ok(files)
}

fn collect_csv_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_csv_files(&path, files)?;
        } else if path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
        {
            files.push(path);
        }
    }

    Ok(())
}

fn classify_csv_file(path: &Path) -> io::Result<CsvFileKind> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.lines();
    let Some(first_line) = lines.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is empty", path.display()),
        ));
    };

    let first_row =
        parse_csv_row(first_line).map_err(|error| invalid_csv(path, 1, "row", error))?;
    if row_has_columns(
        &first_row,
        &["Date", "Action", "Symbol", "Quantity", "Amount"],
    ) {
        return Ok(CsvFileKind::Transactions);
    }
    if row_has_columns(
        &first_row,
        &["Transaction Date", "Symbol", "Transaction Amount"],
    ) {
        return Ok(CsvFileKind::InvestmentIncome);
    }
    if first_row
        .first()
        .is_some_and(|field| field.starts_with("Investment Income Transactions"))
    {
        return Ok(CsvFileKind::InvestmentIncome);
    }

    let Some(second_line) = lines.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} does not contain a recognized Schwab CSV header",
                path.display()
            ),
        ));
    };
    let second_row =
        parse_csv_row(second_line).map_err(|error| invalid_csv(path, 2, "row", error))?;
    if row_has_columns(
        &second_row,
        &["Transaction Date", "Symbol", "Transaction Amount"],
    ) {
        return Ok(CsvFileKind::InvestmentIncome);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{} does not contain a recognized Schwab CSV header",
            path.display()
        ),
    ))
}

fn row_has_columns(row: &[String], columns: &[&str]) -> bool {
    columns
        .iter()
        .all(|column| row.iter().any(|field| field.eq_ignore_ascii_case(column)))
}

struct TransactionsParseResult {
    transactions: Vec<Transaction>,
    income_records: Vec<IncomeRecord>,
    ignored_non_trade_count: usize,
}

fn read_transactions(path: &Path) -> io::Result<TransactionsParseResult> {
    let contents = fs::read_to_string(path)?;
    let mut rows = contents.lines();
    let Some(header) = rows.next() else {
        return Ok(TransactionsParseResult {
            transactions: Vec::new(),
            income_records: Vec::new(),
            ignored_non_trade_count: 0,
        });
    };
    let header = parse_csv_row(header).map_err(|error| invalid_csv(path, 1, "header", error))?;
    let index = HeaderIndex::from_header(&header)?;

    let mut transactions = Vec::new();
    let mut income_records = Vec::new();
    let mut ignored_non_trade_count = 0;
    for (row_index, row) in rows.enumerate() {
        if row.trim().is_empty() {
            continue;
        }
        let line_number = row_index + 2;
        let fields =
            parse_csv_row(row).map_err(|error| invalid_csv(path, line_number, "row", error))?;
        let action = get_field(&fields, index.action, "Action")
            .map_err(|error| invalid_csv(path, line_number, "Action", error))?;
        if is_income_action(action) {
            let income = IncomeRecord::from_transaction_fields(&fields, &index)
                .map_err(|error| invalid_csv(path, line_number, "income", error))?;
            income_records.push(income);
            continue;
        }
        if !is_trade_action(action) {
            ignored_non_trade_count += 1;
            continue;
        }
        let transaction = Transaction::from_fields(row_index, &fields, &index)
            .map_err(|error| invalid_csv(path, line_number, "transaction", error))?;
        transactions.push(transaction);
    }
    Ok(TransactionsParseResult {
        transactions,
        income_records,
        ignored_non_trade_count,
    })
}

fn read_investment_income(path: &Path) -> io::Result<IncomeParseResult> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.lines().enumerate();
    let mut header = None;

    for (line_index, line) in lines.by_ref() {
        if line.trim().is_empty() {
            continue;
        }
        let row =
            parse_csv_row(line).map_err(|error| invalid_csv(path, line_index + 1, "row", error))?;
        if row_has_columns(&row, &["Transaction Date", "Symbol", "Transaction Amount"]) {
            header = Some((line_index + 1, IncomeHeaderIndex::from_header(&row)?));
            break;
        }
    }

    let Some((_header_line_number, index)) = header else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is missing investment income header", path.display()),
        ));
    };

    let mut records = Vec::new();

    for (line_index, row) in lines {
        if row.trim().is_empty() {
            continue;
        }
        let line_number = line_index + 1;
        let fields =
            parse_csv_row(row).map_err(|error| invalid_csv(path, line_number, "row", error))?;
        let income = IncomeTransaction::from_fields(&fields, &index)
            .map_err(|error| invalid_csv(path, line_number, "income", error))?;
        records.push(income.into_record());
    }

    Ok(IncomeParseResult { records })
}

struct IncomeAggregation {
    income_by_symbol: HashMap<String, f64>,
    income_count: usize,
    unattributed_income_count: usize,
}

fn aggregate_income(records: Vec<IncomeRecord>) -> IncomeAggregation {
    let mut seen = HashSet::new();
    let mut income_by_symbol = HashMap::new();
    let mut income_count = 0;
    let mut unattributed_income_count = 0;

    for record in records {
        if !seen.insert(record.key()) {
            continue;
        }

        income_count += 1;
        if record.symbol == "NO NUMBER" || record.symbol.is_empty() {
            unattributed_income_count += 1;
            continue;
        }
        *income_by_symbol.entry(record.symbol).or_insert(0.0) += record.amount;
    }

    IncomeAggregation {
        income_by_symbol,
        income_count,
        unattributed_income_count,
    }
}

struct HeaderIndex {
    date: usize,
    action: usize,
    symbol: usize,
    quantity: usize,
    amount: usize,
}

impl HeaderIndex {
    fn from_header(header: &[String]) -> io::Result<Self> {
        let find = |name: &str| {
            header
                .iter()
                .position(|field| field.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("missing required CSV column: {name}"),
                    )
                })
        };

        Ok(Self {
            date: find("Date")?,
            action: find("Action")?,
            symbol: find("Symbol")?,
            quantity: find("Quantity")?,
            amount: find("Amount")?,
        })
    }
}

struct IncomeHeaderIndex {
    date: usize,
    symbol: usize,
    activity: usize,
    amount: usize,
}

impl IncomeHeaderIndex {
    fn from_header(header: &[String]) -> io::Result<Self> {
        let find = |name: &str| {
            header
                .iter()
                .position(|field| field.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("missing required CSV column: {name}"),
                    )
                })
        };

        Ok(Self {
            date: find("Transaction Date")?,
            symbol: find("Symbol")?,
            activity: find("Transaction Type")?,
            amount: find("Transaction Amount")?,
        })
    }
}

struct IncomeTransaction {
    date_key: u32,
    symbol: String,
    activity: IncomeActivity,
    amount: f64,
}

impl IncomeTransaction {
    fn from_fields(fields: &[String], index: &IncomeHeaderIndex) -> Result<Self, String> {
        let date_key = parse_date_key(get_field(fields, index.date, "Transaction Date")?)?;
        let symbol = get_field(fields, index.symbol, "Symbol")?
            .trim()
            .to_string();
        let activity =
            parse_income_activity(get_field(fields, index.activity, "Transaction Type")?)?;
        let amount = parse_money(
            get_field(fields, index.amount, "Transaction Amount")?,
            "Transaction Amount",
        )?;

        Ok(Self {
            date_key,
            symbol,
            activity,
            amount,
        })
    }

    fn into_record(self) -> IncomeRecord {
        IncomeRecord {
            date_key: self.date_key,
            symbol: self.symbol,
            activity: self.activity,
            amount: self.amount,
        }
    }
}

impl IncomeRecord {
    fn from_transaction_fields(fields: &[String], index: &HeaderIndex) -> Result<Self, String> {
        let date_key = parse_date_key(get_field(fields, index.date, "Date")?)?;
        let symbol = get_field(fields, index.symbol, "Symbol")?
            .trim()
            .to_string();
        let activity = parse_income_activity(get_field(fields, index.action, "Action")?)?;
        let amount = parse_money(get_field(fields, index.amount, "Amount")?, "Amount")?;
        if amount <= 0.0 {
            return Err(format!("income Amount must be positive, got {amount}"));
        }

        Ok(Self {
            date_key,
            symbol,
            activity,
            amount,
        })
    }

    fn key(&self) -> (u32, String, IncomeActivity, i64) {
        (
            self.date_key,
            self.symbol.clone(),
            self.activity.clone(),
            (self.amount * 100.0).round() as i64,
        )
    }
}

impl Transaction {
    fn from_fields(
        row_index: usize,
        fields: &[String],
        index: &HeaderIndex,
    ) -> Result<Self, String> {
        let action = parse_action(get_field(fields, index.action, "Action")?)?;
        let symbol = get_field(fields, index.symbol, "Symbol")?
            .trim()
            .to_string();
        if symbol.is_empty() {
            return Err("Symbol must not be empty".to_string());
        }

        let quantity = parse_number(get_field(fields, index.quantity, "Quantity")?, "Quantity")?;
        if quantity <= 0.0 {
            return Err(format!("Quantity must be positive, got {quantity}"));
        }

        let amount = parse_money(get_field(fields, index.amount, "Amount")?, "Amount")?;
        match action {
            Action::Buy if amount >= 0.0 => {
                return Err(format!("Buy Amount must be negative, got {amount}"));
            }
            Action::Sell if amount <= 0.0 => {
                return Err(format!("Sell Amount must be positive, got {amount}"));
            }
            _ => {}
        }

        Ok(Self {
            row_index,
            date_key: parse_date_key(get_field(fields, index.date, "Date")?)?,
            action,
            symbol,
            quantity,
            amount,
        })
    }
}

fn get_field<'a>(fields: &'a [String], index: usize, name: &str) -> Result<&'a str, String> {
    fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing field {name} at column index {index}"))
}

fn is_trade_action(action: &str) -> bool {
    matches!(action.trim(), "Buy" | "Sell")
}

fn is_income_action(action: &str) -> bool {
    parse_income_activity(action).is_ok()
}

fn parse_action(action: &str) -> Result<Action, String> {
    match action.trim() {
        "Buy" => Ok(Action::Buy),
        "Sell" => Ok(Action::Sell),
        _ => Err(format!("unrecognized action: {}", action)),
    }
}

fn parse_income_activity(action: &str) -> Result<IncomeActivity, String> {
    match action.trim() {
        "Cash Dividend" => Ok(IncomeActivity::CashDividend),
        "Qualified Dividend" => Ok(IncomeActivity::QualifiedDividend),
        "Pr Yr Non-Qual Div" => Ok(IncomeActivity::PriorYearNonQualifiedDividend),
        "Bank Interest" => Ok(IncomeActivity::BankInterest),
        other => Err(format!("unrecognized income activity: {other}")),
    }
}

fn parse_date_key(date: &str) -> Result<u32, String> {
    let date = date
        .trim()
        .split_once(" as of ")
        .map_or(date.trim(), |(_, as_of_date)| as_of_date.trim());
    let mut parts = date.trim().split('/');
    let month = parse_date_part(parts.next(), "month", date)?;
    let day = parse_date_part(parts.next(), "day", date)?;
    let year = parse_date_part(parts.next(), "year", date)?;
    if parts.next().is_some() {
        return Err(format!("Date has too many parts: {date}"));
    }
    Ok(year * 10_000 + month * 100 + day)
}

fn parse_date_part(part: Option<&str>, name: &str, date: &str) -> Result<u32, String> {
    part.ok_or_else(|| format!("Date is missing {name}: {date}"))?
        .parse::<u32>()
        .map_err(|error| format!("Date has invalid {name} in {date}: {error}"))
}

fn parse_money(value: &str, field_name: &str) -> Result<f64, String> {
    parse_number(&value.replace(['$', ','], ""), field_name)
}

fn parse_number(value: &str, field_name: &str) -> Result<f64, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }

    let number = value
        .parse::<f64>()
        .map_err(|error| format!("{field_name} is not a valid number ({value}): {error}"))?;
    if !number.is_finite() {
        return Err(format!("{field_name} must be finite, got {value}"));
    }
    Ok(number)
}

fn parse_csv_row(row: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = row.chars().peekable();
    let mut quoted = false;

    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(field);
                field = String::new();
            }
            _ => field.push(character),
        }
    }
    if quoted {
        return Err("unterminated quoted field".to_string());
    }
    fields.push(field);
    Ok(fields)
}

fn invalid_csv(path: &Path, line_number: usize, field: &str, error: String) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}:{line_number} {field}: {error}", path.display()),
    )
}

fn money(value: f64) -> String {
    if value < 0.0 {
        format!("-${:.2}", -value)
    } else {
        format!("${:.2}", value)
    }
}

fn percent(value: f64) -> String {
    let percent = value * 100.0;
    if percent.abs() < 0.05 {
        "0.0%".to_string()
    } else {
        format!("{percent:.1}%")
    }
}

fn quantity(value: f64) -> String {
    format!("{:.4}", clean_zero(value))
}

fn clean_zero(value: f64) -> f64 {
    if value.abs() < 0.000_05 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_csv_fields() {
        assert_eq!(
            parse_csv_row("\"Date\",\"Description\",\"Amount\"\n".trim()).unwrap(),
            vec!["Date", "Description", "Amount"]
        );
        assert_eq!(
            parse_csv_row("\"04/28/2026\",\"A, B ETF\",\"$1,107.17\"").unwrap(),
            vec!["04/28/2026", "A, B ETF", "$1,107.17"]
        );
    }

    #[test]
    fn rejects_malformed_csv_rows() {
        assert!(parse_csv_row("\"04/28/2026\",\"Buy").is_err());
    }

    #[test]
    fn rejects_invalid_trade_fields() {
        let index = HeaderIndex {
            date: 0,
            action: 1,
            symbol: 2,
            quantity: 3,
            amount: 4,
        };
        let fields = vec![
            "04/28/2026".to_string(),
            "Buy".to_string(),
            "VTI".to_string(),
            "".to_string(),
            "-$100.00".to_string(),
        ];

        let error = Transaction::from_fields(0, &fields, &index).unwrap_err();
        assert!(error.contains("Quantity must not be empty"));
    }

    #[test]
    fn parses_transaction_income_rows() {
        let path = std::env::temp_dir().join(format!(
            "ocotelolco-transactions-test-{}.csv",
            std::process::id()
        ));
        fs::write(
            &path,
            "\"Date\",\"Action\",\"Symbol\",\"Description\",\"Quantity\",\"Price\",\"Fees & Comm\",\"Amount\"\n\
             \"04/28/2026\",\"Cash Dividend\",\"VTI\",\"Dividend\",\"\",\"\",\"\",\"$1.00\"\n\
             \"04/28/2026\",\"Journal\",\"\",\"Non trade\",\"\",\"\",\"\",\"$1.00\"\n\
             \"04/28/2026\",\"Buy\",\"VTI\",\"ETF\",\"1\",\"$100.00\",\"\",\"-$100.00\"\n",
        )
        .unwrap();

        let result = read_transactions(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(result.transactions.len(), 1);
        assert_eq!(result.income_records.len(), 1);
        assert_eq!(result.income_records[0].symbol, "VTI");
        assert_eq!(result.income_records[0].amount, 1.0);
        assert_eq!(result.ignored_non_trade_count, 1);
    }

    #[test]
    fn parses_investment_income_rows() {
        let path =
            std::env::temp_dir().join(format!("ocotelolco-income-test-{}.CSV", std::process::id()));
        fs::write(
            &path,
            "\"Investment Income Transactions as of 05/09/2026 19:20:11 ET\"\n\
             Transaction Date,Account Number,Account Name,Account Type,Security Description,Symbol,Security Type,Transaction Type,Transaction Amount,Income Type,\n\
             \"04/07/2026\",\"...061\",\"Individual\",\"BROKERAGE\",\"iShares 20+ Year Treasury Bond ETF\",\"TLT\",\"ETFs & Closed End Funds\",\"Cash Dividend\",\"1.03\",\"Received\",\n\
             \"04/15/2026\",\"...061\",\"Individual\",\"BROKERAGE\",\"Cash & Money Market\",\"NO NUMBER\",\"Cash & Money Market\",\"Bank Interest\",\"0.02\",\"Received\",\n",
        )
        .unwrap();

        assert!(matches!(
            classify_csv_file(&path).unwrap(),
            CsvFileKind::InvestmentIncome
        ));
        let result = read_investment_income(&path).unwrap();
        let aggregation = aggregate_income(result.records);
        fs::remove_file(path).unwrap();

        assert_eq!(aggregation.income_count, 2);
        assert_eq!(aggregation.unattributed_income_count, 1);
        assert_eq!(aggregation.income_by_symbol.get("TLT"), Some(&1.03));
    }

    #[test]
    fn calculates_fifo_realized_performance() {
        let transactions = vec![
            transaction(1, Action::Buy, "ABC", 10.0, -100.0),
            transaction(2, Action::Buy, "ABC", 10.0, -120.0),
            transaction(3, Action::Sell, "ABC", 15.0, 180.0),
        ];

        let analysis = analyze_test_transactions(transactions, HashMap::new());
        let summary = analysis
            .summaries
            .iter()
            .find(|summary| summary.symbol == "ABC")
            .unwrap();

        assert_eq!(summary.matched_quantity, 15.0);
        assert_eq!(summary.matched_cost, 160.0);
        assert_eq!(summary.realized_gain, 20.0);
        assert_eq!(summary.open_quantity, 5.0);
        assert_eq!(summary.open_cost, 60.0);
    }

    #[test]
    fn leaves_unmatched_sells_out_of_performance() {
        let analysis = analyze_test_transactions(
            vec![transaction(1, Action::Sell, "ABC", 4.0, 48.0)],
            HashMap::new(),
        );
        let summary = &analysis.summaries[0];

        assert_eq!(summary.matched_quantity, 0.0);
        assert_eq!(summary.unmatched_sell_quantity, 4.0);
        assert_eq!(summary.realized_return(), None);
    }

    #[test]
    fn includes_income_in_ticker_return() {
        let transactions = vec![
            transaction(1, Action::Buy, "ABC", 10.0, -100.0),
            transaction(2, Action::Sell, "ABC", 10.0, 110.0),
        ];
        let mut income = HashMap::new();
        income.insert("ABC".to_string(), 5.0);

        let analysis = analyze_test_transactions(transactions, income);
        let summary = &analysis.summaries[0];

        assert_eq!(summary.realized_gain, 10.0);
        assert_eq!(summary.income, 5.0);
        assert_eq!(summary.total_gain(), 15.0);
        assert_eq!(summary.realized_return(), Some(0.15));
    }

    #[test]
    fn includes_deduped_income_with_partial_fifo_lots() {
        let transactions = vec![
            transaction(1, Action::Buy, "ABC", 10.0, -100.0),
            transaction(2, Action::Buy, "ABC", 5.0, -75.0),
            transaction(3, Action::Sell, "ABC", 12.0, 156.0),
            transaction(4, Action::Buy, "XYZ", 4.0, -200.0),
        ];
        let income = aggregate_income(vec![
            income_record(20260115, "ABC", IncomeActivity::CashDividend, 4.0),
            income_record(20260115, "ABC", IncomeActivity::CashDividend, 4.0),
            income_record(20260215, "ABC", IncomeActivity::QualifiedDividend, 1.0),
            income_record(20260215, "XYZ", IncomeActivity::CashDividend, 3.0),
            income_record(20260215, "NO NUMBER", IncomeActivity::BankInterest, 0.5),
        ]);

        assert_eq!(income.income_count, 4);
        assert_eq!(income.unattributed_income_count, 1);

        let analysis = analyze_test_transactions(transactions, income.income_by_symbol);
        let abc = analysis
            .summaries
            .iter()
            .find(|summary| summary.symbol == "ABC")
            .unwrap();
        let xyz = analysis
            .summaries
            .iter()
            .find(|summary| summary.symbol == "XYZ")
            .unwrap();

        assert_eq!(abc.matched_quantity, 12.0);
        assert_eq!(abc.matched_cost, 130.0);
        assert_eq!(abc.realized_gain, 26.0);
        assert_eq!(abc.income, 5.0);
        assert_eq!(abc.total_gain(), 31.0);
        assert!((abc.realized_return().unwrap() - (31.0 / 130.0)).abs() < 0.000_000_1);
        assert_eq!(abc.open_quantity, 3.0);
        assert_eq!(abc.open_cost, 45.0);

        assert_eq!(xyz.income, 3.0);
        assert_eq!(xyz.realized_return(), None);
        assert_eq!(xyz.open_quantity, 4.0);
    }

    fn analyze_test_transactions(
        transactions: Vec<Transaction>,
        income_by_symbol: HashMap<String, f64>,
    ) -> Analysis {
        analyze_transactions(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            transactions,
            income_by_symbol,
            0,
            0,
            0,
        )
    }

    fn income_record(
        date_key: u32,
        symbol: &str,
        activity: IncomeActivity,
        amount: f64,
    ) -> IncomeRecord {
        IncomeRecord {
            date_key,
            symbol: symbol.to_string(),
            activity,
            amount,
        }
    }

    fn transaction(
        row_index: usize,
        action: Action,
        symbol: &str,
        quantity: f64,
        amount: f64,
    ) -> Transaction {
        Transaction {
            row_index,
            date_key: 20260101,
            action,
            symbol: symbol.to_string(),
            quantity,
            amount,
        }
    }
}
