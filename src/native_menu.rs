//! Native macOS application menu and About panel.

use eframe::egui;
use muda::accelerator::Accelerator;
use muda::{AboutMetadata, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use crate::i18n::I18n;
use crate::settings::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeAction {
    NewTab,
    Open,
    Save,
    SaveAs,
    CloseTab,
    EditMode,
    SplitMode,
    PreviewMode,
    ExportPdf,
    ExportWord,
    Settings,
    Quit,
    GitHub,
    Email,
}

pub struct NativeMenu {
    _menu: Menu,
    actions: Vec<(MenuId, NativeAction)>,
    pub language: Language,
}

impl NativeMenu {
    pub fn install(ctx: &egui::Context, language: Language) -> Option<Self> {
        let i18n = I18n::new(language);
        let menu = Menu::new();
        let mut actions = Vec::new();

        let about = PredefinedMenuItem::about(
            Some("MarkLite"),
            Some(AboutMetadata {
                name: Some("MarkLite".to_owned()),
                version: Some(mdlite::APP_VERSION.to_owned()),
                short_version: Some(mdlite::APP_VERSION.to_owned()),
                copyright: Some("© 2026 Carl Lau · MIT License".to_owned()),
                credits: Some(format!(
                    "Carl Lau <{}>\n{}\n\nMIT License",
                    mdlite::AUTHOR_EMAIL,
                    mdlite::GITHUB_URL
                )),
                ..Default::default()
            }),
        );
        let settings_item =
            MenuItem::new(format!("{}…", i18n.settings()), true, accelerator("cmd+,"));
        actions.push((settings_item.id().clone(), NativeAction::Settings));
        let services = PredefinedMenuItem::services(None);
        let hide = PredefinedMenuItem::hide(None);
        let hide_others = PredefinedMenuItem::hide_others(None);
        let show_all = PredefinedMenuItem::show_all(None);
        let quit = MenuItem::new(i18n.exit(), true, accelerator("cmd+q"));
        actions.push((quit.id().clone(), NativeAction::Quit));

        let app_menu = Submenu::new("MarkLite", true);
        app_menu
            .append_items(&[
                &about,
                &settings_item,
                &PredefinedMenuItem::separator(),
                &services,
                &PredefinedMenuItem::separator(),
                &hide,
                &hide_others,
                &show_all,
                &PredefinedMenuItem::separator(),
                &quit,
            ])
            .ok()?;

        let new_tab = MenuItem::new(i18n.new_tab(), true, accelerator("cmd+n"));
        let open = MenuItem::new(i18n.open(), true, accelerator("cmd+o"));
        let save = MenuItem::new(i18n.save(), true, accelerator("cmd+s"));
        let save_as = MenuItem::new(i18n.save_as(), true, accelerator("cmd+shift+s"));
        let close_tab = MenuItem::new(i18n.close_tab(), true, accelerator("cmd+w"));
        actions.extend([
            (new_tab.id().clone(), NativeAction::NewTab),
            (open.id().clone(), NativeAction::Open),
            (save.id().clone(), NativeAction::Save),
            (save_as.id().clone(), NativeAction::SaveAs),
            (close_tab.id().clone(), NativeAction::CloseTab),
        ]);
        let file_menu = Submenu::new(i18n.file_menu(), true);
        file_menu
            .append_items(&[
                &new_tab,
                &open,
                &PredefinedMenuItem::separator(),
                &save,
                &save_as,
                &PredefinedMenuItem::separator(),
                &close_tab,
            ])
            .ok()?;

        let edit = MenuItem::new(i18n.edit_mode(), true, accelerator("cmd+1"));
        let split = MenuItem::new(i18n.split_mode(), true, accelerator("cmd+2"));
        let preview = MenuItem::new(i18n.preview_mode(), true, accelerator("cmd+3"));
        actions.extend([
            (edit.id().clone(), NativeAction::EditMode),
            (split.id().clone(), NativeAction::SplitMode),
            (preview.id().clone(), NativeAction::PreviewMode),
        ]);
        let view_menu = Submenu::new(i18n.view_menu(), true);
        view_menu.append_items(&[&edit, &split, &preview]).ok()?;

        let export_pdf = MenuItem::new(i18n.export_pdf(), true, None);
        let export_word = MenuItem::new(i18n.export_word(), true, None);
        actions.extend([
            (export_pdf.id().clone(), NativeAction::ExportPdf),
            (export_word.id().clone(), NativeAction::ExportWord),
        ]);
        let export_menu = Submenu::new(i18n.export_menu(), true);
        export_menu
            .append_items(&[&export_pdf, &export_word])
            .ok()?;

        let github = MenuItem::new(i18n.github_repository(), true, None);
        let email = MenuItem::new(i18n.email_author(), true, None);
        actions.extend([
            (github.id().clone(), NativeAction::GitHub),
            (email.id().clone(), NativeAction::Email),
        ]);
        let help_menu = Submenu::new(i18n.help_menu(), true);
        help_menu.append_items(&[&github, &email]).ok()?;

        menu.append_items(&[&app_menu, &file_menu, &view_menu, &export_menu, &help_menu])
            .ok()?;
        menu.init_for_nsapp();

        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |_| {
            ctx.request_repaint();
        }));

        Some(Self {
            _menu: menu,
            actions,
            language,
        })
    }

    pub fn poll(&self) -> Vec<NativeAction> {
        let mut actions = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some((_, action)) = self.actions.iter().find(|(id, _)| id == &event.id) {
                actions.push(*action);
            }
        }
        actions
    }
}

fn accelerator(spec: &str) -> Option<Accelerator> {
    spec.parse().ok()
}
