//! egui desktop front-end: the application shell (menu, navigation, status
//! bar, database lifecycle) and one module per page.

mod charts;
mod data;
mod filters;
mod launch;
pub mod pages;
mod state;
mod theme;
mod widgets;

use crate::db::Database;
use crate::demo::DemoWorkspace;
use crate::help::tips;
use crate::settings::{AppSettings, Theme};
use data::DataCache;
use egui::{Context, RichText};
pub use launch::LaunchOptions;
use pages::help::HelpPage;
use pages::start::StartPage;
use state::{AppAction, Outbox, PageCtx, Status, View};
use std::path::{Path, PathBuf};
use std::time::Duration;
use theme::Palette;

/// How long an informational status message stays visible.
const STATUS_TTL: Duration = Duration::from_secs(8);

#[derive(Default)]
struct Pages {
    dashboard: pages::dashboard::DashboardPage,
    transactions: pages::transactions::TransactionsPage,
    import: pages::import::ImportPage,
    categorize: pages::categorize::CategorizePage,
    categories: pages::categories::CategoriesPage,
    accounts: pages::accounts::AccountsPage,
    analytics: pages::analytics::AnalyticsPage,
    budgets: pages::budgets::BudgetsPage,
    mortgage: pages::mortgage::MortgagePage,
}

/// An open database with its cached data and page states.
struct Session {
    db: Database,
    data: DataCache,
    pages: Pages,
    /// Set in demo mode. Declared after `db` so the database closes before the
    /// temporary folder is deleted.
    demo: Option<DemoWorkspace>,
}

impl Session {
    fn new(db: Database) -> anyhow::Result<Self> {
        let data = DataCache::load(&db, 1)?;
        Ok(Self { db, data, pages: Pages::default(), demo: None })
    }

    fn demo() -> anyhow::Result<Self> {
        let ws = DemoWorkspace::create()?;
        let mut s = Self::new(Database::open(&ws.demo.database)?)?;
        s.demo = Some(ws);
        Ok(s)
    }

    /// The database folder, unless this is the throwaway demo.
    fn real_root(&self) -> Option<&Path> {
        self.demo.is_none().then(|| self.db.root())
    }

    fn reload(&mut self) -> anyhow::Result<()> {
        self.data = DataCache::load(&self.db, self.data.generation + 1)?;
        Ok(())
    }

    fn show(&mut self, view: View, ui: &mut egui::Ui, settings: &AppSettings, out: &mut Outbox, palette: Palette) {
        let mut cx = PageCtx { db: &self.db, data: &self.data, settings, out, palette };
        let p = &mut self.pages;
        match view {
            View::Dashboard => p.dashboard.show(ui, &mut cx),
            View::Transactions => p.transactions.show(ui, &mut cx),
            View::Import => p.import.show(ui, &mut cx),
            View::Categorize => p.categorize.show(ui, &mut cx),
            View::Categories => p.categories.show(ui, &mut cx),
            View::Accounts => p.accounts.show(ui, &mut cx),
            View::Analytics => p.analytics.show(ui, &mut cx),
            View::Budgets => p.budgets.show(ui, &mut cx),
            View::Mortgage => p.mortgage.show(ui, &mut cx),
            View::Settings | View::Help => unreachable!("handled by the shell"),
        }
    }
}

pub struct FinTrackApp {
    settings: AppSettings,
    settings_path: PathBuf,
    session: Option<Session>,
    view: View,
    start: StartPage,
    help: HelpPage,
    status: Option<Status>,
    show_about: bool,
}

fn apply_theme(ctx: &Context, theme: Theme) {
    ctx.set_theme(match theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
}

impl FinTrackApp {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: LaunchOptions) -> Self {
        let settings_path = AppSettings::default_path();
        let settings = AppSettings::load_from(&settings_path);
        apply_theme(&cc.egui_ctx, settings.theme);
        let mut app = Self {
            settings,
            settings_path,
            session: None,
            view: View::Dashboard,
            start: StartPage::default(),
            help: HelpPage::default(),
            status: None,
            show_about: false,
        };
        if launch.demo {
            app.open_demo();
        } else if app.settings.open_default_on_startup
            && let Some(path) = app.settings.default_database.clone()
        {
            app.open_with(&path, Database::open);
        }
        if let Some(v) = launch.page.as_deref().and_then(View::from_key) {
            app.view = v;
        }
        if !launch.import.is_empty() {
            match &mut app.session {
                Some(s) => {
                    s.pages.import.enqueue(launch.import);
                    app.view = View::Import;
                }
                None => app.set_status("Open or create a database before importing files.", true),
            }
        }
        app
    }

    fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(Status { text: text.into(), error, at: std::time::Instant::now() });
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save_to(&self.settings_path) {
            self.set_status(format!("Could not save settings: {e:#}"), true);
        }
    }

    fn open_with(&mut self, path: &Path, open: fn(&Path) -> anyhow::Result<Database>) {
        match open(path).and_then(Session::new) {
            Ok(session) => {
                let root = session.db.root().to_path_buf();
                self.set_status(format!("Opened {}", root.display()), false);
                self.settings.remember(&root);
                if self.settings.default_database.is_none() {
                    self.settings.default_database = Some(root);
                }
                self.save_settings();
                self.start_session(session);
            }
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }

    /// Demo mode: a freshly generated database in a temporary folder. It is not
    /// added to recent databases or made the default.
    fn open_demo(&mut self) {
        // Close any open session first so a previous demo folder is released.
        self.session = None;
        match Session::demo() {
            Ok(session) => {
                self.set_status("Demo mode — explore freely; the sample data is deleted when you exit the demo.", false);
                self.start_session(session);
            }
            Err(e) => self.set_status(format!("Could not create the demo: {e:#}"), true),
        }
    }

    fn start_session(&mut self, session: Session) {
        self.session = Some(session);
        self.view = View::Dashboard;
    }

    /// Banner shown above every page in demo mode.
    fn demo_banner(ui: &mut egui::Ui, session: &Session, palette: &Palette, out: &mut Outbox) {
        let Some(ws) = &session.demo else { return };
        egui::Frame::new()
            .fill(palette.series(0).gamma_multiply(0.18))
            .stroke(egui::Stroke::new(1.0_f32, palette.series(0)))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("🎓 Demo mode").strong());
                    ui.label("Sample data in a temporary folder — change anything you like; it's deleted when you exit.");
                    widgets::help(ui, tips::DEMO);
                    if ui.button("Try importing the sample CSVs").on_hover_text(tips::DEMO_IMPORT).clicked() {
                        out.import_files = sample_csvs(&ws.demo.sample_csvs);
                    }
                    if ui.button("Exit demo").clicked() {
                        out.app = Some(AppAction::Close);
                    }
                });
            });
        ui.add_space(6.0);
    }

    fn backup(&mut self) {
        let Some(session) = &self.session else { return };
        let name = format!("{}-backup-{}.sqlite", session.db.name(), chrono::Local::now().format("%Y%m%d"));
        let Some(dest) = rfd::FileDialog::new()
            .set_title("Back up database")
            .set_file_name(name)
            .add_filter("SQLite", &["sqlite"])
            .save_file()
        else {
            return;
        };
        match session.db.backup_to(&dest) {
            Ok(()) => self.set_status(format!("Backup written to {}", dest.display()), false),
            Err(e) => self.set_status(format!("Backup failed: {e:#}"), true),
        }
    }

    /// Apply the side effects pages requested during this frame.
    fn apply(&mut self, ctx: &Context, out: Outbox) {
        if let Some(s) = out.status {
            self.status = Some(s);
        }
        if out.settings_changed {
            apply_theme(ctx, self.settings.theme);
            self.save_settings();
        }
        if let Some(session) = &mut self.session {
            if out.reload
                && let Err(e) = session.reload()
            {
                self.status = Some(Status { text: format!("Reload failed: {e:#}"), error: true, at: std::time::Instant::now() });
            }
            if let Some(f) = out.txn_filter {
                session.pages.transactions.set_filter(f);
            }
            if let Some(r) = out.rule_draft {
                session.pages.categorize.open_editor(r);
            }
            if !out.import_files.is_empty() {
                session.pages.import.enqueue(out.import_files);
                self.view = View::Import;
            }
        }
        if let Some(v) = out.navigate {
            self.view = v;
        }
        match out.app {
            Some(AppAction::Open(p)) => self.open_with(&p, Database::open),
            Some(AppAction::Create(p)) => self.open_with(&p, Database::create),
            Some(AppAction::Close) => {
                self.session = None;
                self.view = View::Dashboard;
            }
            Some(AppAction::Demo) => self.open_demo(),
            Some(AppAction::Backup) => self.backup(),
            None => {}
        }
    }

    fn menu_bar(&mut self, ctx: &Context, out: &mut Outbox) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New database…").on_hover_text(tips::NEW_DATABASE).clicked() {
                        out.app = Some(AppAction::Close);
                        ui.close_menu();
                    }
                    if ui.button("Open database…").on_hover_text(tips::OPEN_DATABASE).clicked() {
                        if let Some(dir) = rfd::FileDialog::new().set_title("Open FinTrack database folder").pick_folder() {
                            out.app = Some(AppAction::Open(dir));
                        }
                        ui.close_menu();
                    }
                    ui.menu_button("Open recent", |ui| {
                        for p in &self.settings.recent_databases {
                            if ui.button(p.display().to_string()).clicked() {
                                out.app = Some(AppAction::Open(p.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                    if ui.button("🎓 Try the demo").on_hover_text(tips::DEMO).clicked() {
                        out.app = Some(AppAction::Demo);
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_db = self.session.is_some();
                    if ui.add_enabled(has_db, egui::Button::new("Import CSV files…")).clicked() {
                        if let Some(files) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_files() {
                            out.import_files = files;
                        }
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_db, egui::Button::new("Back up database…")).on_hover_text(tips::BACKUP).clicked() {
                        out.app = Some(AppAction::Backup);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_db, egui::Button::new("Show database folder")).clicked() {
                        if let Some(s) = &self.session {
                            reveal_in_file_manager(s.db.root());
                        }
                        ui.close_menu();
                    }
                    let close_label =
                        if self.session.as_ref().is_some_and(|s| s.demo.is_some()) { "Exit demo" } else { "Close database" };
                    if ui.add_enabled(has_db, egui::Button::new(close_label)).clicked() {
                        out.app = Some(AppAction::Close);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("User guide").clicked() {
                        out.navigate = Some(View::Help);
                        ui.close_menu();
                    }
                    if ui.button("About FinTrack").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dark = ui.visuals().dark_mode;
                    let (icon, next, tip) = if dark {
                        ("☀ Light", Theme::Light, "Switch to light mode")
                    } else {
                        ("🌙 Dark", Theme::Dark, "Switch to dark mode")
                    };
                    if ui.button(icon).on_hover_text(format!("{tip} (more options in Settings ▸ Appearance)")).clicked() {
                        self.settings.theme = next;
                        out.settings_changed = true;
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ctx: &Context, palette: &Palette) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match &self.session {
                    Some(s) if s.demo.is_some() => ui.label(RichText::new("🎓 Demo database (temporary)").small()),
                    Some(s) => ui.label(RichText::new(format!("🗄 {}", s.db.root().display())).small()),
                    None => ui.label(RichText::new("No database open").small()),
                };
                ui.label(RichText::new("· 🔒 offline").small().weak()).on_hover_text(tips::PRIVACY);
                if let Some(s) = &self.status {
                    let expired = !s.error && s.at.elapsed() > STATUS_TTL;
                    if !expired {
                        ui.separator();
                        let text = RichText::new(&s.text).small();
                        ui.label(if s.error { text.color(palette.critical) } else { text });
                        if s.error && ui.small_button("✖").clicked() {
                            self.status = None;
                        } else if !s.error {
                            ctx.request_repaint_after(STATUS_TTL);
                        }
                    }
                }
            });
        });
    }

    fn nav(&mut self, ctx: &Context) {
        let views: &[View] = if self.session.is_some() { View::ALL } else { &[View::Dashboard, View::Settings, View::Help] };
        let unmapped = self.session.as_ref().map_or(0, |s| s.data.uncategorized_count());
        egui::SidePanel::left("nav").resizable(false).exact_width(180.0).show(ctx, |ui| {
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.spacing_mut().button_padding = egui::vec2(8.0, 5.0);
                for &v in views {
                    let label = match v {
                        View::Dashboard if self.session.is_none() => "🏠 Start".to_string(),
                        View::Categorize if unmapped > 0 => format!("{} {}  ({unmapped})", v.icon(), v.label()),
                        _ => format!("{} {}", v.icon(), v.label()),
                    };
                    if ui.selectable_label(self.view == v, RichText::new(label).size(14.0)).clicked() {
                        self.view = v;
                    }
                    if matches!(v, View::Import | View::Accounts | View::Mortgage) {
                        ui.separator();
                    }
                }
            });
        });
    }

    fn handle_dropped_files(&self, ctx: &Context, out: &mut Outbox) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if dropped.is_empty() {
            return;
        }
        if self.session.is_some() {
            out.import_files = dropped;
        } else {
            out.error("Open or create a database before importing files.");
        }
    }

    fn drop_overlay(ctx: &Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop-overlay")));
        let rect = ctx.screen_rect();
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(160));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Drop CSV files to import",
            egui::FontId::proportional(28.0),
            egui::Color32::WHITE,
        );
    }

    fn about_window(&mut self, ctx: &Context) {
        egui::Window::new("About FinTrack").open(&mut self.show_about).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new(format!("FinTrack {}", env!("CARGO_PKG_VERSION"))).strong());
            ui.label("An offline household finance tracker.");
            ui.label(tips::PRIVACY);
        });
    }
}

/// The CSV files in the demo's sample folder.
fn sample_csvs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "csv")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

fn reveal_in_file_manager(path: &Path) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program).arg(path).spawn();
}

impl eframe::App for FinTrackApp {
    /// Close the database (and delete a demo's temporary folder) when the window closes.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.session = None;
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let palette = Palette::for_mode(ctx.style().visuals.dark_mode);
        let mut out = Outbox::default();
        self.handle_dropped_files(ctx, &mut out);
        self.menu_bar(ctx, &mut out);
        self.status_bar(ctx, &palette);
        self.nav(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(session) = &self.session {
                Self::demo_banner(ui, session, &palette, &mut out);
            }
            match (self.view, &mut self.session) {
                (View::Settings, session) => {
                    // The demo can't become the default database.
                    let current = session.as_ref().and_then(|s| s.real_root()).map(Path::to_path_buf);
                    pages::settings::show(ui, &mut self.settings, current.as_deref(), &mut out);
                }
                (View::Help, _) => self.help.show(ui),
                (view, Some(session)) => session.show(view, ui, &self.settings, &mut out, palette),
                (_, None) => self.start.show(ui, &self.settings, &mut out),
            }
        });
        self.about_window(ctx);
        Self::drop_overlay(ctx);
        self.apply(ctx, out);
    }
}
