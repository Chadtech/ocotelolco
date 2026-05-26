use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn default_output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("outputs")
        .join("ocotelolco.html")
}

pub fn write_site(output_path: impl AsRef<Path>) -> io::Result<()> {
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, render_site())
}

fn render_site() -> String {
    let template = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Ocotelolco</title>
  <style>
    :root {
      color-scheme: dark;
      --green-1: #030907;
      --green-2: #071d10;
      --green-3: #082208;
      --gray-1: #131610;
      --gray-2: #2c2826;
      --gray-3: #57524f;
      --gray-5: #b0a69a;
      --gray-6: #e0d6ca;
    }

    * {
      box-sizing: border-box;
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
      opacity: 0.38;
    }

    .page {
      min-height: 100vh;
      padding: 32px;
      display: flex;
      align-items: stretch;
    }

    .window {
      width: min(960px, 100%);
      min-height: min(620px, calc(100vh - 64px));
      margin: auto;
      padding: 4px;
      position: relative;
      background: var(--gray-2);
      color: var(--gray-6);
    }

    .window::before,
    .window::after,
    .frame::before,
    .frame::after {
      position: absolute;
      content: "";
      pointer-events: none;
    }

    .window::before {
      top: 0;
      left: 0;
      right: 0;
      height: 2px;
      background: var(--gray-3);
    }

    .window::after {
      top: 0;
      bottom: 0;
      left: 0;
      width: 2px;
      background: var(--gray-3);
    }

    .frame {
      min-height: calc(min(620px, calc(100vh - 64px)) - 8px);
      position: relative;
      display: flex;
      flex-direction: column;
      background: var(--gray-2);
    }

    .frame::before {
      right: -4px;
      bottom: -4px;
      left: -2px;
      height: 2px;
      background: var(--gray-1);
    }

    .frame::after {
      top: -2px;
      right: -4px;
      bottom: -4px;
      width: 2px;
      background: var(--gray-1);
    }

    .titlebar {
      padding: 4px 8px;
      display: flex;
      justify-content: space-between;
      gap: 16px;
      color: var(--green-1);
      background: var(--gray-5);
      font-weight: 700;
      line-height: 1.3;
    }

    .body {
      margin-top: 4px;
      padding: 4px;
      flex: 1;
      position: relative;
      background: var(--green-2);
    }

    .body::before {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 2px;
      background: var(--gray-1);
      content: "";
    }

    .body::after {
      position: absolute;
      top: 0;
      bottom: 0;
      left: 0;
      width: 2px;
      background: var(--gray-1);
      content: "";
    }

    @media (max-width: 820px) {
      .page {
        padding: 16px;
      }

      .window,
      .frame {
        min-height: calc(100vh - 32px);
      }
    }
  </style>
</head>
<body>
  <main class="page">
    <div class="window" aria-label="Ocotelolco website">
      <div class="frame">
        <header class="titlebar">
          <span>ocotelolco</span>
        </header>
        <section class="body" aria-label="Report content"></section>
      </div>
    </div>
  </main>
</body>
</html>
"#;

    template.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_complete_html_document() {
        let site = render_site();

        assert!(site.starts_with("<!doctype html>"));
        assert!(site.contains("<title>Ocotelolco</title>"));
        assert!(site.contains("ocotelolco"));
        assert!(site.ends_with("</html>\n"));
    }

    #[test]
    fn renders_empty_report_body() {
        let site = render_site();

        assert!(site.contains(r#"<section class="body" aria-label="Report content"></section>"#));
        assert!(!site.contains("<table"));
        assert!(!site.contains(r#"class="metric""#));
    }
}
