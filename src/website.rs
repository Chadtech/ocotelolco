use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
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

const BANNER_IMAGE_WEBP: &[u8] = include_bytes!("../ocotelolco_banner.webp");
const BACKGROUND_IMAGE_WEBP: &[u8] = include_bytes!("../ocotelolco_bg.webp");
const HFNSS_FONT_TTF: &[u8] = include_bytes!("../HFNSS.ttf");
const FAVICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" shape-rendering="crispEdges"><path fill="#6f4a05" fill-rule="evenodd" d="M8 6h18v4h2v14h-2v4H8v-2H6V8h2zm4 6v12h10V12z"/><path fill="#dba51e" fill-rule="evenodd" d="M6 4h18v4h2v14h-2v4H6v-2H4V6h2zm4 6v12h10V10z"/><path fill="#fff0a0" d="M8 6h14v2H8zM6 8h2v14H6z"/><path fill="#8a6208" d="M10 24h14v2H10zM24 10h2v12h-2z"/></svg>"##;
const ASSET_DIRECTORY: &str = "assets";
const BACKGROUND_ASSET: &str = "ocotelolco-bg.webp";
const BANNER_ASSET: &str = "ocotelolco-banner.webp";
const PERFORMANCE_START: Date = Date::new(2025, 10, 28);
const PERFORMANCE_END: Date = Date::new(2026, 4, 28);
const CONTENT_COPY: &str = "w-full max-w-none text-gray-5 text-base break-anywhere";
const OVERVIEW_PANEL: &str = "min-w-0 bg-green-2 p-3 text-base edge-inset";
const OVERVIEW_GRID: &str = "grid grid-cols-2 gap-2 max-md:grid-cols-1";
const METRIC_GRID: &str = "grid grid-cols-2 gap-2 max-md:grid-cols-1";
const METRIC_BOX: &str = "grid min-w-0 items-center bg-green-2 edge-inset p-3";
const METRIC_PRIMARY: &str =
    "col-span-full grid-cols-1 justify-items-center gap-2-5 py-4 text-center max-md:col-span-1";
const METRIC_SECONDARY: &str = "grid-cols-max justify-center gap-4";
const METRIC_LABEL: &str = "text-base text-gray-5";
const METRIC_VALUE: &str = "text-base text-gray-6 whitespace-nowrap";
const THESIS_TABLE: &str = "grid bg-green-2 edge-inset";
const THESIS_ROW: &str =
    "grid grid-cols-thesis items-start gap-3 min-w-0 p-3 text-base max-md:grid-cols-1";
const THESIS_HEADER: &str =
    "grid grid-cols-thesis items-start gap-3 min-w-0 bg-table-header p-3 text-base text-gray-5 max-md:grid-cols-1";
const THESIS_RETURN: &str = "text-base text-right whitespace-nowrap max-md:text-left";
const DETAIL_LIST: &str = "grid gap-1-5";
const DETAIL_SECTION: &str = "bg-gray-2 edge-outline";
const DETAIL_SUMMARY: &str =
    "detail-summary grid grid-cols-detail items-center gap-3 p-3 text-base max-md:grid-cols-1";
const DETAIL_CONTENT: &str = "grid gap-3 bg-gray-2 px-3 py-4";
const DETAIL_SUBSECTION: &str = "grid gap-2 pt-1";
const DETAIL_TITLE: &str = "text-gray-6";
const DETAIL_TEASER: &str = "text-gray-5 text-sm";
const DETAIL_ACTION: &str = "justify-self-end min-w-detail-action bg-gray-2 text-gray-5 text-center whitespace-nowrap edge-outset px-2 py-1 max-md:justify-self-start";
const DETAIL_ACTION_CLOSE: &str = "hidden";

pub fn default_output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("outputs")
        .join("ocotelolco.html")
}

pub fn github_pages_output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("index.html")
}

pub fn write_site(output_path: impl AsRef<Path>) -> io::Result<()> {
    let output_path = output_path.as_ref();
    let view = load_site_view()?;
    let html = render_site(&view);

    let output_directory = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_directory)?;

    fs::write(output_path, html)?;
    write_site_assets(output_directory)
}

fn write_site_assets(output_directory: &Path) -> io::Result<()> {
    let asset_directory = output_directory.join(ASSET_DIRECTORY);
    fs::create_dir_all(&asset_directory)?;
    fs::write(
        asset_directory.join(BACKGROUND_ASSET),
        BACKGROUND_IMAGE_WEBP,
    )?;
    fs::write(asset_directory.join(BANNER_ASSET), BANNER_IMAGE_WEBP)
}

pub fn deploy_site() -> io::Result<()> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    require_main_branch(repository)?;

    let output_path = github_pages_output_path();
    write_site(&output_path)?;
    let nojekyll_path = output_path
        .parent()
        .expect("GitHub Pages output path has a parent")
        .join(".nojekyll");
    if !nojekyll_path.exists() {
        fs::write(&nojekyll_path, "")?;
    }
    println!("Generated {}", output_path.display());

    require_git_success(
        repository,
        &[
            "add",
            "--",
            "docs/index.html",
            "docs/.nojekyll",
            "docs/assets/ocotelolco-bg.webp",
            "docs/assets/ocotelolco-banner.webp",
        ],
    )?;

    match git_status(
        repository,
        &[
            "diff",
            "--cached",
            "--quiet",
            "--",
            "docs/index.html",
            "docs/.nojekyll",
            "docs/assets/ocotelolco-bg.webp",
            "docs/assets/ocotelolco-banner.webp",
        ],
    )? {
        GitStatus::Success => println!("Website output is unchanged; no commit needed."),
        GitStatus::Difference => {
            require_git_success(
                repository,
                &[
                    "commit",
                    "-m",
                    "Deploy website",
                    "--only",
                    "--",
                    "docs/index.html",
                    "docs/.nojekyll",
                    "docs/assets/ocotelolco-bg.webp",
                    "docs/assets/ocotelolco-banner.webp",
                ],
            )?;
        }
        GitStatus::Failure(status) => {
            return Err(command_error("git diff", status));
        }
    }

    require_git_success(repository, &["push", "origin", "main"])?;
    println!("Deployed website to GitHub Pages.");
    Ok(())
}

fn require_main_branch(repository: &Path) -> io::Result<()> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repository)
        .output()?;
    if !output.status.success() {
        return Err(command_error("git branch --show-current", output.status));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if branch != "main" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("website deployment must run from main; current branch is {branch:?}"),
        ));
    }
    Ok(())
}

enum GitStatus {
    Success,
    Difference,
    Failure(ExitStatus),
}

fn git_status(repository: &Path, arguments: &[&str]) -> io::Result<GitStatus> {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .status()?;
    Ok(match status.code() {
        Some(0) => GitStatus::Success,
        Some(1) => GitStatus::Difference,
        _ => GitStatus::Failure(status),
    })
}

fn require_git_success(repository: &Path, arguments: &[&str]) -> io::Result<()> {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(command_error(
            &format!("git {}", arguments.join(" ")),
            status,
        ))
    }
}

fn command_error(command: &str, status: ExitStatus) -> io::Error {
    io::Error::other(format!("{command} exited with {status}"))
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
      min-height: 100vh;
      color: var(--gray-6);
      background: var(--green-3);
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-kerning: none;
      font-synthesis: none;
      isolation: isolate;
      line-height: 1;
    }

    .page-background {
      position: fixed;
      inset: 0;
      z-index: 0;
      width: 100%;
      height: 100%;
      object-fit: cover;
      opacity: 0.32;
      pointer-events: none;
      user-select: none;
    }

    body::before {
      position: fixed;
      inset: 0;
      z-index: 1;
      background: rgb(3 24 11 / 72%);
      content: "";
      pointer-events: none;
    }

    body > main {
      position: relative;
      z-index: 2;
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
    .justify-center { justify-content: center; }
    .justify-items-center { justify-items: center; }
    .justify-self-end { justify-self: end; }
    .grid-cols-1 { grid-template-columns: minmax(0, 1fr); }
    .grid-cols-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .grid-cols-max { grid-template-columns: max-content max-content; }
    .grid-cols-thesis { grid-template-columns: minmax(0, 1fr) minmax(3.666667rem, max-content); }
    .grid-cols-detail { grid-template-columns: minmax(7.5rem, 0.34fr) minmax(0, 1fr) max-content; }
    .grid-cols-return { grid-template-columns: 1fr minmax(4.583333rem, max-content); }
    .col-span-full { grid-column: 1 / -1; }
    .w-full { width: 100%; }
    .h-auto { height: auto; }
    .h-banner { height: clamp(6.25rem, 24vw, 11.666667rem); }
    .max-w-window { max-width: 46.666667rem; }
    .max-w-copy { max-width: 70ch; }
    .max-w-none { max-width: none; }
    .min-w-0 { min-width: 0; }
    .min-w-chart { min-width: 38.333333rem; }
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
    .pt-1 { padding-top: 0.166667rem; }
    .gap-1 { gap: 0.166667rem; }
    .gap-2 { gap: 0.333333rem; }
    .gap-2-5 { gap: 0.416667rem; }
    .gap-3 { gap: 0.5rem; }
    .gap-4 { gap: 0.666667rem; }
    .gap-1-5 { gap: 0.25rem; }
    .break-anywhere { overflow-wrap: anywhere; }
    .whitespace-nowrap { white-space: nowrap; }
    .overflow-visible { overflow: visible; }
    .overflow-x-auto { overflow-x: auto; }
    .object-cover { object-fit: cover; }
    .object-center { object-position: center; }
    .bg-current { background: currentColor; }
    .bg-gray-2 { background: var(--gray-2); }
    .bg-gray-5 { background: var(--gray-5); }
    .bg-green-1 { background: var(--green-1); }
    .bg-green-2 { background: var(--green-2); }
    .bg-green-3 { background: var(--green-3); }
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
    .text-2x { font-size: calc(var(--pixel-font-size) * 2); }
    .text-center { text-align: center; }
    .text-right { text-align: right; }
    .cursor-pointer { cursor: pointer; }
    .list-none { list-style: none; }
    .outline-none { outline: none; }
    .select-none { user-select: none; }
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
    .result-positive { color: var(--green-7); }
    .result-negative { color: var(--red-2); }
    .result-neutral { color: var(--gray-6); }
    details.edge-outline:hover,
    details.edge-outline:focus-within {
      border-color: var(--gray-4);
    }
    details > summary.list-none::-webkit-details-marker {
      display: none;
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
    svg {
      font-family: var(--pixel-font-stack);
      font-size: var(--pixel-font-size);
      font-kerning: none;
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
      .max-md\:col-span-1 { grid-column: span 1 / span 1; }
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
    html(attrs(&[("lang", "en")]), vec![site_head(), site_body(view)])
}

fn site_head() -> Node {
    head(
        Vec::new(),
        vec![
            meta(attrs(&[("charset", "utf-8")]), Vec::new()),
            meta(
                attrs(&[
                    ("name", "viewport"),
                    ("content", "width=device-width, initial-scale=1"),
                ]),
                Vec::new(),
            ),
            title(Vec::new(), vec![Node::text("Ocotelolco")]),
            link(
                vec![
                    attr("rel", "icon"),
                    attr("type", "image/svg+xml"),
                    attr("href", favicon_data_uri()),
                ],
                Vec::new(),
            ),
            style(Vec::new(), vec![Node::raw_text(site_css())]),
        ],
    )
}

fn site_body(view: &SiteView) -> Node {
    body(
        Vec::new(),
        vec![
            background_image(),
            main_element(
                vec![class("min-h-screen grid gap-4 p-8 max-md:p-4")],
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

fn background_image() -> Node {
    img(
        vec![
            class("page-background"),
            attr("src", format!("{ASSET_DIRECTORY}/{BACKGROUND_ASSET}")),
            attr("alt", ""),
            attr("aria-hidden", "true"),
            attr("width", "1448"),
            attr("height", "1086"),
        ],
        Vec::new(),
    )
}

fn site_banner() -> Node {
    header(
        vec![class("grid gap-2")],
        vec![
            banner_image(),
            p(
                vec![class(
                    "mx-auto w-full max-w-window break-anywhere text-center text-base text-gray-6",
                )],
                vec![Node::text(
                    "a trading, predicting, and betting project, by Chadtech",
                )],
            ),
        ],
    )
}

fn banner_image() -> Node {
    img(
        vec![
            class("block h-banner mx-auto w-full max-w-window object-cover object-center"),
            attr("src", format!("{ASSET_DIRECTORY}/{BANNER_ASSET}")),
            attr("alt", "Ocotelolco"),
            attr("width", "1672"),
            attr("height", "941"),
        ],
        Vec::new(),
    )
}

fn desktop_window(view: &SiteView) -> Node {
    div(
        vec![
            class(
                "relative mx-auto min-h-window w-full max-w-window bg-gray-2 edge-outset p-1 text-gray-6 max-md:min-h-window",
            ),
            attr("aria-label", "Ocotelolco website"),
        ],
        vec![div(
            vec![class(
                "relative flex min-h-window-inner flex-col bg-gray-2 max-md:min-h-window",
            )],
            vec![
                header(
                    vec![class("min-h-7 bg-gray-5 px-2 py-1 text-green-1")],
                    vec![span(Vec::new(), vec![Node::text(&view.content.title)])],
                ),
                report_body(view),
            ],
        )],
    )
}

fn report_body(view: &SiteView) -> Node {
    section(
        vec![
            class("relative mt-1 flex-1 bg-gray-2 p-5 max-md:p-3"),
            attr("aria-label", "Report content"),
        ],
        vec![div(
            vec![class("grid min-w-0 gap-4")],
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
        vec![
            class("grid grid-cols-1 items-start gap-4 section-separator pb-4"),
            attr("aria-label", "Campaign overview"),
        ],
        vec![
            div(
                vec![class("grid gap-2")],
                vec![p(
                    vec![class(CONTENT_COPY)],
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
        vec![
            class("grid gap-4 section-separator pb-4"),
            attr("aria-label", "Campaign context"),
        ],
        vec![
            div(
                vec![class(format!("grid gap-2 {CONTENT_COPY}"))],
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
        vec![
            class("relative min-w-0 bg-gray-2 text-gray-6"),
            attr("aria-label", "Rate of return chart"),
        ],
        vec![
            chart_copy(view),
            div(
                vec![class("relative bg-green-2 edge-inset")],
                vec![
                    chart_viewport(),
                    return_table(
                        account_label,
                        &view.account_return,
                        sp500_label,
                        &view.sp500_return,
                    ),
                ],
            ),
        ],
    )
}

fn chart_copy(view: &SiteView) -> Node {
    let performance = &view.content.performance;
    div(
        vec![class("mb-4 grid gap-1")],
        vec![
            h2(
                vec![class("text-base font-normal text-gray-4")],
                vec![Node::text(&performance.title)],
            ),
            p(
                vec![class(CONTENT_COPY)],
                vec![
                    Node::text(&performance.summary),
                    Node::text(" Account balance return was "),
                    span(
                        vec![class("text-green-7")],
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
        vec![class(METRIC_GRID)],
        metrics
            .iter()
            .enumerate()
            .map(|(index, metric)| key_metric(metric, index == 0))
            .collect(),
    )
}

fn key_metric(metric: &KeyMetric, is_primary: bool) -> Node {
    let layout_class = if is_primary {
        METRIC_PRIMARY
    } else {
        METRIC_SECONDARY
    };
    let value_size_class = if is_primary { " text-2x" } else { "" };

    div(
        vec![class(format!("{METRIC_BOX} {layout_class}"))],
        vec![
            div(vec![class(METRIC_LABEL)], vec![Node::text(&metric.label)]),
            div(
                vec![class(format!(
                    "{METRIC_VALUE}{value_size_class} {}",
                    metric_value_class(&metric.value)
                ))],
                vec![Node::text(format_metric_value(&metric.value))],
            ),
        ],
    )
}

fn rules_summary(rules: &[String], takeaway: &str) -> Node {
    div(
        vec![class(OVERVIEW_GRID)],
        vec![
            div(
                vec![class(OVERVIEW_PANEL)],
                vec![
                    h2(
                        vec![class("text-base font-normal text-gray-4")],
                        vec![Node::text("Rules")],
                    ),
                    ol(
                        vec![class(format!("content-list {CONTENT_COPY}"))],
                        rules
                            .iter()
                            .map(|rule| li(Vec::new(), vec![Node::text(rule)]))
                            .collect(),
                    ),
                ],
            ),
            div(
                vec![class(OVERVIEW_PANEL)],
                vec![
                    h2(
                        vec![class("text-base font-normal text-gray-4")],
                        vec![Node::text("Prediction is not position")],
                    ),
                    p(vec![class(CONTENT_COPY)], vec![Node::text(takeaway)]),
                ],
            ),
        ],
    )
}

fn thesis_scoreboard(scoreboard: &ThesisScoreboard) -> Node {
    section(
        vec![
            class("grid gap-2 section-separator pb-4"),
            attr("aria-label", "Thesis scoreboard"),
        ],
        vec![
            h2(
                vec![class("text-base font-normal text-gray-4")],
                vec![Node::text(&scoreboard.title)],
            ),
            p(
                vec![class(CONTENT_COPY)],
                vec![Node::text(&scoreboard.summary)],
            ),
            div(vec![class(THESIS_TABLE)], thesis_rows(&scoreboard.rows)),
        ],
    )
}

fn thesis_rows(rows: &[ThesisRow]) -> Vec<Node> {
    let mut nodes = Vec::with_capacity(rows.len() + 1);
    nodes.push(div(
        vec![class(THESIS_HEADER)],
        vec![
            span(Vec::new(), vec![Node::text("Thesis/tag")]),
            span(vec![class(THESIS_RETURN)], vec![Node::text("Returns")]),
        ],
    ));
    nodes.extend(
        rows.iter()
            .enumerate()
            .map(|(index, row)| thesis_row(row, index)),
    );
    nodes
}

fn thesis_row(row: &ThesisRow, index: usize) -> Node {
    let background_class = if index % 2 == 0 {
        "bg-green-2"
    } else {
        "bg-green-3"
    };

    div(
        vec![class(format!("{THESIS_ROW} {background_class}"))],
        vec![
            div(
                vec![class("grid gap-1")],
                vec![
                    span(vec![class("text-gray-5")], vec![Node::text(&row.title)]),
                    p(
                        vec![class("text-sm")],
                        vec![Node::text(&row.visible_summary)],
                    ),
                ],
            ),
            span(
                vec![class(format!(
                    "{THESIS_RETURN} {}",
                    return_figure_class(row.realized_return)
                ))],
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
        vec![class("grid gap-2"), attr("aria-label", "Full report")],
        vec![
            h2(
                vec![class("text-base font-normal text-gray-4")],
                vec![Node::text(&report.title)],
            ),
            p(vec![class(CONTENT_COPY)], vec![Node::text(&report.summary)]),
            div(
                vec![class(DETAIL_LIST)],
                report.sections.iter().map(detail_section).collect(),
            ),
        ],
    )
}

fn detail_section(section: &DetailSection) -> Node {
    let mut attributes = vec![
        attr("id", detail_topic_id(section.topic)),
        class(DETAIL_SECTION),
    ];
    if section.default_disclosure == DisclosureState::Expanded {
        attributes.push(attr("open", ""));
    }

    details(
        attributes,
        vec![
            summary(
                vec![class("cursor-pointer list-none outline-none select-none")],
                vec![div(
                    vec![class(DETAIL_SUMMARY)],
                    vec![
                        span(vec![class(DETAIL_TITLE)], vec![Node::text(&section.title)]),
                        span(
                            vec![class(DETAIL_TEASER)],
                            vec![Node::text(&section.summary)],
                        ),
                        span(
                            vec![class(DETAIL_ACTION), attr("data-detail-action", "open")],
                            vec![Node::text("Open >")],
                        ),
                        span(
                            vec![
                                class(format!("{DETAIL_ACTION} {DETAIL_ACTION_CLOSE}")),
                                attr("data-detail-action", "close"),
                            ],
                            vec![Node::text("Close v")],
                        ),
                    ],
                )],
            ),
            div(
                vec![class(DETAIL_CONTENT)],
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
        DetailBlock::Paragraph(text) => p(vec![class(CONTENT_COPY)], vec![Node::text(text)]),
        DetailBlock::OrderedList(items) => ol(
            vec![class(format!("content-list {CONTENT_COPY}"))],
            items
                .iter()
                .map(|item| li(Vec::new(), vec![Node::text(item)]))
                .collect(),
        ),
        DetailBlock::UnorderedList(items) => ul(
            vec![class(format!("content-list {CONTENT_COPY}"))],
            items.iter().map(render_list_item).collect(),
        ),
        DetailBlock::Subsection(subsection) => detail_subsection(subsection),
    }
}

fn detail_subsection(subsection: &DetailSubsection) -> Node {
    section(
        vec![class(DETAIL_SUBSECTION)],
        vec![
            h3(
                vec![class("text-base font-normal text-gray-5")],
                vec![Node::text(&subsection.title)],
            ),
            div(
                vec![class("grid gap-2")],
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
        vec![class(
            "relative min-w-0 min-h-chart overflow-x-auto bg-green-2 border-b-solid p-3 max-md:min-h-chart max-md:p-2",
        )],
        vec![svg(
            vec![
                attr("id", "performance-chart"),
                class(
                    "block h-auto min-w-chart w-full max-w-none min-h-svg overflow-visible text-gray-6 max-md:min-h-svg",
                ),
                attr("role", "img"),
                attr("aria-labelledby", "performance-chart-title"),
                attr("width", "920"),
                attr("height", "330"),
                attr("viewBox", "0 0 920 330"),
                attr("preserveAspectRatio", "xMidYMid meet"),
            ],
            vec![
                title(
                    attrs(&[("id", "performance-chart-title")]),
                    vec![Node::text(
                        "Account balance return compared with S&P 500 return",
                    )],
                ),
                text(
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
        vec![
            class("relative bg-green-2 text-gray-6"),
            attr("aria-label", "Rate of return table"),
        ],
        vec![
            div(
                vec![class(
                    "grid min-h-row-header grid-cols-return items-center gap-4 border-b-solid bg-table-header px-3 py-2 text-base text-gray-5 max-md:grid-cols-1 max-md:gap-1",
                )],
                vec![
                    span(Vec::new(), vec![Node::text("Portfolio/Index")]),
                    span(
                        vec![class("text-right text-gray-5 max-md:text-left")],
                        vec![Node::text("Rate of Return")],
                    ),
                ],
            ),
            return_row("account", account_label, account_return, true),
            return_row("index", sp500_label, sp500_return, false),
        ],
    )
}

fn return_row(legend_class: &str, name: &str, value: &str, has_bottom_separator: bool) -> Node {
    let separator_class = if has_bottom_separator {
        " border-b-dotted"
    } else {
        ""
    };

    div(
        vec![class(format!(
            "grid min-h-row grid-cols-return items-center gap-4 px-3 py-2 text-base max-md:grid-cols-1 max-md:gap-1{separator_class}"
        ))],
        vec![
            span(
                vec![class("flex items-center gap-2")],
                vec![
                    span(
                        vec![
                            class(format!(
                                "square-3 flex-none bg-current text-chart-{legend_class}"
                            )),
                            attr("aria-hidden", "true"),
                        ],
                        Vec::new(),
                    ),
                    Node::text(name),
                ],
            ),
            span(
                vec![class("text-right text-green-7 max-md:text-left")],
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

macro_rules! tag_helpers {
    ($($function:ident => $tag:literal),+ $(,)?) => {
        $(
            fn $function(attributes: Vec<Attribute>, children: Vec<Node>) -> Node {
                element($tag, attributes, children)
            }
        )+
    };
}

tag_helpers! {
    html => "html",
    head => "head",
    meta => "meta",
    title => "title",
    link => "link",
    style => "style",
    body => "body",
    main_element => "main",
    header => "header",
    img => "img",
    div => "div",
    span => "span",
    section => "section",
    h2 => "h2",
    h3 => "h3",
    p => "p",
    ol => "ol",
    ul => "ul",
    li => "li",
    code => "code",
    details => "details",
    summary => "summary",
    svg => "svg",
    text => "text",
    script => "script",
}

fn attr(name: &str, value: impl Into<String>) -> Attribute {
    (name.to_string(), value.into())
}

fn class(value: impl Into<String>) -> Attribute {
    attr("class", value)
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
            vec![
                class("external-link"),
                attr("href", "https://example.com?name=S&P 500&label=\"next\""),
            ],
            vec![Node::text("S&P 500 <returns>")],
        )
        .to_html();

        assert_eq!(
            html,
            r#"<a class="external-link" href="https://example.com?name=S&amp;P 500&amp;label=&quot;next&quot;">S&amp;P 500 &lt;returns&gt;</a>"#
        );
    }

    #[test]
    fn universal_reset_clears_default_margins() {
        let css = site_css();

        assert!(css.contains("* {\n      box-sizing: border-box;\n      margin: 0;\n    }"));
        assert!(!css.contains("h1,\n    h2,\n    h3,"));
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
        assert!(site.contains(
            r#"class="grid grid-cols-thesis items-start gap-3 min-w-0 bg-table-header p-3 text-base text-gray-5 max-md:grid-cols-1""#
        ));
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
        assert!(site.contains(
            r#"class="grid min-w-0 items-center bg-green-2 edge-inset p-3 col-span-full grid-cols-1 justify-items-center gap-2-5 py-4 text-center max-md:col-span-1""#
        ));
        assert!(site.contains(r#"class="min-w-0 bg-green-2 p-3 text-base edge-inset""#));
        assert!(site.contains(r#"class="grid bg-green-2 edge-inset""#));
        assert!(site.contains(r#"class="relative bg-green-2 edge-inset">"#));
        assert!(site.contains("overflow-x-auto bg-green-2 border-b-solid p-3"));
        assert!(site.contains(r#"class="relative bg-green-2 text-gray-6""#));
        assert!(!site.contains(r#"class="relative bg-green-2 edge-inset text-gray-6""#));
        assert!(!site.contains(".bg-green-2,\n"));
        assert!(!site.contains("box-shadow"));
    }

    #[test]
    fn performance_chart_and_table_share_one_inset_frame() {
        let site = render_site(&fixture_view());

        assert!(site.contains(
            r#"<div class="relative bg-green-2 edge-inset"><div class="relative min-w-0 min-h-chart overflow-x-auto bg-green-2 border-b-solid p-3"#
        ));
        assert!(site.contains(
            r#"<div class="relative bg-green-2 text-gray-6" aria-label="Rate of return table">"#
        ));
        assert!(site.contains(
            r#"max-md:grid-cols-1 max-md:gap-1 border-b-dotted"><span class="flex items-center gap-2">"#
        ));
        assert!(site.contains(
            r#"max-md:grid-cols-1 max-md:gap-1"><span class="flex items-center gap-2"><span class="square-3 flex-none bg-current text-chart-index""#
        ));
    }

    #[test]
    fn thesis_scoreboard_is_one_alternating_green_panel() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#"class="grid bg-green-2 edge-inset""#));
        assert!(site.contains(
            r#"class="grid grid-cols-thesis items-start gap-3 min-w-0 bg-table-header p-3 text-base text-gray-5 max-md:grid-cols-1""#
        ));
        assert!(site.contains(
            r#"class="grid grid-cols-thesis items-start gap-3 min-w-0 p-3 text-base max-md:grid-cols-1 bg-green-2""#
        ));
        assert!(site.contains(
            r#"class="grid grid-cols-thesis items-start gap-3 min-w-0 p-3 text-base max-md:grid-cols-1 bg-green-3""#
        ));
        assert!(!site.contains(".thesis-row"));
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
        assert!(site.contains(r#"class="grid gap-3 bg-gray-2 px-3 py-4""#));
        assert!(
            site.contains(r#"<summary class="cursor-pointer list-none outline-none select-none">"#)
        );
        assert!(!site.contains(".detail-content"));
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
        assert!(site.contains(".col-span-full { grid-column: 1 / -1; }"));
        assert!(site.contains(".text-2x { font-size: calc(var(--pixel-font-size) * 2); }"));
        assert!(site.contains(".grid-cols-max { grid-template-columns: max-content max-content; }"));
        assert!(!site.contains(".metric-box"));
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
    fn performance_section_uses_report_content_width_without_vertical_padding() {
        let site = render_site(&fixture_view());

        assert!(site.contains(
            r#"<section class="relative min-w-0 bg-gray-2 text-gray-6" aria-label="Rate of return chart">"#
        ));
        assert!(!site.contains(
            r#"<section class="relative min-w-0 bg-gray-2 py-4 text-gray-6 max-md:py-2" aria-label="Rate of return chart">"#
        ));
        assert!(!site.contains("max-md:pb-2"));
    }

    #[test]
    fn chart_resizes_to_its_viewport() {
        let site = render_site(&fixture_view());

        assert!(site.contains(".min-w-chart { min-width: 38.333333rem; }"));
        assert!(site.contains(
            r#"class="block h-auto min-w-chart w-full max-w-none min-h-svg overflow-visible text-gray-6 max-md:min-h-svg""#
        ));
        assert!(!site.contains(".performance-chart-svg"));
        assert!(site.contains("const chartViewport = svg.parentElement;"));
        assert!(site.contains("const minimumWidth = 920;"));
        assert!(site.contains("const width = Math.max(minimumWidth, Math.floor(availableWidth));"));
        assert!(site.contains("const margin = { top: 24, right: 74, bottom: 60, left: 20 };"));
        assert!(site.contains("new ResizeObserver(renderChart).observe(chartViewport);"));
        assert!(site.contains(r#"<title id="performance-chart-title">"#));
    }

    #[test]
    fn embeds_raw_chart_data_and_references_cacheable_images() {
        let site = render_site(&fixture_view());

        assert!(site.contains(r#""account_points":[{"date":"2025-10-28""#));
        assert!(site.contains(r#""sp500_points":[{"date":"2025-10-28""#));
        assert!(site.contains(r#"<img class="page-background" src="assets/ocotelolco-bg.webp""#));
        assert!(site.contains(r#"alt="" aria-hidden="true" width="1448" height="1086">"#));
        assert!(site.contains(".page-background {"));
        assert!(site.contains("background: rgb(3 24 11 / 72%);"));
        assert!(site.contains("body > main {"));
        assert!(!site.contains("--page-background-image"));
        assert!(!site.contains("ocotelolco_bg.png"));
        assert!(site.contains(
            r#"<img class="block h-banner mx-auto w-full max-w-window object-cover object-center""#
        ));
        assert!(site.contains(r#"src="assets/ocotelolco-banner.webp""#));
        assert!(!site.contains("data:image/png;base64,"));
    }

    #[test]
    fn optimized_images_are_webp_assets() {
        assert_eq!(&BANNER_IMAGE_WEBP[..4], b"RIFF");
        assert_eq!(&BANNER_IMAGE_WEBP[8..12], b"WEBP");
        assert!(BANNER_IMAGE_WEBP.len() < 200_000);
        assert_eq!(&BACKGROUND_IMAGE_WEBP[..4], b"RIFF");
        assert_eq!(&BACKGROUND_IMAGE_WEBP[8..12], b"WEBP");
        assert!(BACKGROUND_IMAGE_WEBP.len() < 350_000);
    }

    #[test]
    fn writes_cacheable_image_assets_next_to_site_output() {
        let directory = std::env::temp_dir().join(format!(
            "ocotelolco-website-assets-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);

        write_site_assets(&directory).unwrap();

        assert_eq!(
            fs::read(directory.join(ASSET_DIRECTORY).join(BANNER_ASSET)).unwrap(),
            BANNER_IMAGE_WEBP
        );
        assert_eq!(
            fs::read(directory.join(ASSET_DIRECTORY).join(BACKGROUND_ASSET)).unwrap(),
            BACKGROUND_IMAGE_WEBP
        );

        fs::remove_dir_all(directory).unwrap();
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
