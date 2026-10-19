//! PDF export through the locally installed Edge/Chrome/Chromium.
//!
//! Rather than shipping a browser engine (hundreds of MB), MarkLite renders a
//! self-contained print stylesheet and asks the browser already present on the
//! machine to print it to PDF. Windows 10/11 ships with Edge, so the common
//! Windows case has no extra runtime dependency.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use tempfile::TempDir;
use url::Url;

use crate::export::ExportError;
use crate::markdown::{escape_html, to_print_html};

const PRINT_CSS: &str = r##"
:root {
    color-scheme: light;
}
* { box-sizing: border-box; }
html {
    background: #ffffff;
    -webkit-text-size-adjust: 100%;
}
body {
    margin: 0;
    padding: 0;
    color: #24292f;
    background: #ffffff;
    font-family: "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei",
                 "PingFang SC", "Noto Sans CJK SC", "Hiragino Sans GB", sans-serif;
    font-size: 10.5pt;
    line-height: 1.75;
    overflow-wrap: break-word;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
}
main {
    width: 100%;
    max-width: none;
    margin: 0 auto;
}
.markdown-body > :first-child { margin-top: 0 !important; }
.markdown-body > :last-child { margin-bottom: 0 !important; }

h1, h2, h3, h4, h5, h6 {
    color: #1f2328;
    font-weight: 700;
    line-height: 1.32;
    margin: 1.7em 0 0.65em;
    break-after: avoid-page;
    page-break-after: avoid;
}
h1 {
    font-size: 2em;
    margin-top: 0;
    padding-bottom: 0.28em;
    border-bottom: 1.5px solid #d8dee4;
}
h2 {
    font-size: 1.52em;
    padding-bottom: 0.22em;
    border-bottom: 1px solid #e6e9ec;
}
h3 { font-size: 1.28em; }
h4 { font-size: 1.1em; }
h5 { font-size: 1em; }
h6 { font-size: 0.92em; color: #57606a; }

p {
    margin: 0 0 1em;
    orphans: 3;
    widows: 3;
}
a {
    color: #0b628c;
    text-decoration: none;
    border-bottom: 1px solid rgba(11, 98, 140, 0.28);
}
strong { font-weight: 700; color: #1f2328; }
del { color: #6e7781; }
mark { background: #fff3b0; padding: 0 0.1em; }
kbd {
    display: inline-block;
    padding: 0.06em 0.4em;
    border: 1px solid #d0d7de;
    border-bottom-width: 2px;
    border-radius: 4px;
    background: #f6f8fa;
    font-family: inherit;
    font-size: 0.88em;
}

code {
    font-family: "Cascadia Code", "Cascadia Mono", Consolas, "SFMono-Regular",
                 Menlo, "Noto Sans Mono CJK SC", monospace;
    font-size: 0.88em;
    padding: 0.12em 0.36em;
    border-radius: 4px;
    background: #f2f4f6;
    color: #24292f;
}
pre {
    margin: 1.1em 0;
    padding: 12px 14px;
    background: #f6f8fa;
    border: 1px solid #e4e8eb;
    border-radius: 7px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    word-break: break-word;
    line-height: 1.55;
}
pre code {
    display: block;
    padding: 0;
    background: transparent;
    font-size: 0.9em;
}
blockquote {
    margin: 1.2em 0;
    padding: 0.15em 0 0.15em 1em;
    color: #4e5964;
    border-left: 3px solid #b7c7c2;
    background: #fafbfb;
    break-inside: avoid;
}
blockquote > :last-child { margin-bottom: 0; }

ul, ol { margin: 0 0 1em; padding-left: 1.65em; }
li { margin: 0.2em 0; }
li > p { margin: 0.25em 0; }
li > ul, li > ol { margin-top: 0.2em; margin-bottom: 0.3em; }
ul.contains-task-list { list-style: none; padding-left: 0.4em; }
li.task-item { list-style: none; }
li.task-item::marker { content: ""; }

table {
    width: 100%;
    margin: 1.2em 0;
    border-collapse: collapse;
    border-spacing: 0;
    font-size: 0.94em;
    break-inside: auto;
}
thead { display: table-header-group; }
tr { break-inside: avoid; page-break-inside: avoid; }
th, td {
    padding: 6px 9px;
    border: 1px solid #d5dce1;
    text-align: left;
    vertical-align: top;
}
th {
    background: #f0f4f3;
    font-weight: 650;
    color: #1f2328;
}
tbody tr:nth-child(even) td { background: #fbfcfc; }

img {
    max-width: 100%;
    height: auto;
    border-radius: 5px;
}
hr {
    height: 0;
    margin: 2em 0;
    border: 0;
    border-top: 1px solid #d8dee4;
}
sup, sub { font-size: 0.72em; line-height: 0; }

.markdown-alert {
    margin: 1.2em 0;
    padding: 0.15em 1em 0.15em 1.1em;
    border-left: 4px solid #0969da;
    border-radius: 0 6px 6px 0;
    background: #f3f8fd;
    break-inside: avoid;
}
.markdown-alert-title {
    margin: 0.55em 0 0.35em;
    font-weight: 700;
    color: #0969da;
}
.markdown-alert-tip { border-color: #1a7f37; background: #f2faf4; }
.markdown-alert-tip .markdown-alert-title { color: #1a7f37; }
.markdown-alert-important { border-color: #8250df; background: #f8f4fd; }
.markdown-alert-important .markdown-alert-title { color: #8250df; }
.markdown-alert-warning { border-color: #9a6700; background: #fffaf0; }
.markdown-alert-warning .markdown-alert-title { color: #9a6700; }
.markdown-alert-caution { border-color: #cf222e; background: #fff5f5; }
.markdown-alert-caution .markdown-alert-title { color: #cf222e; }

.footnotes {
    margin-top: 2.2em;
    padding-top: 0.7em;
    border-top: 1px solid #e4e8eb;
    color: #57606a;
    font-size: 0.88em;
}
.footnotes ol { padding-left: 1.4em; }
.footnote-backref { border-bottom: 0; }

dl { margin: 1em 0; }
dt { font-weight: 650; margin-top: 0.6em; }
dd { margin: 0.15em 0 0.5em 1.4em; }

@page {
    size: A4;
    margin: 18mm 16mm 20mm;
}
@media print {
    a { color: #0b628c; }
    h1, h2, h3, h4 { break-after: avoid-page; }
    pre { break-inside: auto; }
    table, blockquote, img, .markdown-alert { break-inside: avoid; }
}
"##;

/// Build a complete, self-contained printable HTML document.
pub fn markdown_to_print_html(markdown: &str, title: &str, base_dir: Option<&Path>) -> String {
    let body = to_print_html(markdown);
    let title = if title.trim().is_empty() {
        crate::APP_NAME
    } else {
        title.trim()
    };
    let base = base_dir
        .and_then(|dir| Url::from_directory_path(dir).ok())
        .map(|url| format!("<base href=\"{}\">", escape_html(url.as_str())))
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="{app} {version}">
<title>{title}</title>
{base}
<style>{css}</style>
</head>
<body>
<main class="markdown-body">
{body}
</main>
</body>
</html>
"#,
        app = crate::APP_NAME,
        version = crate::APP_VERSION,
        title = escape_html(title),
        base = base,
        css = PRINT_CSS,
        body = body,
    )
}

/// Export Markdown as an A4 PDF.
///
/// `source_dir` is used as the base for relative image paths.
pub fn export_pdf(
    markdown: &str,
    output: &Path,
    title: &str,
    source_dir: Option<&Path>,
) -> Result<(), ExportError> {
    let html = markdown_to_print_html(markdown, title, source_dir);
    let temp = TempDir::new()?;
    let html_path = temp.path().join("document.html");
    let pdf_path = temp.path().join("document.pdf");
    fs::write(&html_path, html.as_bytes())?;

    let file_url = Url::from_file_path(&html_path).map_err(|_| {
        ExportError::Pdf("Could not create a file URL for the temporary HTML.".to_owned())
    })?;

    let candidates = browser_candidates();
    let mut spawned_any = false;
    let mut failures: Vec<String> = Vec::new();

    for (browser_index, program) in candidates.iter().enumerate() {
        let mut not_installed = false;

        for (mode_index, headless) in ["--headless=new", "--headless"].iter().enumerate() {
            let profile = temp
                .path()
                .join(format!("profile-{browser_index}-{mode_index}"));
            if let Err(error) = fs::create_dir_all(&profile) {
                failures.push(format!(
                    "Could not create temporary browser profile directory: {error}"
                ));
                continue;
            }

            let mut command = Command::new(program);
            command
                .arg(headless)
                .arg("--disable-gpu")
                .arg("--hide-scrollbars")
                .arg("--mute-audio")
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("--disable-extensions")
                .arg("--disable-background-networking")
                .arg("--disable-sync")
                .arg("--run-all-compositor-stages-before-draw")
                .arg("--virtual-time-budget=6000")
                .arg("--no-pdf-header-footer")
                .arg("--print-to-pdf-no-header")
                .arg("--allow-file-access-from-files")
                .arg(format!("--user-data-dir={}", profile.display()))
                .arg(format!("--print-to-pdf={}", pdf_path.display()))
                .arg(file_url.as_str())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());

            match run_browser(command, Duration::from_secs(30)) {
                Ok(Some((status, stderr))) => {
                    spawned_any = true;
                    if pdf_is_ready(&pdf_path) {
                        fs::copy(&pdf_path, output)?;
                        return Ok(());
                    }
                    failures.push(format!(
                        "{} ({headless}) exited with status {status}{}",
                        program.display(),
                        format_stderr(&stderr)
                    ));
                }
                Ok(None) => {
                    spawned_any = true;
                    failures.push(format!(
                        "{} ({headless}) did not finish within 30 seconds.",
                        program.display()
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    not_installed = true;
                    break;
                }
                Err(error) => {
                    spawned_any = true;
                    failures.push(format!("Could not start {}: {error}", program.display()));
                }
            }

            let _ = fs::remove_file(&pdf_path);
        }

        if not_installed {
            continue;
        }
    }

    if !spawned_any {
        return Err(ExportError::BrowserNotFound);
    }

    let detail = failures
        .into_iter()
        .next()
        .unwrap_or_else(|| "The browser did not produce a PDF file.".to_owned());
    Err(ExportError::Pdf(detail))
}

fn pdf_is_ready(path: &Path) -> bool {
    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.len() > 4)
            .unwrap_or(false)
}

fn format_stderr(stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        let excerpt: String = stderr.chars().take(500).collect();
        format!("：{excerpt}")
    }
}

fn run_browser(
    mut command: Command,
    timeout: Duration,
) -> std::io::Result<Option<(ExitStatus, String)>> {
    use wait_timeout::ChildExt;

    let mut child = command.spawn()?;
    match child.wait_timeout(timeout)? {
        Some(status) => {
            let mut stderr = String::new();
            if let Some(pipe) = child.stderr.take() {
                let _ = pipe.take(16 * 1024).read_to_string(&mut stderr);
            }
            Ok(Some((status, stderr)))
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Ok(None)
        }
    }
}

fn browser_candidates() -> Vec<PathBuf> {
    let mut candidates = platform_browser_candidates();
    candidates.dedup();
    candidates
}

#[cfg(target_os = "windows")]
fn platform_browser_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(base) = std::env::var_os(key) {
            let base = PathBuf::from(base);
            candidates.push(base.join("Microsoft/Edge/Application/msedge.exe"));
            candidates.push(base.join("Google/Chrome/Application/chrome.exe"));
            candidates.push(base.join("Chromium/Application/chrome.exe"));
            candidates.push(base.join("BraveSoftware/Brave-Browser/Application/brave.exe"));
        }
    }

    for name in ["msedge.exe", "chrome.exe", "chromium.exe", "brave.exe"] {
        candidates.push(PathBuf::from(name));
    }

    base_filter(candidates)
}

#[cfg(target_os = "macos")]
fn platform_browser_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut app_roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        app_roots.push(PathBuf::from(home).join("Applications"));
    }

    for root in app_roots {
        candidates.push(root.join("Microsoft Edge.app/Contents/MacOS/Microsoft Edge"));
        candidates.push(root.join("Google Chrome.app/Contents/MacOS/Google Chrome"));
        candidates.push(root.join("Chromium.app/Contents/MacOS/Chromium"));
        candidates.push(root.join("Brave Browser.app/Contents/MacOS/Brave Browser"));
    }

    for name in [
        "microsoft-edge",
        "google-chrome",
        "chromium",
        "brave-browser",
    ] {
        candidates.push(PathBuf::from(name));
    }

    base_filter(candidates)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_browser_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
        "microsoft-edge-stable",
        "brave-browser",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();

    for path in [
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/microsoft-edge",
        "/snap/bin/chromium",
    ] {
        candidates.push(PathBuf::from(path));
    }

    base_filter(candidates)
}

/// Drop absolute candidate paths that do not exist; keep bare command names so
/// the OS can resolve them through PATH.
fn base_filter(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut filtered = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.is_absolute() && !candidate.is_file() {
            continue;
        }
        if !filtered.contains(&candidate) {
            filtered.push(candidate);
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_html_is_self_contained() {
        let html = markdown_to_print_html(
            "# 你好\n\n**粗体** 与 `代码`\n\n- [x] 完成\n\n> [!TIP]\n> 提示内容",
            "测试 <文档>",
            Some(Path::new("/tmp/docs")),
        );
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<title>测试 &lt;文档&gt;</title>"));
        assert!(html.contains("☑ 完成"));
        assert!(html.contains("class=\"task-item\""));
        assert!(html.contains("技巧"));
        assert!(html.contains("file:///tmp/docs/"));
        assert!(html.contains("print-color-adjust"));
        assert!(!html.contains("http://"));
    }

    #[test]
    fn pdf_export_smoke() {
        if std::env::var_os("MDLITE_TEST_PDF").is_none() {
            return;
        }

        let output = std::env::temp_dir().join(format!("mdlite-test-{}.pdf", std::process::id()));
        export_pdf(
            "# MarkLite PDF 测试\n\n中文段落，混合 **English** 和 `code`。\n\n| 列 | 值 |\n|---|---|\n| A | 1 |\n",
            &output,
            "MarkLite PDF 测试",
            None,
        )
        .expect("PDF export failed");
        let bytes = fs::read(&output).expect("missing PDF");
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 1000);
        let _ = fs::remove_file(output);
    }
}
