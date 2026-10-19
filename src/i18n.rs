use std::path::Path;

use mdlite::export::ExportError;

use crate::settings::{AccentColor, Language};

#[derive(Clone, Copy)]
pub struct I18n {
    pub language: Language,
}

#[derive(Clone, Copy)]
pub enum UnsavedReason {
    CloseTab,
}

impl I18n {
    pub const fn new(language: Language) -> Self {
        Self { language }
    }

    fn t(&self, zh: &'static str, en: &'static str) -> &'static str {
        match self.language {
            Language::Zh => zh,
            Language::En => en,
        }
    }

    pub fn language_toggle_label(&self) -> &'static str {
        match self.language {
            Language::Zh => "EN",
            Language::En => "中文",
        }
    }

    pub fn language_toggle_tooltip(&self) -> &'static str {
        match self.language {
            Language::Zh => "Switch to English",
            Language::En => "切换到中文",
        }
    }

    pub fn new_document(&self) -> &'static str {
        self.t("新建", "New")
    }

    pub fn open(&self) -> &'static str {
        self.t("打开", "Open")
    }

    pub fn save(&self) -> &'static str {
        self.t("保存", "Save")
    }

    pub fn save_as(&self) -> &'static str {
        self.t("另存为", "Save As")
    }

    pub fn settings(&self) -> &'static str {
        self.t("设置", "Settings")
    }

    pub fn about(&self) -> &'static str {
        self.t("关于", "About")
    }

    pub fn close(&self) -> &'static str {
        self.t("关闭", "Close")
    }

    pub fn export_pdf(&self) -> &'static str {
        self.t("导出 PDF", "Export PDF")
    }

    pub fn export_word(&self) -> &'static str {
        self.t("导出 Word", "Export Word")
    }

    pub fn edit_mode(&self) -> &'static str {
        self.t("编辑", "Edit")
    }

    pub fn split_mode(&self) -> &'static str {
        self.t("分栏", "Split")
    }

    pub fn preview_mode(&self) -> &'static str {
        self.t("预览", "Preview")
    }

    pub fn modifier_key(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        }
    }

    pub fn shortcut(&self, keys: &str) -> String {
        format!("{}+{keys}", self.modifier_key())
    }

    pub fn tooltip_new(&self) -> String {
        format!(
            "{}  {}",
            self.t("新建文档", "New document"),
            self.shortcut("N")
        )
    }

    pub fn tooltip_open(&self) -> String {
        format!(
            "{}  {}",
            self.t("打开 Markdown 文件", "Open a Markdown file"),
            self.shortcut("O")
        )
    }

    pub fn tooltip_save(&self) -> String {
        format!("{}  {}", self.save(), self.shortcut("S"))
    }

    pub fn tooltip_save_as(&self) -> String {
        format!(
            "{}  {}",
            self.t("另存为 Markdown", "Save as Markdown"),
            self.shortcut("Shift+S")
        )
    }

    pub fn tooltip_export_pdf(&self) -> &'static str {
        self.t(
            "导出为 PDF（使用本机 Edge/Chrome）",
            "Export as PDF (uses local Edge/Chrome)",
        )
    }

    pub fn tooltip_export_word(&self) -> &'static str {
        self.t("导出为 Word .docx", "Export as Word .docx")
    }

    pub fn untitled_markdown_numbered(&self, number: u64) -> String {
        match self.language {
            Language::Zh => format!("未命名-{number}.md"),
            Language::En => format!("Untitled-{number}.md"),
        }
    }

    pub fn untitled_stem_numbered(&self, number: u64) -> String {
        match self.language {
            Language::Zh => format!("未命名-{number}"),
            Language::En => format!("Untitled-{number}"),
        }
    }

    pub fn welcome_notice(&self) -> &'static str {
        self.t(
            "欢迎使用 MarkLite · 新建文档",
            "Welcome to MarkLite · New document",
        )
    }

    pub fn new_document_notice(&self) -> &'static str {
        self.t("新建文档", "New document")
    }

    pub fn opened_notice(&self, count: usize) -> String {
        match (self.language, count) {
            (Language::Zh, 1) => "已打开".to_owned(),
            (Language::Zh, n) => format!("已打开 {n} 个文件"),
            (Language::En, 1) => "Opened".to_owned(),
            (Language::En, n) => format!("Opened {n} files"),
        }
    }

    pub fn saved_notice(&self) -> &'static str {
        self.t("已保存", "Saved")
    }

    pub fn exported_notice(&self, file_name: &str) -> String {
        match self.language {
            Language::Zh => format!("已导出 {file_name}"),
            Language::En => format!("Exported {file_name}"),
        }
    }

    pub fn status_unsaved(&self) -> &'static str {
        self.t("● 未保存", "● Unsaved")
    }

    pub fn status_saved(&self) -> &'static str {
        self.t("已保存", "Saved")
    }

    pub fn status_new(&self) -> &'static str {
        self.t("新文档", "New document")
    }

    pub fn stats(&self, words: usize, lines: usize) -> String {
        match self.language {
            Language::Zh => format!("{words} 字 · {lines} 行"),
            Language::En => {
                let word_label = if words == 1 { "word" } else { "words" };
                let line_label = if lines == 1 { "line" } else { "lines" };
                format!("{words} {word_label} · {lines} {line_label}")
            }
        }
    }

    pub fn editor_placeholder(&self) -> &'static str {
        self.t(
            "开始写作，或打开一个 Markdown 文件…",
            "Start writing, or open a Markdown file…",
        )
    }

    pub fn preview_empty_title(&self) -> &'static str {
        self.t("预览区域", "Preview")
    }

    pub fn preview_empty_hint(&self) -> &'static str {
        self.t(
            "在编辑区写点什么，或按 Ctrl+O 打开文件",
            "Start writing, or press Ctrl+O to open a file",
        )
    }

    pub fn unsaved_title(&self) -> &'static str {
        self.t("有未保存的修改", "Unsaved changes")
    }

    pub fn unsaved_message(&self, name: &str, _reason: UnsavedReason) -> String {
        match self.language {
            Language::Zh => format!("“{name}”尚未保存。关闭标签页前要保存吗？"),
            Language::En => {
                format!("“{name}” has unsaved changes. Save before closing this tab?")
            }
        }
    }

    pub fn close_file_title(&self) -> &'static str {
        self.t("关闭文件", "Close file")
    }

    pub fn close_file_message(&self, name: &str) -> String {
        match self.language {
            Language::Zh => format!("确定要关闭“{name}”吗？"),
            Language::En => format!("Close “{name}”?"),
        }
    }

    pub fn external_link_failed(&self, url: &str, error: &str) -> String {
        match self.language {
            Language::Zh => format!("无法打开链接：{url}\n\n{error}"),
            Language::En => format!("Could not open link: {url}\n\n{error}"),
        }
    }

    pub fn unsaved_tabs_title(&self) -> &'static str {
        self.t("有未保存的标签页", "Unsaved tabs")
    }

    pub fn unsaved_tabs_message(&self, names: &str) -> String {
        match self.language {
            Language::Zh => {
                format!("以下标签页尚未保存：\n{names}\n\n退出前要全部保存吗？")
            }
            Language::En => {
                format!(
                    "The following tabs have unsaved changes:\n{names}\n\nSave all before quitting?"
                )
            }
        }
    }

    pub fn save_all(&self) -> &'static str {
        self.t("全部保存", "Save All")
    }

    pub fn discard_all(&self) -> &'static str {
        self.t("全部不保存", "Discard All")
    }

    pub fn file_menu(&self) -> &'static str {
        self.t("文件", "File")
    }

    pub fn view_menu(&self) -> &'static str {
        self.t("查看", "View")
    }

    pub fn export_menu(&self) -> &'static str {
        self.t("导出", "Export")
    }

    pub fn help_menu(&self) -> &'static str {
        self.t("帮助", "Help")
    }

    pub fn new_tab(&self) -> &'static str {
        self.t("新建标签页", "New Tab")
    }

    pub fn close_tab(&self) -> &'static str {
        self.t("关闭标签页", "Close Tab")
    }

    pub fn exit(&self) -> &'static str {
        self.t("退出", "Exit")
    }

    pub fn new_tab_tooltip(&self) -> String {
        format!("{}  {}", self.new_tab(), self.shortcut("N"))
    }

    pub fn close_tab_tooltip(&self) -> String {
        format!("{}  {}", self.close_tab(), self.shortcut("W"))
    }

    pub fn tab_untitled(&self, number: u64) -> String {
        self.untitled_markdown_numbered(number)
    }

    pub fn discard(&self) -> &'static str {
        self.t("不保存", "Don't save")
    }

    pub fn cancel(&self) -> &'static str {
        self.t("取消", "Cancel")
    }

    pub fn error_title(&self) -> &'static str {
        self.t("操作失败", "Something went wrong")
    }

    pub fn open_dialog_title(&self) -> &'static str {
        self.t("打开 Markdown 文件", "Open Markdown file")
    }

    pub fn save_as_dialog_title(&self) -> &'static str {
        self.t("另存为 Markdown", "Save as Markdown")
    }

    pub fn export_pdf_dialog_title(&self) -> &'static str {
        self.t("导出为 PDF", "Export as PDF")
    }

    pub fn export_word_dialog_title(&self) -> &'static str {
        self.t("导出为 Word", "Export as Word")
    }

    pub fn markdown_filter(&self) -> &'static str {
        "Markdown"
    }

    pub fn all_files_filter(&self) -> &'static str {
        self.t("所有文件", "All files")
    }

    pub fn pdf_filter(&self) -> &'static str {
        self.t("PDF 文档", "PDF document")
    }

    pub fn word_filter(&self) -> &'static str {
        self.t("Word 文档", "Word document")
    }

    pub fn read_failed(&self, path: &Path, error: &str) -> String {
        match self.language {
            Language::Zh => format!("无法读取文件：{}\n\n{error}", path.display()),
            Language::En => format!("Could not read file: {}\n\n{error}", path.display()),
        }
    }

    pub fn encoding_failed(&self, path: &Path) -> String {
        match self.language {
            Language::Zh => format!(
                "无法打开文件：{}\n\n文件不是 UTF-8 或 GB18030 编码，当前版本暂不支持该编码。",
                path.display()
            ),
            Language::En => format!(
                "Could not open file: {}\n\nThe file is not UTF-8 or GB18030 encoded. This encoding is not supported yet.",
                path.display()
            ),
        }
    }

    pub fn save_failed(&self, path: &Path, error: &str) -> String {
        match self.language {
            Language::Zh => format!("保存失败：{}\n\n{error}", path.display()),
            Language::En => format!("Could not save: {}\n\n{error}", path.display()),
        }
    }

    pub fn settings_save_failed(&self, error: &str) -> String {
        match self.language {
            Language::Zh => format!("设置保存失败：{error}"),
            Language::En => format!("Could not save settings: {error}"),
        }
    }

    pub fn export_error(&self, error: &ExportError) -> String {
        match error {
            ExportError::BrowserNotFound => self.browser_missing().to_owned(),
            ExportError::Io(error) => self.file_io_failed(&error.to_string()),
            ExportError::Pdf(detail) => match self.language {
                Language::Zh => format!("PDF 生成失败：{detail}"),
                Language::En => format!("PDF generation failed: {detail}"),
            },
            ExportError::Docx(detail) => match self.language {
                Language::Zh => format!("Word 文档生成失败：{detail}"),
                Language::En => format!("Word document generation failed: {detail}"),
            },
        }
    }

    fn file_io_failed(&self, error: &str) -> String {
        match self.language {
            Language::Zh => format!("文件读写失败：{error}"),
            Language::En => format!("File I/O failed: {error}"),
        }
    }

    fn browser_missing(&self) -> &'static str {
        self.t(
            "未找到可用的浏览器内核。\n\nPDF 导出需要 Microsoft Edge 或 Google Chrome/Chromium。\nWindows 10/11 通常已自带 Edge；安装后无需额外配置，重新导出即可。",
            "No usable browser engine was found.\n\nPDF export requires Microsoft Edge or Google Chrome/Chromium.\nWindows 10/11 usually includes Edge. Install a browser and try again.",
        )
    }

    pub fn settings_title(&self) -> &'static str {
        self.t("设置", "Settings")
    }

    pub fn language_label(&self) -> &'static str {
        self.t("界面语言", "Language")
    }

    pub fn appearance_label(&self) -> &'static str {
        self.t("外观", "Appearance")
    }

    pub fn follow_system(&self) -> &'static str {
        self.t("跟随系统", "System")
    }

    pub fn light(&self) -> &'static str {
        self.t("浅色", "Light")
    }

    pub fn dark(&self) -> &'static str {
        self.t("深色", "Dark")
    }

    pub fn accent_label(&self) -> &'static str {
        self.t("主题色", "Accent color")
    }

    pub fn settings_note(&self) -> &'static str {
        self.t(
            "更改会立即生效并自动保存。",
            "Changes apply immediately and are saved automatically.",
        )
    }

    pub fn system_current(&self, dark: bool) -> String {
        match (self.language, dark) {
            (Language::Zh, true) => "跟随系统（当前：深色）".to_owned(),
            (Language::Zh, false) => "跟随系统（当前：浅色）".to_owned(),
            (Language::En, true) => "System (currently Dark)".to_owned(),
            (Language::En, false) => "System (currently Light)".to_owned(),
        }
    }

    pub fn accent_name(&self, accent: AccentColor) -> &'static str {
        match (self.language, accent) {
            (Language::Zh, AccentColor::Teal) => "松石绿",
            (Language::Zh, AccentColor::Ocean) => "海盐蓝",
            (Language::Zh, AccentColor::Amber) => "琥珀",
            (Language::Zh, AccentColor::Violet) => "紫罗兰",
            (Language::Zh, AccentColor::Rose) => "玫瑰",
            (Language::En, AccentColor::Teal) => "Teal",
            (Language::En, AccentColor::Ocean) => "Ocean",
            (Language::En, AccentColor::Amber) => "Amber",
            (Language::En, AccentColor::Violet) => "Violet",
            (Language::En, AccentColor::Rose) => "Rose",
        }
    }

    pub fn about_tagline(&self) -> &'static str {
        self.t(
            "轻量、专注的 Markdown 写作工具",
            "A lightweight, focused Markdown writing tool",
        )
    }

    pub fn about_description(&self) -> &'static str {
        self.t(
            "Rust · egui · comrak · docx-rs\nPDF 导出调用本机 Edge / Chrome 内核，Word 导出不需要安装 Office。\n文件默认以 UTF-8 保存；打开时会兼容 GB18030 编码。",
            "Rust · egui · comrak · docx-rs\nPDF export uses your local Edge/Chrome engine; Word export does not require Office.\nFiles are saved as UTF-8; GB18030 input is supported.",
        )
    }

    pub fn github_repository(&self) -> &'static str {
        self.t("GitHub 仓库", "GitHub Repository")
    }

    pub fn email_author(&self) -> &'static str {
        self.t("联系作者", "Email Author")
    }

    pub fn author_label(&self) -> &'static str {
        self.t("作者", "Author")
    }

    pub fn about_shortcuts_heading(&self) -> &'static str {
        self.t("快捷键", "Shortcuts")
    }

    pub fn about_shortcuts(&self) -> Vec<(String, &'static str)> {
        vec![
            (self.shortcut("N"), self.t("新建", "New")),
            (self.shortcut("O"), self.t("打开", "Open")),
            (self.shortcut("S"), self.t("保存", "Save")),
            (self.shortcut("Shift+S"), self.t("另存为", "Save as")),
            (
                format!(
                    "{} / {} / {}",
                    self.shortcut("1"),
                    self.shortcut("2"),
                    self.shortcut("3")
                ),
                self.t("编辑 / 分栏 / 预览", "Edit / Split / Preview"),
            ),
            (self.shortcut("W"), self.t("关闭标签页", "Close tab")),
            ("Ctrl+Tab".to_owned(), self.t("下一个标签页", "Next tab")),
            (
                "Ctrl+Shift+Tab".to_owned(),
                self.t("上一个标签页", "Previous tab"),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_static_labels() {
        assert_eq!(I18n::new(Language::Zh).export_pdf(), "导出 PDF");
        assert_eq!(I18n::new(Language::En).export_pdf(), "Export PDF");
    }

    #[test]
    fn translates_unsaved_dialog() {
        let zh = I18n::new(Language::Zh).unsaved_message("note.md", UnsavedReason::CloseTab);
        assert!(zh.contains("关闭标签页前"));
        let en = I18n::new(Language::En).unsaved_message("note.md", UnsavedReason::CloseTab);
        assert!(en.contains("before closing this tab"));
    }

    #[test]
    fn formats_clean_close_confirmation() {
        let zh = I18n::new(Language::Zh);
        assert_eq!(zh.close_file_title(), "关闭文件");
        assert!(zh.close_file_message("a.md").contains("a.md"));
        let en = I18n::new(Language::En);
        assert_eq!(en.close_file_title(), "Close file");
        assert_eq!(en.close_file_message("a.md"), "Close “a.md”?");
    }

    #[test]
    fn numbers_untitled_tabs() {
        let zh = I18n::new(Language::Zh);
        assert_eq!(zh.tab_untitled(3), "未命名-3.md");
        let en = I18n::new(Language::En);
        assert_eq!(en.tab_untitled(3), "Untitled-3.md");
    }

    #[test]
    fn formats_english_stats() {
        let i18n = I18n::new(Language::En);
        assert_eq!(i18n.stats(1, 1), "1 word · 1 line");
        assert_eq!(i18n.stats(3, 2), "3 words · 2 lines");
    }
}
