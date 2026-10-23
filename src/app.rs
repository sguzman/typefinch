use crate::{
    devices::{self, Keyboard},
    engine::{Session, State, TestMode},
    export,
    history::{self, Record},
    lesson,
    layouts::{self, Mode},
    preferences::{Preferences, Profile, Theme},
    registry::{self, InstalledLayout},
    xkb::Emulator,
};
use eframe::egui::{self, Color32, RichText};
use std::time::Duration;

const PERIWINKLE: Color32 = Color32::from_rgb(134, 150, 245);
// Wayland normally combines physical keyboards into a seat-level event stream.
// Only tag OS input with a detected layout when all reported keyboards agree;
// selecting a device for inspection does NOT attribute its events to it.
fn recorded_layout(mode:Mode, target:&str, keyboards:&[Keyboard], installed:&[InstalledLayout])->String {
    if mode == Mode::Emulated { return target.to_owned(); }
    let Some(first)=keyboards.first() else { return "os:unknown".to_owned() };
    if first.active_keymap.trim().is_empty() ||
        keyboards.iter().any(|k|k.active_keymap != first.active_keymap) {
        return "os:unknown".to_owned();
    }
    match registry::resolve_active(installed,&first.active_keymap) {
        Some(layout)=>format!("detected/{}",layout.id),
        None=>"os:unknown".to_owned(),
    }
}

// The brand swatch is too light for text on white. Use theme-specific ink.
fn accent_ink(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(170,181,255) }
    else { Color32::from_rgb(69,89,181) }
}
fn correct_ink(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(124,213,165) }
    else { Color32::from_rgb(39,114,73) }
}
fn error_ink(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(255,136,148) }
    else { Color32::from_rgb(177,47,70) }
}


pub struct Typefinch {
    prefs: Preferences,
    session: Session,
    mode: Mode,
    session_layout: String,
    session_input_mode: String,
    emulator: Option<Emulator>,
    installed_layouts: Vec<InstalledLayout>,
    layout_search: String,
    history: Vec<Record>,
    completed_lessons: std::collections::HashSet<String>,
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
            s.visuals.hyperlink_color = accent_ink(s.visuals.dark_mode);
        });
        let (keyboards, device_error) = Self::query_devices();
        let input = if prefs.test_mode == TestMode::Lesson {lesson::get(&prefs.lesson_id).passage} else {""};
        let duration = if prefs.test_mode == TestMode::Lesson {0} else {prefs.duration_seconds};
        let session = Session::new_with_custom(duration, prefs.test_mode, input);
        let emulator = Emulator::compile(&prefs.layout).ok();
        let installed_layouts = registry::discover().unwrap_or_default();
        let history = history::recent(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        let completed_lessons = history::completed_lessons(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        Self { prefs, session, mode: Mode::System, session_layout: "os:unknown".into(), session_input_mode: "OS".into(), emulator, installed_layouts, layout_search: String::new(), history, completed_lessons, keyboards, device_error,
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
        let mut record = Record::completed(&identity, &self.session_layout, &self.session_input_mode, &self.session);
        if self.session.mode==TestMode::Lesson {
            record.test_mode=format!("lesson/{}", self.prefs.lesson_id);
        }
        match history::insert(&record) {
            Ok(_) => {
                if self.session.mode==TestMode::Lesson {
                    if history::mastered_lesson(&record) {
                        self.completed_lessons.insert(self.prefs.lesson_id.clone());
                        self.message = format!("Lesson mastered: {}. You can continue to the next lesson.",
                            lesson::get(&self.prefs.lesson_id).name);
                    } else {
                        self.message = "Practice saved. Correct every character to master this lesson. Retry when ready.".into();
                    }
                }
                match history::recent(&identity) {
                    Ok(records) => self.history = records,
                    Err(e) => self.message = format!("History read failed: {e}"),
                }
            },
            Err(e) => self.message = format!("History not saved: {e}"),
        }
        let profile = &mut self.prefs.profiles[self.prefs.active_profile];
        profile.tests = profile.tests.saturating_add(1);
        if self.session.mode.counts_for_benchmark() {
            profile.best_wpm = profile.best_wpm.max(result.wpm);
            profile.best_accuracy = profile.best_accuracy.max(result.accuracy);
        }
        let completed_tests = profile.tests;
        self.save();
        if let Err(err) = crate::qa::write_smoke_report(&self.session, completed_tests) {
            self.message = format!("QA report failed: {err}");
        }
    }
    fn rebuild_session(&mut self) {
        let input=if self.prefs.test_mode==TestMode::Lesson {
            lesson::get(&self.prefs.lesson_id).passage
        } else { &self.custom_text };
        let duration=if self.prefs.test_mode==TestMode::Lesson {0} else {self.prefs.duration_seconds};
        self.session = Session::new_with_custom(duration, self.prefs.test_mode, input);
    }
    fn start_test(&mut self, ctx: &egui::Context) {
        if self.session.target.is_empty() {
            self.message = "Enter some custom text before starting this test.".into();
            return;
        }
        self.session_layout = recorded_layout(self.mode,&self.prefs.layout,
            &self.keyboards,&self.installed_layouts);
        self.session_input_mode = if self.mode == Mode::System { "OS".into() } else { "Emulated".into() };
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
            self.completed_lessons = history::completed_lessons(&id).unwrap_or_default();
            self.save();
        }
        let p = &self.prefs.profiles[self.prefs.active_profile];
        ui.label(format!("{} sessions · benchmark best {:.1} WPM · {:.1}% accuracy",
            p.tests, p.best_wpm, p.best_accuracy));
        ui.add_space(9.0);
        ui.label(RichText::new("Personal goals").strong());
        let mut changed=false;
        ui.add_enabled_ui(self.session.state!=State::Running,|ui| {
            let goals=&mut self.prefs.profiles[self.prefs.active_profile].goals;
            changed |= ui.add(egui::Slider::new(&mut goals.target_wpm,10.0..=250.0)
                .text("WPM target")).changed();
            changed |= ui.add(egui::Slider::new(&mut goals.target_accuracy,70.0..=100.0)
                .text("Accuracy target")).changed();
        });
        if changed { self.save(); }
        let current=&self.prefs.profiles[self.prefs.active_profile];
        let goals=&current.goals;
        ui.label(format!("Speed: {:.0} / {:.0} WPM",current.best_wpm,goals.target_wpm));
        ui.add(egui::ProgressBar::new(goals.wpm_progress(current.best_wpm)).show_percentage());
        ui.label(format!("Accuracy: {:.1}% / {:.1}%",current.best_accuracy,goals.target_accuracy));
        ui.add(egui::ProgressBar::new(goals.accuracy_progress(current.best_accuracy)).show_percentage());
        ui.small("Goals and progress belong to the selected local profile.");
    }
    fn show_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Test setup");
        ui.vertical(|ui| {
            ui.label("Duration");
            let previous = self.prefs.duration_seconds;
            ui.add_enabled_ui(self.prefs.test_mode!=TestMode::Lesson, |ui| {
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
            });
            if self.prefs.test_mode==TestMode::Lesson { ui.small("Untimed: finish the passage to complete the lesson."); }
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
            if self.prefs.test_mode == TestMode::Lesson {
                ui.add_space(6.0);
                ui.label("Lesson");
                let total=lesson::LESSONS.len().max(1);
                let completed=lesson::LESSONS.iter().filter(|l|self.completed_lessons.contains(l.id)).count();
                ui.add(egui::ProgressBar::new(completed as f32 / total as f32)
                    .text(format!("{completed}/{total} lessons completed")).show_percentage());
                let before=self.prefs.lesson_id.clone();
                ui.add_enabled_ui(self.session.state != State::Running, |ui| {
                    egui::ComboBox::from_id_salt("lesson").selected_text(lesson::get(&self.prefs.lesson_id).name)
                        .show_ui(ui, |ui| {
                            for l in lesson::LESSONS {
                                let mark=if self.completed_lessons.contains(l.id) {"Completed"} else {"New"};
                                ui.selectable_value(&mut self.prefs.lesson_id,l.id.to_owned(),format!("{} · {} · {}",mark,l.level,l.name));
                            }
                        });
                });
                if before!=self.prefs.lesson_id {
                    self.rebuild_session();
                    self.save();
                }
                ui.small(lesson::get(&self.prefs.lesson_id).hint);
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
        ui.add_enabled_ui(self.session.state != State::Running, |ui| {
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
        });
        ui.add(egui::TextEdit::singleline(&mut self.layout_search).hint_text("Search installed layouts / variants").desired_width(285.0));
        ui.small(format!("{} system XKB layouts and variants discovered · emulation compiled: {}",
            self.installed_layouts.len(), if self.emulator.is_some() { "yes" } else { "no" }));
        ui.small("OS input follows the active system layout; this selection controls the emulation target.");
        if self.mode==Mode::System {
            let inferred=recorded_layout(self.mode,&self.prefs.layout,&self.keyboards,&self.installed_layouts);
            ui.small(format!("Scoring layout attribution: {inferred} (seat-level, not per-keyboard)"));
        }
        if self.session.state == State::Running { ui.small("Layout and input mode are locked while a test is running."); }
        if self.mode == Mode::Emulated {
            ui.colored_label(accent_ink(ui.visuals().dark_mode), "Emulation uses physical ANSI key positions inside Typefinch only; it never changes your system layout.");
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
                State::Finished if self.session.mode == TestMode::Lesson =>
                    if self.session.score().accuracy >= 99.999 { "Lesson mastered!" }
                    else { "Passage finished · retry to master at 100% final accuracy" },
                State::Finished=>"Finished · result saved to this local profile",
            });
        });
        let score = self.session.score();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{:.0} WPM", score.wpm)).size(23.0).color(accent_ink(ui.visuals().dark_mode)).strong());
            ui.label(format!("{:.1}% accuracy", score.accuracy));
            ui.label(format!("{:.0} raw WPM", score.raw_wpm));
            ui.label(format!("{:.1}% raw accuracy", score.raw_accuracy));
            ui.label(format!("{} mistakes · {} corrections", score.mistakes, self.session.corrections));
            ui.label(format!("{} chars · {:.1} s elapsed", score.entered, score.elapsed.as_secs_f64()));
            if self.session.duration.is_zero() {
                ui.label("Untimed practice");
            } else {
                ui.label(format!("{:.1} s left", self.session.seconds_left()));
            }
        });
        ui.add(egui::ProgressBar::new(
            if self.session.duration.is_zero() {
                let total=self.session.target.chars().count().max(1);
                self.session.typed.chars().count() as f32 / total as f32
            } else {
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
                Some(c) if c == ch => correct_ink(ui.visuals().dark_mode),
                Some(_) => error_ink(ui.visuals().dark_mode),
                None if position == entered && self.session.state == State::Running => accent_ink(ui.visuals().dark_mode),
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
    fn show_layout_preview(&self, ui: &mut egui::Ui) {
        // This is a map of the selected typing target, not a device-level OS
        // keyboard reading. An unknown key is shown explicitly, never guessed.
        const ROWS:[[egui::Key;10];3] = [
            [egui::Key::Q,egui::Key::W,egui::Key::E,egui::Key::R,egui::Key::T,
             egui::Key::Y,egui::Key::U,egui::Key::I,egui::Key::O,egui::Key::P],
            [egui::Key::A,egui::Key::S,egui::Key::D,egui::Key::F,egui::Key::G,
             egui::Key::H,egui::Key::J,egui::Key::K,egui::Key::L,egui::Key::Semicolon],
            [egui::Key::Z,egui::Key::X,egui::Key::C,egui::Key::V,egui::Key::B,
             egui::Key::N,egui::Key::M,egui::Key::Comma,egui::Key::Period,egui::Key::Slash],
        ];
        ui.heading("Keyboard mapping preview");
        ui.small(format!("Selected target: {} · ANSI base positions · {}",
            self.prefs.layout,
            if self.emulator.is_some() {"installed XKB symbols"} else {"built-in map where available"}));
        ui.add_space(6.0);
        for (row_index, keys) in ROWS.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space((row_index*12) as f32);
                for key in keys {
                    let value = self.emulator.as_ref().and_then(|x|x.translate(*key,false))
                        .or_else(||layouts::emulate(*key,&self.prefs.layout,false).map(|c|c.to_string()))
                        .unwrap_or_else(||"–".into());
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(7,7))
                        .show(ui,|ui| {
                            ui.add_sized([27.0,27.0],egui::Label::new(
                                RichText::new(value).monospace().size(15.0)
                            ).selectable(false));
                        });
                }
            });
            ui.add_space(2.0);
        }
        ui.small("Preview only. Actual OS input may differ; device-specific routing, ISO/JIS keys and composition remain separate work.");
    }
    fn show_history(&mut self, ui: &mut egui::Ui) {
        ui.heading("Recent tests");
        if self.history.is_empty() {
            ui.label("No completed scored tests yet. Results stay on this device.");
            return;
        }
        let mode=if self.mode==Mode::System { "OS" } else { "Emulated" };
        let identity=recorded_layout(self.mode,&self.prefs.layout,&self.keyboards,&self.installed_layouts);
        let comparable=history::comparable(&self.history, &identity,
            self.prefs.test_mode.key(), self.prefs.duration_seconds as i64*1000, mode);
        if comparable.len()>=2 {
            if let Some(stats)=history::trend(&comparable) {
                ui.label(format!("Comparable sessions: {} · Avg {:.1} WPM · Best {:.1} WPM · Std. dev. {:.1} WPM",
                    comparable.len(), stats.mean_wpm, stats.best_wpm, stats.deviation_wpm));
            }
            let h=146.0_f32;
            let (rect,_) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(160.0),h),egui::Sense::hover());
            let painter=ui.painter_at(rect);
            let area=rect.shrink2(egui::vec2(14.0,14.0));
            let low=comparable.iter().map(|r|r.wpm).fold(f64::MAX,f64::min).max(0.0);
            let high=comparable.iter().map(|r|r.wpm).fold(0.0_f64,f64::max);
            let bottom=(low-4.0).max(0.0);
            let top=high+4.0;
            painter.line_segment(
                [area.left_bottom(),area.right_bottom()],
                egui::Stroke::new(1.0,ui.visuals().weak_text_color())
            );
            let mut previous:Option<egui::Pos2>=None;
            for (index,result) in comparable.iter().enumerate() {
                let x=area.left()+area.width()*index as f32/(comparable.len()-1) as f32;
                let y=area.bottom()-area.height()*((result.wpm-bottom)/(top-bottom)) as f32;
                let point=egui::pos2(x,y);
                if let Some(last)=previous {
                    painter.line_segment([last,point],egui::Stroke::new(2.0,PERIWINKLE));
                }
                painter.circle_filled(point,3.0,PERIWINKLE);
                previous=Some(point);
            }
            ui.small("Trend: comparable layout, content, input mode, duration and scoring version only.");
        } else if self.prefs.test_mode==TestMode::Custom {
            ui.small("Custom passages are intentionally not ranked against different custom texts.");
        } else {
            ui.small("Complete two tests with the same layout, content, input mode and duration to see your trend.");
        }
        ui.add_space(10.0);
        ui.label("Latest sessions");
        egui::ScrollArea::horizontal().id_salt("recent-history-horizontal").show(ui, |ui| {
        egui::Grid::new("recent-tests").striped(true).min_col_width(75.0).show(ui, |ui| {
            ui.strong("WPM");
            ui.strong("Raw WPM");
            ui.strong("Content");
            ui.strong("Accuracy");
            ui.strong("Duration");
            ui.strong("Layout");
            ui.strong("Input");
            ui.end_row();
            for r in self.history.iter().take(8) {
                ui.label(format!("{:.1}", r.wpm));
                ui.label(if r.attempts>0 {format!("{:.1}",r.raw_wpm)} else {"—".to_owned()});
                ui.label(&r.test_mode);
                ui.label(format!("{:.1}%", r.accuracy));
                ui.label(format!("{}s", r.duration_ms / 1000));
                ui.label(&r.layout);
                ui.label(&r.input_mode);
                ui.end_row();
            }
        });
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
    fn export_statistics(&mut self) {
        let profile=&self.prefs.profiles[self.prefs.active_profile];
        self.message=match export::save_stats_json(profile) {
            Ok(path)=>format!("Exported all saved sessions: {}",path.display()),
            Err(err)=>format!("Statistics export failed: {err}"),
        };
    }
    fn show_export(&mut self, ui: &mut egui::Ui) {
        ui.heading("GitHub profile card");
        ui.label("Export a static SVG card and matching Markdown for your GitHub profile.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export SVG + Markdown").clicked() { self.export_widget(); }
            if ui.button("Export session history JSON").clicked() {self.export_statistics();}
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
        if ctx.input(|i| i.key_pressed(egui::Key::F7)) {
            self.export_statistics();
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
                    self.show_layout_preview(ui);
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
                        ui.colored_label(accent_ink(ui.visuals().dark_mode), &self.message);
                    }
                    ui.add_space(24.0);
                });
            });
    }
}

#[cfg(test)]
mod palette_tests {
    use super::*;
    fn brightness(c:Color32)->f64 {
        let v=[c.r(),c.g(),c.b()].map(|v|{
            let x=v as f64/255.0;
            if x<=0.04045 { x/12.92 } else { ((x+0.055)/1.055).powf(2.4) }
        });
        v[0]*0.2126 + v[1]*0.7152 + v[2]*0.0722
    }
    fn contrast(a:Color32,b:Color32)->f64 {
        let x=brightness(a);let y=brightness(b);
        (x.max(y)+0.05)/(x.min(y)+0.05)
    }
    #[test] fn keyboard_layout_attribution_never_uses_selected_inspection_device_as_input_source() {
        let xkb=vec![
            InstalledLayout { id:"us".into(),name:"English (US)".into() },
            InstalledLayout { id:"de".into(),name:"German".into() },
        ];
        let mut keyboards=vec![Keyboard {
            name:"A".into(),active_keymap:"English (US)".into(),..Default::default()
        }];
        assert_eq!(recorded_layout(Mode::System,"de",&keyboards,&xkb),"detected/us");
        assert_eq!(recorded_layout(Mode::Emulated,"us(colemak)",&keyboards,&xkb),"us(colemak)");
        keyboards.push(Keyboard {
            name:"B".into(),active_keymap:"German".into(),..Default::default()
        });
        assert_eq!(recorded_layout(Mode::System,"us",&keyboards,&xkb),"os:unknown");
        assert_eq!(recorded_layout(Mode::System,"us",&[],&xkb),"os:unknown");
    }
    #[test] fn brand_ink_and_feedback_meet_normal_text_contrast() {
        let light=Color32::WHITE;
        let dark=Color32::from_rgb(32,38,69);
        for ink in [accent_ink(false),correct_ink(false),error_ink(false)] {
            assert!(contrast(ink,light)>=4.5, "light foreground {ink:?}");
        }
        for ink in [accent_ink(true),correct_ink(true),error_ink(true)] {
            assert!(contrast(ink,dark)>=4.5, "dark foreground {ink:?}");
        }
        assert!(contrast(PERIWINKLE,light)<4.5); // Never use accent swatch as white-background text.
    }
}
