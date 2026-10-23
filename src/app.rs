use crate::{
    devices::{self, Keyboard},
    engine::{Session, State, TestMode},
    export,
    history::{self, Record},
    layouts::{self, Mode},
    preferences::{Preferences, Profile, Theme},
    registry::{self, InstalledLayout},
    xkb::Emulator,
};
use eframe::egui::{self, Color32, RichText};
use std::time::Duration;

const PERIWINKLE: Color32 = Color32::from_rgb(134, 150, 245);

pub struct Typefinch {
    prefs: Preferences,
    session: Session,
    mode: Mode,
    emulator: Option<Emulator>,
    installed_layouts: Vec<InstalledLayout>,
    layout_search: String,
    history: Vec<Record>,
    keyboards: Vec<Keyboard>,
    device_index: usize,
    device_error: Option<String>,
    new_profile: String,
    custom_text: String,
    message: String,
}
impl Typefinch {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut prefs = Preferences::load();
        if std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
            // Opt-in synthetic GUI test process only, never persisted to a real profile.
            prefs.duration_seconds = 2;
        }
        cc.egui_ctx.set_theme(match prefs.theme {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        });
        cc.egui_ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = egui::vec2(10.0, 9.0);
            s.spacing.button_padding = egui::vec2(11.0, 7.0);
            s.visuals.selection.bg_fill = PERIWINKLE;
            s.visuals.hyperlink_color = PERIWINKLE;
        });
        let (keyboards, device_error) = Self::query_devices();
        let session = Session::new_with_mode(prefs.duration_seconds, prefs.test_mode);
        let emulator = Emulator::compile(&prefs.layout).ok();
        let installed_layouts = registry::discover().unwrap_or_default();
        let history = history::recent(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        Self { prefs, session, mode: Mode::System, emulator, installed_layouts, layout_search: String::new(), history, keyboards, device_error,
               device_index: 0, new_profile: String::new(), custom_text: String::new(), message: String::new() }
    }
    fn query_devices() -> (Vec<Keyboard>, Option<String>) {
        match devices::inspect() {
            Ok(k) => (k, None),
            Err(e) => (Vec::new(), Some(e)),
        }
    }
    fn save(&mut self) {
        if let Err(e) = self.prefs.save() { self.message = format!("Could not save settings: {e}"); }
    }
    fn finish(&mut self) {
        let result = self.session.score();
        if result.entered == 0 {
            self.message = "Empty session not recorded. Start another test when ready.".into();
            return;
        }
        let identity = self.prefs.profiles[self.prefs.active_profile].id.clone();
        let mode = if self.mode == Mode::System { "OS" } else { "Emulated" };
        let record = Record::completed(&identity, &self.prefs.layout, mode, &self.session);
        match history::insert(&record) {
            Ok(_) => match history::recent(&identity) {
                Ok(records) => self.history = records,
                Err(e) => self.message = format!("History read failed: {e}"),
            },
            Err(e) => self.message = format!("History not saved: {e}"),
        }
        let profile = &mut self.prefs.profiles[self.prefs.active_profile];
        profile.tests = profile.tests.saturating_add(1);
        profile.best_wpm = profile.best_wpm.max(result.wpm);
        profile.best_accuracy = profile.best_accuracy.max(result.accuracy);
        let completed_tests = profile.tests;
        self.save();
        if let Err(err) = crate::qa::write_smoke_report(&self.session, completed_tests) {
            self.message = format!("QA report failed: {err}");
        }
    }
    fn rebuild_session(&mut self) {
        self.session = Session::new_with_custom(self.prefs.duration_seconds, self.prefs.test_mode, &self.custom_text);
    }
    fn start_test(&mut self, ctx: &egui::Context) {
        if self.session.target.is_empty() {
            self.message = "Enter some custom text before starting this test.".into();
            return;
        }
        self.session.begin();
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("typing_surface")));
    }
    fn apply_events(&mut self, ctx: &egui::Context) {
        if self.session.state != State::Running { return; }
        if !ctx.input(|i| i.focused) {
            // A window losing focus must not collect invisible typing events.
            self.session.state = State::Ready;
            self.message = "Session canceled on window focus loss. No result saved.".into();
            return;
        }
        if !ctx.memory(|m| m.has_focus(egui::Id::new("typing_surface"))) {
            // Deliberately do not count keystrokes aimed at settings/profile widgets.
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                    self.session.state = State::Ready;
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, repeat: false, .. } =>
                    self.session.backspace(),
                egui::Event::Key { key, physical_key, pressed: true, repeat: false, modifiers, .. }
                    if self.mode == Mode::Emulated && !modifiers.command && !modifiers.ctrl && !modifiers.alt =>
                {
                    // Crucially use the physical key, not an OS-translated logical letter.
                    // If unavailable, do not silently claim emulation works.
                    if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let mapped = if let Some(emulator) = &self.emulator {
                            emulator.translate(physical, modifiers.shift)
                        } else {
                            layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c| c.to_string())
                        };
                        if let Some(text) = mapped { self.session.type_text(&text); }
                    }
                }
                egui::Event::Text(text) if self.mode == Mode::System =>
                    self.session.type_text(&text),
                // Paste is intentionally disallowed in scored sessions.
                _ => {}
            }
        }
    }
    fn show_profile(&mut self, ui: &mut egui::Ui) {
        ui.heading("Your perch");
        let previous = self.prefs.active_profile;
        ui.add_enabled_ui(self.session.state != State::Running, |ui| {
          ui.vertical(|ui| {
            egui::ComboBox::from_id_salt("profile").selected_text(&self.prefs.profiles[self.prefs.active_profile].name)
                .show_ui(ui, |ui| {
                    for (i, profile) in self.prefs.profiles.iter().enumerate() {
                        ui.selectable_value(&mut self.prefs.active_profile, i, &profile.name);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut self.new_profile).hint_text("New local profile").desired_width(190.0));
            if ui.button("Create profile").clicked() {
                let name = self.new_profile.trim();
                if !name.is_empty() && name.chars().count() <= 32 {
                    self.prefs.profiles.push(Profile { name: name.to_owned(), ..Default::default() });
                    self.prefs.active_profile = self.prefs.profiles.len() - 1;
                    self.new_profile.clear();
                    self.save();
                } else {
                    self.message = "Profile name must contain 1–32 characters.".into();
                }
            }
          });
        });
        if self.session.state == State::Running {
            ui.small("Profile selection is locked until this typing test finishes.");
        }
        if self.prefs.active_profile != previous {
            let id = self.prefs.profiles[self.prefs.active_profile].id.clone();
            self.history = history::recent(&id).unwrap_or_default();
            self.save();
        }
        let p = &self.prefs.profiles[self.prefs.active_profile];
        ui.label(format!("{} completed tests  ·  best {:.1} WPM  ·  best accuracy {:.1}%",
            p.tests, p.best_wpm, p.best_accuracy));
    }
    fn show_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Test setup");
        ui.vertical(|ui| {
            ui.label("Duration");
            let previous = self.prefs.duration_seconds;
            egui::ComboBox::from_id_salt("duration").selected_text(format!("{} seconds", previous))
                .show_ui(ui, |ui| {
                    for sec in [15, 30, 60, 120] {
                        ui.selectable_value(&mut self.prefs.duration_seconds, sec, format!("{sec} s"));
                    }
                });
            if self.prefs.duration_seconds != previous {
                if self.session.state == State::Running {
                    self.prefs.duration_seconds = previous;
                } else {
                    self.rebuild_session();
                    self.save();
                }
            }
            ui.add_space(6.0);
            ui.label("Test content");
            let last_mode = self.prefs.test_mode;
            egui::ComboBox::from_id_salt("test-content").selected_text(last_mode.label())
                .show_ui(ui, |ui| {
                    for mode in TestMode::ALL {
                        ui.selectable_value(&mut self.prefs.test_mode, mode, mode.label());
                    }
                });
            if self.prefs.test_mode != last_mode {
                if self.session.state == State::Running {
                    self.prefs.test_mode = last_mode;
                } else {
                    self.rebuild_session();
                    self.save();
                }
            }
            if self.prefs.test_mode == TestMode::Custom {
                ui.add_space(6.0);
                ui.label("Custom passage (in memory only)");
                let edit = ui.add_enabled(
                    self.session.state != State::Running,
                    egui::TextEdit::multiline(&mut self.custom_text)
                        .desired_rows(4)
                        .desired_width(ui.available_width())
                        .hint_text("Paste or write text for your private typing test")
                );
                if edit.changed() { self.rebuild_session(); }
                ui.small(format!("{} characters entered; only the first 4096 will be used. Text is not saved.", self.custom_text.chars().count()));
            }
            ui.add_space(6.0);
            ui.label("Theme");
            let previous = self.prefs.theme;
            egui::ComboBox::from_id_salt("theme").selected_text(match previous {
                Theme::System=>"System (default)", Theme::Light=>"Light", Theme::Dark=>"Dark",
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.prefs.theme, Theme::System, "System");
                ui.selectable_value(&mut self.prefs.theme, Theme::Light, "Light");
                ui.selectable_value(&mut self.prefs.theme, Theme::Dark, "Dark");
            });
            if previous != self.prefs.theme {
                ctx.set_theme(match self.prefs.theme {
                    Theme::System=>egui::ThemePreference::System,
                    Theme::Light=>egui::ThemePreference::Light,
                    Theme::Dark=>egui::ThemePreference::Dark,
                });
                self.save();
            }
        });
        ui.vertical(|ui| {
            ui.label("Target layout");
            let last = self.prefs.layout.clone();
            let selected_name = self.installed_layouts.iter()
                .find(|l|l.id==self.prefs.layout).map(|l|l.name.as_str())
                .or_else(||layouts::find(&self.prefs.layout).map(|l|l.name))
                .unwrap_or("Unknown layout");
            egui::ComboBox::from_id_salt("layout").width(ui.available_width().min(270.0))
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    let filter = self.layout_search.trim().to_lowercase();
                    for l in layouts::CATALOG {
                        if !filter.is_empty() && !format!("{} {} {}",l.name,l.id,l.region).to_lowercase().contains(&filter) { continue; }
                        let suffix = if l.rows.is_some() { " · built-in" } else { " · XKB if installed" };
                        ui.selectable_value(&mut self.prefs.layout, l.id.to_owned(), format!("{}{}", l.name, suffix));
                    }
                    if filter.is_empty() {
                        ui.label("Search below to browse installed XKB variants.");
                    } else {
                        let mut count=0;
                        for l in &self.installed_layouts {
                            if layouts::find(&l.id).is_some() { continue; }
                            if !format!("{} {}",l.name,l.id).to_lowercase().contains(&filter) { continue; }
                            if count>=60 { break; }
                            ui.selectable_value(&mut self.prefs.layout, l.id.clone(), format!("{} · XKB",l.name));
                            count+=1;
                        }
                        if count>=60 { ui.label("Showing first 60 matches; refine search."); }
                    }
                });
            if self.prefs.layout != last {
                self.emulator = Emulator::compile(&self.prefs.layout).ok();
                if self.emulator.is_none() && !layouts::find(&self.prefs.layout).is_some_and(|l|l.rows.is_some()) {
                    self.mode = Mode::System;
                    self.message = "The selected XKB layout/variant is not installed; using OS input.".into();
                }
                self.save();
            }
            ui.separator();
            ui.selectable_value(&mut self.mode, Mode::System, "OS input");
            if self.emulator.is_some() || layouts::find(&self.prefs.layout).is_some_and(|l|l.rows.is_some()) {
                ui.selectable_value(&mut self.mode, Mode::Emulated, "Software emulation");
            } else {
                ui.add_enabled(false, egui::Button::new("Software emulation (not available)"));
            }
        });
        ui.add(egui::TextEdit::singleline(&mut self.layout_search).hint_text("Search installed layouts / variants").desired_width(285.0));
        ui.small(format!("{} system XKB layouts and variants discovered · emulation compiled: {}",
            self.installed_layouts.len(), if self.emulator.is_some() { "yes" } else { "no" }));
        ui.small("OS input follows the active system layout; this selection controls the emulation target.");
        if self.mode == Mode::Emulated {
            ui.colored_label(PERIWINKLE, "Emulation uses physical ANSI key positions inside Typefinch only; it never changes your system layout.");
            ui.small(if self.emulator.is_some() { "Compiled using installed XKB symbols; physical-key mapping covers the initial ANSI positions. Compose, AltGr, and IME are pending." } else { "Using starter ANSI table; physical key events required. Compose, AltGr, and IME are pending." });
        }
    }
    fn show_devices(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connected keyboards");
        ui.vertical(|ui| {
            if ui.button("Refresh device list").clicked() {
                let previous = self.keyboards.get(self.device_index).map(|k|k.address.clone());
                (self.keyboards, self.device_error) = Self::query_devices();
                self.device_index = previous.and_then(|addr|self.keyboards.iter().position(|k|k.address==addr)).unwrap_or(0);
            }
            if !self.keyboards.is_empty() {
                self.device_index = self.device_index.min(self.keyboards.len() - 1);
                egui::ComboBox::from_id_salt("device").selected_text(&self.keyboards[self.device_index].name)
                    .show_ui(ui, |ui| {
                        for (i, k) in self.keyboards.iter().enumerate() {
                            ui.selectable_value(&mut self.device_index, i, &k.name);
                        }
                    });
                let k = &self.keyboards[self.device_index];
                let active_map = k.active_keymap.clone();
                ui.label(format!("Active map: {} · configured: {} {}", k.active_keymap, k.layout, k.variant));
                ui.small(format!("Device: {}", k.address));
                let matched = registry::resolve_active(&self.installed_layouts, &active_map).map(|l|l.id.clone());
                if let Some(id) = matched {
                    if ui.add_enabled(self.session.state != State::Running, egui::Button::new("Use detected layout")).clicked() {
                        self.prefs.layout = id;
                        self.emulator = Emulator::compile(&self.prefs.layout).ok();
                        self.save();
                        self.message = "Target layout updated from this device's reported active keymap. Your system layout is unchanged.".into();
                    }
                } else {
                    ui.small("No unique installed XKB match for this device's active map.");
                }
            } else {
                ui.label(self.device_error.as_deref().unwrap_or("No keyboards listed by Hyprland."));
            }
        });
        ui.small("This selector inspects keyboard metadata. Ordinary Wayland typing events are seat-level: this is not yet a per-device input filter.");
    }
    fn show_test(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading(RichText::new("Typing test").size(24.0).strong());
        ui.horizontal_wrapped(|ui| {
            let name = if self.session.state == State::Running { "Restart test" } else { "Start test" };
            if ui.add_enabled(!self.session.target.is_empty(), egui::Button::new(name)).clicked() { self.start_test(ctx); }
            if self.session.state == State::Running && ui.button("Stop").clicked() { self.session.state=State::Ready; }
            ui.label(match self.session.state {
                State::Ready=>"Ready · press Start to capture typing",
                State::Running=>"Typing · Esc cancels · Backspace corrects",
                State::Finished=>"Finished · result saved to this local profile",
            });
        });
        let score = self.session.score();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{:.0} WPM", score.wpm)).size(23.0).color(PERIWINKLE).strong());
            ui.label(format!("{:.1}% accuracy", score.accuracy));
            ui.label(format!("{:.0} raw WPM", score.raw_wpm));
            ui.label(format!("{:.1}% raw accuracy", score.raw_accuracy));
            ui.label(format!("{} mistakes · {} corrections", score.mistakes, self.session.corrections));
            ui.label(format!("{} chars · {:.1} s elapsed", score.entered, score.elapsed.as_secs_f64()));
            ui.label(format!("{:.1} s left", self.session.seconds_left()));
        });
        ui.add(egui::ProgressBar::new(
            if self.session.duration.is_zero() { 0.0 } else {
                (1.0 - self.session.seconds_left() / self.session.duration.as_secs_f64()) as f32
            }
        ).show_percentage());
        ui.add_space(8.0);
        let entered = self.session.typed.chars().count();
        let total = self.session.target.chars().count();
        let (first, last) = crate::engine::viewport(total, entered, 230);
        let expected: Vec<char> = self.session.target.chars().skip(first).take(last-first).collect();
        let actual: Vec<char> = self.session.typed.chars().skip(first).collect();
        let mut job = egui::text::LayoutJob::default();
        for (i, ch) in expected.iter().enumerate() {
            let position = first + i;
            let color = match actual.get(i) {
                Some(c) if c == ch => Color32::from_rgb(91, 184, 137),
                Some(_) => Color32::from_rgb(239, 102, 112),
                None if position == entered && self.session.state == State::Running => PERIWINKLE,
                None => ui.visuals().text_color().gamma_multiply(0.75),
            };
            job.append(&ch.to_string(), 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(21.0),
                color,
                ..Default::default()
            });
        }
        let typing_area = egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_height(135.0);
                ui.label(job);
                ui.small(format!("Reference characters {}–{} of {} · Click here to refocus or press F5 to restart.", first.saturating_add(1), last, total));
            });
        let response = ui.interact(
            typing_area.response.rect,
            egui::Id::new("typing_surface"),
            egui::Sense::click()
        );
        if response.clicked() { response.request_focus(); }
        if self.session.state == State::Running { ctx.request_repaint_after(Duration::from_millis(50)); }
    }
    fn show_history(&mut self, ui: &mut egui::Ui) {
        ui.heading("Recent tests");
        if self.history.is_empty() {
            ui.label("No completed scored tests yet. Results stay on this device.");
            return;
        }
        egui::Grid::new("recent-tests").striped(true).min_col_width(75.0).show(ui, |ui| {
            ui.strong("WPM");
            ui.strong("Raw WPM");
            ui.strong("Content");
            ui.strong("Accuracy");
            ui.strong("Duration");
            ui.strong("Layout");
            ui.strong("Input");
            ui.end_row();
            for r in &self.history {
                ui.label(format!("{:.1}", r.wpm));
                ui.label(format!("{:.1}", r.raw_wpm));
                ui.label(&r.test_mode);
                ui.label(format!("{:.1}%", r.accuracy));
                ui.label(format!("{}s", r.duration_ms / 1000));
                ui.label(&r.layout);
                ui.label(&r.input_mode);
                ui.end_row();
            }
        });
    }
    fn export_widget(&mut self) {
        let p = &self.prefs.profiles[self.prefs.active_profile];
        let display = self.installed_layouts.iter().find(|l|l.id==self.prefs.layout)
            .map(|l|l.name.as_str()).or_else(||layouts::find(&self.prefs.layout).map(|l|l.name))
            .unwrap_or("Custom XKB");
        self.message = match export::save_widget(p, display) {
            Ok((svg, md)) => format!("Exported: {} and {}", svg.display(), md.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }
    fn show_export(&mut self, ui: &mut egui::Ui) {
        ui.heading("GitHub profile card");
        ui.label("Export a static SVG card and matching Markdown for your GitHub profile.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export SVG + Markdown").clicked() { self.export_widget(); }
            if ui.button("Copy GitHub Markdown").clicked() {
                ui.ctx().copy_text(export::markdown_embed().to_owned());
                self.message = "Markdown image embed copied to clipboard.".into();
            }
        });
    }
}
impl eframe::App for Typefinch {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.start_test(ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F6)) {
            self.export_widget();
        }
        let before = self.session.state;
        self.apply_events(ctx);
        self.session.tick();
        if before == State::Running && self.session.state == State::Finished { self.finish(); }
        egui::TopBottomPanel::top("typefinch-header")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(24, 15)).fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::hover());
                    let painter = ui.painter();
                    let c = rect.center();
                    painter.rect_filled(rect, 12.0, PERIWINKLE.gamma_multiply(0.18));
                    painter.circle_filled(c + egui::vec2(-4.0, 3.0), 13.0, PERIWINKLE);
                    painter.circle_filled(c + egui::vec2(7.0, -6.0), 8.0, PERIWINKLE);
                    painter.circle_filled(c + egui::vec2(10.0, -8.0), 2.0, Color32::from_rgb(27, 34, 65));
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            c + egui::vec2(14.0, -6.0),
                            c + egui::vec2(22.0, -3.0),
                            c + egui::vec2(14.0, 0.0)
                        ],
                        Color32::from_rgb(246, 195, 118),
                        egui::Stroke::NONE
                    ));
                    ui.add_space(5.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Typefinch").size(28.0).strong().color(PERIWINKLE));
                        ui.label(RichText::new("Find your rhythm.").size(12.0).weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("LOCAL-FIRST  ·  DEVELOPMENT PREVIEW").size(11.0).weak());
                    });
                });
            });
        let wide = ctx.content_rect().width() >= 900.0;
        if wide {
            egui::SidePanel::left("typefinch-tools")
                .resizable(true)
                .default_width(295.0)
                .min_width(255.0)
                .max_width(370.0)
                .frame(egui::Frame::default().inner_margin(egui::Margin::same(18)).fill(ctx.style().visuals.panel_fill))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(6.0);
                        self.show_profile(ui);
                        ui.add_space(17.0);
                        ui.separator();
                        ui.add_space(12.0);
                        self.show_settings(ui, ctx);
                        ui.add_space(17.0);
                        ui.separator();
                        ui.add_space(12.0);
                        self.show_devices(ui);
                        ui.add_space(14.0);
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(24)).fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if !wide {
                        egui::CollapsingHeader::new("Profiles, appearance and keyboard settings")
                            .default_open(true).show(ui, |ui| {
                                self.show_profile(ui);
                                ui.add_space(12.0);
                                self.show_settings(ui, ctx);
                                ui.add_space(12.0);
                                self.show_devices(ui);
                            });
                        ui.add_space(18.0);
                    }
                    ui.add_space(8.0);
                    self.show_test(ui, ctx);
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(16.0);
                    self.show_history(ui);
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(16.0);
                    self.show_export(ui);
                    if !self.message.is_empty() {
                        ui.add_space(12.0);
                        ui.colored_label(PERIWINKLE, &self.message);
                    }
                    ui.add_space(24.0);
                });
            });
    }
}
