//! Markdown parsing, statistics, document titles, and small source rewrites
//! used by the preview and the exporters.

use std::path::Path;

use comrak::{Options, markdown_to_html};

/// Shared flavor used by the preview and every exporter.
///
/// Raw HTML is deliberately not passed through (`render.r#unsafe` stays false):
/// the PDF exporter renders a local file in a browser engine, so untrusted
/// raw HTML should not become executable content.
pub fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.alerts = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.github_pre_lang = false;
    options.render.r#unsafe = false;
    options.render.escape = false;
    options
}

/// Markdown -> HTML fragment (without `<html>`/`<head>`).
pub fn to_html(markdown: &str) -> String {
    markdown_to_html(markdown, &options())
}

/// Markdown -> HTML fragment prepared for PDF printing.
///
/// GFM task markers are replaced with Unicode boxes because comrak only emits
/// interactive checkbox markup when unsafe raw HTML is enabled. This keeps the
/// exporter safe while still producing visible checkboxes.
pub fn to_print_html(markdown: &str) -> String {
    let html = to_html(&task_markers_to_glyphs(markdown));
    localize_alert_titles(add_task_marker_classes(html))
}

fn add_task_marker_classes(html: String) -> String {
    html.replace("<li>☐", "<li class=\"task-item\">☐")
        .replace("<li>☑", "<li class=\"task-item\">☑")
        .replace("<li>\n<p>☐", "<li class=\"task-item\">\n<p>☐")
        .replace("<li>\n<p>☑", "<li class=\"task-item\">\n<p>☑")
}

fn localize_alert_titles(html: String) -> String {
    let mut html = html;
    for (english, chinese) in [
        ("Note", "提示"),
        ("Tip", "技巧"),
        ("Important", "重要"),
        ("Warning", "警告"),
        ("Caution", "注意"),
    ] {
        html = html.replace(
            &format!("<p class=\"markdown-alert-title\">{english}</p>"),
            &format!("<p class=\"markdown-alert-title\">{chinese}</p>"),
        );
    }
    html
}

/// Replace lines such as `- [x] done` with `- ☑ done`.
///
/// Only list items are changed. The rewrite is intentionally conservative so
/// inline code and ordinary prose are left untouched.
pub fn task_markers_to_glyphs(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len());
    for line in markdown.split_inclusive('\n') {
        output.push_str(&rewrite_task_marker_line(line));
    }
    output
}

fn rewrite_task_marker_line(line: &str) -> String {
    let (body, newline) = match line.strip_suffix("\r\n") {
        Some(body) => (body, "\r\n"),
        None => match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        },
    };

    let indent_len = body.len() - body.trim_start_matches(' ').len();
    let rest = &body[indent_len..];

    // Find an optional block-quote prefix first, then a list marker.
    let mut prefix_len = 0;
    let mut cursor = 0;
    while rest[cursor..].starts_with('>') {
        cursor += 1;
        if rest[cursor..].starts_with(' ') {
            cursor += 1;
        }
        prefix_len = cursor;
    }
    let after_quote = &rest[prefix_len..];
    let quote_prefix = &rest[..prefix_len];

    let after_list = strip_list_marker(after_quote);
    if after_list.len() == after_quote.len() {
        return line.to_owned();
    }
    let list_prefix_len = after_quote.len() - after_list.len();
    let list_prefix = &after_quote[..list_prefix_len];

    let item = after_list.trim_start();
    let leading_space = after_list.len() - item.len();
    let (glyph, rest_start) = if let Some(after) = item.strip_prefix("[ ]") {
        ("☐", after)
    } else if let Some(after) = item
        .strip_prefix("[x]")
        .or_else(|| item.strip_prefix("[X]"))
    {
        ("☑", after)
    } else {
        return line.to_owned();
    };

    if !(rest_start.is_empty() || rest_start.starts_with(char::is_whitespace)) {
        return line.to_owned();
    }

    let mut rewritten = String::with_capacity(line.len() + 4);
    rewritten.push_str(&body[..indent_len]);
    rewritten.push_str(quote_prefix);
    rewritten.push_str(list_prefix);
    rewritten.push_str(&" ".repeat(leading_space));
    rewritten.push_str(glyph);
    rewritten.push_str(rest_start);
    rewritten.push_str(newline);
    rewritten
}

/// Returns the part after a bullet/ordered-list marker, or the input unchanged.
fn strip_list_marker(input: &str) -> &str {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return input;
    }

    match bytes[0] {
        b'-' | b'*' | b'+' => {
            if bytes.len() > 1 && bytes[1] == b' ' {
                &input[2..]
            } else {
                input
            }
        }
        b'0'..=b'9' => {
            let mut i = 0;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i <= 9
                && i < bytes.len()
                && matches!(bytes[i], b'.' | b')')
                && i + 1 < bytes.len()
                && bytes[i + 1] == b' '
            {
                &input[i + 2..]
            } else {
                input
            }
        }
        _ => input,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// All Unicode scalar values, including whitespace.
    pub chars: usize,
    /// CJK characters count individually; Latin-like runs count as one word.
    pub words: usize,
    /// Physical lines, at least one for an empty editor.
    pub lines: usize,
}

pub fn stats(markdown: &str) -> Stats {
    let mut words = 0usize;
    let mut in_word = false;

    for ch in markdown.chars() {
        if is_cjk(ch) {
            words += 1;
            in_word = false;
        } else if ch.is_alphanumeric() || ch == '\'' || ch == '\u{2019}' {
            if !in_word {
                words += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }

    Stats {
        chars: markdown.chars().count(),
        words,
        lines: markdown.lines().count().max(1),
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
            | 0x3040..=0x30FF
            | 0xAC00..=0xD7AF
    )
}

/// Best-effort title: first ATX heading, otherwise the file stem.
pub fn document_title(markdown: &str, source: Option<&Path>) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if !(1..=6).contains(&hashes) {
            continue;
        }
        let rest = trimmed[hashes..].trim();
        let title = rest.trim_end_matches('#').trim().trim_matches('*').trim();
        if !title.is_empty() {
            return strip_inline_markup(title);
        }
    }

    source
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("未命名文档")
        .to_owned()
}

fn strip_inline_markup(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' | '*' | '_' | '~' => {}
            '[' => {
                // Keep the label, drop a following `(target)`.
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                    output.push(inner);
                }
                if chars.peek() == Some(&'(') {
                    chars.next();
                    let mut depth = 1;
                    for inner in chars.by_ref() {
                        match inner {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => output.push(ch),
        }
    }
    output.trim().to_owned()
}

/// Make relative image destinations absolute so the egui preview can find
/// images next to the Markdown file (egui resolves relative paths from the
/// process working directory).
pub fn absolutize_image_paths(markdown: &str, base_dir: &Path) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut cursor = 0;

    while let Some(relative_start) = markdown[cursor..].find("![") {
        let start = cursor + relative_start;
        let after_bang = start + 2;

        let Some(alt_close_rel) = markdown[after_bang..].find("](") else {
            break;
        };
        let alt_close = after_bang + alt_close_rel;
        let destination_start = alt_close + 2;

        let Some(dest_close_rel) = markdown[destination_start..].find(')') else {
            break;
        };
        let destination_close = destination_start + dest_close_rel;

        let raw_destination = &markdown[destination_start..destination_close];
        let (destination, suffix) = split_destination_and_title(raw_destination);

        output.push_str(&markdown[cursor..destination_start]);
        if should_absolutize(destination) {
            let path = base_dir.join(destination);
            match url::Url::from_file_path(&path) {
                Ok(url) => output.push_str(&format!("{}{}", url.as_str(), suffix)),
                Err(_) => output.push_str(raw_destination),
            }
        } else {
            output.push_str(raw_destination);
        }

        cursor = destination_close;
    }

    output.push_str(&markdown[cursor..]);
    output
}

fn split_destination_and_title(input: &str) -> (&str, &str) {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix('<')
        && let Some(end) = rest.find('>')
    {
        let destination = &rest[..end];
        let suffix = &rest[end + 1..];
        return (destination, suffix);
    }

    match trimmed.find(char::is_whitespace) {
        Some(index) => (&trimmed[..index], &trimmed[index..]),
        None => (trimmed, ""),
    }
}

fn should_absolutize(destination: &str) -> bool {
    if destination.is_empty()
        || destination.starts_with('#')
        || destination.starts_with('/')
        || destination.starts_with("\\\\")
        || destination.starts_with("data:")
        || destination.starts_with("file:")
        || destination.starts_with("http://")
        || destination.starts_with("https://")
    {
        return false;
    }

    // Windows drive paths (`C:\...`) are already absolute.
    let bytes = destination.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
    {
        return false;
    }

    if let Ok(url) = url::Url::parse(destination)
        && url.scheme().len() > 1
    {
        return false;
    }

    true
}

/// Strip a leading YAML front matter block (`---` ... `---`).
///
/// The PDF/Word exporters already hide front matter; this is used by the egui
/// preview, whose Markdown renderer does not know about front matter.
pub fn strip_front_matter(markdown: &str) -> &str {
    let text = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    if !(text.starts_with("---\n") || text.starts_with("---\r\n")) {
        return markdown;
    }

    let mut offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        offset += line.len();
        if index == 0 {
            continue;
        }
        if line.trim() == "---" {
            return &text[offset..];
        }
    }
    markdown
}

/// Replace GitHub alert markers with a localized bold title for the preview.
///
/// `egui_commonmark` renders alerts with English default titles. The PDF and
/// Word exporters localize their own titles. This rewrite only affects the
/// preview string; the editor content is never modified.
pub fn localize_alerts_for_preview(markdown: &str) -> String {
    const LABELS: [(&str, &str); 5] = [
        ("NOTE", "提示"),
        ("TIP", "技巧"),
        ("IMPORTANT", "重要"),
        ("WARNING", "警告"),
        ("CAUTION", "注意"),
    ];

    if !markdown.contains("[!") {
        return markdown.to_owned();
    }

    let mut output = String::with_capacity(markdown.len());
    for line in markdown.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix("\r\n") {
            Some(body) => (body, "\r\n"),
            None => match line.strip_suffix('\n') {
                Some(body) => (body, "\n"),
                None => (line, ""),
            },
        };

        let indent_len = body.len() - body.trim_start_matches(' ').len();
        let indent = &body[..indent_len];
        let mut rest = &body[indent_len..];
        let mut quote = String::new();
        while let Some(after) = rest.strip_prefix('>') {
            quote.push('>');
            rest = after;
            if let Some(after_space) = rest.strip_prefix(' ') {
                quote.push(' ');
                rest = after_space;
            }
        }

        let marker = rest.trim();
        let Some(inner) = marker
            .strip_prefix("[!")
            .and_then(|value| value.strip_suffix(']'))
        else {
            output.push_str(line);
            continue;
        };
        let Some((_, label)) = LABELS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(inner.trim()))
        else {
            output.push_str(line);
            continue;
        };

        output.push_str(indent);
        output.push_str(&quote);
        output.push_str("**");
        output.push_str(label);
        output.push_str("**");
        output.push_str(newline);

        let bare_quote = quote.trim_end();
        if !bare_quote.is_empty() {
            output.push_str(indent);
            output.push_str(bare_quote);
            output.push_str(if newline.is_empty() { "\n" } else { newline });
        }
    }
    output
}

/// Minimal HTML text escaping for values interpolated into the print template.
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm_extensions() {
        let html = to_html(
            "# 标题\n\n| 列 A | 列 B |\n| --- | --- |\n| 1 | 2 |\n\n~~删除~~\n\n- [ ] 待办\n",
        );
        assert!(html.contains("<table>"));
        assert!(html.contains("<del>删除</del>"));
        assert!(html.contains("type=\"checkbox\"") || html.contains("待办"));
    }

    #[test]
    fn task_markers_become_glyphs() {
        let input = "- [ ] 第一项\n- [x] 第二项\n> - [X] 引用中的任务\n\n普通 [ ] 文本";
        let output = task_markers_to_glyphs(input);
        assert!(output.contains("- ☐ 第一项"));
        assert!(output.contains("- ☑ 第二项"));
        assert!(output.contains("> - ☑ 引用中的任务"));
        assert!(output.contains("普通 [ ] 文本"));
    }

    #[test]
    fn counts_chinese_and_latin_words() {
        let stats = stats("Hello world\n你好，世界");
        assert_eq!(stats.words, 6);
        assert_eq!(stats.lines, 2);
    }

    #[test]
    fn finds_first_heading_for_title() {
        assert_eq!(
            document_title("---\ntitle: x\n---\n\n# **项目** 说明\n", None),
            "项目 说明"
        );
        assert_eq!(
            document_title("no heading", Some(Path::new("/tmp/note.md"))),
            "note"
        );
    }

    #[test]
    fn strips_yaml_front_matter() {
        let input = "---\ntitle: demo\n---\n\n# 标题\n";
        assert_eq!(strip_front_matter(input), "\n# 标题\n");
        assert_eq!(strip_front_matter("# 标题"), "# 标题");
        assert_eq!(strip_front_matter("---\nno closing"), "---\nno closing");
    }

    #[test]
    fn localizes_alerts_for_preview() {
        let input = "> [!TIP]\n> 内容\n\n> [!warning]\n> 小心\n";
        let output = localize_alerts_for_preview(input);
        assert!(output.contains("> **技巧**\n>\n> 内容"));
        assert!(output.contains("> **警告**\n>\n> 小心"));
        assert!(!output.contains("[!TIP]"));
    }

    #[test]
    fn absolutizes_local_images_only() {
        let base = Path::new("/tmp/docs");
        let input = "![a](pics/a.png) ![b](<pics/b c.png>) ![c](https://x/y.png)";
        let output = absolutize_image_paths(input, base);
        assert!(output.contains("file:///tmp/docs/pics/a.png"));
        assert!(output.contains("file:///tmp/docs/pics/b%20c.png"));
        assert!(output.contains("https://x/y.png"));
    }
}
