use crate::{
    devices::{self, Keyboard},
    engine::{Session, State},
    export,
    history::{self, Record},
    layouts::{self, Mode},
    preferences::{Preferences, Profile, Theme},
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
    history: Vec<Record>,
    keyboards: Vec<Keyboard>,
    device_index: usize,
    device_error: Option<String>,
    new_profile: String,
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
        let session = Session::new(prefs.duration_seconds);
        let emulator = Emulator::compile(&prefs.layout).ok();
        let history = history::recent(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        Self { prefs, session, mode: Mode::System, emulator, history, keyboards, device_error,
               device_index: 0, new_profile: String::new(), message: String::new() }
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
    fn start_test(&mut self, ctx: &egui::Context) {
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
        ui.heading("🐦  Your perch");
        let previous = self.prefs.active_profile;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("profile").selected_text(&self.prefs.profiles[self.prefs.active_profile].name)
                .show_ui(ui, |ui| {
                    for (i, profile) in self.prefs.profiles.iter().enumerate() {
                        ui.selectable_value(&mut self.prefs.active_profile, i, &profile.name);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut self.new_profile).hint_text("New local profile").desired_width(190.0));
            if ui.button("＋ Create").clicked() {
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
        ui.heading("🎛️  Test setup");
        ui.horizontal_wrapped(|ui| {
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
                    self.session = Session::new(self.prefs.duration_seconds);
                    self.save();
                }
            }
            ui.separator();
            ui.label("Theme");
            let previous = self.prefs.theme;
            egui::ComboBox::from_id_salt("theme").selected_text(match previous {
                Theme::System=>"System (default)", Theme::Light=>"Light", Theme::Dark=>"Dark",
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.prefs.theme, Theme::System, "System");
                ui.selectable_value(&mut self.prefs.theme, Theme::Light, "☀ Light");
                ui.selectable_value(&mut self.prefs.theme, Theme::Dark, "🌙 Dark");
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
        ui.horizontal_wrapped(|ui| {
            ui.label("Keyboard layout");
            let last = self.prefs.layout.clone();
            egui::ComboBox::from_id_salt("layout").width(270.0)
                .selected_text(layouts::get(&self.prefs.layout).name)
                .show_ui(ui, |ui| {
                    for l in layouts::CATALOG {
                        let suffix = if l.rows.is_some() { " · built-in" } else { " · XKB if installed" };
                        ui.selectable_value(&mut self.prefs.layout, l.id.to_owned(), format!("{}{}", l.name, suffix));
                    }
                });
            if self.prefs.layout != last {
                self.emulator = Emulator::compile(&self.prefs.layout).ok();
                if self.emulator.is_none() && layouts::get(&self.prefs.layout).rows.is_none() {
                    self.mode = Mode::System;
                    self.message = "The selected XKB layout/variant is not installed; using OS input.".into();
                }
                self.save();
            }
            ui.separator();
            ui.selectable_value(&mut self.mode, Mode::System, "OS input");
            if self.emulator.is_some() || layouts::get(&self.prefs.layout).rows.is_some() {
                ui.selectable_value(&mut self.mode, Mode::Emulated, "Software emulation");
            } else {
                ui.add_enabled(false, egui::Button::new("Software emulation (not available)"));
            }
        });
        ui.small(format!("Layout family: {} · XKB compiled: {}", layouts::get(&self.prefs.layout).region, if self.emulator.is_some() { "yes" } else { "no" }));
        if self.mode == Mode::Emulated {
            ui.colored_label(PERIWINKLE, "Emulation uses physical ANSI key positions inside Typefinch only; it never changes your system layout.");
            ui.small(if self.emulator.is_some() { "Compiled using installed XKB symbols; physical-key mapping covers the initial ANSI positions. Compose, AltGr, and IME are pending." } else { "Using starter ANSI table; physical key events required. Compose, AltGr, and IME are pending." });
        }
    }
    fn show_devices(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔌  Connected keyboards");
        ui.horizontal_wrapped(|ui| {
            if ui.button("↻ Refresh").clicked() {
                (self.keyboards, self.device_error) = Self::query_devices();
                self.device_index = 0;
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
                ui.label(format!("Active map: {} · configured: {} {}", k.active_keymap, k.layout, k.variant));
                ui.small(format!("Device: {}", k.address));
            } else {
                ui.label(self.device_error.as_deref().unwrap_or("No keyboards listed by Hyprland."));
            }
        });
        ui.small("This selector inspects keyboard metadata. Ordinary Wayland typing events are seat-level: this is not yet a per-device input filter.");
    }
    fn show_test(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("⌨️  Typing test");
        ui.horizontal_wrapped(|ui| {
            let name = if self.session.state == State::Running { "↻ Restart" } else { "▶ Start test" };
            if ui.button(name).clicked() { self.start_test(ctx); }
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
            ui.label(format!("{} corrections", self.session.corrections));
            ui.label(format!("{} chars · {:.1} s elapsed", score.entered, score.elapsed.as_secs_f64()));
            ui.label(format!("{:.1} s left", self.session.seconds_left()));
        });
        ui.add(egui::ProgressBar::new(
            if self.session.duration.is_zero() { 0.0 } else {
                (1.0 - self.session.seconds_left() / self.session.duration.as_secs_f64()) as f32
            }
        ).show_percentage());
        ui.add_space(8.0);
        let expected: Vec<char> = self.session.target.chars().take(230).collect();
        let actual: Vec<char> = self.session.typed.chars().collect();
        let mut job = egui::text::LayoutJob::default();
        for (i, ch) in expected.iter().enumerate() {
            let color = match actual.get(i) {
                Some(c) if c == ch => Color32::from_rgb(91, 184, 137),
                Some(_) => Color32::from_rgb(239, 102, 112),
                None if i == actual.len() && self.session.state == State::Running => PERIWINKLE,
                None => ui.visuals().weak_text_color(),
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
                ui.small("Click here to refocus typing, or press F5 to start/restart.");
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
        ui.heading("📊  Recent tests");
        if self.history.is_empty() {
            ui.label("No completed scored tests yet. Results stay on this device.");
            return;
        }
        egui::Grid::new("recent-tests").striped(true).min_col_width(75.0).show(ui, |ui| {
            ui.strong("WPM");
            ui.strong("Accuracy");
            ui.strong("Duration");
            ui.strong("Layout");
            ui.strong("Input");
            ui.end_row();
            for r in &self.history {
                ui.label(format!("{:.1}", r.wpm));
                ui.label(format!("{:.1}%", r.accuracy));
                ui.label(format!("{}s", r.duration_ms / 1000));
                ui.label(&r.layout);
                ui.label(&r.input_mode);
                ui.end_row();
            }
        });
    }
    fn show_export(&mut self, ui: &mut egui::Ui) {
        ui.heading("🎨  GitHub card");
        ui.label("Static SVG snapshot, no scripts or hosting required. Share it in a profile README.");
        if ui.button("↓ Export SVG").clicked() {
            let p = &self.prefs.profiles[self.prefs.active_profile];
            self.message = match export::save_card(p, layouts::get(&self.prefs.layout).name) {
                Ok(path) => format!("Exported: {}", path.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }
}
impl eframe::App for Typefinch {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.start_test(ctx);
        }
        let before = self.session.state;
        self.apply_events(ctx);
        self.session.tick();
        if before == State::Running && self.session.state == State::Finished { self.finish(); }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("🐦 Typefinch").size(31.0).color(PERIWINKLE));
                    ui.label("Find your rhythm.");
                });
                ui.small("Local-first typing studio · early developer preview");
                ui.separator();
                self.show_profile(ui);
                ui.separator();
                self.show_settings(ui, ctx);
                ui.separator();
                self.show_test(ui, ctx);
                ui.separator();
                self.show_history(ui);
                ui.separator();
                self.show_devices(ui);
                ui.separator();
                self.show_export(ui);
                if !self.message.is_empty() {
                    ui.add_space(9.0);
                    ui.colored_label(PERIWINKLE, &self.message);
                }
                ui.add_space(14.0);
            });
        });
    }
}
