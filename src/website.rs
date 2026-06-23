use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::Serialize;

use crate::{
    palette,
    website_content::{
        self, CalendarDate, CampaignContent, DetailBlock, DetailReport, DetailSection,
        DetailSubsection, DetailTopic, DisclosureState, KeyMetric, ListItem, MetricValue,
        PercentageFigure, PercentageUnit, ThesisRow, ThesisScoreboard,
    },
};

const BANNER_IMAGE_PNG: &[u8] = include_bytes!("../ocotelolco_banner.png");
const HFNSS_FONT_TTF: &[u8] = include_bytes!("../HFNSS.ttf");
const FAVICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" shape-rendering="crispEdges"><path fill="#6f4a05" fill-rule="evenodd" d="M8 6h18v4h2v14h-2v4H8v-2H6V8h2zm4 6v12h10V12z"/><path fill="#dba51e" fill-rule="evenodd" d="M6 4h18v4h2v14h-2v4H6v-2H4V6h2zm4 6v12h10V10z"/><path fill="#fff0a0" d="M8 6h14v2H8zM6 8h2v14H6z"/><path fill="#8a6208" d="M10 24h14v2H10zM24 10h2v12h-2z"/></svg>"##;
const PERFORMANCE_START: Date = Date::new(2025, 10, 28);
const PERFORMANCE_END: Date = Date::new(2026, 4, 28);
const CONTENT_COPY: &str = "w-full max-w-none text-gray-5 text-base break-anywhere";
const OVERVIEW_PANEL: &str = "min-w-0 bg-green-2 p-3 text-base edge-inset";
const THESIS_TABLE: &str = "grid bg-green-2 edge-inset";
const THESIS_ROW: &str = "thesis-row bg-green-2";
const THESIS_HEADER: &str = "thesis-row bg-table-header text-gray-5";
const THESIS_RETURN: &str = "text-base text-right whitespace-nowrap max-md:text-left";
const DETAIL_LIST: &str = "grid gap-1-5";
const DETAIL_SECTION: &str = "bg-gray-2 edge-outline";
const DETAIL_TITLE: &str = "text-gray-6";
const DETAIL_TEASER: &str = "text-gray-5 text-sm";
const DETAIL_ACTION: &str = "justify-self-end min-w-detail-action bg-gray-2 text-gray-5 text-center whitespace-nowrap edge-outset px-2 py-1 max-md:justify-self-start";
const DETAIL_ACTION_CLOSE: &str = "hidden";

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

fn site_css() -> String {
    let mut css = format!(
        r#"
    @font-face {{
      font-family: "HFNSS";
      src: url("{}") format("truetype");
      font-weight: 400;
      font-style: normal;
      font-display: block;
    }}

    :root {{
      color-scheme: dark;
      --pixel-font-size: 24px;
      --pixel-font-stack: "HFNSS", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
"#,
        hfnss_font_data_uri()
    );
    for (name, color) in palette::CSS_COLORS {
        css.push_str("      --");
        css.push_str(name);
        css.push_str(": ");
        css.push_str(&color.css());
        css.push_str(";\n");
    }
    css.push_str(
        r##"      --chart-account: var(--yellow-6);
      --chart-index: var(--gray-6);
      --chart-axis: var(--gray-5);
      --chart-zero: var(--gray-5);
      --table-header: #06170d;
    }
"##,
    );
    css.push_str(SITE_CSS_RULES);
    css
}

fn hfnss_font_data_uri() -> String {
    format!(
        "data:font/ttf;base64,{}",
        BASE64_STANDARD.encode(HFNSS_FONT_TTF)
    )
}

fn favicon_data_uri() -> String {
    format!(
        "data:image/svg+xml;base64,{}",
        BASE64_STANDARD.encode(FAVICON_SVG)
    )
}

const SITE_CSS_RULES: &str = r##"
    * {
      box-sizing: border-box;
    }

    h1,
    h2,
    h3,
    p,
    ol,
    ul {
      margin: 0;
    }

    html {
      font-size: var(--pixel-font-size);
      -webkit-font-smoothing: none;
      font-smooth: never;
      text-rendering: geometricPrecision;
      text-size-adjust: 100%;
      -webkit-text-size-adjust: 100%;
    }

    body {
      margin: 0;
      min-height: 100vh;
      color: var(--gray-6);
      background: var(--green-3);
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-kerning: none;
      font-synthesis: none;
      line-height: 1;
    }

    body::before {
      position: fixed;
      inset: 0;
      z-index: -1;
      background: url("../ocotelolco_bg.png") center / cover;
      content: "";
      opacity: 0.34;
    }

    .site-banner {
      height: clamp(6.25rem, 24vw, 11.666667rem);
      object-fit: cover;
      object-position: center;
    }
    .site-subheader {
      overflow-wrap: anywhere;
    }

    a {
      color: var(--green-7);
      text-decoration: none;
    }
    a:hover {
      text-decoration: underline;
    }
    code {
      color: var(--green-7);
      font-family: var(--pixel-font-stack);
    }

    .relative { position: relative; }
    .block { display: block; }
    .flex { display: flex; }
    .grid { display: grid; }
    .hidden { display: none; }
    .flex-1 { flex: 1; }
    .flex-col { flex-direction: column; }
    .flex-none { flex: none; }
    .items-center { align-items: center; }
    .items-start { align-items: start; }
    .justify-self-end { justify-self: end; }
    .grid-cols-1 { grid-template-columns: minmax(0, 1fr); }
    .grid-cols-return { grid-template-columns: 1fr minmax(4.583333rem, max-content); }
    .w-full { width: 100%; }
    .max-w-window { max-width: 46.666667rem; }
    .max-w-copy { max-width: 70ch; }
    .max-w-none { max-width: none; }
    .min-w-0 { min-width: 0; }
    .min-w-detail-action { min-width: 3.833333rem; }
    .min-h-screen { min-height: 100vh; }
    .min-h-window { min-height: min(30rem, calc(100vh - 2.666667rem)); }
    .min-h-window-inner { min-height: calc(min(30rem, calc(100vh - 2.666667rem)) - 0.333333rem); }
    .min-h-7 { min-height: 1.166667rem; }
    .min-h-chart { min-height: 12.5rem; }
    .min-h-svg { min-height: 10.833333rem; }
    .min-h-row { min-height: 1.916667rem; }
    .min-h-row-header { min-height: 1.666667rem; }
    .mx-auto { margin-right: auto; margin-left: auto; }
    .mt-1 { margin-top: 0.166667rem; }
    .mt-4 { margin-top: 0.666667rem; }
    .mb-2 { margin-bottom: 0.333333rem; }
    .mb-4 { margin-bottom: 0.666667rem; }
    .p-1 { padding: 0.166667rem; }
    .p-2 { padding: 0.333333rem; }
    .p-3 { padding: 0.5rem; }
    .p-4 { padding: 0.666667rem; }
    .p-5 { padding: 0.833333rem; }
    .p-8 { padding: 1.333333rem; }
    .px-2 { padding-right: 0.333333rem; padding-left: 0.333333rem; }
    .px-3 { padding-right: 0.5rem; padding-left: 0.5rem; }
    .pb-4 { padding-bottom: 0.666667rem; }
    .py-1 { padding-top: 0.166667rem; padding-bottom: 0.166667rem; }
    .py-2 { padding-top: 0.333333rem; padding-bottom: 0.333333rem; }
    .py-4 { padding-top: 0.666667rem; padding-bottom: 0.666667rem; }
    .gap-1 { gap: 0.166667rem; }
    .gap-2 { gap: 0.333333rem; }
    .gap-4 { gap: 0.666667rem; }
    .gap-1-5 { gap: 0.25rem; }
    .break-anywhere { overflow-wrap: anywhere; }
    .whitespace-nowrap { white-space: nowrap; }
    .overflow-visible { overflow: visible; }
    .overflow-x-auto { overflow-x: auto; }
    .bg-current { background: currentColor; }
    .bg-gray-2 { background: var(--gray-2); }
    .bg-gray-5 { background: var(--gray-5); }
    .bg-green-1 { background: var(--green-1); }
    .bg-green-2 { background: var(--green-2); }
    .edge-inset {
      border-width: 2px;
      border-style: solid;
      border-color: var(--gray-1) var(--gray-3) var(--gray-3) var(--gray-1);
    }
    .edge-outset {
      border-width: 2px;
      border-style: solid;
      border-color: var(--gray-3) var(--gray-1) var(--gray-1) var(--gray-3);
    }
    .edge-outline {
      border: 2px solid var(--gray-3);
    }
    .bg-table-header { background: var(--table-header); }
    .text-gray-4 { color: var(--gray-4); }
    .text-gray-5 { color: var(--gray-5); }
    .text-gray-6 { color: var(--gray-6); }
    .text-green-1 { color: var(--green-1); }
    .text-green-7 { color: var(--green-7); }
    .text-chart-account { color: var(--chart-account); }
    .text-chart-index { color: var(--chart-index); }
    .text-base { font-size: var(--pixel-font-size); }
    .text-sm { font-size: var(--pixel-font-size); }
    .font-normal { font-weight: 400; }
    .text-center { text-align: center; }
    .text-right { text-align: right; }
    .square-3 { width: 0.583333rem; height: 0.583333rem; }
    .border-b-dotted,
    .section-separator,
    details[open] .detail-summary {
      position: relative;
    }
    .border-b-dotted::after,
    .section-separator::after,
    details[open] .detail-summary::after {
      content: "";
      position: absolute;
      right: 0;
      bottom: 0;
      left: 0;
      height: 4px;
      background: linear-gradient(
        to bottom,
        var(--gray-1) 0 2px,
        var(--gray-3) 2px 4px
      );
      pointer-events: none;
    }
    .border-b-solid { border-bottom: 1px solid var(--gray-3); }
    .content-list {
      display: grid;
      gap: 0.333333rem;
      padding-left: 1.333333rem;
    }
    .metric-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.333333rem;
    }
    .metric-box {
      min-width: 0;
      background: var(--green-2);
      padding: 0.5rem;
    }
    .metric-box {
      display: grid;
      grid-template-columns: minmax(0, 1fr) max-content;
      align-items: center;
      gap: 1rem;
    }
    .metric-box:first-child {
      grid-column: 1 / -1;
      grid-template-columns: minmax(0, 1fr);
      justify-items: center;
      gap: 0.416667rem;
      padding: 0.666667rem 0.5rem;
      text-align: center;
    }
    .metric-box:first-child .metric-value {
      font-size: calc(var(--pixel-font-size) * 2);
    }
    .metric-box:not(:first-child) {
      grid-template-columns: max-content max-content;
      justify-content: center;
      gap: 0.666667rem;
    }
    .metric-label {
      color: var(--gray-5);
      font-size: var(--pixel-font-size);
    }
    .metric-value {
      color: var(--gray-6);
      font-size: var(--pixel-font-size);
      white-space: nowrap;
    }
    .overview-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.333333rem;
    }
    .thesis-row {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(3.666667rem, max-content);
      align-items: start;
      gap: 0.5rem;
      min-width: 0;
      padding: 0.5rem;
      font-size: var(--pixel-font-size);
    }
    .thesis-row:nth-child(odd):not(:first-child) {
      background: var(--green-3);
    }
    .detail-summary {
      font-size: var(--pixel-font-size);
    }
    .result-positive { color: var(--green-7); }
    .result-negative { color: var(--red-2); }
    .result-neutral { color: var(--gray-6); }
    details.edge-outline:hover,
    details.edge-outline:focus-within {
      border-color: var(--gray-4);
    }
    details > summary {
      cursor: pointer;
      list-style: none;
      outline: none;
      user-select: none;
    }
    details > summary::-webkit-details-marker {
      display: none;
    }
    .detail-summary {
      display: grid;
      grid-template-columns: minmax(7.5rem, 0.34fr) minmax(0, 1fr) max-content;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem;
    }
    [data-detail-action]:hover {
      color: var(--gray-6);
    }
    details[open] [data-detail-action="open"] {
      display: none;
    }
    details[open] [data-detail-action="close"] {
      display: inline-block;
    }
    .detail-content {
      display: grid;
      gap: 0.5rem;
      background: var(--gray-2);
      padding: 0.666667rem 0.5rem 0.666667rem;
    }
    .detail-subsection {
      display: grid;
      gap: 0.333333rem;
      padding-top: 0.166667rem;
    }
    svg {
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-kerning: none;
    }
    .performance-chart-svg {
      width: 100%;
      min-width: 38.333333rem;
      height: auto;
      max-width: none;
    }

    .axis-label {
      fill: var(--gray-6);
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-weight: 400;
    }

    .chart-background {
      fill: var(--green-2);
    }

    .month-label {
      fill: var(--gray-6);
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-weight: 400;
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
      .max-md\:min-h-window { min-height: calc(100vh - 1.333333rem); }
      .max-md\:min-h-chart { min-height: 10rem; }
      .max-md\:min-h-svg { min-height: 9.166667rem; }
      .max-md\:grid-cols-1 { grid-template-columns: minmax(0, 1fr); }
      .max-md\:gap-1 { gap: 0.166667rem; }
      .max-md\:p-2 { padding: 0.333333rem; }
      .max-md\:p-3 { padding: 0.5rem; }
      .max-md\:p-4 { padding: 0.666667rem; }
      .max-md\:py-2 { padding-top: 0.333333rem; padding-bottom: 0.333333rem; }
      .max-md\:text-left { text-align: left; }
      .metric-grid,
      .overview-grid,
      .thesis-row,
      .detail-summary {
        grid-template-columns: minmax(0, 1fr);
      }
      .max-md\:justify-self-start {
        justify-self: start;
      }
    }
"##;

const PERFORMANCE_CHART_SCRIPT: &str = r##"
    (function () {
      const dataElement = document.getElementById("performance-chart-data");
      const svg = document.getElementById("performance-chart");
      if (!dataElement || !svg) return;

      const data = JSON.parse(dataElement.textContent);
      const chartViewport = svg.parentElement;
      const minimumWidth = 920;
      const baseHeight = 330;
      const maximumHeight = 430;
      const chartTitle = "Account balance return compared with S&P 500 return";
      const readPixelValue = (value) => Number.parseFloat(value) || 0;
      const chartDimensions = () => {
        if (!chartViewport) return { width: minimumWidth, height: baseHeight };

        const style = window.getComputedStyle(chartViewport);
        const horizontalPadding =
          readPixelValue(style.paddingLeft) + readPixelValue(style.paddingRight);
        const availableWidth = Math.max(
          0,
          chartViewport.clientWidth - horizontalPadding
        );
        const width = Math.max(minimumWidth, Math.floor(availableWidth));
        const height = Math.min(
          maximumHeight,
          Math.round(baseHeight + Math.max(0, width - minimumWidth) * 0.12)
        );

        return { width, height };
      };
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

      const renderChart = () => {
        const { width, height } = chartDimensions();
        const margin = { top: 24, right: 74, bottom: 60, left: 20 };
        const plotWidth = width - margin.left - margin.right;
        const plotHeight = height - margin.top - margin.bottom;
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

        svg.setAttribute("width", `${width}`);
        svg.setAttribute("height", `${height}`);
        svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

        const nodes = [];
        nodes.push(`<title id="performance-chart-title">${chartTitle}</title>`);
        nodes.push(`<rect class="chart-background" x="0" y="0" width="${width}" height="${height}"></rect>`);
        yTicks.forEach((tick) => {
          const yPosition = y(tick);
          const className = tick === 0 ? "zero-line" : "grid-line";
          nodes.push(`<line class="${className}" x1="${margin.left}" y1="${yPosition.toFixed(2)}" x2="${(width - margin.right).toFixed(2)}" y2="${yPosition.toFixed(2)}"></line>`);
          nodes.push(`<text class="axis-label" x="${(width - margin.right + 12).toFixed(2)}" y="${(yPosition + 12).toFixed(2)}">${tick}%</text>`);
        });
        monthTicks().forEach((tick) => {
          const xPosition = x(tick.toISOString().slice(0, 10));
          nodes.push(`<line class="axis-line" x1="${xPosition.toFixed(2)}" y1="${(height - margin.bottom).toFixed(2)}" x2="${xPosition.toFixed(2)}" y2="${(height - margin.bottom + 12).toFixed(2)}"></line>`);
          nodes.push(`<text class="month-label" x="${xPosition.toFixed(2)}" y="${(height - 14).toFixed(2)}" text-anchor="middle">${monthLabel(tick)}</text>`);
        });
        nodes.push(`<line class="axis-line" x1="${margin.left}" y1="${(height - margin.bottom).toFixed(2)}" x2="${(width - margin.right).toFixed(2)}" y2="${(height - margin.bottom).toFixed(2)}"></line>`);
        nodes.push(`<line class="axis-line" x1="${(width - margin.right).toFixed(2)}" y1="${margin.top}" x2="${(width - margin.right).toFixed(2)}" y2="${(height - margin.bottom).toFixed(2)}"></line>`);
        nodes.push(`<polyline class="account-line" points="${line(data.account_points)}"></polyline>`);
        nodes.push(`<polyline class="index-line" points="${line(data.sp500_points)}"></polyline>`);

        svg.innerHTML = nodes.join("");
      };

      renderChart();
      if (chartViewport && "ResizeObserver" in window) {
        new ResizeObserver(renderChart).observe(chartViewport);
      } else {
        window.addEventListener("resize", renderChart);
      }
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
    content: CampaignContent,
    chart_json: String,
    actual_window: String,
    account_return: String,
    sp500_return: String,
}

impl SiteView {
    fn from_chart(chart: &PerformanceChart) -> io::Result<Self> {
        let chart_data = EmbeddedChartData::from(chart);
        Ok(Self {
            content: website_content::campaign_1_content(),
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
            element(
                "link",
                vec![
                    attr("rel", "icon"),
                    attr("type", "image/svg+xml"),
                    attr("href", favicon_data_uri()),
                ],
                Vec::new(),
            ),
            element("style", Vec::new(), vec![Node::raw_text(site_css())]),
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
                attrs(&[("class", "min-h-screen grid gap-4 p-8 max-md:p-4")]),
                vec![site_banner(), desktop_window(view)],
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

fn site_banner() -> Node {
    element(
        "header",
        attrs(&[("class", "grid gap-2")]),
        vec![
            banner_image(),
            p(
                attrs(&[(
                    "class",
                    "site-subheader mx-auto w-full max-w-window text-center text-base text-gray-6",
                )]),
                vec![Node::text(
                    "a trading, predicting, and betting project, by Chadtech",
                )],
            ),
        ],
    )
}

fn banner_image() -> Node {
    element(
        "img",
        vec![
            attr("class", "site-banner block mx-auto w-full max-w-window"),
            attr("src", banner_image_data_uri()),
            attr("alt", "Ocotelolco"),
            attr("width", "1672"),
            attr("height", "941"),
        ],
        Vec::new(),
    )
}

fn banner_image_data_uri() -> String {
    format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(BANNER_IMAGE_PNG)
    )
}

fn desktop_window(view: &SiteView) -> Node {
    div(
        attrs(&[
            (
                "class",
                "relative mx-auto min-h-window w-full max-w-window bg-gray-2 edge-outset p-1 text-gray-6 max-md:min-h-window",
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
                        "min-h-7 bg-gray-5 px-2 py-1 text-green-1",
                    )]),
                    vec![span(Vec::new(), vec![Node::text(&view.content.title)])],
                ),
                report_body(view),
            ],
        )],
    )
}

fn report_body(view: &SiteView) -> Node {
    section(
        attrs(&[
            ("class", "relative mt-1 flex-1 bg-gray-2 p-5 max-md:p-3"),
            ("aria-label", "Report content"),
        ]),
        vec![div(
            attrs(&[("class", "grid min-w-0 gap-4")]),
            vec![
                overview_section(&view.content),
                performance_panel(view),
                campaign_context_section(&view.content),
                thesis_scoreboard(&view.content.thesis_scoreboard),
                detail_report(&view.content.detail_report),
            ],
        )],
    )
}

fn overview_section(content: &CampaignContent) -> Node {
    let overview = &content.overview;
    section(
        attrs(&[
            (
                "class",
                "grid grid-cols-1 items-start gap-4 section-separator pb-4",
            ),
            ("aria-label", "Campaign overview"),
        ]),
        vec![
            div(
                attrs(&[("class", "grid gap-2")]),
                vec![p(
                    attrs(&[("class", CONTENT_COPY)]),
                    vec![Node::text(&overview.summary)],
                )],
            ),
            key_metrics(&overview.key_metrics),
        ],
    )
}

fn campaign_context_section(content: &CampaignContent) -> Node {
    let overview = &content.overview;
    section(
        attrs(&[
            ("class", "grid gap-4 section-separator pb-4"),
            ("aria-label", "Campaign context"),
        ]),
        vec![
            div(
                vec![attr("class", format!("grid gap-2 {CONTENT_COPY}"))],
                overview
                    .context
                    .iter()
                    .map(|paragraph| p(Vec::new(), vec![Node::text(paragraph)]))
                    .collect(),
            ),
            rules_summary(&overview.rules, &overview.takeaway),
        ],
    )
}

fn performance_panel(view: &SiteView) -> Node {
    let performance = &view.content.performance;
    let account_label = performance
        .comparisons
        .first()
        .map(|comparison| comparison.label.as_str())
        .unwrap_or("Ocotelolco Campaign 1");
    let sp500_label = performance
        .comparisons
        .get(1)
        .map(|comparison| comparison.label.as_str())
        .unwrap_or("S&P 500");

    section(
        attrs(&[
            (
                "class",
                "relative min-w-0 bg-gray-2 py-4 text-gray-6 max-md:py-2",
            ),
            ("aria-label", "Rate of return chart"),
        ]),
        vec![
            chart_copy(view),
            chart_viewport(),
            return_table(
                account_label,
                &view.account_return,
                sp500_label,
                &view.sp500_return,
            ),
        ],
    )
}

fn chart_copy(view: &SiteView) -> Node {
    let performance = &view.content.performance;
    div(
        attrs(&[("class", "mb-4 grid gap-1")]),
        vec![
            element(
                "h2",
                attrs(&[("class", "text-base font-normal text-gray-4")]),
                vec![Node::text(&performance.title)],
            ),
            p(
                attrs(&[("class", CONTENT_COPY)]),
                vec![
                    Node::text(&performance.summary),
                    Node::text(" Account balance return was "),
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

fn key_metrics(metrics: &[KeyMetric]) -> Node {
    div(
        attrs(&[("class", "metric-grid")]),
        metrics.iter().map(key_metric).collect(),
    )
}

fn key_metric(metric: &KeyMetric) -> Node {
    div(
        attrs(&[("class", "metric-box edge-inset")]),
        vec![
            div(
                attrs(&[("class", "metric-label")]),
                vec![Node::text(&metric.label)],
            ),
            div(
                vec![attr(
                    "class",
                    format!("metric-value {}", metric_value_class(&metric.value)),
                )],
                vec![Node::text(format_metric_value(&metric.value))],
            ),
        ],
    )
}

fn rules_summary(rules: &[String], takeaway: &str) -> Node {
    div(
        attrs(&[("class", "overview-grid")]),
        vec![
            div(
                attrs(&[("class", OVERVIEW_PANEL)]),
                vec![
                    element(
                        "h2",
                        attrs(&[("class", "text-base font-normal text-gray-4")]),
                        vec![Node::text("Rules")],
                    ),
                    ol(
                        vec![attr("class", format!("content-list {CONTENT_COPY}"))],
                        rules
                            .iter()
                            .map(|rule| li(Vec::new(), vec![Node::text(rule)]))
                            .collect(),
                    ),
                ],
            ),
            div(
                attrs(&[("class", OVERVIEW_PANEL)]),
                vec![
                    element(
                        "h2",
                        attrs(&[("class", "text-base font-normal text-gray-4")]),
                        vec![Node::text("Prediction is not position")],
                    ),
                    p(
                        attrs(&[("class", CONTENT_COPY)]),
                        vec![Node::text(takeaway)],
                    ),
                ],
            ),
        ],
    )
}

fn thesis_scoreboard(scoreboard: &ThesisScoreboard) -> Node {
    section(
        attrs(&[
            ("class", "grid gap-2 section-separator pb-4"),
            ("aria-label", "Thesis scoreboard"),
        ]),
        vec![
            element(
                "h2",
                attrs(&[("class", "text-base font-normal text-gray-4")]),
                vec![Node::text(&scoreboard.title)],
            ),
            p(
                attrs(&[("class", CONTENT_COPY)]),
                vec![Node::text(&scoreboard.summary)],
            ),
            div(
                attrs(&[("class", THESIS_TABLE)]),
                thesis_rows(&scoreboard.rows),
            ),
        ],
    )
}

fn thesis_rows(rows: &[ThesisRow]) -> Vec<Node> {
    let mut nodes = Vec::with_capacity(rows.len() + 1);
    nodes.push(div(
        attrs(&[("class", THESIS_HEADER)]),
        vec![
            span(Vec::new(), vec![Node::text("Thesis/tag")]),
            span(
                attrs(&[("class", THESIS_RETURN)]),
                vec![Node::text("Returns")],
            ),
        ],
    ));
    nodes.extend(rows.iter().map(thesis_row));
    nodes
}

fn thesis_row(row: &ThesisRow) -> Node {
    div(
        attrs(&[("class", THESIS_ROW)]),
        vec![
            div(
                attrs(&[("class", "grid gap-1")]),
                vec![
                    span(
                        attrs(&[("class", "text-gray-5")]),
                        vec![Node::text(&row.title)],
                    ),
                    p(
                        attrs(&[("class", "text-sm")]),
                        vec![Node::text(&row.visible_summary)],
                    ),
                ],
            ),
            span(
                vec![attr(
                    "class",
                    format!(
                        "{THESIS_RETURN} {}",
                        return_figure_class(row.realized_return)
                    ),
                )],
                vec![Node::text(
                    row.realized_return
                        .map(|figure| format_percentage_figure(figure, true))
                        .unwrap_or_else(|| "-".to_string()),
                )],
            ),
        ],
    )
}

fn detail_report(report: &DetailReport) -> Node {
    section(
        attrs(&[("class", "grid gap-2"), ("aria-label", "Full report")]),
        vec![
            element(
                "h2",
                attrs(&[("class", "text-base font-normal text-gray-4")]),
                vec![Node::text(&report.title)],
            ),
            p(
                attrs(&[("class", CONTENT_COPY)]),
                vec![Node::text(&report.summary)],
            ),
            div(
                attrs(&[("class", DETAIL_LIST)]),
                report.sections.iter().map(detail_section).collect(),
            ),
        ],
    )
}

fn detail_section(section: &DetailSection) -> Node {
    let mut attributes = attrs(&[
        ("id", detail_topic_id(section.topic)),
        ("class", DETAIL_SECTION),
    ]);
    if section.default_disclosure == DisclosureState::Expanded {
        attributes.push(attr("open", ""));
    }

    element(
        "details",
        attributes,
        vec![
            element(
                "summary",
                Vec::new(),
                vec![div(
                    attrs(&[("class", "detail-summary")]),
                    vec![
                        span(
                            attrs(&[("class", DETAIL_TITLE)]),
                            vec![Node::text(&section.title)],
                        ),
                        span(
                            attrs(&[("class", DETAIL_TEASER)]),
                            vec![Node::text(&section.summary)],
                        ),
                        span(
                            attrs(&[("class", DETAIL_ACTION), ("data-detail-action", "open")]),
                            vec![Node::text("Open >")],
                        ),
                        span(
                            vec![
                                attr("class", format!("{DETAIL_ACTION} {DETAIL_ACTION_CLOSE}")),
                                attr("data-detail-action", "close"),
                            ],
                            vec![Node::text("Close v")],
                        ),
                    ],
                )],
            ),
            div(
                attrs(&[("class", "detail-content")]),
                render_detail_blocks(&section.blocks),
            ),
        ],
    )
}

fn render_detail_blocks(blocks: &[DetailBlock]) -> Vec<Node> {
    blocks.iter().map(render_detail_block).collect()
}

fn render_detail_block(block: &DetailBlock) -> Node {
    match block {
        DetailBlock::Paragraph(text) => {
            p(attrs(&[("class", CONTENT_COPY)]), vec![Node::text(text)])
        }
        DetailBlock::OrderedList(items) => ol(
            vec![attr("class", format!("content-list {CONTENT_COPY}"))],
            items
                .iter()
                .map(|item| li(Vec::new(), vec![Node::text(item)]))
                .collect(),
        ),
        DetailBlock::UnorderedList(items) => ul(
            vec![attr("class", format!("content-list {CONTENT_COPY}"))],
            items.iter().map(render_list_item).collect(),
        ),
        DetailBlock::Subsection(subsection) => detail_subsection(subsection),
    }
}

fn detail_subsection(subsection: &DetailSubsection) -> Node {
    section(
        attrs(&[("class", "detail-subsection")]),
        vec![
            element(
                "h3",
                attrs(&[("class", "text-base font-normal text-gray-5")]),
                vec![Node::text(&subsection.title)],
            ),
            div(
                attrs(&[("class", "grid gap-2")]),
                render_detail_blocks(&subsection.blocks),
            ),
        ],
    )
}

fn render_list_item(item: &ListItem) -> Node {
    match item {
        ListItem::Text(text) => li(Vec::new(), vec![Node::text(text)]),
        ListItem::Code(text) => li(Vec::new(), vec![code(Vec::new(), vec![Node::text(text)])]),
    }
}

fn chart_viewport() -> Node {
    div(
        attrs(&[(
            "class",
            "relative min-w-0 min-h-chart overflow-x-auto bg-green-2 edge-inset p-3 max-md:min-h-chart max-md:p-2",
        )]),
        vec![element(
            "svg",
            attrs(&[
                ("id", "performance-chart"),
                (
                    "class",
                    "performance-chart-svg block min-h-svg overflow-visible text-gray-6 max-md:min-h-svg",
                ),
                ("role", "img"),
                ("aria-labelledby", "performance-chart-title"),
                ("width", "920"),
                ("height", "330"),
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

fn return_table(
    account_label: &str,
    account_return: &str,
    sp500_label: &str,
    sp500_return: &str,
) -> Node {
    div(
        attrs(&[
            (
                "class",
                "relative mt-4 bg-green-2 edge-inset text-gray-6",
            ),
            ("aria-label", "Rate of return table"),
        ]),
        vec![
            div(
                attrs(&[(
                    "class",
                    "grid min-h-row-header grid-cols-return items-center gap-4 border-b-solid bg-table-header px-3 py-2 text-base text-gray-5 max-md:grid-cols-1 max-md:gap-1",
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
            return_row("account", account_label, account_return),
            return_row("index", sp500_label, sp500_return),
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

fn format_metric_value(value: &MetricValue) -> String {
    match value {
        MetricValue::Text(text) => text.clone(),
        MetricValue::Percentage(figure) => format_metric_percentage_figure(*figure),
        MetricValue::DateRange(date_range) => format_date_range(*date_range),
    }
}

fn metric_value_class(value: &MetricValue) -> &'static str {
    match value {
        MetricValue::Percentage(figure) => return_figure_class(Some(*figure)),
        _ => "result-neutral",
    }
}

fn format_metric_percentage_figure(figure: PercentageFigure) -> String {
    let mut value = format_basis_points(figure.basis_points);
    if value.ends_with(".0") {
        value.truncate(value.len() - 2);
    }
    let sign = if figure.basis_points > 0 { "+" } else { "" };

    format!("{sign}{value}%")
}

fn format_percentage_figure(figure: PercentageFigure, signed: bool) -> String {
    let value = format_basis_points(figure.basis_points);
    let sign = if signed && figure.basis_points > 0 {
        "+"
    } else {
        ""
    };
    match figure.unit {
        PercentageUnit::Percent => format!("{sign}{value}%"),
        PercentageUnit::PercentagePoints => format!("{sign}{value} percentage points"),
    }
}

fn format_basis_points(basis_points: i32) -> String {
    let sign = if basis_points < 0 { "-" } else { "" };
    let absolute_basis_points = basis_points.abs();
    let whole = absolute_basis_points / 100;
    let fractional = absolute_basis_points % 100;
    if fractional == 0 {
        format!("{sign}{whole}.0")
    } else if fractional % 10 == 0 {
        format!("{sign}{whole}.{}", fractional / 10)
    } else {
        format!("{sign}{whole}.{fractional:02}")
    }
}

fn format_date_range(date_range: crate::website_content::DateRange) -> String {
    format!(
        "{} to {}",
        format_calendar_date(date_range.start),
        format_calendar_date(date_range.end)
    )
}

fn format_calendar_date(date: CalendarDate) -> String {
    format!(
        "{} {}, {}",
        month_name(date.month),
        ordinal_day(date.day),
        date.year
    )
}

fn return_figure_class(figure: Option<PercentageFigure>) -> &'static str {
    match figure.map(|figure| figure.basis_points) {
        Some(value) if value > 0 => "result-positive",
        Some(value) if value < 0 => "result-negative",
        _ => "result-neutral",
    }
}

fn detail_topic_id(topic: DetailTopic) -> &'static str {
    match topic {
        DetailTopic::Wars => "detail-wars",
        DetailTopic::ScotusTariffRuling => "detail-scotus-tariff-ruling",
        DetailTopic::GoldSilverUsCredibility => "detail-gold-silver-us-credibility",
        DetailTopic::TechAi => "detail-tech-ai",
        DetailTopic::Experts => "detail-experts",
        DetailTopic::StoppingWhenTheEdgeIsGone => "detail-stopping-when-the-edge-is-gone",
        DetailTopic::Lessons => "detail-lessons",
    }
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

fn ol(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("ol", attributes, children)
}

fn ul(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("ul", attributes, children)
}

fn li(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("li", attributes, children)
}

fn code(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
    element("code", attributes, children)
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
    fn formats_negative_fractional_percentages() {
        assert_eq!(format_basis_points(-40), "-0.4");
        assert_eq!(
            format_percentage_figure(PercentageFigure::percent(-40), true),
            "-0.4%"
        );
    }

    #[test]
    fn formats_metric_percentages_as_signed_compact_values() {
        assert_eq!(
            format_metric_value(&MetricValue::Percentage(
                PercentageFigure::percentage_points(788)
            )),
            "+7.88%"
        );
        assert_eq!(
            format_metric_value(&MetricValue::Percentage(
                PercentageFigure::percentage_points(2107)
            )),
            "+21.07%"
        );
        assert_eq!(
            format_metric_value(&MetricValue::Percentage(
                PercentageFigure::percentage_points(-40)
            )),
            "-0.4%"
        );
        assert_eq!(
            format_metric_value(&MetricValue::Percentage(
                PercentageFigure::percentage_points(-146)
            )),
            "-1.46%"
        );
        assert_eq!(
            metric_value_class(&MetricValue::Percentage(
                PercentageFigure::percentage_points(-146)
            )),
            "result-negative"
        );
    }

    #[test]
    fn colors_thesis_returns_by_sign_not_thesis_tone() {
        assert_eq!(
            return_figure_class(Some(PercentageFigure::percent(120))),
            "result-positive"
        );
        assert_eq!(
            return_figure_class(Some(PercentageFigure::percent(-40))),
            "result-negative"
        );
        assert_eq!(return_figure_class(None), "result-neutral");

        let site = render_site(&fixture_view());

        assert!(site.contains(
            r#"<span class="text-base text-right whitespace-nowrap max-md:text-left result-positive">+1.2%</span>"#
        ));
        assert!(!site.contains("result-mixed"));
        assert!(site.contains(".whitespace-nowrap { white-space: nowrap; }"));
        assert!(!site.contains("thesis-return"));
    }

    #[test]
    fn table_headers_use_near_content_green() {
        let site = render_site(&fixture_view());

        assert!(site.contains("--table-header: #06170d;"));
        assert!(site.contains(".bg-table-header { background: var(--table-header); }"));
        assert!(site.contains(r#"class="thesis-row bg-table-header text-gray-5""#));
        assert!(site.contains("border-b-solid bg-table-header"));
        assert!(!site.contains("border-b-solid bg-green-1"));
        assert!(!site.contains("thesis-header"));
    }

    #[test]
    fn horizontal_separators_use_inset_slit_style() {
        let site = render_site(&fixture_view());

        assert!(site.contains(
            ".border-b-dotted,\n    .section-separator,\n    details[open] .detail-summary {\n      position: relative;\n    }"
        ));
        assert!(site.contains(
            ".border-b-dotted::after,\n    .section-separator::after,\n    details[open] .detail-summary::after"
        ));
        assert!(site.contains("height: 4px;"));
        assert!(site.contains("var(--gray-1) 0 2px"));
        assert!(site.contains("var(--gray-3) 2px 4px"));
        assert!(site.contains(".pb-4 { padding-bottom: 0.666667rem; }"));
        assert!(site.contains(
            r#"<section class="grid gap-2 section-separator pb-4" aria-label="Thesis scoreboard">"#
        ));
        assert!(site.contains(r#"<section class="grid gap-2" aria-label="Full report">"#));
        assert!(!site.contains(
            r#"<section class="grid gap-2 section-separator pb-4" aria-label="Full report">"#
        ));
        assert!(!site.contains("1px dotted"));
    }

    #[test]
    fn dark_green_surfaces_render_as_depressions() {
        let site = render_site(&fixture_view());

        assert!(site.contains(".edge-inset {\n      border-width: 2px;\n      border-style: solid;\n      border-color: var(--gray-1) var(--gray-3) var(--gray-3) var(--gray-1);"));
        assert!(site.contains(".edge-outset {\n      border-width: 2px;\n      border-style: solid;\n      border-color: var(--gray-3) var(--gray-1) var(--gray-1) var(--gray-3);"));
        assert!(site.contains(".edge-outline {\n      border: 2px solid var(--gray-3);"));
        assert!(site.contains(r#"class="metric-box edge-inset""#));
        assert!(site.contains(r#"class="min-w-0 bg-green-2 p-3 text-base edge-inset""#));
        assert!(site.contains(r#"class="grid bg-green-2 edge-inset""#));
        assert!(site.contains("overflow-x-auto bg-green-2 edge-inset p-3"));
        assert!(site.contains(r#"class="relative mt-4 bg-green-2 edge-inset text-gray-6""#));
        assert!(!site.contains(".bg-green-2,\n"));
        assert!(!site.contains("box-shadow"));
    }

    #[test]
    fn thesis_scoreboard_is_one_alternating_green_panel() {
        let site = render_site(&fixture_view());

        assert!(site.contains(
            ".thesis-row:nth-child(odd):not(:first-child) {\n      background: var(--green-3);\n    }"
        ));
        assert!(site.contains(r#"class="grid bg-green-2 edge-inset""#));
        assert!(site.contains(r#"class="thesis-row bg-table-header text-gray-5""#));
        assert!(site.contains(r#"class="thesis-row bg-green-2""#));
        assert!(!site.contains("thesis-table"));
        assert!(!site.contains("thesis-header"));
        assert!(
            !site.contains(".thesis-table {\n      gap: 1px;\n      background: var(--gray-3);")
        );
    }

    #[test]
    fn detail_sections_render_as_clickable_controls() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#"class="text-gray-6">Wars</span>"#));
        assert!(site.contains(r#"class="text-gray-5 text-sm""#));
        assert!(site.contains(r#"class="justify-self-end min-w-detail-action bg-gray-2 text-gray-5 text-center whitespace-nowrap edge-outset px-2 py-1 max-md:justify-self-start" data-detail-action="open">Open &gt;</span>"#));
        assert!(site.contains(r#"class="justify-self-end min-w-detail-action bg-gray-2 text-gray-5 text-center whitespace-nowrap edge-outset px-2 py-1 max-md:justify-self-start hidden" data-detail-action="close">Close v</span>"#));
        assert!(site.contains(r#"details[open] [data-detail-action="close"]"#));
        assert!(site.contains(r#"class="bg-gray-2 edge-outline""#));
        assert!(site.contains(".edge-outline {\n      border: 2px solid var(--gray-3);"));
        assert!(site.contains(
            "details.edge-outline:hover,\n    details.edge-outline:focus-within {\n      border-color: var(--gray-4);"
        ));
        assert!(site.contains("[data-detail-action]:hover {\n      color: var(--gray-6);"));
        assert!(!site.contains(".detail-section > summary:hover .detail-summary"));
        assert!(site.contains(".detail-content {\n      display: grid;\n      gap: 0.5rem;\n      background: var(--gray-2);"));
        assert!(!site.contains("detail-action-close"));
        assert!(!site.contains("detail-section"));
    }

    #[test]
    fn website_css_uses_shared_palette() {
        let css = site_css();

        for (name, color) in palette::CSS_COLORS {
            assert!(css.contains(&format!("--{name}: {};", color.css())));
        }
    }

    #[test]
    fn embeds_hfnss_pixel_font_css() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#"font-family: "HFNSS";"#));
        assert!(site.contains(r#"src: url("data:font/ttf;base64,"#));
        assert!(site.contains(r#"--pixel-font-size: 24px;"#));
        assert!(site.contains("-webkit-font-smoothing: none;"));
        assert!(site.contains("font-smooth: never;"));
        assert!(site.contains("line-height: 1;"));
        assert!(!site.contains("leading-"));
        assert!(site.contains(".text-base { font-size: var(--pixel-font-size); }"));
        assert!(site.contains(".text-sm { font-size: var(--pixel-font-size); }"));
        assert!(site.contains(".text-gray-4 { color: var(--gray-4); }"));
        assert!(site.contains(".max-w-none { max-width: none; }"));
        assert!(site.contains(".break-anywhere { overflow-wrap: anywhere; }"));
        assert!(site.contains(r#"class="w-full max-w-none text-gray-5 text-base break-anywhere""#));
        assert!(!site.contains("content-copy"));
        assert!(!site.contains(".metric-context"));
        assert!(site.contains(".metric-box:first-child {\n      grid-column: 1 / -1;"));
        assert!(site.contains("font-size: calc(var(--pixel-font-size) * 2);"));
        assert!(site.contains(
            ".metric-box:not(:first-child) {\n      grid-template-columns: max-content max-content;"
        ));
        assert!(site.contains("font-size: var(--pixel-font-size);"));
        assert!(!site.contains("line-height: 1.05;"));
        assert!(!site.contains("max-width: 78ch;"));
        assert!(site.contains(r#"font-family: var(--pixel-font-stack);"#));
        assert!(!site.contains("cdn.rawgit.com/Chadtech-Online-1"));
        assert!(!site.contains("Fira Code"));
    }

    #[test]
    fn renders_complete_html_document() {
        let site = render_site(&fixture_view());

        assert!(site.starts_with("<!doctype html>"));
        assert!(site.contains("<title>Ocotelolco</title>"));
        assert!(site
            .contains(r#"<link rel="icon" type="image/svg+xml" href="data:image/svg+xml;base64,"#));
        assert!(site.contains(r#"<h2 class="text-base font-normal text-gray-4">Performance</h2>"#));
        assert!(site
            .contains(r#"<h2 class="text-base font-normal text-gray-4">Thesis Scoreboard</h2>"#));
        assert!(site.contains(r#"<h2 class="text-base font-normal text-gray-4">Full Report</h2>"#));
        assert!(!site.contains("Campaign 1 Performance"));
        assert!(site.contains("Ocotelolco Campaign 1"));
        assert!(site.contains("a trading, predicting, and betting project, by Chadtech"));
        assert!(site.contains("Final return vs S&amp;P 500"));
        assert!(site.contains("High vs S&amp;P 500"));
        assert!(site.contains("Low vs S&amp;P 500"));
        assert!(site.contains("Long-form writings on my trades, context, caveats, and lessons"));
        assert!(site.contains(
            r#"<span class="text-base text-right whitespace-nowrap max-md:text-left">Returns</span>"#
        ));
        assert!(!site.contains("<span>Details</span>"));
        assert!(!site.contains("Sources And Notes"));
        assert!(!site.contains("detail-sources-and-notes"));
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
        assert!(site.contains("edge-outset"));
        assert!(!site.contains("shadow"));
        assert!(!site.contains("box-shadow"));
        assert!(!site.contains("relative bg-gray-2 p-4 text-gray-6 edge-outset"));
        assert!(site.contains("max-md:grid-cols-1"));
        assert!(!site.contains("performance.exe"));
        assert!(!site.contains(r#"class="chart-window""#));
        assert!(site.contains(r#"<script id="performance-chart-data" type="application/json">"#));
        assert!(site.ends_with("</html>\n"));
    }

    #[test]
    fn performance_section_uses_report_content_width() {
        let site = render_site(&fixture_view());

        assert!(site.contains(
            r#"<section class="relative min-w-0 bg-gray-2 py-4 text-gray-6 max-md:py-2" aria-label="Rate of return chart">"#
        ));
        assert!(!site.contains(
            r#"<section class="relative min-w-0 bg-gray-2 p-4 text-gray-6 max-md:p-2" aria-label="Rate of return chart">"#
        ));
    }

    #[test]
    fn chart_resizes_to_its_viewport() {
        let site = render_site(&fixture_view());

        assert!(site.contains(".performance-chart-svg {\n      width: 100%;"));
        assert!(site.contains("const chartViewport = svg.parentElement;"));
        assert!(site.contains("const minimumWidth = 920;"));
        assert!(site.contains("const width = Math.max(minimumWidth, Math.floor(availableWidth));"));
        assert!(site.contains("const margin = { top: 24, right: 74, bottom: 60, left: 20 };"));
        assert!(site.contains("new ResizeObserver(renderChart).observe(chartViewport);"));
        assert!(site.contains(r#"<title id="performance-chart-title">"#));
    }

    #[test]
    fn embeds_raw_chart_and_banner_data() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#""account_points":[{"date":"2025-10-28""#));
        assert!(site.contains(r#""sp500_points":[{"date":"2025-10-28""#));
        assert!(site.contains(r#"<img class="site-banner "#));
        assert!(site.contains(r#"src="data:image/png;base64,iVBORw0KGgo"#));
        assert!(!site.contains(r#"src="../ocotelolco_banner.png""#));
    }

    #[test]
    fn banner_png_has_alpha_channel() {
        assert_eq!(&BANNER_IMAGE_PNG[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&BANNER_IMAGE_PNG[12..16], b"IHDR");
        assert_eq!(BANNER_IMAGE_PNG[25], 6);
    }

    #[test]
    fn hfnss_ttf_is_embedded_font_data() {
        assert_eq!(&HFNSS_FONT_TTF[..4], b"\x00\x01\x00\x00");
        assert!(HFNSS_FONT_TTF.len() > 16_000);
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
