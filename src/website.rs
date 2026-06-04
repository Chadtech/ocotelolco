use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;

const PERFORMANCE_START: Date = Date::new(2025, 10, 28);
const PERFORMANCE_END: Date = Date::new(2026, 4, 28);

pub fn default_output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("outputs")
        .join("ocotelolco.html")
}

pub fn write_site(output_path: impl AsRef<Path>) -> io::Result<()> {
    let output_path = output_path.as_ref();
    let view = load_site_view()?;
    let html = render_site(&view);

    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, html)
}

fn load_site_view() -> io::Result<SiteView> {
    let chart = load_performance_chart(
        default_sp500_data_path(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("imports")
            .join("schwab")
            .join("balances"),
    )?;
    SiteView::from_chart(&chart)
}

fn default_sp500_data_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spreadsheets")
        .join("sp500_daily_2021-05-14_to_2026-05-13.csv")
}

fn render_site(view: &SiteView) -> String {
    format!("<!doctype html>\n{}\n", site_document(view).to_html())
}

fn load_performance_chart(
    sp500_path: impl AsRef<Path>,
    balance_dir: impl AsRef<Path>,
) -> io::Result<PerformanceChart> {
    let sp500_values = read_sp500_values(sp500_path.as_ref())?;
    let balance_values = read_balance_values(balance_dir.as_ref())?;

    build_performance_chart(
        sp500_values,
        balance_values,
        PERFORMANCE_START,
        PERFORMANCE_END,
    )
}

fn build_performance_chart(
    sp500_values: Vec<ValuePoint>,
    balance_values: Vec<ValuePoint>,
    requested_start: Date,
    requested_end: Date,
) -> io::Result<PerformanceChart> {
    let account_first = first_date_in_range(&balance_values, requested_start, requested_end)
        .ok_or_else(|| missing_data("account balance", requested_start, requested_end))?;
    let account_last = last_date_in_range(&balance_values, requested_start, requested_end)
        .ok_or_else(|| missing_data("account balance", requested_start, requested_end))?;
    let sp500_first = first_date_in_range(&sp500_values, requested_start, requested_end)
        .ok_or_else(|| missing_data("S&P 500", requested_start, requested_end))?;
    let sp500_last = last_date_in_range(&sp500_values, requested_start, requested_end)
        .ok_or_else(|| missing_data("S&P 500", requested_start, requested_end))?;

    let actual_start = account_first.max(sp500_first);
    let actual_end = account_last.min(sp500_last).min(requested_end);
    if actual_start > actual_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "account balance and S&P 500 data do not overlap between {} and {}",
                requested_start.display_long(),
                requested_end.display_long()
            ),
        ));
    }

    let account_points =
        build_return_points("account balance", &balance_values, actual_start, actual_end)?;
    let sp500_points = build_return_points("S&P 500", &sp500_values, actual_start, actual_end)?;
    let actual_start = account_points
        .first()
        .map(|point| point.date)
        .zip(sp500_points.first().map(|point| point.date))
        .map(|(account_date, sp500_date)| account_date.max(sp500_date))
        .unwrap_or(actual_start);
    let actual_end = account_points
        .last()
        .map(|point| point.date)
        .zip(sp500_points.last().map(|point| point.date))
        .map(|(account_date, sp500_date)| account_date.min(sp500_date))
        .unwrap_or(actual_end);
    let account_points = trim_return_points(account_points, actual_start, actual_end);
    let sp500_points = trim_return_points(sp500_points, actual_start, actual_end);
    if account_points.is_empty() || sp500_points.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "account balance and S&P 500 data did not produce comparable chart points",
        ));
    }

    Ok(PerformanceChart {
        actual_start,
        actual_end,
        account_return_pct: account_points
            .last()
            .map(|point| point.return_pct)
            .unwrap_or(0.0),
        sp500_return_pct: sp500_points
            .last()
            .map(|point| point.return_pct)
            .unwrap_or(0.0),
        account_points,
        sp500_points,
    })
}

fn read_sp500_values(path: &Path) -> io::Result<Vec<ValuePoint>> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.lines();
    let Some(header_line) = lines.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is empty", path.display()),
        ));
    };
    let header = parse_csv_row(header_line).map_err(|error| invalid_csv(path, 1, error))?;
    let date_index = find_column(&header, "date", path)?;
    let value_index = find_column(&header, "value", path)?;

    let mut points = Vec::new();
    for (row_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let line_number = row_index + 2;
        let row = parse_csv_row(line).map_err(|error| invalid_csv(path, line_number, error))?;
        let date = Date::parse_iso(get_field(&row, date_index, "date")?)
            .map_err(|error| invalid_csv(path, line_number, error))?;
        let value = get_field(&row, value_index, "value")?.trim();
        if value.is_empty() {
            continue;
        }

        let value = value
            .parse::<f64>()
            .map_err(|error| invalid_csv(path, line_number, format!("invalid value: {error}")))?;
        points.push(ValuePoint { date, value });
    }

    points.sort_by_key(|point| point.date);
    Ok(points)
}

fn read_balance_values(directory: &Path) -> io::Result<Vec<ValuePoint>> {
    let mut values_by_date = BTreeMap::new();
    for path in csv_files(directory)? {
        for point in read_balance_history_file(&path)? {
            values_by_date.insert(point.date, point.value);
        }
    }

    let points = values_by_date
        .into_iter()
        .map(|(date, value)| ValuePoint { date, value })
        .collect::<Vec<_>>();
    if points.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} does not contain a Schwab balance history CSV with Date and Amount columns",
                directory.display()
            ),
        ));
    }

    Ok(points)
}

fn read_balance_history_file(path: &Path) -> io::Result<Vec<ValuePoint>> {
    let contents = fs::read_to_string(path)?;
    let mut rows = contents.lines().enumerate();
    let mut header = None;

    for (line_index, line) in rows.by_ref() {
        if line.trim().is_empty() {
            continue;
        }

        let row = parse_csv_row(line).map_err(|error| invalid_csv(path, line_index + 1, error))?;
        if row_has_columns(&row, &["Date", "Amount"]) {
            header = Some((
                line_index + 1,
                find_column(&row, "Date", path)?,
                find_column(&row, "Amount", path)?,
            ));
            break;
        }
    }

    let Some((_header_line, date_index, amount_index)) = header else {
        return Ok(Vec::new());
    };

    let mut points = Vec::new();
    for (line_index, line) in rows {
        if line.trim().is_empty() {
            continue;
        }

        let row = parse_csv_row(line).map_err(|error| invalid_csv(path, line_index + 1, error))?;
        let date = Date::parse_us(get_field(&row, date_index, "Date")?)
            .map_err(|error| invalid_csv(path, line_index + 1, error))?;
        let amount = parse_money(get_field(&row, amount_index, "Amount")?)
            .map_err(|error| invalid_csv(path, line_index + 1, error))?;
        points.push(ValuePoint {
            date,
            value: amount,
        });
    }

    Ok(points)
}

fn csv_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !directory.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(csv_files(&path)?);
        } else if path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn first_date_in_range(points: &[ValuePoint], start: Date, end: Date) -> Option<Date> {
    points
        .iter()
        .find(|point| point.date >= start && point.date <= end)
        .map(|point| point.date)
}

fn last_date_in_range(points: &[ValuePoint], start: Date, end: Date) -> Option<Date> {
    points
        .iter()
        .rev()
        .find(|point| point.date >= start && point.date <= end)
        .map(|point| point.date)
}

fn build_return_points(
    label: &str,
    values: &[ValuePoint],
    start: Date,
    end: Date,
) -> io::Result<Vec<ReturnPoint>> {
    let points = values
        .iter()
        .filter(|point| point.date >= start && point.date <= end)
        .copied()
        .collect::<Vec<_>>();
    let Some(baseline) = points.first().map(|point| point.value) else {
        return Err(missing_data(label, start, end));
    };
    if baseline == 0.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} baseline value is zero on {}", start.display_long()),
        ));
    }

    Ok(points
        .into_iter()
        .map(|point| ReturnPoint {
            date: point.date,
            value: point.value,
            return_pct: ((point.value / baseline) - 1.0) * 100.0,
        })
        .collect())
}

fn trim_return_points(points: Vec<ReturnPoint>, start: Date, end: Date) -> Vec<ReturnPoint> {
    points
        .into_iter()
        .filter(|point| point.date >= start && point.date <= end)
        .collect()
}

fn missing_data(label: &str, start: Date, end: Date) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "missing {label} data between {} and {}",
            start.display_long(),
            end.display_long()
        ),
    )
}

fn find_column(header: &[String], name: &str, path: &Path) -> io::Result<usize> {
    header
        .iter()
        .position(|field| field.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is missing required column {name}", path.display()),
            )
        })
}

fn row_has_columns(row: &[String], columns: &[&str]) -> bool {
    columns
        .iter()
        .all(|column| row.iter().any(|field| field.eq_ignore_ascii_case(column)))
}

fn get_field<'a>(row: &'a [String], index: usize, name: &str) -> io::Result<&'a str> {
    row.get(index)
        .map(String::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing {name} field")))
}

fn invalid_csv(path: &Path, line_number: usize, error: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} line {}: {error}", path.display(), line_number),
    )
}

fn parse_csv_row(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(field);
                field = String::new();
            }
            _ => field.push(character),
        }
    }

    if in_quotes {
        return Err("unterminated quoted field".to_string());
    }

    fields.push(field);
    Ok(fields)
}

fn parse_money(value: &str) -> Result<f64, String> {
    let mut value = value.trim();
    let negative_parentheses = value.starts_with('(') && value.ends_with(')');
    if negative_parentheses {
        value = &value[1..value.len() - 1];
    }

    let normalized = value.replace(['$', ','], "");
    let parsed = normalized
        .parse::<f64>()
        .map_err(|error| format!("invalid money amount {value:?}: {error}"))?;
    if negative_parentheses {
        Ok(-parsed)
    } else {
        Ok(parsed)
    }
}

fn percent(value: f64) -> String {
    format!("{value:.2}%")
}

const SITE_CSS: &str = r##"
    :root {
      color-scheme: dark;
      --green-1: #030907;
      --green-2: #071d10;
      --green-3: #082208;
      --green-7: #0aca1a;
      --gray-1: #131610;
      --gray-2: #2c2826;
      --gray-3: #57524f;
      --gray-5: #b0a69a;
      --gray-6: #e0d6ca;
      --yellow-6: #e3d34b;
      --chart-account: var(--yellow-6);
      --chart-index: var(--gray-6);
      --chart-axis: #b0a69a;
      --chart-zero: var(--gray-5);
    }

    * {
      box-sizing: border-box;
    }

    h1,
    h2,
    p {
      margin: 0;
    }

    body {
      margin: 0;
      min-height: 100vh;
      color: var(--gray-6);
      background: var(--green-3);
      font-family: "Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    }

    body::before {
      position: fixed;
      inset: 0;
      z-index: -1;
      background: url("../ocotelolco_bg.png") center / cover;
      content: "";
      opacity: 0.34;
    }

    .relative { position: relative; }
    .block { display: block; }
    .flex { display: flex; }
    .grid { display: grid; }
    .flex-1 { flex: 1; }
    .flex-col { flex-direction: column; }
    .flex-none { flex: none; }
    .items-center { align-items: center; }
    .items-start { align-items: start; }
    .grid-cols-1 { grid-template-columns: minmax(0, 1fr); }
    .grid-cols-return { grid-template-columns: 1fr minmax(110px, max-content); }
    .w-full { width: 100%; }
    .max-w-window { max-width: 1120px; }
    .max-w-copy { max-width: 70ch; }
    .min-h-screen { min-height: 100vh; }
    .min-h-window { min-height: min(720px, calc(100vh - 64px)); }
    .min-h-window-inner { min-height: calc(min(720px, calc(100vh - 64px)) - 8px); }
    .min-h-7 { min-height: 28px; }
    .min-h-chart { min-height: 300px; }
    .min-h-svg { min-height: 260px; }
    .min-h-row { min-height: 46px; }
    .min-h-row-header { min-height: 40px; }
    .mx-auto { margin-right: auto; margin-left: auto; }
    .mt-1 { margin-top: 4px; }
    .mt-4 { margin-top: 16px; }
    .mb-2 { margin-bottom: 8px; }
    .mb-4 { margin-bottom: 16px; }
    .p-1 { padding: 4px; }
    .p-2 { padding: 8px; }
    .p-3 { padding: 12px; }
    .p-4 { padding: 16px; }
    .p-5 { padding: 20px; }
    .p-8 { padding: 32px; }
    .px-2 { padding-right: 8px; padding-left: 8px; }
    .px-3 { padding-right: 12px; padding-left: 12px; }
    .py-1 { padding-top: 4px; padding-bottom: 4px; }
    .py-2 { padding-top: 8px; padding-bottom: 8px; }
    .gap-1 { gap: 4px; }
    .gap-2 { gap: 8px; }
    .gap-4 { gap: 16px; }
    .overflow-visible { overflow: visible; }
    .bg-current { background: currentColor; }
    .bg-gray-2 { background: var(--gray-2); }
    .bg-gray-5 { background: var(--gray-5); }
    .bg-green-1 { background: var(--green-1); }
    .bg-green-2 { background: var(--green-2); }
    .text-gray-5 { color: var(--gray-5); }
    .text-gray-6 { color: var(--gray-6); }
    .text-green-1 { color: var(--green-1); }
    .text-green-7 { color: var(--green-7); }
    .text-chart-account { color: var(--chart-account); }
    .text-chart-index { color: var(--chart-index); }
    .text-base { font-size: 1rem; }
    .font-normal { font-weight: 400; }
    .leading-titlebar { line-height: 1.3; }
    .leading-copy { line-height: 1.65; }
    .leading-body { line-height: 1.45; }
    .text-right { text-align: right; }
    .square-3 { width: 14px; height: 14px; }
    .border-b-dotted { border-bottom: 1px dotted var(--gray-3); }
    .border-b-solid { border-bottom: 1px solid var(--gray-3); }
    .shadow-outset {
      box-shadow: inset 2px 2px 0 var(--gray-3), inset -2px -2px 0 var(--gray-1);
    }
    .shadow-inset {
      box-shadow: inset 2px 2px 0 var(--gray-1), inset -2px -2px 0 var(--gray-3);
    }

    .axis-label {
      fill: var(--gray-6);
      font: 18px "Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    }

    .chart-background {
      fill: var(--green-2);
    }

    .month-label {
      fill: var(--gray-6);
      font: 16px "Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    }

    .grid-line {
      stroke: var(--gray-3);
      stroke-width: 1.5;
    }

    .zero-line {
      stroke: var(--chart-zero);
      stroke-width: 2.5;
      stroke-dasharray: 3 4;
    }

    .axis-line {
      stroke: var(--chart-axis);
      stroke-width: 1.5;
    }

    .account-line,
    .index-line {
      fill: none;
      stroke-linejoin: round;
      stroke-linecap: round;
      stroke-width: 3;
    }

    .account-line {
      stroke: var(--chart-account);
    }

    .index-line {
      stroke: var(--chart-index);
      stroke-width: 2;
    }

    @media (max-width: 820px) {
      .max-md\:min-h-window { min-height: calc(100vh - 32px); }
      .max-md\:min-h-chart { min-height: 240px; }
      .max-md\:min-h-svg { min-height: 220px; }
      .max-md\:grid-cols-1 { grid-template-columns: minmax(0, 1fr); }
      .max-md\:gap-1 { gap: 4px; }
      .max-md\:p-2 { padding: 8px; }
      .max-md\:p-3 { padding: 12px; }
      .max-md\:p-4 { padding: 16px; }
      .max-md\:text-left { text-align: left; }
    }
"##;

const PERFORMANCE_CHART_SCRIPT: &str = r##"
    (function () {
      const dataElement = document.getElementById("performance-chart-data");
      const svg = document.getElementById("performance-chart");
      if (!dataElement || !svg) return;

      const data = JSON.parse(dataElement.textContent);
      const width = 920;
      const height = 330;
      const margin = { top: 24, right: 74, bottom: 44, left: 20 };
      const plotWidth = width - margin.left - margin.right;
      const plotHeight = height - margin.top - margin.bottom;
      const parseDate = (value) => new Date(value + "T00:00:00");
      const start = parseDate(data.actual_start);
      const end = parseDate(data.actual_end);
      const day = 24 * 60 * 60 * 1000;
      const duration = Math.max(day, end.getTime() - start.getTime());
      const allReturns = data.account_points
        .concat(data.sp500_points)
        .map((point) => point.return_pct);
      const rawMin = Math.min(0, ...allReturns);
      const rawMax = Math.max(0, ...allReturns);
      const yMin = Math.min(-10, Math.floor(rawMin / 10) * 10);
      const yMax = Math.max(30, Math.ceil(rawMax / 10) * 10);
      const ySpan = Math.max(1, yMax - yMin);
      const x = (dateText) => {
        const value = parseDate(dateText).getTime() - start.getTime();
        return margin.left + (value / duration) * plotWidth;
      };
      const y = (returnPct) => {
        return margin.top + ((yMax - returnPct) / ySpan) * plotHeight;
      };
      const line = (points) => points
        .map((point) => `${x(point.date).toFixed(2)},${y(point.return_pct).toFixed(2)}`)
        .join(" ");
      const monthLabel = (date) => {
        const month = date.toLocaleString("en-US", { month: "short" });
        return `${month} ${date.getDate()}`;
      };
      const monthTicks = () => {
        const ticks = [];
        const cursor = new Date(start.getFullYear(), start.getMonth(), 1);
        cursor.setMonth(cursor.getMonth() + 1);
        while (cursor <= end) {
          ticks.push(new Date(cursor));
          cursor.setMonth(cursor.getMonth() + 1);
        }
        return ticks;
      };
      const yTicks = [];
      for (let tick = yMin; tick <= yMax; tick += 10) yTicks.push(tick);

      const nodes = [];
      nodes.push(`<rect class="chart-background" x="0" y="0" width="${width}" height="${height}"></rect>`);
      yTicks.forEach((tick) => {
        const yPosition = y(tick);
        const className = tick === 0 ? "zero-line" : "grid-line";
        nodes.push(`<line class="${className}" x1="${margin.left}" y1="${yPosition.toFixed(2)}" x2="${(width - margin.right).toFixed(2)}" y2="${yPosition.toFixed(2)}"></line>`);
        nodes.push(`<text class="axis-label" x="${(width - margin.right + 14).toFixed(2)}" y="${(yPosition + 6).toFixed(2)}">${tick}%</text>`);
      });
      monthTicks().forEach((tick) => {
        const xPosition = x(tick.toISOString().slice(0, 10));
        nodes.push(`<line class="axis-line" x1="${xPosition.toFixed(2)}" y1="${(height - margin.bottom).toFixed(2)}" x2="${xPosition.toFixed(2)}" y2="${(height - margin.bottom + 12).toFixed(2)}"></line>`);
        nodes.push(`<text class="month-label" x="${xPosition.toFixed(2)}" y="${(height - 10).toFixed(2)}" text-anchor="middle">${monthLabel(tick)}</text>`);
      });
      nodes.push(`<line class="axis-line" x1="${margin.left}" y1="${(height - margin.bottom).toFixed(2)}" x2="${(width - margin.right).toFixed(2)}" y2="${(height - margin.bottom).toFixed(2)}"></line>`);
      nodes.push(`<line class="axis-line" x1="${(width - margin.right).toFixed(2)}" y1="${margin.top}" x2="${(width - margin.right).toFixed(2)}" y2="${(height - margin.bottom).toFixed(2)}"></line>`);
      nodes.push(`<polyline class="account-line" points="${line(data.account_points)}"></polyline>`);
      nodes.push(`<polyline class="index-line" points="${line(data.sp500_points)}"></polyline>`);

      svg.innerHTML = nodes.join("");
    })();
"##;

type Attribute = (String, String);

#[derive(Clone, Debug)]
enum Node {
    Element(ElementNode),
    Text(TextNode),
}

#[derive(Clone, Debug)]
struct ElementNode {
    tag: String,
    attributes: Vec<Attribute>,
    children: Vec<Node>,
}

#[derive(Clone, Debug)]
enum TextNode {
    Escaped(String),
    Raw(String),
}

impl Node {
    fn element(tag: impl Into<String>, attributes: Vec<Attribute>, children: Vec<Node>) -> Self {
        Self::Element(ElementNode {
            tag: tag.into(),
            attributes,
            children,
        })
    }

    fn text(text: impl Into<String>) -> Self {
        Self::Text(TextNode::Escaped(text.into()))
    }

    fn raw_text(text: impl Into<String>) -> Self {
        Self::Text(TextNode::Raw(text.into()))
    }

    fn to_html(&self) -> String {
        match self {
            Self::Element(element) => element.to_html(),
            Self::Text(text) => text.to_html(),
        }
    }
}

impl ElementNode {
    fn to_html(&self) -> String {
        let mut html = String::new();
        html.push('<');
        html.push_str(&self.tag);
        for (name, value) in &self.attributes {
            html.push(' ');
            html.push_str(name);
            html.push_str("=\"");
            html.push_str(&escaped_attribute(value));
            html.push('"');
        }
        html.push('>');

        if is_void_tag(&self.tag) && self.children.is_empty() {
            return html;
        }

        for child in &self.children {
            html.push_str(&child.to_html());
        }

        html.push_str("</");
        html.push_str(&self.tag);
        html.push('>');
        html
    }
}

impl TextNode {
    fn to_html(&self) -> String {
        match self {
            Self::Escaped(text) => escaped_text(text),
            Self::Raw(text) => text.clone(),
        }
    }
}

struct SiteView {
    chart_json: String,
    actual_window: String,
    account_return: String,
    sp500_return: String,
}

impl SiteView {
    fn from_chart(chart: &PerformanceChart) -> io::Result<Self> {
        let chart_data = EmbeddedChartData::from(chart);
        Ok(Self {
            chart_json: serde_json::to_string(&chart_data).map_err(io::Error::other)?,
            actual_window: format!(
                "{} to {}",
                chart.actual_start.display_long(),
                chart.actual_end.display_long()
            ),
            account_return: percent(chart.account_return_pct),
            sp500_return: percent(chart.sp500_return_pct),
        })
    }
}

fn site_document(view: &SiteView) -> Node {
    element(
        "html",
        attrs(&[("lang", "en")]),
        vec![site_head(), site_body(view)],
    )
}

fn site_head() -> Node {
    element(
        "head",
        Vec::new(),
        vec![
            element("meta", attrs(&[("charset", "utf-8")]), Vec::new()),
            element(
                "meta",
                attrs(&[
                    ("name", "viewport"),
                    ("content", "width=device-width, initial-scale=1"),
                ]),
                Vec::new(),
            ),
            element("title", Vec::new(), vec![Node::text("Ocotelolco")]),
            element("style", Vec::new(), vec![Node::raw_text(SITE_CSS)]),
        ],
    )
}

fn site_body(view: &SiteView) -> Node {
    element(
        "body",
        Vec::new(),
        vec![
            element(
                "main",
                attrs(&[("class", "min-h-screen p-8 max-md:p-4")]),
                vec![desktop_window(view)],
            ),
            script(
                attrs(&[
                    ("id", "performance-chart-data"),
                    ("type", "application/json"),
                ]),
                vec![Node::raw_text(view.chart_json.clone())],
            ),
            script(Vec::new(), vec![Node::raw_text(PERFORMANCE_CHART_SCRIPT)]),
        ],
    )
}

fn desktop_window(view: &SiteView) -> Node {
    div(
        attrs(&[
            (
                "class",
                "relative mx-auto min-h-window w-full max-w-window bg-gray-2 p-1 text-gray-6 shadow-outset max-md:min-h-window",
            ),
            ("aria-label", "Ocotelolco website"),
        ]),
        vec![div(
            attrs(&[(
                "class",
                "relative flex min-h-window-inner flex-col bg-gray-2 max-md:min-h-window",
            )]),
            vec![
                element(
                    "header",
                    attrs(&[(
                        "class",
                        "min-h-7 bg-gray-5 px-2 py-1 text-green-1 leading-titlebar",
                    )]),
                    vec![span(
                        Vec::new(),
                        vec![Node::text("Campaign 1 Performance")],
                    )],
                ),
                report_body(view),
            ],
        )],
    )
}

fn report_body(view: &SiteView) -> Node {
    section(
        attrs(&[
            (
                "class",
                "relative mt-1 flex-1 bg-gray-2 p-5 shadow-inset max-md:p-3",
            ),
            ("aria-label", "Report content"),
        ]),
        vec![div(
            attrs(&[("class", "grid gap-4")]),
            vec![intro_section(view), performance_panel(view)],
        )],
    )
}

fn intro_section(view: &SiteView) -> Node {
    section(
        attrs(&[
            ("class", "grid grid-cols-1 items-start gap-4"),
            ("aria-label", "Performance summary"),
        ]),
        vec![div(
            Vec::new(),
            vec![
                element(
                    "h1",
                    attrs(&[(
                        "class",
                        "mb-2 text-base font-normal leading-body text-gray-5",
                    )]),
                    vec![Node::text("Campaign 1 Performance")],
                ),
                p(
                    attrs(&[("class", "max-w-copy text-gray-6 leading-copy")]),
                    vec![Node::text(format!(
                        "Portfolio balance and S&P 500 comparison from {}.",
                        view.actual_window
                    ))],
                ),
            ],
        )],
    )
}

fn performance_panel(view: &SiteView) -> Node {
    section(
        attrs(&[
            (
                "class",
                "relative bg-gray-2 p-4 text-gray-6 shadow-outset max-md:p-2",
            ),
            ("aria-label", "Rate of return chart"),
        ]),
        vec![
            chart_copy(view),
            chart_viewport(),
            return_table(&view.account_return, &view.sp500_return),
        ],
    )
}

fn chart_copy(view: &SiteView) -> Node {
    div(
        attrs(&[("class", "mb-4 grid gap-1")]),
        vec![
            element(
                "h2",
                attrs(&[("class", "text-base font-normal leading-body text-gray-5")]),
                vec![Node::text("Rate of return")],
            ),
            p(
                attrs(&[("class", "text-base leading-body text-gray-6")]),
                vec![
                    Node::text("Your account balance return was "),
                    span(
                        attrs(&[("class", "text-green-7")]),
                        vec![Node::text(view.account_return.clone())],
                    ),
                    Node::text(format!(" from {}.", view.actual_window)),
                ],
            ),
        ],
    )
}

fn chart_viewport() -> Node {
    div(
        attrs(&[(
            "class",
            "relative min-h-chart bg-green-2 p-3 shadow-inset max-md:min-h-chart max-md:p-2",
        )]),
        vec![element(
            "svg",
            attrs(&[
                ("id", "performance-chart"),
                (
                    "class",
                    "block min-h-svg w-full overflow-visible text-gray-6 max-md:min-h-svg",
                ),
                ("role", "img"),
                ("aria-labelledby", "performance-chart-title"),
                ("viewBox", "0 0 920 330"),
                ("preserveAspectRatio", "xMidYMid meet"),
            ]),
            vec![
                element(
                    "title",
                    attrs(&[("id", "performance-chart-title")]),
                    vec![Node::text(
                        "Account balance return compared with S&P 500 return",
                    )],
                ),
                element(
                    "text",
                    attrs(&[("x", "24"), ("y", "40"), ("fill", "currentColor")]),
                    vec![Node::text("Loading chart...")],
                ),
            ],
        )],
    )
}

fn return_table(account_return: &str, sp500_return: &str) -> Node {
    div(
        attrs(&[
            (
                "class",
                "relative mt-4 bg-green-2 text-gray-6 shadow-inset",
            ),
            ("aria-label", "Rate of return table"),
        ]),
        vec![
            div(
                attrs(&[(
                    "class",
                    "grid min-h-row-header grid-cols-return items-center gap-4 border-b-solid bg-green-1 px-3 py-2 text-base text-gray-5 max-md:grid-cols-1 max-md:gap-1",
                )]),
                vec![
                    span(Vec::new(), vec![Node::text("Portfolio/Index")]),
                    span(
                        attrs(&[(
                            "class",
                            "text-right text-gray-5 max-md:text-left",
                        )]),
                        vec![Node::text("Rate of Return")],
                    ),
                ],
            ),
            return_row("account", "Ocotelolco Campaign 1", account_return),
            return_row("index", "S&P 500", sp500_return),
        ],
    )
}

fn return_row(legend_class: &str, name: &str, value: &str) -> Node {
    div(
        attrs(&[(
            "class",
            "grid min-h-row grid-cols-return items-center gap-4 border-b-dotted px-3 py-2 text-base max-md:grid-cols-1 max-md:gap-1",
        )]),
        vec![
            span(
                attrs(&[("class", "flex items-center gap-2")]),
                vec![
                    span(
                        vec![
                            attr(
                                "class",
                                format!(
                                    "square-3 flex-none bg-current text-chart-{legend_class}"
                                ),
                            ),
                            attr("aria-hidden", "true"),
                        ],
                        Vec::new(),
                    ),
                    Node::text(name),
                ],
            ),
            span(
                attrs(&[(
                    "class",
                    "text-right text-green-7 max-md:text-left",
                )]),
                vec![Node::text(value)],
            ),
        ],
    )
}

fn element(tag: &str, attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    Node::element(tag, attributes, children)
}

fn div(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("div", attributes, children)
}

fn span(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("span", attributes, children)
}

fn section(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("section", attributes, children)
}

fn p(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("p", attributes, children)
}

fn script(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("script", attributes, children)
}

fn attr(name: &str, value: impl Into<String>) -> Attribute {
    (name.to_string(), value.into())
}

fn attrs(values: &[(&str, &str)]) -> Vec<Attribute> {
    values
        .iter()
        .map(|(name, value)| attr(name, *value))
        .collect()
}

fn escaped_text(value: &str) -> String {
    let mut html = String::new();
    for character in value.chars() {
        match character {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            _ => html.push(character),
        }
    }
    html
}

fn escaped_attribute(value: &str) -> String {
    let mut html = String::new();
    for character in value.chars() {
        match character {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            _ => html.push(character),
        }
    }
    html
}

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[derive(Clone, Copy, Debug)]
struct ValuePoint {
    date: Date,
    value: f64,
}

#[derive(Clone, Debug)]
struct PerformanceChart {
    actual_start: Date,
    actual_end: Date,
    account_return_pct: f64,
    sp500_return_pct: f64,
    account_points: Vec<ReturnPoint>,
    sp500_points: Vec<ReturnPoint>,
}

#[derive(Clone, Copy, Debug)]
struct ReturnPoint {
    date: Date,
    value: f64,
    return_pct: f64,
}

#[derive(Serialize)]
struct EmbeddedChartData {
    actual_start: String,
    actual_end: String,
    account_points: Vec<EmbeddedPoint>,
    sp500_points: Vec<EmbeddedPoint>,
}

impl From<&PerformanceChart> for EmbeddedChartData {
    fn from(chart: &PerformanceChart) -> Self {
        Self {
            actual_start: chart.actual_start.iso_string(),
            actual_end: chart.actual_end.iso_string(),
            account_points: chart
                .account_points
                .iter()
                .copied()
                .map(EmbeddedPoint::from)
                .collect(),
            sp500_points: chart
                .sp500_points
                .iter()
                .copied()
                .map(EmbeddedPoint::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct EmbeddedPoint {
    date: String,
    value: f64,
    return_pct: f64,
}

impl From<ReturnPoint> for EmbeddedPoint {
    fn from(point: ReturnPoint) -> Self {
        Self {
            date: point.date.iso_string(),
            value: point.value,
            return_pct: point.return_pct,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    fn parse_iso(value: &str) -> Result<Self, String> {
        let mut parts = value.trim().split('-');
        let year = parts
            .next()
            .ok_or_else(|| format!("invalid ISO date {value:?}"))?
            .parse::<u16>()
            .map_err(|error| format!("invalid ISO date {value:?}: {error}"))?;
        let month = parts
            .next()
            .ok_or_else(|| format!("invalid ISO date {value:?}"))?
            .parse::<u8>()
            .map_err(|error| format!("invalid ISO date {value:?}: {error}"))?;
        let day = parts
            .next()
            .ok_or_else(|| format!("invalid ISO date {value:?}"))?
            .parse::<u8>()
            .map_err(|error| format!("invalid ISO date {value:?}: {error}"))?;
        if parts.next().is_some() {
            return Err(format!("invalid ISO date {value:?}"));
        }

        Self::checked(year, month, day)
    }

    fn parse_us(value: &str) -> Result<Self, String> {
        let mut parts = value.trim().split('/');
        let month = parts
            .next()
            .ok_or_else(|| format!("invalid US date {value:?}"))?
            .parse::<u8>()
            .map_err(|error| format!("invalid US date {value:?}: {error}"))?;
        let day = parts
            .next()
            .ok_or_else(|| format!("invalid US date {value:?}"))?
            .parse::<u8>()
            .map_err(|error| format!("invalid US date {value:?}: {error}"))?;
        let year = parts
            .next()
            .ok_or_else(|| format!("invalid US date {value:?}"))?
            .parse::<u16>()
            .map_err(|error| format!("invalid US date {value:?}: {error}"))?;
        if parts.next().is_some() {
            return Err(format!("invalid US date {value:?}"));
        }

        Self::checked(year, month, day)
    }

    fn checked(year: u16, month: u8, day: u8) -> Result<Self, String> {
        if !(1..=12).contains(&month) {
            return Err(format!("invalid month {month}"));
        }

        let days = days_in_month(year, month);
        if day == 0 || day > days {
            return Err(format!("invalid day {day} for month {month}"));
        }

        Ok(Self { year, month, day })
    }

    fn iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    fn display_long(self) -> String {
        format!(
            "{} {}, {}",
            month_name(self.month),
            ordinal_day(self.day),
            self.year
        )
    }
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

fn ordinal_day(day: u8) -> String {
    let suffix = match day {
        11 | 12 | 13 => "th",
        _ if day % 10 == 1 => "st",
        _ if day % 10 == 2 => "nd",
        _ if day % 10 == 3 => "rd",
        _ => "th",
    };
    format!("{day}{suffix}")
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_renderer_escapes_text_and_attributes() {
        let html = element(
            "a",
            vec![attr(
                "href",
                "https://example.com?name=S&P 500&label=\"next\"",
            )],
            vec![Node::text("S&P 500 <returns>")],
        )
        .to_html();

        assert_eq!(
            html,
            r#"<a href="https://example.com?name=S&amp;P 500&amp;label=&quot;next&quot;">S&amp;P 500 &lt;returns&gt;</a>"#
        );
    }

    #[test]
    fn renders_complete_html_document() {
        let site = render_site(&fixture_view());

        assert!(site.starts_with("<!doctype html>"));
        assert!(site.contains("<title>Ocotelolco</title>"));
        assert!(site.contains("Campaign 1 Performance"));
        assert!(site.contains("Ocotelolco Campaign 1"));
        assert!(site.contains(r#"<header class="min-h-7 bg-gray-5"#));
        assert!(!site.contains(r#"<span>ocotelolco</span>"#));
        assert!(!site.contains("window-controls"));
        assert!(!site.contains("window-control"));
        assert!(!site.contains("requested_start"));
        assert!(!site.contains("requested_end"));
        assert!(!site.contains("Requested window"));
        assert!(!site.contains("Rendered window"));
        assert!(!site.contains("NASDAQ"));
        assert!(site.contains(r#"aria-label="Rate of return chart""#));
        assert!(site.contains("shadow-outset"));
        assert!(site.contains("max-md:grid-cols-1"));
        assert!(!site.contains("performance.exe"));
        assert!(!site.contains(r#"class="chart-window""#));
        assert!(site.contains(r#"<script id="performance-chart-data" type="application/json">"#));
        assert!(site.ends_with("</html>\n"));
    }

    #[test]
    fn embeds_raw_chart_data_without_image_tags() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#""account_points":[{"date":"2025-10-28""#));
        assert!(site.contains(r#""sp500_points":[{"date":"2025-10-28""#));
        assert!(!site.contains("<img"));
    }

    #[test]
    fn builds_performance_chart_from_account_and_sp500_values() {
        let chart = build_performance_chart(
            vec![
                ValuePoint {
                    date: Date::new(2025, 10, 28),
                    value: 100.0,
                },
                ValuePoint {
                    date: Date::new(2025, 10, 29),
                    value: 110.0,
                },
            ],
            vec![
                ValuePoint {
                    date: Date::new(2025, 10, 28),
                    value: 200.0,
                },
                ValuePoint {
                    date: Date::new(2025, 10, 29),
                    value: 220.0,
                },
            ],
            Date::new(2025, 10, 28),
            Date::new(2027, 4, 28),
        )
        .unwrap();

        assert_eq!(chart.actual_start, Date::new(2025, 10, 28));
        assert_eq!(chart.actual_end, Date::new(2025, 10, 29));
        assert!((chart.account_return_pct - 10.0).abs() < f64::EPSILON * 100.0);
        assert!((chart.sp500_return_pct - 10.0).abs() < f64::EPSILON * 100.0);
    }

    #[test]
    fn parses_schwab_balance_history_files() {
        let directory = std::env::temp_dir().join(format!(
            "ocotelolco-website-balance-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("history.CSV"),
            "Date,Amount\n\"10/29/2025\",\"$110.00\"\n\"10/28/2025\",\"$100.00\"\n",
        )
        .unwrap();
        fs::write(
            directory.join("summary.CSV"),
            "\"Balances for account XXXX as of 10/29/2025\"\n\nAccount Value,\"$110.00\"\n",
        )
        .unwrap();

        let values = read_balance_values(&directory).unwrap();
        fs::remove_dir_all(directory).unwrap();

        assert_eq!(values.len(), 2);
        assert_eq!(values[0].date, Date::new(2025, 10, 28));
        assert_eq!(values[0].value, 100.0);
    }

    fn fixture_chart() -> PerformanceChart {
        PerformanceChart {
            actual_start: Date::new(2025, 10, 28),
            actual_end: Date::new(2025, 10, 29),
            account_return_pct: 10.0,
            sp500_return_pct: 5.0,
            account_points: vec![
                ReturnPoint {
                    date: Date::new(2025, 10, 28),
                    value: 100.0,
                    return_pct: 0.0,
                },
                ReturnPoint {
                    date: Date::new(2025, 10, 29),
                    value: 110.0,
                    return_pct: 10.0,
                },
            ],
            sp500_points: vec![
                ReturnPoint {
                    date: Date::new(2025, 10, 28),
                    value: 200.0,
                    return_pct: 0.0,
                },
                ReturnPoint {
                    date: Date::new(2025, 10, 29),
                    value: 210.0,
                    return_pct: 5.0,
                },
            ],
        }
    }

    fn fixture_view() -> SiteView {
        SiteView::from_chart(&fixture_chart()).unwrap()
    }
}
