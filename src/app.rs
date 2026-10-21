use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::icons;
use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Id, Key, Label, Layout, Margin, Modal,
    Modifiers, Panel, PointerButton, RichText, ScrollArea, Stroke, TextEdit, TextStyle,
    ThemePreference, ViewportCommand,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use mdlite::export::{docx, pdf};
use mdlite::markdown;

use crate::i18n::{I18n, UnsavedReason};
use crate::settings::{AccentColor, Language, Settings, ThemeMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Edit,
    Split,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn from_dark(dark: bool) -> Self {
        if dark { Self::Dark } else { Self::Light }
    }

    fn palette(self, accent: AccentColor) -> Palette {
        let colors = accent.colors(self == Self::Dark);
        if cfg!(target_os = "macos") {
            return match self {
                Self::Light => Palette {
                    toolbar: Color32::from_rgb(0xec, 0xec, 0xee),
                    editor_bg: Color32::from_rgb(0xf5, 0xf5, 0xf7),
                    preview_bg: Color32::from_rgb(0xf2, 0xf2, 0xf7),
                    page: Color32::from_rgb(0xff, 0xff, 0xff),
                    text: Color32::from_rgb(0x1d, 0x1d, 0x1f),
                    muted: Color32::from_rgb(0x6e, 0x6e, 0x73),
                    accent: colors.accent,
                    accent_button: colors.button,
                    accent_soft: colors.soft,
                    border: Color32::from_rgb(0xd1, 0xd1, 0xd6),
                    status: Color32::from_rgb(0xec, 0xec, 0xee),
                    code_bg: Color32::from_rgb(0xf2, 0xf2, 0xf7),
                    error: Color32::from_rgb(0xc0, 0x36, 0x2c),
                },
                Self::Dark => Palette {
                    toolbar: Color32::from_rgb(0x2c, 0x2c, 0x2e),
                    editor_bg: Color32::from_rgb(0x1c, 0x1c, 0x1e),
                    preview_bg: Color32::from_rgb(0x1c, 0x1c, 0x1e),
                    page: Color32::from_rgb(0x2c, 0x2c, 0x2e),
                    text: Color32::from_rgb(0xf2, 0xf2, 0xf7),
                    muted: Color32::from_rgb(0x98, 0x98, 0x9d),
                    accent: colors.accent,
                    accent_button: colors.button,
                    accent_soft: colors.soft,
                    border: Color32::from_rgb(0x3a, 0x3a, 0x3c),
                    status: Color32::from_rgb(0x2c, 0x2c, 0x2e),
                    code_bg: Color32::from_rgb(0x3a, 0x3a, 0x3c),
                    error: Color32::from_rgb(0xff, 0x69, 0x61),
                },
            };
        }
        match self {
            Self::Light => Palette {
                toolbar: Color32::from_rgb(0xfb, 0xfa, 0xf6),
                editor_bg: Color32::from_rgb(0xf4, 0xf1, 0xea),
                preview_bg: Color32::from_rgb(0xf0, 0xed, 0xe6),
                page: Color32::from_rgb(0xff, 0xff, 0xff),
                text: Color32::from_rgb(0x28, 0x25, 0x1f),
                muted: Color32::from_rgb(0x7e, 0x77, 0x6c),
                accent: colors.accent,
                accent_button: colors.button,
                accent_soft: colors.soft,
                border: Color32::from_rgb(0xe4, 0xde, 0xd3),
                status: Color32::from_rgb(0xf8, 0xf6, 0xf1),
                code_bg: Color32::from_rgb(0xf1, 0xf3, 0xf2),
                error: Color32::from_rgb(0xb3, 0x26, 0x1e),
            },
            Self::Dark => Palette {
                toolbar: Color32::from_rgb(0x20, 0x21, 0x1e),
                editor_bg: Color32::from_rgb(0x19, 0x1a, 0x17),
                preview_bg: Color32::from_rgb(0x15, 0x16, 0x13),
                page: Color32::from_rgb(0x23, 0x24, 0x1f),
                text: Color32::from_rgb(0xed, 0xea, 0xe2),
                muted: Color32::from_rgb(0xa5, 0x9f, 0x92),
                accent: colors.accent,
                accent_button: colors.button,
                accent_soft: colors.soft,
                border: Color32::from_rgb(0x36, 0x39, 0x32),
                status: Color32::from_rgb(0x1d, 0x1e, 0x1b),
                code_bg: Color32::from_rgb(0x2b, 0x2d, 0x28),
                error: Color32::from_rgb(0xf2, 0xa6, 0x9a),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    toolbar: Color32,
    editor_bg: Color32,
    preview_bg: Color32,
    page: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    accent_button: Color32,
    accent_soft: Color32,
    border: Color32,
    status: Color32,
    code_bg: Color32,
    error: Color32,
}

struct Document {
    id: u64,
    path: Option<PathBuf>,
    content: String,
    dirty: bool,
}

impl Document {
    fn untitled(id: u64) -> Self {
        Self {
            id,
            path: None,
            content: String::new(),
            dirty: false,
        }
    }

    fn from_file(id: u64, path: PathBuf, content: String) -> Self {
        Self {
            id,
            path: Some(path),
            content,
            dirty: false,
        }
    }

    fn display_name(&self, i18n: &I18n) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| i18n.tab_untitled(self.id))
    }

    fn base_name(&self, i18n: &I18n) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| i18n.untitled_stem_numbered(self.id))
    }
}

#[derive(Clone, Debug)]
enum PendingAction {
    CloseTab(u64),
    CloseApp,
}

#[derive(Clone, Copy)]
enum EditorAction {
    Copy,
    Cut,
    Paste,
    Find { replace: bool },
}

#[derive(Clone, Copy)]
enum ConfirmChoice {
    Save,
    Discard,
    Cancel,
}

pub struct MarkLiteApp {
    docs: Vec<Document>,
    active: usize,
    next_doc_id: u64,
    mode: Mode,
    settings: Settings,
    effective_theme: Theme,
    preview_cache: CommonMarkCache,
    pending: Option<PendingAction>,
    allow_close: bool,
    error_message: Option<String>,
    about_open: bool,
    settings_open: bool,
    notice: Option<String>,
    last_dir: Option<PathBuf>,
    focus_editor: bool,
    editor_id: Option<Id>,
    pending_editor_action: Option<EditorAction>,
    pending_editor_selection: Option<(u64, usize, usize)>,
    restore_editor_selection: bool,
    find_replace_open: bool,
    find_show_replace: bool,
    find_query: String,
    replace_query: String,
    find_current: usize,
    find_selected: bool,
    find_focus: bool,
    last_editor_selection: Option<(u64, usize, usize)>,
    window_title: String,
    #[cfg(target_os = "macos")]
    native_menu: Option<crate::native_menu::NativeMenu>,
    #[cfg(target_os = "macos")]
    native_menu_failed: bool,
}

impl MarkLiteApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Vec<PathBuf>) -> Self {
        let settings = Settings::load();
        let welcome = I18n::new(settings.language).welcome_notice().to_owned();
        let mut app = Self {
            docs: Vec::new(),
            active: 0,
            next_doc_id: 1,
            mode: Mode::Split,
            settings,
            effective_theme: Theme::Light,
            preview_cache: CommonMarkCache::default(),
            pending: None,
            allow_close: false,
            error_message: None,
            about_open: false,
            settings_open: false,
            notice: Some(welcome.clone()),
            last_dir: std::env::current_dir().ok(),
            focus_editor: true,
            editor_id: None,
            pending_editor_action: None,
            pending_editor_selection: None,
            restore_editor_selection: false,
            find_replace_open: false,
            find_show_replace: false,
            find_query: String::new(),
            replace_query: String::new(),
            find_current: 0,
            find_selected: false,
            find_focus: false,
            last_editor_selection: None,
            window_title: String::new(),
            #[cfg(target_os = "macos")]
            native_menu: None,
            #[cfg(target_os = "macos")]
            native_menu_failed: false,
        };

        app.install_fonts(&cc.egui_ctx);
        app.apply_styles(&cc.egui_ctx);
        app.update_effective_theme(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        if initial.is_empty() {
            app.add_untitled_document();
            app.notice = Some(welcome);
        } else {
            app.open_paths(&initial);
            if app.docs.is_empty() {
                app.add_untitled_document();
            }
        }
        app
    }

    fn install_fonts(&self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let data = egui::FontData::from_static(include_bytes!("../assets/fonts/Phosphor.ttf"));
        fonts.font_data.insert("phosphor".into(), data.into());
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(family.len().min(1), "phosphor".into());
        }

        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        if let Some(data) = load_system_font(&db, false) {
            fonts.font_data.insert("system-ui".into(), data.into());
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "system-ui".into());
            }
        }
        if let Some(data) = load_system_font(&db, true) {
            fonts.font_data.insert("system-mono".into(), data.into());
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "system-mono".into());
            }
        }

        ctx.set_fonts(fonts);
    }

    fn i18n(&self) -> I18n {
        I18n::new(self.settings.language)
    }

    fn palette(&self) -> Palette {
        self.effective_theme.palette(self.settings.accent)
    }

    fn update_effective_theme(&mut self, ctx: &egui::Context) {
        self.effective_theme = Theme::from_dark(ctx.theme() == egui::Theme::Dark);
    }

    fn apply_styles(&self, ctx: &egui::Context) {
        for egui_theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark = egui_theme == egui::Theme::Dark;
            let palette = Theme::from_dark(dark).palette(self.settings.accent);
            let mut style = (*ctx.style_of(egui_theme)).clone();
            let mut visuals = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };

            visuals.override_text_color = Some(palette.text);
            visuals.panel_fill = palette.preview_bg;
            visuals.window_fill = palette.page;
            visuals.window_stroke = Stroke::new(1.0, palette.border);
            visuals.faint_bg_color = palette.accent_soft;
            visuals.extreme_bg_color = palette.page;
            visuals.text_edit_bg_color = Some(palette.page);
            visuals.code_bg_color = palette.code_bg;
            visuals.hyperlink_color = palette.accent;
            visuals.error_fg_color = palette.error;
            let (widget_radius, window_radius, menu_radius) = if cfg!(target_os = "macos") {
                (6, 10, 8)
            } else if cfg!(target_os = "windows") {
                (4, 8, 4)
            } else {
                (7, 12, 10)
            };
            visuals.window_corner_radius = CornerRadius::same(window_radius);
            visuals.menu_corner_radius = CornerRadius::same(menu_radius);
            visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(
                palette.accent.r(),
                palette.accent.g(),
                palette.accent.b(),
                72,
            );
            visuals.selection.stroke = Stroke::new(1.0, palette.text);

            for widget in [
                &mut visuals.widgets.noninteractive,
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
                &mut visuals.widgets.open,
            ] {
                widget.corner_radius = CornerRadius::same(widget_radius);
            }

            visuals.widgets.noninteractive.bg_fill = palette.page;
            visuals.widgets.noninteractive.weak_bg_fill = palette.toolbar;
            visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.muted);
            visuals.widgets.inactive.bg_fill = palette.page;
            visuals.widgets.inactive.weak_bg_fill = palette.toolbar;
            visuals.widgets.inactive.bg_stroke = Stroke::NONE;
            visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
            visuals.widgets.hovered.bg_fill = palette.accent_soft;
            visuals.widgets.hovered.weak_bg_fill = palette.accent_soft;
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.accent);
            visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.accent);
            visuals.widgets.active.bg_fill = palette.accent_soft;
            visuals.widgets.active.weak_bg_fill = palette.accent_soft;
            visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
            visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.accent);
            visuals.widgets.open.bg_fill = palette.accent_soft;
            visuals.widgets.open.weak_bg_fill = palette.accent_soft;

            style.visuals = visuals;
            style.url_in_tooltip = true;
            if cfg!(target_os = "macos") {
                style.spacing.item_spacing = egui::vec2(7.0, 6.0);
                style.spacing.button_padding = egui::vec2(9.0, 4.0);
                style.spacing.interact_size.y = 26.0;
            } else if cfg!(target_os = "windows") {
                style.spacing.item_spacing = egui::vec2(8.0, 6.0);
                style.spacing.button_padding = egui::vec2(10.0, 4.0);
                style.spacing.interact_size.y = 27.0;
            } else {
                style.spacing.item_spacing = egui::vec2(8.0, 7.0);
                style.spacing.button_padding = egui::vec2(11.0, 5.0);
                style.spacing.interact_size.y = 28.0;
            }
            style
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(14.5));
            style
                .text_styles
                .insert(TextStyle::Button, FontId::proportional(13.5));
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(11.5));
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(20.0));
            style
                .text_styles
                .insert(TextStyle::Monospace, FontId::monospace(14.5));

            ctx.set_style_of(egui_theme, style);
        }

        ctx.set_theme(match self.settings.theme {
            ThemeMode::System => ThemePreference::System,
            ThemeMode::Light => ThemePreference::Light,
            ThemeMode::Dark => ThemePreference::Dark,
        });
    }

    fn persist_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.error_message = Some(self.i18n().settings_save_failed(&error));
        }
    }

    fn apply_settings_change(&mut self, ctx: &egui::Context) {
        self.apply_styles(ctx);
        self.update_effective_theme(ctx);
        self.persist_settings();
    }

    fn open_external(&mut self, ctx: &egui::Context, url: &str) {
        if let Err(error) = webbrowser::open(url) {
            self.error_message = Some(self.i18n().external_link_failed(url, &error.to_string()));
        }
        ctx.request_repaint();
    }

    fn request_editor_action(&mut self, action: EditorAction) {
        if self.mode == Mode::Preview {
            self.mode = Mode::Split;
        }
        match action {
            EditorAction::Find { replace } => {
                if !self.find_replace_open || self.find_show_replace != replace {
                    self.find_current = 0;
                    self.find_selected = false;
                }
                self.find_replace_open = true;
                self.find_show_replace = replace;
                self.find_focus = true;
                self.focus_editor = false;
            }
            _ => {
                self.pending_editor_selection = self.last_editor_selection;
                self.restore_editor_selection = self.pending_editor_selection.is_some();
                self.pending_editor_action = Some(action);
                self.focus_editor = true;
            }
        }
    }

    fn cache_editor_selection(&mut self, ctx: &egui::Context, editor_id: egui::Id, doc_id: u64) {
        let Some(state) = egui::widgets::text_edit::TextEditState::load(ctx, editor_id) else {
            return;
        };
        let Some(range) = state.cursor.char_range() else {
            return;
        };
        self.last_editor_selection = Some((doc_id, range.primary.index.0, range.secondary.index.0));
    }

    fn apply_pending_selection(&self, ctx: &egui::Context, editor_id: egui::Id) {
        let Some((doc_id, primary, secondary)) = self.pending_editor_selection else {
            return;
        };
        if doc_id != self.active_doc().id {
            return;
        }
        let char_count = self.active_doc().content.chars().count();
        let primary = primary.min(char_count);
        let secondary = secondary.min(char_count);
        let range = CCursorRange::two(CCursor::new(primary), CCursor::new(secondary));
        let mut state =
            egui::widgets::text_edit::TextEditState::load(ctx, editor_id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(ctx, editor_id);
    }

    fn dispatch_editor_action(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_editor_action else {
            return;
        };
        if self.mode == Mode::Preview {
            self.mode = Mode::Split;
            self.focus_editor = true;
            return;
        }
        let Some(editor_id) = self.editor_id else {
            self.focus_editor = true;
            return;
        };
        let focused = ctx.memory(|memory| memory.has_focus(editor_id));
        if self.restore_editor_selection {
            if !focused {
                self.focus_editor = true;
                return;
            }
            if self.pending_editor_selection.is_some() {
                self.apply_pending_selection(ctx, editor_id);
                self.restore_editor_selection = false;
                return;
            }
            self.restore_editor_selection = false;
        }
        if !focused {
            self.focus_editor = true;
            return;
        }
        match action {
            EditorAction::Copy => {
                ctx.input_mut(|input| input.events.push(egui::Event::Copy));
            }
            EditorAction::Cut => {
                ctx.input_mut(|input| input.events.push(egui::Event::Cut));
            }
            EditorAction::Paste => {
                match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                    Ok(text) => {
                        ctx.input_mut(|input| input.events.push(egui::Event::Paste(text)));
                    }
                    Err(error) => {
                        self.error_message = Some(self.i18n().clipboard_failed(&error.to_string()));
                    }
                }
            }
            EditorAction::Find { .. } => {}
        }
        self.pending_editor_action = None;
        self.pending_editor_selection = None;
    }

    fn uses_native_menu(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.native_menu.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    #[cfg(target_os = "macos")]
    fn ensure_native_menu(&mut self, ctx: &egui::Context) {
        if self.native_menu_failed {
            return;
        }
        if self.native_menu.as_ref().is_some_and(|menu| {
            menu.language == self.settings.language
                && menu.recent_files == self.settings.recent_files
        }) {
            return;
        }
        match crate::native_menu::NativeMenu::install(
            ctx,
            self.settings.language,
            &self.settings.recent_files,
        ) {
            Some(menu) => self.native_menu = Some(menu),
            None => self.native_menu_failed = true,
        }
    }

    #[cfg(target_os = "macos")]
    fn process_native_menu(&mut self, ctx: &egui::Context) {
        let actions = self
            .native_menu
            .as_ref()
            .map(|menu| menu.poll())
            .unwrap_or_default();
        let modal_open = self.settings_open
            || self.about_open
            || self.pending.is_some()
            || self.error_message.is_some();
        for action in actions {
            if modal_open && !matches!(action, crate::native_menu::NativeAction::Quit) {
                continue;
            }
            match action {
                crate::native_menu::NativeAction::NewTab => {
                    self.add_untitled_document();
                }
                crate::native_menu::NativeAction::Open => self.open_dialog(),
                crate::native_menu::NativeAction::Save => {
                    self.save_active();
                }
                crate::native_menu::NativeAction::SaveAs => {
                    self.save_active_as();
                }
                crate::native_menu::NativeAction::CloseTab => {
                    self.request_close_tab(self.active, ctx);
                }
                crate::native_menu::NativeAction::OpenRecent(path) => {
                    self.open_paths(&[path]);
                }
                crate::native_menu::NativeAction::ClearRecent => {
                    self.clear_recent_files();
                }
                crate::native_menu::NativeAction::Copy => {
                    self.request_editor_action(EditorAction::Copy);
                }
                crate::native_menu::NativeAction::Cut => {
                    self.request_editor_action(EditorAction::Cut);
                }
                crate::native_menu::NativeAction::Paste => {
                    self.request_editor_action(EditorAction::Paste);
                }
                crate::native_menu::NativeAction::Find => {
                    self.request_editor_action(EditorAction::Find { replace: false });
                }
                crate::native_menu::NativeAction::Replace => {
                    self.request_editor_action(EditorAction::Find { replace: true });
                }
                crate::native_menu::NativeAction::EditMode => {
                    self.mode = Mode::Edit;
                    self.focus_editor = true;
                }
                crate::native_menu::NativeAction::SplitMode => {
                    self.mode = Mode::Split;
                    self.focus_editor = true;
                }
                crate::native_menu::NativeAction::PreviewMode => {
                    self.mode = Mode::Preview;
                }
                crate::native_menu::NativeAction::ExportPdf => self.export_pdf(),
                crate::native_menu::NativeAction::ExportWord => self.export_docx(),
                crate::native_menu::NativeAction::Settings => self.settings_open = true,
                crate::native_menu::NativeAction::Quit => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                crate::native_menu::NativeAction::GitHub => {
                    self.open_external(ctx, mdlite::GITHUB_URL);
                }
            }
        }
    }

    fn active_doc(&self) -> &Document {
        &self.docs[self.active]
    }

    fn doc_index(&self, id: u64) -> Option<usize> {
        self.docs.iter().position(|doc| doc.id == id)
    }

    fn dirty_documents(&self) -> Vec<&Document> {
        self.docs.iter().filter(|doc| doc.dirty).collect()
    }

    fn has_dirty_documents(&self) -> bool {
        self.docs.iter().any(|doc| doc.dirty)
    }

    fn add_untitled_document(&mut self) -> u64 {
        let id = self.next_doc_id;
        self.next_doc_id = self.next_doc_id.wrapping_add(1).max(1);
        self.docs.push(Document::untitled(id));
        self.active = self.docs.len() - 1;
        self.focus_editor = true;
        self.notice = Some(self.i18n().new_document_notice().to_owned());
        id
    }

    fn active_display_name(&self) -> String {
        self.active_doc().display_name(&self.i18n())
    }

    fn dialog_dir(&self) -> Option<PathBuf> {
        self.active_doc()
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .or_else(|| self.last_dir.clone())
    }

    fn update_window_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "{}{} — MarkLite",
            self.active_display_name(),
            if self.active_doc().dirty { " *" } else { "" }
        );
        if self.window_title != title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if self.allow_close {
            return;
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            if self.has_dirty_documents() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.pending = Some(PendingAction::CloseApp);
            } else {
                self.allow_close = true;
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| {
                    let path = file.path();
                    if path.as_os_str().is_empty() || !path.is_file() {
                        None
                    } else {
                        Some(path.to_path_buf())
                    }
                })
                .collect()
        });
        if !dropped.is_empty() {
            self.open_paths(&dropped);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some()
            || self.error_message.is_some()
            || self.about_open
            || self.settings_open
        {
            return;
        }

        let native_menu = self.uses_native_menu();
        let mut new_mode = None;
        let mut action = None;
        let mut edit_action = None;
        let (replace_mods, replace_key) = if cfg!(target_os = "macos") {
            (Modifiers::COMMAND | Modifiers::ALT, Key::F)
        } else {
            (Modifiers::COMMAND, Key::H)
        };

        ctx.input_mut(|input| {
            if !native_menu {
                if input.consume_key(Modifiers::COMMAND, Key::N) {
                    action = Some(Shortcut::New);
                } else if input.consume_key(Modifiers::COMMAND, Key::O) {
                    action = Some(Shortcut::Open);
                } else if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
                    action = Some(Shortcut::SaveAs);
                } else if input.consume_key(Modifiers::COMMAND, Key::S) {
                    action = Some(Shortcut::Save);
                } else if input.consume_key(Modifiers::COMMAND, Key::W) {
                    action = Some(Shortcut::CloseTab);
                } else if input.consume_key(Modifiers::COMMAND, Key::Num1) {
                    new_mode = Some(Mode::Edit);
                } else if input.consume_key(Modifiers::COMMAND, Key::Num2) {
                    new_mode = Some(Mode::Split);
                } else if input.consume_key(Modifiers::COMMAND, Key::Num3) {
                    new_mode = Some(Mode::Preview);
                } else if input.consume_key(Modifiers::COMMAND, Key::F) {
                    edit_action = Some(EditorAction::Find { replace: false });
                } else if input.consume_key(replace_mods, replace_key) {
                    edit_action = Some(EditorAction::Find { replace: true });
                }
            }

            if action.is_none() && new_mode.is_none() {
                if input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab) {
                    action = Some(Shortcut::PreviousTab);
                } else if input.consume_key(Modifiers::CTRL, Key::Tab) {
                    action = Some(Shortcut::NextTab);
                }
            }
        });

        if let Some(mode) = new_mode {
            self.mode = mode;
            if mode != Mode::Preview {
                self.focus_editor = true;
            }
        }

        if let Some(edit_action) = edit_action {
            self.request_editor_action(edit_action);
        }

        match action {
            Some(Shortcut::New) => {
                self.add_untitled_document();
            }
            Some(Shortcut::Open) => self.open_dialog(),
            Some(Shortcut::Save) => {
                self.save_active();
            }
            Some(Shortcut::SaveAs) => {
                self.save_active_as();
            }
            Some(Shortcut::CloseTab) => {
                self.request_close_tab(self.active, ctx);
            }
            Some(Shortcut::NextTab) => self.cycle_tab(1),
            Some(Shortcut::PreviousTab) => self.cycle_tab(-1),
            None => {}
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        let i18n = self.i18n();
        let ctx = ui.ctx().clone();
        let shortcut_new = i18n.shortcut("N");
        let shortcut_open = i18n.shortcut("O");
        let shortcut_save = i18n.shortcut("S");
        let shortcut_save_as = i18n.shortcut("Shift+S");
        let shortcut_close_tab = i18n.shortcut("W");
        let shortcut_copy = i18n.shortcut("C");
        let shortcut_cut = i18n.shortcut("X");
        let shortcut_paste = i18n.shortcut("V");
        let shortcut_find = i18n.find_shortcut_label();
        let shortcut_replace = i18n.replace_shortcut_label();

        Panel::top("mdlite_toolbar")
            .frame(
                Frame::new()
                    .fill(palette.toolbar)
                    .inner_margin(Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Frame::new()
                        .fill(palette.accent)
                        .corner_radius(6)
                        .inner_margin(Margin::symmetric(7, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new("M").size(15.0).strong().color(Color32::WHITE));
                        });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("MarkLite")
                            .size(18.0)
                            .strong()
                            .color(palette.text),
                    );
                    ui.add_space(10.0);

                    if self.uses_native_menu() {
                        ui.label(
                            RichText::new(self.active_display_name())
                                .size(12.5)
                                .color(palette.muted),
                        );
                        if self.active_doc().dirty {
                            ui.label(RichText::new("●").size(8.0).color(palette.accent));
                        }
                    }

                    if !self.uses_native_menu() {
                        ui.menu_button(icon_text(icons::FILES, i18n.file_menu()), |ui| {
                            if menu_button(
                                ui,
                                icons::FILE_PLUS,
                                i18n.new_tab(),
                                Some(&shortcut_new),
                            )
                            .clicked()
                            {
                                self.add_untitled_document();
                                ui.close();
                            }
                            if menu_button(
                                ui,
                                icons::FOLDER_OPEN,
                                i18n.open(),
                                Some(&shortcut_open),
                            )
                            .clicked()
                            {
                                self.open_dialog();
                                ui.close();
                            }
                            ui.menu_button(icon_text(icons::CLOCK, i18n.recent_files()), |ui| {
                                if self.settings.recent_files.is_empty() {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(i18n.no_recent_files()).frame(false),
                                    );
                                } else {
                                    for raw in self.settings.recent_files.clone() {
                                        let path = PathBuf::from(&raw);
                                        let name = path
                                            .file_name()
                                            .map(|name| name.to_string_lossy().into_owned())
                                            .unwrap_or_else(|| raw.clone());
                                        if menu_button(ui, icons::FILE_TEXT, &name, None)
                                            .on_hover_text(raw)
                                            .clicked()
                                        {
                                            self.open_paths(&[path]);
                                            ui.close();
                                        }
                                    }
                                    ui.separator();
                                    if menu_button(
                                        ui,
                                        icons::TRASH,
                                        i18n.clear_recent_files(),
                                        None,
                                    )
                                    .clicked()
                                    {
                                        self.clear_recent_files();
                                        ui.close();
                                    }
                                }
                            });
                            ui.separator();
                            if menu_button(
                                ui,
                                icons::FLOPPY_DISK,
                                i18n.save(),
                                Some(&shortcut_save),
                            )
                            .clicked()
                            {
                                self.save_active();
                                ui.close();
                            }
                            if menu_button(
                                ui,
                                icons::COPY_SIMPLE,
                                i18n.save_as(),
                                Some(&shortcut_save_as),
                            )
                            .clicked()
                            {
                                self.save_active_as();
                                ui.close();
                            }
                            ui.separator();
                            if menu_button(
                                ui,
                                icons::X,
                                i18n.close_tab(),
                                Some(&shortcut_close_tab),
                            )
                            .clicked()
                            {
                                self.request_close_tab(self.active, &ctx);
                                ui.close();
                            }
                            if menu_button(ui, icons::SIGN_OUT, i18n.exit(), None).clicked() {
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                                ui.close();
                            }
                        });

                        ui.menu_button(icon_text(icons::PENCIL_SIMPLE, i18n.edit_menu()), |ui| {
                            if menu_button(ui, icons::COPY, i18n.copy(), Some(&shortcut_copy))
                                .clicked()
                            {
                                self.request_editor_action(EditorAction::Copy);
                                ui.close();
                            }
                            if menu_button(ui, icons::SCISSORS, i18n.cut(), Some(&shortcut_cut))
                                .clicked()
                            {
                                self.request_editor_action(EditorAction::Cut);
                                ui.close();
                            }
                            if menu_button(
                                ui,
                                icons::CLIPBOARD_TEXT,
                                i18n.paste(),
                                Some(&shortcut_paste),
                            )
                            .clicked()
                            {
                                self.request_editor_action(EditorAction::Paste);
                                ui.close();
                            }
                            ui.separator();
                            if menu_button(
                                ui,
                                icons::MAGNIFYING_GLASS,
                                i18n.find(),
                                Some(&shortcut_find),
                            )
                            .clicked()
                            {
                                self.request_editor_action(EditorAction::Find { replace: false });
                                ui.close();
                            }
                            if menu_button(
                                ui,
                                icons::REPEAT,
                                i18n.replace(),
                                Some(&shortcut_replace),
                            )
                            .clicked()
                            {
                                self.request_editor_action(EditorAction::Find { replace: true });
                                ui.close();
                            }
                        });

                        ui.menu_button(icon_text(icons::EYE, i18n.view_menu()), |ui| {
                            for (mode, icon, label) in [
                                (Mode::Edit, icons::PENCIL_SIMPLE, i18n.edit_mode()),
                                (Mode::Split, icons::COLUMNS, i18n.split_mode()),
                                (Mode::Preview, icons::EYE, i18n.preview_mode()),
                            ] {
                                if ui
                                    .selectable_label(self.mode == mode, icon_text(icon, label))
                                    .clicked()
                                {
                                    self.mode = mode;
                                    if mode != Mode::Preview {
                                        self.focus_editor = true;
                                    }
                                    ui.close();
                                }
                            }
                        });

                        ui.menu_button(icon_text(icons::EXPORT, i18n.export_menu()), |ui| {
                            if menu_button(ui, icons::FILE_PDF, i18n.export_pdf(), None).clicked() {
                                self.export_pdf();
                                ui.close();
                            }
                            if menu_button(ui, icons::FILE_DOC, i18n.export_word(), None).clicked()
                            {
                                self.export_docx();
                                ui.close();
                            }
                        });

                        ui.menu_button(icon_text(icons::QUESTION, i18n.help_menu()), |ui| {
                            if menu_button(ui, icons::INFO, i18n.about(), None).clicked() {
                                self.about_open = true;
                                ui.close();
                            }
                            if menu_button(ui, icons::GEAR, i18n.settings(), None).clicked() {
                                self.settings_open = true;
                                ui.close();
                            }
                            ui.separator();
                            if menu_button(ui, icons::GITHUB_LOGO, i18n.github_repository(), None)
                                .clicked()
                            {
                                self.open_external(ui.ctx(), mdlite::GITHUB_URL);
                                ui.close();
                            }
                        });
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if small_button(
                            ui,
                            &icon_text(icons::TRANSLATE, i18n.language_toggle_label()),
                            &palette,
                        )
                        .on_hover_text(i18n.language_toggle_tooltip())
                        .clicked()
                        {
                            self.settings.language = match self.settings.language {
                                Language::Zh => Language::En,
                                Language::En => Language::Zh,
                            };
                            self.notice = None;
                            self.persist_settings();
                        }
                        if small_button(ui, &icon_text(icons::GEAR, i18n.settings()), &palette)
                            .clicked()
                        {
                            self.settings_open = true;
                        }
                    });
                });

                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    if toolbar_button(ui, icons::FILE_PLUS, i18n.new_document())
                        .on_hover_text(i18n.tooltip_new())
                        .clicked()
                    {
                        self.add_untitled_document();
                    }
                    if toolbar_button(ui, icons::FOLDER_OPEN, i18n.open())
                        .on_hover_text(i18n.tooltip_open())
                        .clicked()
                    {
                        self.open_dialog();
                    }
                    if toolbar_button(ui, icons::FLOPPY_DISK, i18n.save())
                        .on_hover_text(i18n.tooltip_save())
                        .clicked()
                    {
                        self.save_active();
                    }
                    if toolbar_button(ui, icons::COPY_SIMPLE, i18n.save_as())
                        .on_hover_text(i18n.tooltip_save_as())
                        .clicked()
                    {
                        self.save_active_as();
                    }

                    ui.separator();

                    for (mode, icon, label) in [
                        (Mode::Edit, icons::PENCIL_SIMPLE, i18n.edit_mode()),
                        (Mode::Split, icons::COLUMNS, i18n.split_mode()),
                        (Mode::Preview, icons::EYE, i18n.preview_mode()),
                    ] {
                        if mode_button(ui, icon, label, self.mode == mode, &palette).clicked() {
                            self.mode = mode;
                            if mode != Mode::Preview {
                                self.focus_editor = true;
                            }
                        }
                    }

                    ui.separator();

                    if primary_button(ui, icons::FILE_PDF, i18n.export_pdf(), &palette)
                        .on_hover_text(i18n.tooltip_export_pdf())
                        .clicked()
                    {
                        self.export_pdf();
                    }
                    if toolbar_button(ui, icons::FILE_DOC, i18n.export_word())
                        .on_hover_text(i18n.tooltip_export_word())
                        .clicked()
                    {
                        self.export_docx();
                    }
                });
            });
    }

    fn show_tab_bar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        let i18n = self.i18n();
        let ctx = ui.ctx().clone();
        let mut activate = None;
        let mut close = None;

        Panel::top("mdlite_tab_bar")
            .frame(
                Frame::new()
                    .fill(palette.status)
                    .inner_margin(Margin::symmetric(10, 5)),
            )
            .show(ui, |ui| {
                ScrollArea::horizontal()
                    .id_salt("mdlite_tabs_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, doc) in self.docs.iter().enumerate() {
                                let active = index == self.active;
                                let name = doc.display_name(&i18n);
                                let tooltip = doc
                                    .path
                                    .as_ref()
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_else(|| name.clone());

                                let tab = Frame::new()
                                    .fill(if active {
                                        palette.page
                                    } else {
                                        Color32::TRANSPARENT
                                    })
                                    .stroke(if active {
                                        Stroke::new(1.0, palette.accent)
                                    } else {
                                        Stroke::NONE
                                    })
                                    .corner_radius(7)
                                    .inner_margin(Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let color =
                                                if active { palette.text } else { palette.muted };
                                            let mut label_text = format!(
                                                "{}  {}",
                                                icons::MARKDOWN_LOGO,
                                                truncate_chars(&name, 28)
                                            );
                                            if doc.dirty {
                                                label_text.push_str("  ●");
                                            }
                                            let label = ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(label_text)
                                                            .size(12.5)
                                                            .color(color),
                                                    )
                                                    .frame(false)
                                                    .min_size(egui::vec2(0.0, 20.0)),
                                                )
                                                .on_hover_text(tooltip);
                                            let close = ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(icons::X)
                                                            .size(11.5)
                                                            .color(palette.muted),
                                                    )
                                                    .frame(false)
                                                    .min_size(egui::vec2(18.0, 18.0)),
                                                )
                                                .on_hover_text(i18n.close_tab_tooltip());
                                            (label, close)
                                        })
                                        .inner
                                    });

                                let (label, close_button) = tab.inner;
                                if close_button.clicked() || label.clicked_by(PointerButton::Middle)
                                {
                                    close = Some(index);
                                } else if label.clicked() {
                                    activate = Some(index);
                                }
                            }

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(icons::PLUS).size(14.0).color(palette.muted),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text(i18n.new_tab_tooltip())
                                .clicked()
                            {
                                self.add_untitled_document();
                            }
                        });
                    });
            });

        if let Some(index) = activate {
            self.active = index;
            self.focus_editor = true;
        }
        if let Some(index) = close {
            self.request_close_tab(index, &ctx);
        }
    }

    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        let i18n = self.i18n();
        let doc = self.active_doc();
        let stats = markdown::stats(&doc.content);
        let active = self.active + 1;
        let total = self.docs.len();

        Panel::bottom("mdlite_status")
            .frame(
                Frame::new()
                    .fill(palette.status)
                    .inner_margin(Margin::symmetric(14, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (state, color) = if doc.dirty {
                        (i18n.status_unsaved(), palette.accent)
                    } else if doc.path.is_some() {
                        (i18n.status_saved(), palette.muted)
                    } else {
                        (i18n.status_new(), palette.muted)
                    };
                    ui.label(RichText::new(state).size(11.5).color(color));

                    ui.label(RichText::new("·").size(11.5).color(palette.border));

                    let short_path = doc.display_name(&i18n);
                    let response =
                        ui.label(RichText::new(short_path).size(11.5).color(palette.muted));
                    if let Some(path) = &doc.path {
                        response.on_hover_text(path.display().to_string());
                    }

                    if let Some(notice) = &self.notice {
                        ui.label(RichText::new("·").size(11.5).color(palette.border));
                        ui.add(
                            Label::new(
                                RichText::new(notice.as_str())
                                    .size(11.5)
                                    .color(palette.accent),
                            )
                            .truncate(),
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} · {active}/{total}",
                                i18n.stats(stats.words, stats.lines)
                            ))
                            .size(11.5)
                            .color(palette.muted),
                        );
                    });
                });
            });
    }

    fn show_find_replace(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let palette = self.palette();
        let i18n = self.i18n();
        let active = self.active;
        let mut step = 0isize;
        let mut replace_one = false;
        let mut replace_all = false;
        let mut close = false;

        Frame::new()
            .fill(palette.accent_soft)
            .corner_radius(6)
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let find_response = ui.add(
                        TextEdit::singleline(&mut self.find_query)
                            .hint_text(i18n.find_hint())
                            .desired_width(150.0),
                    );
                    if self.find_focus {
                        find_response.request_focus();
                        self.find_focus = false;
                    }
                    if find_response.changed() {
                        self.find_current = 0;
                        self.find_selected = false;
                    }

                    let matches = find_matches_in(&self.docs[active].content, &self.find_query);
                    if matches.is_empty() {
                        self.find_current = 0;
                    } else if self.find_current >= matches.len() {
                        self.find_current = matches.len() - 1;
                    }
                    let count_label = if self.find_query.is_empty() {
                        String::new()
                    } else if matches.is_empty() {
                        i18n.no_matches().to_owned()
                    } else {
                        format!("{}/{}", self.find_current + 1, matches.len())
                    };
                    ui.label(RichText::new(count_label).size(11.5).color(palette.muted));

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(icons::ARROW_UP)
                                    .size(11.0)
                                    .color(palette.muted),
                            )
                            .frame(false)
                            .small(),
                        )
                        .on_hover_text(i18n.find_previous())
                        .clicked()
                    {
                        step = -1;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(icons::ARROW_DOWN)
                                    .size(11.0)
                                    .color(palette.muted),
                            )
                            .frame(false)
                            .small(),
                        )
                        .on_hover_text(i18n.find_next())
                        .clicked()
                    {
                        step = 1;
                    }
                    if find_response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter))
                    {
                        step = 1;
                    }
                    if find_response.has_focus() && ui.input(|input| input.key_pressed(Key::Escape))
                    {
                        close = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(icons::X).size(11.0).color(palette.muted),
                            )
                            .frame(false)
                            .small(),
                        )
                        .on_hover_text(i18n.close())
                        .clicked()
                    {
                        close = true;
                    }
                });

                if self.find_show_replace {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        let replace_response = ui.add(
                            TextEdit::singleline(&mut self.replace_query)
                                .hint_text(i18n.replace_hint())
                                .desired_width(150.0),
                        );
                        if ui.button(i18n.replace()).clicked() {
                            replace_one = true;
                        }
                        if ui.button(i18n.replace_all()).clicked() {
                            replace_all = true;
                        }
                        if replace_response.has_focus()
                            && ui.input(|input| input.key_pressed(Key::Enter))
                        {
                            replace_one = true;
                        }
                    });
                }
            });

        if close {
            self.find_replace_open = false;
        } else if replace_one {
            self.replace_current_match(ctx);
        } else if replace_all {
            self.replace_all_matches();
        } else if step != 0 {
            self.find_step(ctx, step);
        }
    }

    fn find_step(&mut self, ctx: &egui::Context, delta: isize) {
        let matches = find_matches_in(&self.active_doc().content, &self.find_query);
        if matches.is_empty() {
            return;
        }
        let len = matches.len() as isize;
        if !self.find_selected {
            self.find_selected = true;
            self.find_current = if delta >= 0 { 0 } else { matches.len() - 1 };
        } else {
            self.find_current = (self.find_current as isize + delta).rem_euclid(len) as usize;
        }
        let (start, end) = matches[self.find_current];
        self.select_editor_bytes(ctx, start, end);
    }

    fn replace_current_match(&mut self, ctx: &egui::Context) {
        if self.find_query.is_empty() {
            return;
        }
        let active = self.active;
        let matches = find_matches_in(&self.docs[active].content, &self.find_query);
        let Some(&(start, end)) = matches.get(self.find_current) else {
            return;
        };
        let replacement = self.replace_query.clone();
        self.docs[active]
            .content
            .replace_range(start..end, &replacement);
        self.last_editor_selection = None;
        self.docs[active].dirty = true;
        self.notice = None;
        self.find_selected = false;
        self.find_current = 0;
        let inserted_end = start + replacement.len();
        self.select_editor_bytes(ctx, inserted_end, inserted_end);
    }

    fn replace_all_matches(&mut self) {
        if self.find_query.is_empty() {
            return;
        }
        let query = self.find_query.clone();
        let replacement = self.replace_query.clone();
        let active = self.active;
        if self.docs[active].content.contains(&query) {
            self.docs[active].content = self.docs[active].content.replace(&query, &replacement);
            self.last_editor_selection = None;
            self.docs[active].dirty = true;
            self.notice = None;
            self.find_current = 0;
            self.find_selected = false;
        }
    }

    fn select_editor_bytes(&mut self, ctx: &egui::Context, start: usize, end: usize) {
        let Some(editor_id) = self.editor_id else {
            self.focus_editor = true;
            return;
        };
        let content = &self.active_doc().content;
        let start = start.min(content.len());
        let end = end.min(content.len());
        let start_char = content[..start].chars().count();
        let end_char = content[..end].chars().count();
        let range = CCursorRange::two(CCursor::new(start_char), CCursor::new(end_char));
        let mut state =
            egui::widgets::text_edit::TextEditState::load(ctx, editor_id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(ctx, editor_id);
        self.focus_editor = true;
        self.find_selected = true;
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, fixed_width: Option<f32>) {
        let palette = self.palette();
        let i18n = self.i18n();
        let ctx = ui.ctx().clone();
        let (page_width, side, card_height) = page_metrics(ui, fixed_width);
        let active = self.active;
        let doc_id = self.docs[active].id;
        let editor_id = Id::unique(("mdlite_editor", doc_id));
        self.editor_id = Some(editor_id);

        ui.add_space(16.0);
        ui.horizontal_top(|ui| {
            ui.add_space(side);
            Frame::new()
                .fill(palette.page)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(10)
                .inner_margin(Margin::symmetric(24, 20))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width((page_width - 48.0).max(200.0));
                        ui.set_height((card_height - 40.0).max(120.0));

                        if self.find_replace_open {
                            self.show_find_replace(ui, &ctx);
                            ui.add_space(6.0);
                        }

                        let scroll_height = ui.available_height();
                        ScrollArea::vertical()
                            .id_salt(("mdlite_editor_scroll", doc_id))
                            .auto_shrink([false, false])
                            .max_height(scroll_height)
                            .show(ui, |ui| {
                                let rows = self.docs[active].content.lines().count().max(20);
                                let response = ui.add(
                                    TextEdit::multiline(&mut self.docs[active].content)
                                        .id(editor_id)
                                        .font(FontId::monospace(15.0))
                                        .code_editor()
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(rows)
                                        .margin(Margin::ZERO)
                                        .frame(Frame::NONE)
                                        .hint_text(i18n.editor_placeholder()),
                                );
                                if response.changed() {
                                    self.docs[active].dirty = true;
                                    self.notice = None;
                                }
                                if self.focus_editor {
                                    response.request_focus();
                                    self.focus_editor = false;
                                }
                                if response.has_focus()
                                    || ctx.memory(|memory| memory.has_focus(editor_id))
                                {
                                    self.cache_editor_selection(&ctx, editor_id, doc_id);
                                }
                            });
                    });
                });
        });
    }

    fn show_preview(&mut self, ui: &mut egui::Ui, fixed_width: Option<f32>) {
        let palette = self.palette();
        let i18n = self.i18n();
        let source = self.preview_markdown();
        let empty = self.active_doc().content.trim().is_empty();
        let (page_width, side, card_height) = page_metrics(ui, fixed_width);
        let doc_id = self.active_doc().id;
        let cache = &mut self.preview_cache;

        ui.add_space(16.0);
        ui.horizontal_top(|ui| {
            ui.add_space(side);
            Frame::new()
                .fill(palette.page)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(10)
                .inner_margin(Margin::symmetric(24, 20))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width((page_width - 48.0).max(200.0));
                        ui.set_height((card_height - 40.0).max(120.0));
                        let scroll_height = ui.available_height();
                        ScrollArea::vertical()
                            .id_salt(("mdlite_preview_scroll", doc_id))
                            .auto_shrink([false, false])
                            .max_height(scroll_height)
                            .show(ui, |ui| {
                                ui.style_mut().url_in_tooltip = true;
                                if empty {
                                    ui.add_space(30.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new(i18n.preview_empty_title())
                                                .size(22.0)
                                                .strong()
                                                .color(palette.muted),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(
                                            RichText::new(i18n.preview_empty_hint())
                                                .size(13.0)
                                                .color(palette.muted),
                                        );
                                    });
                                } else {
                                    CommonMarkViewer::new().show(ui, cache, &source);
                                }
                            });
                    });
                });
        });
    }

    fn preview_markdown(&self) -> String {
        let doc = self.active_doc();
        let without_front_matter = markdown::strip_front_matter(&doc.content);
        let localized = markdown::localize_alerts_for_preview(without_front_matter);
        match doc.path.as_ref().and_then(|path| path.parent()) {
            Some(dir) => markdown::absolutize_image_paths(&localized, dir),
            None => localized,
        }
    }

    fn open_dialog(&mut self) {
        let i18n = self.i18n();
        let mut dialog = rfd::FileDialog::new()
            .set_title(i18n.open_dialog_title())
            .add_filter(
                i18n.markdown_filter(),
                &["md", "markdown", "mdown", "mkd", "txt"],
            )
            .add_filter(i18n.all_files_filter(), &["*"]);
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        if let Some(paths) = dialog.pick_files() {
            self.open_paths(&paths);
        }
    }

    fn open_paths(&mut self, paths: &[PathBuf]) {
        let i18n = self.i18n();
        let mut opened = 0usize;
        let mut errors = Vec::new();

        for raw_path in paths {
            let path = std::path::absolute(raw_path).unwrap_or_else(|_| raw_path.clone());
            if let Some(index) = self
                .docs
                .iter()
                .position(|doc| paths_equal(doc.path.as_deref(), &path))
            {
                self.active = index;
                self.focus_editor = true;
                self.record_recent_file(&path);
                continue;
            }

            match read_text_file(&path) {
                Ok(content) => {
                    let id = self.next_doc_id;
                    self.next_doc_id = self.next_doc_id.wrapping_add(1).max(1);
                    self.docs
                        .push(Document::from_file(id, path.clone(), content));
                    self.active = self.docs.len() - 1;
                    self.focus_editor = true;
                    opened += 1;
                    if let Some(parent) = path.parent() {
                        self.last_dir = Some(parent.to_path_buf());
                    }
                    self.record_recent_file(&path);
                }
                Err(ReadError::Io(error)) => {
                    errors.push(i18n.read_failed(&path, &error.to_string()));
                }
                Err(ReadError::Encoding) => {
                    errors.push(i18n.encoding_failed(&path));
                }
            }
        }

        if opened > 0 {
            self.notice = Some(i18n.opened_notice(opened));
        }
        if !errors.is_empty() {
            self.error_message = Some(errors.join("\n\n────────\n\n"));
        }
    }

    fn cycle_tab(&mut self, delta: isize) {
        if self.docs.len() > 1 {
            let len = self.docs.len() as isize;
            self.active = (self.active as isize + delta).rem_euclid(len) as usize;
            self.focus_editor = true;
        }
    }

    fn save_active(&mut self) -> bool {
        let id = self.active_doc().id;
        self.save_doc(id)
    }

    fn save_active_as(&mut self) -> bool {
        let id = self.active_doc().id;
        self.save_doc_as(id)
    }

    fn save_doc(&mut self, id: u64) -> bool {
        let Some(index) = self.doc_index(id) else {
            return false;
        };
        match self.docs[index].path.clone() {
            Some(path) => self.write_doc_to(id, &path),
            None => self.save_doc_as(id),
        }
    }

    fn save_doc_as(&mut self, id: u64) -> bool {
        let Some(index) = self.doc_index(id) else {
            return false;
        };
        let i18n = self.i18n();
        let base_name = self.docs[index].base_name(&i18n);
        let dialog_dir = self.docs[index]
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .or_else(|| self.last_dir.clone());

        let mut dialog = rfd::FileDialog::new()
            .set_title(i18n.save_as_dialog_title())
            .add_filter(i18n.markdown_filter(), &["md", "markdown"])
            .set_file_name(format!("{base_name}.md"));
        if let Some(dir) = dialog_dir {
            dialog = dialog.set_directory(dir);
        }
        match dialog.save_file() {
            Some(path) => {
                let path = ensure_extension(path, "md");
                self.write_doc_to(id, &path)
            }
            None => false,
        }
    }

    fn write_doc_to(&mut self, id: u64, path: &Path) -> bool {
        let Some(index) = self.doc_index(id) else {
            return false;
        };
        let i18n = self.i18n();
        let content = self.docs[index].content.clone();
        match write_text_atomic(path, &content) {
            Ok(()) => {
                self.docs[index].path = Some(path.to_path_buf());
                self.docs[index].dirty = false;
                self.notice = Some(i18n.saved_notice().to_owned());
                if let Some(parent) = path.parent() {
                    self.last_dir = Some(parent.to_path_buf());
                }
                self.record_recent_file(path);
                true
            }
            Err(error) => {
                self.error_message = Some(i18n.save_failed(path, &error));
                false
            }
        }
    }

    fn save_all_dirty(&mut self) -> bool {
        let ids: Vec<u64> = self
            .docs
            .iter()
            .filter(|doc| doc.dirty)
            .map(|doc| doc.id)
            .collect();
        for id in ids {
            if !self.save_doc(id) {
                return false;
            }
        }
        true
    }

    fn record_recent_file(&mut self, path: &Path) {
        self.settings.add_recent_file(path);
        let _ = self.settings.save();
    }

    fn clear_recent_files(&mut self) {
        self.settings.clear_recent_files();
        if let Err(error) = self.settings.save() {
            self.error_message = Some(self.i18n().settings_save_failed(&error));
        }
        self.notice = Some(self.i18n().recent_cleared().to_owned());
    }

    fn request_close_tab(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.docs.len() {
            return;
        }
        let doc = &self.docs[index];
        self.pending = Some(PendingAction::CloseTab(doc.id));
        ctx.request_repaint();
    }

    fn close_tab_by_id(&mut self, id: u64, ctx: &egui::Context) {
        let Some(index) = self.doc_index(id) else {
            return;
        };
        self.docs.remove(index);

        if self.docs.is_empty() {
            self.add_untitled_document();
            self.notice = None;
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        self.active = active_after_tab_close(self.active, index, self.docs.len());
        self.focus_editor = true;
        self.notice = None;
    }

    fn export_pdf(&mut self) {
        let i18n = self.i18n();
        let doc = self.active_doc();
        let content = doc.content.clone();
        let base_name = doc.base_name(&i18n);
        let source_dir = doc
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok());
        let title = markdown::document_title(&content, doc.path.as_deref());

        let mut dialog = rfd::FileDialog::new()
            .set_title(i18n.export_pdf_dialog_title())
            .add_filter(i18n.pdf_filter(), &["pdf"])
            .set_file_name(format!("{base_name}.pdf"));
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = ensure_export_extension(path, "pdf");

        match pdf::export_pdf(&content, &path, &title, source_dir.as_deref()) {
            Ok(()) => {
                self.notice = Some(i18n.exported_notice(&display_file_name(&path)));
                if let Some(parent) = path.parent() {
                    self.last_dir = Some(parent.to_path_buf());
                }
            }
            Err(error) => {
                self.error_message = Some(i18n.export_error(&error));
            }
        }
    }

    fn export_docx(&mut self) {
        let i18n = self.i18n();
        let doc = self.active_doc();
        let content = doc.content.clone();
        let base_name = doc.base_name(&i18n);
        let source_dir = doc
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok());

        let mut dialog = rfd::FileDialog::new()
            .set_title(i18n.export_word_dialog_title())
            .add_filter(i18n.word_filter(), &["docx"])
            .set_file_name(format!("{base_name}.docx"));
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = ensure_export_extension(path, "docx");

        match docx::export_docx(&content, &path, source_dir.as_deref()) {
            Ok(()) => {
                self.notice = Some(i18n.exported_notice(&display_file_name(&path)));
                if let Some(parent) = path.parent() {
                    self.last_dir = Some(parent.to_path_buf());
                }
            }
            Err(error) => {
                self.error_message = Some(i18n.export_error(&error));
            }
        }
    }

    fn show_confirmation(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let Some(pending) = self.pending.clone() else {
            return;
        };

        let i18n = self.i18n();
        let mut clean_close = false;
        let (title, message, save_label, discard_label) = match &pending {
            PendingAction::CloseTab(id) => {
                let Some(index) = self.doc_index(*id) else {
                    self.pending = None;
                    return;
                };
                let doc = &self.docs[index];
                if doc.dirty {
                    (
                        i18n.unsaved_title().to_owned(),
                        i18n.unsaved_message(&doc.display_name(&i18n), UnsavedReason::CloseTab),
                        i18n.save().to_owned(),
                        i18n.discard().to_owned(),
                    )
                } else {
                    clean_close = true;
                    (
                        i18n.close_file_title().to_owned(),
                        i18n.close_file_message(&doc.display_name(&i18n)),
                        i18n.close().to_owned(),
                        String::new(),
                    )
                }
            }
            PendingAction::CloseApp => {
                let dirty = self.dirty_documents();
                if dirty.is_empty() {
                    self.pending = None;
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                    return;
                }
                let names = dirty
                    .iter()
                    .map(|doc| doc.display_name(&i18n))
                    .take(8)
                    .collect::<Vec<_>>()
                    .join(", ");
                (
                    i18n.unsaved_tabs_title().to_owned(),
                    i18n.unsaved_tabs_message(&names),
                    i18n.save_all().to_owned(),
                    i18n.discard_all().to_owned(),
                )
            }
        };

        let cancel_label = i18n.cancel().to_owned();
        let result = if clean_close {
            rfd::MessageDialog::new()
                .set_parent(frame)
                .set_level(rfd::MessageLevel::Info)
                .set_title(title)
                .set_description(message)
                .set_buttons(rfd::MessageButtons::OkCancelCustom(
                    save_label.clone(),
                    cancel_label,
                ))
                .show()
        } else {
            rfd::MessageDialog::new()
                .set_parent(frame)
                .set_level(rfd::MessageLevel::Warning)
                .set_title(title)
                .set_description(message)
                .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                    save_label.clone(),
                    discard_label.clone(),
                    cancel_label,
                ))
                .show()
        };

        let choice = if clean_close {
            match result {
                rfd::MessageDialogResult::Ok => ConfirmChoice::Discard,
                rfd::MessageDialogResult::Custom(text) if text == save_label => {
                    ConfirmChoice::Discard
                }
                _ => ConfirmChoice::Cancel,
            }
        } else {
            match result {
                rfd::MessageDialogResult::Yes => ConfirmChoice::Save,
                rfd::MessageDialogResult::No => ConfirmChoice::Discard,
                rfd::MessageDialogResult::Custom(text) if text == save_label => ConfirmChoice::Save,
                rfd::MessageDialogResult::Custom(text) if text == discard_label => {
                    ConfirmChoice::Discard
                }
                _ => ConfirmChoice::Cancel,
            }
        };

        match (pending, choice) {
            (_, ConfirmChoice::Cancel) => {
                self.pending = None;
            }
            (PendingAction::CloseTab(id), ConfirmChoice::Save) => {
                self.pending = None;
                if self.save_doc(id) {
                    self.close_tab_by_id(id, ctx);
                }
            }
            (PendingAction::CloseTab(id), ConfirmChoice::Discard) => {
                self.pending = None;
                self.close_tab_by_id(id, ctx);
            }
            (PendingAction::CloseApp, ConfirmChoice::Save) => {
                self.pending = None;
                if self.save_all_dirty() {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
            (PendingAction::CloseApp, ConfirmChoice::Discard) => {
                self.pending = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    fn show_error(&mut self, frame: &eframe::Frame) {
        let Some(message) = self.error_message.clone() else {
            return;
        };
        let i18n = self.i18n();
        let _ = rfd::MessageDialog::new()
            .set_parent(frame)
            .set_level(rfd::MessageLevel::Error)
            .set_title(i18n.error_title())
            .set_description(message)
            .set_buttons(rfd::MessageButtons::OkCustom(i18n.close().to_owned()))
            .show();
        self.error_message = None;
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let palette = self.palette();
        let effective_dark = self.effective_theme == Theme::Dark;
        let mut draft = self.settings.clone();
        let mut close = false;

        let response = Modal::new(Id::unique("mdlite_settings_modal")).show(ctx, |ui| {
            let i18n = I18n::new(draft.language);
            ui.set_min_width(480.0);
            ui.label(
                RichText::new(i18n.settings_title())
                    .size(19.0)
                    .strong()
                    .color(palette.text),
            );
            ui.add_space(12.0);

            ui.label(RichText::new(i18n.language_label()).size(13.0).strong());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut draft.language, Language::Zh, "中文");
                ui.selectable_value(&mut draft.language, Language::En, "English");
            });

            ui.add_space(12.0);
            ui.label(RichText::new(i18n.appearance_label()).size(13.0).strong());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut draft.theme, ThemeMode::System, i18n.follow_system());
                ui.selectable_value(&mut draft.theme, ThemeMode::Light, i18n.light());
                ui.selectable_value(&mut draft.theme, ThemeMode::Dark, i18n.dark());
            });
            if draft.theme == ThemeMode::System {
                ui.label(
                    RichText::new(i18n.system_current(effective_dark))
                        .size(11.5)
                        .color(palette.muted),
                );
            }

            ui.add_space(12.0);
            ui.label(RichText::new(i18n.accent_label()).size(13.0).strong());
            ui.horizontal(|ui| {
                for accent in AccentColor::ALL {
                    let color = accent.colors(effective_dark).accent;
                    let selected = draft.accent == accent;
                    let button = egui::Button::new("")
                        .fill(color)
                        .stroke(if selected {
                            Stroke::new(2.0, palette.text)
                        } else {
                            Stroke::new(1.0, palette.border)
                        })
                        .corner_radius(8)
                        .min_size(egui::vec2(34.0, 26.0));
                    if ui
                        .add(button)
                        .on_hover_text(i18n.accent_name(accent))
                        .clicked()
                    {
                        draft.accent = accent;
                    }
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(i18n.accent_name(draft.accent))
                        .size(12.5)
                        .color(palette.muted),
                );
            });

            ui.add_space(14.0);
            ui.label(
                RichText::new(i18n.settings_note())
                    .size(11.5)
                    .color(palette.muted),
            );
            ui.add_space(16.0);

            if ui
                .add(
                    egui::Button::new(RichText::new(i18n.close()).color(Color32::WHITE).strong())
                        .fill(palette.accent_button)
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(72.0, 30.0)),
                )
                .clicked()
            {
                close = true;
            }
        });

        if draft != self.settings {
            let language_changed = draft.language != self.settings.language;
            self.settings = draft;
            if language_changed {
                self.notice = None;
            }
            self.apply_settings_change(ctx);
        }

        if close || response.should_close() {
            self.settings_open = false;
        }
    }

    fn show_about(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let palette = self.palette();
        let i18n = self.i18n();
        let mut close = false;
        let mut external_url: Option<String> = None;

        let response = Modal::new(Id::unique("mdlite_about_modal")).show(ctx, |ui| {
            ui.set_min_width(520.0);
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(palette.accent)
                    .corner_radius(8)
                    .inner_margin(Margin::symmetric(9, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new("M").size(20.0).strong().color(Color32::WHITE));
                    });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("MarkLite {}", mdlite::APP_VERSION))
                            .size(20.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(i18n.about_tagline())
                            .size(12.5)
                            .color(palette.muted),
                    );
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new(i18n.about_description()).size(13.0));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}：", i18n.author_label()))
                        .size(12.5)
                        .color(palette.muted),
                );
                ui.label(mdlite::AUTHOR_NAME);
            });
            if ui.link(mdlite::GITHUB_URL).clicked() {
                external_url = Some(mdlite::GITHUB_URL.to_owned());
            }
            ui.add_space(12.0);
            ui.label(
                RichText::new(i18n.about_shortcuts_heading())
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);

            egui::Grid::new("mdlite_shortcuts")
                .num_columns(2)
                .spacing([26.0, 5.0])
                .show(ui, |ui| {
                    for (keys, label) in i18n.about_shortcuts() {
                        ui.label(
                            RichText::new(keys)
                                .size(12.0)
                                .monospace()
                                .color(palette.accent),
                        );
                        ui.label(RichText::new(label).size(12.5));
                        ui.end_row();
                    }
                });

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(i18n.close()).color(Color32::WHITE).strong(),
                        )
                        .fill(palette.accent_button)
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(72.0, 30.0)),
                    )
                    .clicked()
                {
                    close = true;
                }
                ui.label(
                    RichText::new(format!("MIT License · v{}", mdlite::APP_VERSION))
                        .size(11.5)
                        .color(palette.muted),
                );
            });
        });

        if close || response.should_close() {
            self.about_open = false;
        }
        if let Some(url) = external_url {
            self.open_external(ctx, &url);
        }
    }
}

impl eframe::App for MarkLiteApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.update_effective_theme(&ctx);

        #[cfg(target_os = "macos")]
        {
            self.ensure_native_menu(&ctx);
            self.process_native_menu(&ctx);
        }

        self.dispatch_editor_action(&ctx);
        self.handle_close_request(&ctx);
        self.update_window_title(&ctx);
        self.handle_dropped_files(&ctx);
        self.handle_shortcuts(&ctx);

        self.show_toolbar(ui);
        self.show_tab_bar(ui);
        self.show_status_bar(ui);

        let palette = self.palette();
        match self.mode {
            Mode::Edit => {
                egui::CentralPanel::default()
                    .frame(Frame::new().fill(palette.editor_bg))
                    .show(ui, |ui| self.show_editor(ui, None));
            }
            Mode::Preview => {
                egui::CentralPanel::default()
                    .frame(Frame::new().fill(palette.preview_bg))
                    .show(ui, |ui| self.show_preview(ui, None));
            }
            Mode::Split => {
                let total_width = ui.available_width();
                let side_width = (total_width * 0.5).floor();
                let card_width = (side_width - 40.0).clamp(240.0, 980.0);
                Panel::left("mdlite_editor_panel")
                    .resizable(false)
                    .exact_size(side_width)
                    .frame(Frame::new().fill(palette.editor_bg))
                    .show(ui, |ui| self.show_editor(ui, Some(card_width)));
                egui::CentralPanel::default()
                    .frame(Frame::new().fill(palette.preview_bg))
                    .show(ui, |ui| self.show_preview(ui, Some(card_width)));
            }
        }

        self.show_settings(&ctx);
        self.show_about(&ctx);
        self.show_confirmation(&ctx, frame);
        self.show_error(frame);
    }
}

#[derive(Clone, Copy)]
enum Shortcut {
    New,
    Open,
    Save,
    SaveAs,
    CloseTab,
    NextTab,
    PreviousTab,
}

fn page_metrics(ui: &egui::Ui, fixed_width: Option<f32>) -> (f32, f32, f32) {
    let available_width = ui.available_width();
    let max_fit = (available_width - 24.0).max(200.0);
    let page_width = fixed_width
        .unwrap_or_else(|| (available_width - 48.0).clamp(260.0, 980.0))
        .min(max_fit);
    let side = ((available_width - page_width) / 2.0).max(0.0);
    let card_height = (ui.available_height() - 32.0).max(160.0);
    (page_width, side, card_height)
}

fn find_matches_in(content: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    content
        .match_indices(query)
        .map(|(start, matched)| (start, start + matched.len()))
        .collect()
}

fn icon_text(icon: &str, label: &str) -> String {
    format!("{icon}  {label}")
}

fn active_after_tab_close(active: usize, removed: usize, remaining: usize) -> usize {
    if remaining == 0 {
        0
    } else if removed < active {
        active - 1
    } else if removed == active {
        removed.min(remaining - 1)
    } else {
        active
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn menu_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    shortcut: Option<&str>,
) -> egui::Response {
    let mut button = egui::Button::new(icon_text(icon, label)).frame(false);
    if let Some(shortcut) = shortcut {
        button = button.shortcut_text(shortcut);
    }
    ui.add(button)
}

fn toolbar_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(icon_text(icon, label)).size(13.0))
            .min_size(egui::vec2(0.0, 28.0)),
    )
}

fn small_button(ui: &mut egui::Ui, label: &str, palette: &Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(palette.muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(6)
            .min_size(egui::vec2(0.0, 25.0)),
    )
}

fn primary_button(ui: &mut egui::Ui, icon: &str, label: &str, palette: &Palette) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(icon_text(icon, label))
                .size(13.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(palette.accent_button)
        .stroke(Stroke::NONE)
        .corner_radius(7)
        .min_size(egui::vec2(0.0, 28.0)),
    )
}

fn mode_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    palette: &Palette,
) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(icon_text(icon, label))
                .size(13.0)
                .color(if selected {
                    palette.accent
                } else {
                    palette.muted
                }),
        )
        .selected(selected)
        .fill(if selected {
            palette.accent_soft
        } else {
            Color32::TRANSPARENT
        })
        .stroke(if selected {
            Stroke::new(1.0, palette.accent)
        } else {
            Stroke::NONE
        })
        .corner_radius(7)
        .min_size(egui::vec2(58.0, 28.0)),
    )
}

fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(extension);
    }
    path
}

fn ensure_export_extension(path: PathBuf, extension: &str) -> PathBuf {
    match path.extension().and_then(|value| value.to_str()) {
        Some(existing) if existing.eq_ignore_ascii_case(extension) => path,
        _ => path.with_extension(extension),
    }
}

fn load_system_font(db: &fontdb::Database, monospace: bool) -> Option<egui::FontData> {
    let names: &[&str] = if monospace {
        #[cfg(target_os = "macos")]
        {
            &["SF Mono", "Menlo", "Monaco", "Andale Mono"]
        }
        #[cfg(target_os = "windows")]
        {
            &["Cascadia Mono", "Consolas", "Courier New"]
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            &[
                "JetBrains Mono",
                "Noto Sans Mono",
                "DejaVu Sans Mono",
                "Liberation Mono",
            ]
        }
    } else {
        #[cfg(target_os = "macos")]
        {
            &["SF Pro Text", "SF Pro Display", "Helvetica Neue", "Arial"]
        }
        #[cfg(target_os = "windows")]
        {
            &["Segoe UI Variable Text", "Segoe UI", "Tahoma", "Arial"]
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            &["Noto Sans", "Ubuntu", "DejaVu Sans", "Liberation Sans"]
        }
    };

    let mut families: Vec<fontdb::Family<'_>> = names
        .iter()
        .map(|name| fontdb::Family::Name(name))
        .collect();
    families.push(if monospace {
        fontdb::Family::Monospace
    } else {
        fontdb::Family::SansSerif
    });

    let id = db.query(&fontdb::Query {
        families: &families,
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    })?;
    let (source, index) = db.face_source(id)?;
    let bytes = match source {
        fontdb::Source::File(path) => fs::read(path).ok()?,
        fontdb::Source::Binary(data) => data.as_ref().as_ref().to_vec(),
        fontdb::Source::SharedFile(_, data) => data.as_ref().as_ref().to_vec(),
    };
    let mut font = egui::FontData::from_owned(bytes);
    font.index = index;
    Some(font)
}

fn paths_equal(left: Option<&Path>, right: &Path) -> bool {
    let Some(left) = left else {
        return false;
    };
    let normalize = |path: &Path| {
        std::fs::canonicalize(path)
            .or_else(|_| std::path::absolute(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let left = normalize(left);
    let right = normalize(right);
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    atomicwrites::AtomicFile::new(path, atomicwrites::AllowOverwrite)
        .write(|file| file.write_all(content.as_bytes()))
        .map_err(|error| error.to_string())
}

#[derive(Debug)]
enum ReadError {
    Io(std::io::Error),
    Encoding,
}

fn read_text_file(path: &Path) -> Result<String, ReadError> {
    let bytes = fs::read(path).map_err(ReadError::Io)?;

    let mut text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let bytes = error.into_bytes();
            let (decoded, _, had_errors) = encoding_rs::GB18030.decode(&bytes);
            if had_errors {
                return Err(ReadError::Encoding);
            }
            decoded.into_owned()
        }
    };

    if let Some(stripped) = text.strip_prefix('\u{feff}') {
        text = stripped.to_owned();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_existing_extensions() {
        assert_eq!(
            ensure_extension(PathBuf::from("note.md"), "md"),
            PathBuf::from("note.md")
        );
        assert_eq!(
            ensure_extension(PathBuf::from("note"), "md"),
            PathBuf::from("note.md")
        );
        assert_eq!(
            ensure_export_extension(PathBuf::from("报告.PDF"), "pdf"),
            PathBuf::from("报告.PDF")
        );
        assert_eq!(
            ensure_export_extension(PathBuf::from("报告.txt"), "pdf"),
            PathBuf::from("报告.pdf")
        );
    }

    #[test]
    fn finds_unicode_matches_with_byte_offsets() {
        let content = "你好 world 你好";
        let matches = find_matches_in(content, "你好");
        assert_eq!(matches.len(), 2);
        for (start, end) in matches {
            assert_eq!(&content[start..end], "你好");
        }
    }

    #[test]
    fn chooses_the_right_tab_after_close() {
        assert_eq!(active_after_tab_close(2, 0, 2), 1);
        assert_eq!(active_after_tab_close(1, 1, 2), 1);
        assert_eq!(active_after_tab_close(0, 3, 2), 0);
        assert_eq!(active_after_tab_close(1, 2, 2), 1);
        assert_eq!(active_after_tab_close(0, 0, 0), 0);
    }

    #[test]
    fn compares_canonical_paths() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("mdlite-path-{}.md", std::process::id()));
        fs::write(&path, "x").unwrap();
        assert!(paths_equal(Some(&path), &path));
        #[cfg(not(windows))]
        {
            let relative = path
                .strip_prefix(std::env::current_dir().unwrap())
                .unwrap_or(&path);
            assert!(paths_equal(Some(&path), relative));
        }
        let other = dir.join(format!("mdlite-path-other-{}.md", std::process::id()));
        assert!(!paths_equal(Some(&path), &other));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn truncates_tab_labels_by_characters() {
        assert_eq!(truncate_chars("short.md", 28), "short.md");
        assert_eq!(
            truncate_chars("这是一个很长的标签页名称.md", 5),
            "这是一个很…"
        );
    }

    #[test]
    fn saves_atomically() {
        let path = std::env::temp_dir().join(format!("mdlite-atomic-{}.md", std::process::id()));
        write_text_atomic(&path, "第一版").unwrap();
        write_text_atomic(&path, "第二版").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "第二版");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn decodes_utf8_and_gb18030() {
        let dir = std::env::temp_dir();
        let utf8_path = dir.join(format!("mdlite-utf8-{}.md", std::process::id()));
        fs::write(&utf8_path, "# 你好\n").unwrap();
        assert_eq!(read_text_file(&utf8_path).unwrap(), "# 你好\n");
        let _ = fs::remove_file(utf8_path);

        let gbk_path = dir.join(format!("mdlite-gbk-{}.md", std::process::id()));
        let (gbk, _, _) = encoding_rs::GB18030.encode("# 你好\n");
        fs::write(&gbk_path, gbk.as_ref()).unwrap();
        assert_eq!(read_text_file(&gbk_path).unwrap(), "# 你好\n");
        let _ = fs::remove_file(gbk_path);
    }
}
