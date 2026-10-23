use crate::{
    achievements,
    custom_layout::CustomLayout,
    daily,
    devices::{self, Keyboard},
    engine::{Language, Session, State, TestMode, WordDifficulty},
    export,
    history::{self, Record},
    lesson,
    layouts::{self, Mode},
    preferences::{Preferences, Profile, Theme, CardTheme, CardFont,CardTemplate},
    registry::{self, InstalledLayout},
    xkb::Emulator,
};
use eframe::egui::{self, Color32, RichText};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

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
/// Untyped reference characters are instructions, not disabled controls:
/// keep them readable in either theme instead of fading them below 4.5:1.
fn reference_ink(dark:bool)->Color32 {
    if dark {Color32::from_rgb(185,190,205)}
    else {Color32::from_rgb(85,85,92)}
}
/// In-session key heatmap fills deliberately keep enough contrast against
/// error_ink so count labels and glyphs are legible in light and dark themes.
fn key_heat_fill(errors:u32,dark:bool)->Color32 {
    if dark {
        match errors {
            0=>Color32::from_rgb(35,39,60),
            1=>Color32::from_rgb(59,36,52),
            2|3=>Color32::from_rgb(75,35,53),
            _=>Color32::from_rgb(91,37,55),
        }
    } else {
        match errors {
            0=>Color32::from_rgb(247,247,252),
            1=>Color32::from_rgb(255,235,237),
            2|3=>Color32::from_rgb(255,219,223),
            _=>Color32::from_rgb(255,213,219),
        }
    }
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
    layout_draft:Option<CustomLayout>,
    layout_import_path:String,
    history: Vec<Record>,
    earned_badges: Vec<achievements::Achievement>,
    all_history:Vec<Record>,
    history_only_matching: bool,
    history_metric: history::HistoryMetric,
    completed_lessons: std::collections::HashSet<String>,
    keyboards: Vec<Keyboard>,
    device_index: usize,
    last_window_focused:bool,
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
        let session = Session::new_with_settings(duration, prefs.test_mode, input, prefs.language, prefs.word_difficulty);
        let emulator = Emulator::compile(&prefs.layout).ok();
        let initial_mode=if prefs.custom_layouts.iter().any(|l|l.id==prefs.layout && l.validate().is_ok()) {Mode::Emulated} else {Mode::System};
        let installed_layouts = registry::discover().unwrap_or_default();
        let history = history::recent(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        let completed_lessons = history::completed_lessons(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        let all_history=history::all_for_profile(&prefs.profiles[prefs.active_profile].id).unwrap_or_default();
        let earned_badges=achievements::earned(&all_history);
        Self { prefs, session, mode: initial_mode, session_layout: "os:unknown".into(), session_input_mode: "OS".into(), emulator, installed_layouts, layout_search: String::new(), layout_draft:None, layout_import_path:String::new(), history, history_only_matching: false, history_metric: history::HistoryMetric::CorrectedWpm,
            earned_badges, all_history, completed_lessons, keyboards, device_error,
               device_index: 0, last_window_focused:true, new_profile: String::new(), custom_text: String::new(), message: String::new() }
    }
    fn refresh_keyboards(&mut self) {
        let selected=self.keyboards.get(self.device_index).map(|k|k.address.clone());
        let (updated,error)=Self::query_devices();
        self.device_index=devices::preserve_selection(selected.as_deref(),&updated);
        self.keyboards=updated;
        self.device_error=error;
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
        let prior=self.earned_badges.clone();
        if let Ok(records)=history::all_for_profile(&identity) {
            self.all_history=records;
            self.earned_badges=achievements::earned(&self.all_history);
            if let Some(new_badge)=self.earned_badges.iter().find(|a|!prior.contains(a)) {
                self.message=format!("{} New achievement: {}! {}",new_badge.emoji,new_badge.title,new_badge.reason);
            }
        }
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
        self.session = Session::new_with_settings(duration, self.prefs.test_mode, input, self.prefs.language, self.prefs.word_difficulty);
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
                        let mapped = if let Some(personal)=self.prefs.custom_layouts.iter()
                            .find(|l|l.id==self.prefs.layout) {
                            personal.translate(physical,modifiers.shift)
                        } else if let Some(emulator) = &self.emulator {
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
    fn accept_custom_file(&mut self, ctx:&egui::Context) {
        if self.prefs.test_mode!=TestMode::Custom || self.session.state==State::Running {return;}
        let files=ctx.input(|i|i.raw.dropped_files.clone());
        let Some(drop)=files.into_iter().next() else {return;};
        let Some(path)=drop.path else {
            self.message="Drop a local file with a readable filesystem path.".into();
            return;
        };
        let imported=(|| -> Result<String,String> {
            use std::io::Read;
            let input=std::fs::File::open(&path).map_err(|e|e.to_string())?;
            // Read one extra byte for strict size detection, never the entire
            // contents of a potentially huge file.
            let mut data=Vec::new();
            input.take(65_537).read_to_end(&mut data).map_err(|e|e.to_string())?;
            crate::engine::import_custom_passage(&data)
        })();
        match imported {
            Ok(passage)=>{
                let count=passage.graphemes(true).count();
                self.custom_text=passage;
                self.rebuild_session();
                self.message=format!("Loaded {count} graphemes into memory. The passage will not be saved.");
            },
            Err(reason)=>self.message=format!("Custom text import rejected: {reason}"),
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
            self.all_history=history::all_for_profile(&id).unwrap_or_default();
            self.earned_badges=achievements::earned(&self.all_history);
            self.save();
        }
        let p = &self.prefs.profiles[self.prefs.active_profile];
        ui.label(format!("{} sessions · overall benchmark best {:.1} WPM · {:.1}% accuracy",
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
            changed |= ui.add(egui::Slider::new(&mut goals.daily_sessions,1..=10)
                .text("Daily benchmark goal")).changed();
            changed |= ui.checkbox(&mut goals.show_streak,"Show practice streak (optional)").changed();
        });
        if changed { self.save(); }
        let current=&self.prefs.profiles[self.prefs.active_profile];
        let goals=&current.goals;
        ui.label(format!("Speed: {:.0} / {:.0} WPM",current.best_wpm,goals.target_wpm));
        ui.add(egui::ProgressBar::new(goals.wpm_progress(current.best_wpm)).show_percentage());
        ui.label(format!("Accuracy: {:.1}% / {:.1}%",current.best_accuracy,goals.target_accuracy));
        ui.add(egui::ProgressBar::new(goals.accuracy_progress(current.best_accuracy)).show_percentage());
        let now=chrono::Local::now();
        let today=daily::summary(&self.all_history,now.date_naive(),&chrono::Local);
        ui.add_space(8.0);
        ui.label(RichText::new(format!("Today: {} / {} benchmark sessions",today.sessions,goals.daily_sessions)).strong());
        ui.add(egui::ProgressBar::new(
            (today.sessions as f32/goals.daily_sessions.max(1) as f32).clamp(0.0,1.0)
        ).show_percentage());
        ui.small(format!("{} typed graphemes · {:.1}% weighted accuracy · Local calendar date",
            today.graphemes,today.weighted_accuracy));
        ui.small("Only timed benchmarks of at least 15 s and 20 graphemes count. Practice and custom text are excluded.");
        if goals.show_streak {
            let days=daily::streak(&self.all_history,now.date_naive(),&chrono::Local);
            ui.label(format!("🪶 Local practice streak: {days} day{}",if days==1 {""} else {"s"}));
            ui.small("Includes yesterday while today is still unfinished. Turn off above to hide this display.");
        }
        ui.small("Goals and progress belong to the selected local profile.");
    }
    fn show_custom_layout_editor(&mut self,ui:&mut egui::Ui) {
        egui::CollapsingHeader::new("🛠 Personal ANSI keymaps")
            .default_open(false).show(ui,|ui| {
                let enabled=self.session.state!=State::Running;
                ui.small("Personal layouts remap physical ANSI positions inside Typefinch only.");
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(enabled,egui::Button::new("New mapping")).clicked() {
                        self.layout_draft=Some(CustomLayout::starter());
                    }
                    let current=self.prefs.custom_layouts.iter()
                        .find(|l|l.id==self.prefs.layout).cloned();
                    if ui.add_enabled(enabled && current.is_some(),egui::Button::new("Edit selected")).clicked() {
                        self.layout_draft=current;
                    }
                });
                let mut save=false;
                let mut cancel=false;
                if let Some(draft)=self.layout_draft.as_mut() {
                    ui.add_enabled_ui(enabled,|ui| {
                        ui.label("Name (1–32 graphemes)");
                        ui.add(egui::TextEdit::singleline(&mut draft.name).desired_width(240.0));
                        for i in 0..3 {
                            ui.label(match i {0=>"Top row",1=>"Home row",_=>"Bottom row"});
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Base");
                                ui.add(egui::TextEdit::singleline(&mut draft.base[i]).desired_width(165.0));
                                ui.label("Shift");
                                ui.add(egui::TextEdit::singleline(&mut draft.shifted[i]).desired_width(165.0));
                            });
                        }
                        ui.small("Exactly 10 grapheme mappings per row. No global remapping.");
                        if ui.button("Validate, save and activate").clicked() {save=true;}
                        if ui.button("Cancel editing").clicked() {cancel=true;}
                    });
                }
                if cancel {self.layout_draft=None;}
                if save {
                    if let Some(draft)=self.layout_draft.clone() {
                        match draft.validate() {
                            Err(e)=>self.message=format!("Invalid keymap: {e}"),
                            Ok(())=>{
                                if let Some(old)=self.prefs.custom_layouts.iter_mut()
                                    .find(|l|l.id==draft.id) {
                                    *old=draft.clone();
                                } else if self.prefs.custom_layouts.len()<32 {
                                    self.prefs.custom_layouts.push(draft.clone());
                                } else {
                                    self.message="Maximum of 32 personal keymaps reached.".into();
                                    return;
                                }
                                self.prefs.layout=draft.id;
                                self.emulator=None;
                                self.mode=Mode::Emulated;
                                self.layout_draft=None;
                                self.save();
                                self.message="Personal ANSI keymap saved and activated inside Typefinch.".into();
                            }
                        }
                    }
                }
            });
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
            ui.add_space(6.0);
            ui.label("Typing language");
            let previous_language=self.prefs.language;
            ui.add_enabled_ui(self.session.state != State::Running, |ui| {
                egui::ComboBox::from_id_salt("language")
                    .selected_text(self.prefs.language.label())
                    .show_ui(ui, |ui| {
                        for locale in Language::ALL {
                            ui.selectable_value(&mut self.prefs.language,locale,locale.label());
                        }
                    });
            });
            if self.prefs.language!=previous_language {
                self.rebuild_session();
                self.save();
            }
            if matches!(self.prefs.test_mode,TestMode::Custom|TestMode::Lesson) {
                ui.small("Custom passages use your text; starter lessons are English-only. The language preference is not applied to these modes.");
            } else if self.prefs.test_mode==TestMode::Numbers {
                ui.small("Number passages are language-neutral, but their results remain separated by selected language.");
            } else {
                ui.small("Each supported language has its own original starter passage.");
            }
            if self.prefs.test_mode == TestMode::Words {
                ui.add_space(6.0);
                ui.label("Word difficulty");
                let previous=self.prefs.word_difficulty;
                ui.add_enabled_ui(self.session.state!=State::Running, |ui| {
                    egui::ComboBox::from_id_salt("word-difficulty")
                        .selected_text(previous.label())
                        .show_ui(ui, |ui| {
                            for difficulty in WordDifficulty::ALL {
                                ui.selectable_value(&mut self.prefs.word_difficulty,difficulty,difficulty.label());
                            }
                        });
                });
                if self.prefs.word_difficulty!=previous {
                    self.rebuild_session();
                    self.save();
                }
                ui.small("Starter tiers use curated word length and accent variety, not measured typing difficulty.");
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
                ui.small("Drop a UTF-8 .txt or .md file onto this window to load up to 4096 graphemes.");
                let edit = ui.add_enabled(
                    self.session.state != State::Running,
                    egui::TextEdit::multiline(&mut self.custom_text)
                        .desired_rows(4)
                        .desired_width(ui.available_width())
                        .hint_text("Paste or write text for your private typing test")
                );
                if edit.changed() { self.rebuild_session(); }
                ui.small(format!("{} graphemes entered; only the first 4096 will be used. Text is not saved.", self.custom_text.graphemes(true).count()));
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
                .or_else(||self.prefs.custom_layouts.iter().find(|l|l.id==self.prefs.layout).map(|l|l.name.as_str()))
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
                    for l in &self.prefs.custom_layouts {
                        if filter.is_empty() || format!("{} {}",l.name,l.id).to_lowercase().contains(&filter) {
                            ui.selectable_value(&mut self.prefs.layout,l.id.clone(),format!("{} · personal ANSI",l.name));
                        }
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
                if self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout) {
                    self.mode=Mode::Emulated;
                    self.message="Personal ANSI emulation selected inside Typefinch; the OS layout remains unchanged.".into();
                } else if self.emulator.is_none() && !layouts::find(&self.prefs.layout).is_some_and(|l|l.rows.is_some()) {
                    self.mode = Mode::System;
                    self.message = "The selected XKB layout/variant is not installed; using OS input.".into();
                }
                self.save();
            }
            ui.separator();
            ui.selectable_value(&mut self.mode, Mode::System, "OS input");
            if self.emulator.is_some() || layouts::find(&self.prefs.layout).is_some_and(|l|l.rows.is_some())
                || self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout) {
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
        self.show_custom_layout_editor(ui);
        if self.session.state == State::Running { ui.small("Layout and input mode are locked while a test is running."); }
        if self.mode == Mode::Emulated {
            ui.colored_label(accent_ink(ui.visuals().dark_mode), "Emulation uses physical ANSI key positions inside Typefinch only; it never changes your system layout.");
            ui.small(if self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout) {
                "Personal ANSI mapping uses physical-key events. It cannot change your system layout; Compose/AltGr/IME are not supported."
            } else if self.emulator.is_some() {
                "Compiled using installed XKB symbols; initial physical ANSI keys supported. Compose, AltGr, IME pending."
            } else {
                "Using starter ANSI table; physical-key events required. Compose, AltGr, IME pending."
            });
        }
    }
    fn show_devices(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connected keyboards");
        ui.small("Read-only Hyprland enumeration: updates when this window regains focus or you select Refresh. Event input remains seat-level.");
        ui.vertical(|ui| {
            if ui.button("Refresh device list").clicked() {
                self.refresh_keyboards();
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
    fn start_weak_drill(&mut self, drill:String, ctx:&egui::Context) {
        self.custom_text=drill;
        self.prefs.test_mode=TestMode::Custom;
        self.rebuild_session();
        self.save();
        self.start_test(ctx);
        self.message="Practice drill started using missed reference keys. Text stays in memory only.".into();
    }
    fn show_test(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading(RichText::new("Typing test").size(24.0).strong());
        ui.horizontal_wrapped(|ui| {
            let name = if self.session.state == State::Running { "Restart test" } else { "Start test" };
            if ui.add_enabled(!self.session.target.is_empty(), egui::Button::new(name)).clicked() { self.start_test(ctx); }
            if self.session.state == State::Running && ui.button("Stop").clicked() { self.session.state=State::Ready; }
            if self.session.state == State::Finished {
                if let Some(drill)=self.session.weak_character_drill() {
                    if ui.button("Practice missed keys").clicked() {
                        self.start_weak_drill(drill,ctx);
                    }
                }
            }
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
        let entered = self.session.typed.graphemes(true).count();
        let total = self.session.target.graphemes(true).count();
        let (first, last) = crate::engine::viewport(total, entered, 230);
        let expected:Vec<&str> = self.session.target.graphemes(true).skip(first).take(last-first).collect();
        let actual:Vec<&str> = self.session.typed.graphemes(true).skip(first).collect();
        let mut job = egui::text::LayoutJob::default();
        for (i, ch) in expected.iter().enumerate() {
            let position = first + i;
            let color = match actual.get(i) {
                Some(c) if crate::engine::canonically_equal(c,ch) => correct_ink(ui.visuals().dark_mode),
                Some(_) => error_ink(ui.visuals().dark_mode),
                None if position == entered && self.session.state == State::Running => accent_ink(ui.visuals().dark_mode),
                None => reference_ink(ui.visuals().dark_mode),
            };
            job.append(ch, 0.0, egui::TextFormat {
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
                ui.small(format!("Reference graphemes {}–{} of {} · Click here to refocus or press F5 to restart.", first.saturating_add(1), last, total));
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
            if self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout) {"personal key positions"}
            else if self.emulator.is_some() {"installed XKB symbols"} else {"built-in map where available"}));
        ui.add_space(6.0);
        for (row_index, keys) in ROWS.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space((row_index*12) as f32);
                for key in keys {
                    let value=self.prefs.custom_layouts.iter().find(|l|l.id==self.prefs.layout)
                        .and_then(|l|l.translate(*key,false))
                        .or_else(||self.emulator.as_ref().and_then(|x|x.translate(*key,false)))
                        .or_else(||layouts::emulate(*key,&self.prefs.layout,false).map(|c|c.to_string()))
                        .unwrap_or_else(||"–".into());
                    let missed=if value.chars().count()==1 {
                        value.chars().next().map(|c|self.session.missed_character_count(c)).unwrap_or(0)
                    } else {0};
                    let ink=if missed>0 {error_ink(ui.visuals().dark_mode)} else {ui.visuals().text_color()};
                    let mut frame=egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(7,7));
                    if missed>0 {
                        frame=frame.fill(key_heat_fill(missed,ui.visuals().dark_mode));
                    }
                    let cell=frame.show(ui,|ui| {
                            let finger=layouts::finger_hint(*key).unwrap_or("–");
                            ui.add_sized([34.0,43.0], egui::Label::new(
                                RichText::new(format!("{value}\n{finger}")).monospace().size(13.0).color(ink)
                            ).selectable(false));
                        });
                    if missed>0 {
                        cell.response.on_hover_text(format!("{missed} missed attempt(s) for this character in the current session"));
                    } else if let Some(ms)=value.chars().next().and_then(|ch|self.session.average_key_latency_ms(ch)) {
                        cell.response.on_hover_text(format!("{ms:.0} ms average interval for this character; session only"));
                    }
                }
            });
            ui.add_space(2.0);
        }
        ui.small("Finger hints: L/R = hand, 2 = index, 3 = middle, 4 = ring, 5 = pinky. Standard ANSI guidance only.");
        ui.small("In-session heatmap: darker/redder keycaps indicate more missed attempts. Hover for counts; nothing is stored.");
        let slow=self.session.slowest_keys();
        if !slow.is_empty() {
            ui.small("Slower single-key intervals (current session only; ≥2 samples each):");
            for (ch,ms) in slow.iter().take(5) {
                ui.label(format!("{} · {:.0} ms average between committed keys",ch,ms));
            }
        }
        ui.small("Preview only. Actual OS input may differ; device-specific routing, ISO/JIS keys and composition remain separate work.");
    }
    fn show_history(&mut self, ui: &mut egui::Ui) {
        ui.heading("Typing history");
        if self.history.is_empty() {
            ui.label("No completed scored tests yet. Results stay on this device.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.history_only_matching, "Only comparable sessions");
            egui::ComboBox::from_id_salt("history-metric")
                .selected_text(self.history_metric.label())
                .show_ui(ui, |ui| {
                    for metric in history::HistoryMetric::ALL {
                        ui.selectable_value(&mut self.history_metric,metric,metric.label());
                    }
                });
        });
        let input_mode=if self.mode==Mode::System {"OS"} else {"Emulated"};
        let identity=recorded_layout(self.mode,&self.prefs.layout,&self.keyboards,&self.installed_layouts);
        let comparable=history::comparable(&self.history, &identity,
            self.session.history_mode().as_str(), self.prefs.duration_seconds as i64*1000, input_mode);
        let measurements=comparable.iter()
            .filter_map(|r|self.history_metric.value(r).map(|v|(*r,v))).collect::<Vec<_>>();
        let values=measurements.iter().map(|(_,v)|*v).collect::<Vec<_>>();
        if values.len()>=2 {
            if let Some(summary)=history::metric_summary(&values) {
                ui.label(format!("{} matching sessions · {} · Avg {:.1} {} · Best {:.1} {} · Std. dev. {:.1} {}",
                    values.len(),self.history_metric.label(),
                    summary.mean,self.history_metric.unit(),summary.best,self.history_metric.unit(),
                    summary.deviation,self.history_metric.unit()));
            }
            let (rect,_) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(160.0),150.0),egui::Sense::hover());
            let painter=ui.painter_at(rect);
            let area=rect.shrink2(egui::vec2(16.0,16.0));
            let lowest=values.iter().copied().fold(f64::INFINITY,f64::min);
            let highest=values.iter().copied().fold(f64::NEG_INFINITY,f64::max);
            let padding=(highest-lowest).max(4.0)*0.12;
            let bottom=(lowest-padding).max(0.0);
            let top=highest+padding;
            painter.line_segment([area.left_bottom(),area.right_bottom()],
                egui::Stroke::new(1.0,ui.visuals().weak_text_color()));
            let mut last=None;
            for (index,(_,value)) in measurements.iter().enumerate() {
                let x=area.left()+area.width()*index as f32/(measurements.len()-1) as f32;
                let y=area.bottom()-area.height()*((value-bottom)/(top-bottom)) as f32;
                let point=egui::pos2(x,y);
                if let Some(prev)=last {
                    painter.line_segment([prev,point],egui::Stroke::new(2.0,PERIWINKLE));
                }
                painter.circle_filled(point,3.0,PERIWINKLE);
                last=Some(point);
            }
            ui.small("Oldest → newest · Same layout, language/content, input mode, duration and scoring version.");
        } else {
            ui.small("A trend requires two compatible sessions with available measurements. Unknown OS layouts and private passages are not compared.");
        }
        ui.add_space(10.0);
        let visible:Vec<&Record>=if self.history_only_matching {
            comparable.iter().rev().take(12).copied().collect()
        } else {
            self.history.iter().take(12).collect()
        };
        ui.label(format!("{} · {} displayed",if self.history_only_matching {"Matching history"} else {"Recent history"},visible.len()));
        if visible.is_empty() {
            ui.small("No compatible sessions for these settings. Turn off the filter to inspect all recent sessions.");
            return;
        }
        egui::ScrollArea::horizontal().id_salt("recent-history-horizontal").show(ui, |ui| {
            egui::Grid::new("recent-tests").striped(true).min_col_width(70.0).show(ui, |ui| {
                for heading in ["WPM","Raw WPM","Content","Accuracy","Raw accuracy","Duration","Layout","Input"] {
                    ui.strong(heading);
                }
                ui.end_row();
                for r in &visible {
                    ui.label(format!("{:.1}",r.wpm));
                    ui.label(if r.attempts>0 {format!("{:.1}",r.raw_wpm)} else {"—".to_owned()});
                    ui.label(&r.test_mode);
                    ui.label(format!("{:.1}%",r.accuracy));
                    ui.label(if r.attempts>0 {format!("{:.1}%",r.raw_accuracy)} else {"—".to_owned()});
                    ui.label(format!("{}s",r.duration_ms/1000));
                    ui.label(&r.layout);
                    ui.label(&r.input_mode);
                    ui.end_row();
                }
            });
        });
    }
    fn show_achievements(&self,ui:&mut egui::Ui) {
        egui::CollapsingHeader::new(format!("🏅 Achievements · {}/{}",self.earned_badges.len(),achievements::ALL.len()))
            .default_open(false)
            .show(ui,|ui| {
                ui.small("Earned from this local profile's saved aggregate history. No uploads or raw key records.");
                for badge in achievements::ALL {
                    let earned=self.earned_badges.contains(&badge);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(if earned {badge.emoji} else {"🔒"});
                        ui.label(RichText::new(badge.title).strong());
                        ui.small(badge.reason);
                    });
                }
            });
    }
    fn export_widget(&mut self) {
        let p = &self.prefs.profiles[self.prefs.active_profile];
        let display = self.installed_layouts.iter().find(|l|l.id==self.prefs.layout)
            .map(|l|l.name.as_str()).or_else(||layouts::find(&self.prefs.layout).map(|l|l.name))
            .unwrap_or("Custom XKB");
        self.message = match export::save_widget_with_template(p, display,self.prefs.card_theme,self.prefs.card_font,self.prefs.card_template) {
            Ok((svg, md)) => format!("Exported: {} and {}", svg.display(), md.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }
    fn export_png(&mut self) {
        let p=&self.prefs.profiles[self.prefs.active_profile];
        let display=self.installed_layouts.iter().find(|l|l.id==self.prefs.layout)
            .map(|l|l.name.as_str()).or_else(||layouts::find(&self.prefs.layout).map(|l|l.name))
            .unwrap_or("Custom XKB");
        self.message=match export::save_card_png_with_template(p,display,self.prefs.card_theme,self.prefs.card_font,self.prefs.card_template) {
            Ok(path)=>format!("PNG saved: {}",path.display()),
            Err(e)=>format!("PNG export failed: {e}"),
        };
    }
    fn export_statistics(&mut self) {
        let profile=&self.prefs.profiles[self.prefs.active_profile];
        self.message=match export::save_stats_json(profile) {
            Ok(path)=>format!("Exported all saved sessions: {}",path.display()),
            Err(err)=>format!("Statistics export failed: {err}"),
        };
    }
    fn export_full_backup(&mut self) {
        if let Err(e)=self.prefs.save() {
            self.message=format!("Full backup failed saving preferences: {e}");
            return;
        }
        self.message=match export::save_backup(&self.prefs) {
            Ok((settings, database))=>format!(
                "Private backup created: {} and {}. Keep both files together.",
                settings.display(), database.display()),
            Err(e)=>format!("Full backup failed: {e}"),
        };
    }
    fn show_card_preview(&self,ui:&mut egui::Ui) {
        let palette=export::palette(self.prefs.card_theme);
        let ink=|hex:&str| {
            let rgb=u32::from_str_radix(hex.trim_start_matches('#'),16).unwrap_or(0);
            Color32::from_rgb((rgb>>16) as u8,(rgb>>8) as u8,rgb as u8)
        };
        let compact=self.prefs.card_template==CardTemplate::Compact;
        let width=ui.available_width().min(550.0).max(260.0);
        let scale=width/550.0;
        let height=if compact {145.0} else {205.0};
        let (rect,_)=ui.allocate_exact_size(egui::vec2(width,height*scale),egui::Sense::hover());
        let painter=ui.painter_at(rect);
        let pos=|x:f32,y:f32|rect.min+egui::vec2(x*scale,y*scale);
        let font=|size:f32|if self.prefs.card_font==CardFont::Mono {
            egui::FontId::monospace(size*scale)
        } else {
            egui::FontId::proportional(size*scale)
        };
        painter.rect_filled(rect,22.0*scale,ink(palette.border));
        painter.rect_filled(rect.shrink(2.0*scale),20.0*scale,ink(palette.bg));
        let profile=&self.prefs.profiles[self.prefs.active_profile];
        let name=profile.name.chars().take(32).collect::<String>();
        let fields=[
            ("BEST WPM",format!("{:.0}",profile.best_wpm)),
            ("SESSIONS",profile.tests.to_string()),
            ("BEST ACCURACY",format!("{:.1}%",profile.best_accuracy)),
        ];
        if compact {
            painter.text(pos(23.0,12.0),egui::Align2::LEFT_TOP,"Typefinch",
                font(20.0),ink(palette.heading));
            painter.text(pos(155.0,12.0),egui::Align2::LEFT_TOP,name,
                font(16.0),ink(palette.name));
            painter.rect_filled(egui::Rect::from_min_size(pos(500.0,11.0),
                egui::vec2(34.0,34.0)*scale),10.0*scale,ink(palette.emblem_bg));
            painter.circle_filled(pos(516.0,30.0),9.0*scale,ink(palette.bird));
            painter.circle_filled(pos(521.0,23.0),5.0*scale,ink(palette.wing));
            painter.circle_filled(pos(523.0,21.0),1.5*scale,ink(palette.eye));
            for (index,(label,value)) in fields.iter().enumerate() {
                let x=[23.0,204.0,379.0][index];
                painter.text(pos(x,53.0),egui::Align2::LEFT_TOP,*label,
                    font(11.0),ink(palette.labels));
                painter.text(pos(x,71.0),egui::Align2::LEFT_TOP,value,
                    font(27.0),ink(palette.numbers));
            }
            let layout=self.prefs.layout.chars().take(34).collect::<String>();
            painter.text(pos(23.0,113.0),egui::Align2::LEFT_TOP,
                format!("Layout: {layout} · Timed bests · All sessions counted"),
                font(11.0),ink(palette.footer));
        } else {
            painter.text(pos(28.0,22.0),egui::Align2::LEFT_TOP,"Typefinch",
                font(22.0),ink(palette.heading));
            painter.rect_filled(egui::Rect::from_min_size(pos(465.0,18.0),
                egui::vec2(62.0,62.0)*scale),17.0*scale,ink(palette.emblem_bg));
            painter.circle_filled(pos(492.0,51.0),16.0*scale,ink(palette.bird));
            painter.circle_filled(pos(507.0,36.0),10.0*scale,ink(palette.bird));
            painter.circle_filled(pos(510.0,33.0),2.0*scale,ink(palette.eye));
            painter.add(egui::Shape::convex_polygon(
                vec![pos(517.0,38.0),pos(527.0,42.0),pos(517.0,46.0)],
                ink(palette.beak),egui::Stroke::NONE));
            painter.text(pos(28.0,57.0),egui::Align2::LEFT_TOP,name,
                font(19.0),ink(palette.name));
            for (index,(label,value)) in fields.iter().enumerate() {
                let x=[29.0,200.0,370.0][index];
                painter.text(pos(x,98.0),egui::Align2::LEFT_TOP,*label,
                    font(13.0),ink(palette.labels));
                painter.text(pos(x,120.0),egui::Align2::LEFT_TOP,value,
                    font(28.0),ink(palette.numbers));
            }
            let layout=self.prefs.layout.chars().take(28).collect::<String>();
            painter.text(pos(29.0,169.0),egui::Align2::LEFT_TOP,
                format!("Layout: {layout} · Timed-test bests, all sessions counted"),
                font(12.0),ink(palette.footer));
        }
    }
    fn show_export(&mut self, ui: &mut egui::Ui) {
        ui.heading("GitHub profile card");
        ui.label("Design and export a script-free static SVG card and matching Markdown.");
        let old_theme=self.prefs.card_theme;
        let old_font=self.prefs.card_font;
        let old_template=self.prefs.card_template;
        ui.horizontal_wrapped(|ui| {
            ui.label("Template");
            egui::ComboBox::from_id_salt("card-template")
                .selected_text(self.prefs.card_template.label()).show_ui(ui,|ui| {
                    for template in CardTemplate::ALL {
                        ui.selectable_value(&mut self.prefs.card_template,template,template.label());
                    }
                });
            ui.label("Palette");
            egui::ComboBox::from_id_salt("card-theme")
                .selected_text(self.prefs.card_theme.label()).show_ui(ui,|ui| {
                    for theme in CardTheme::ALL {
                        ui.selectable_value(&mut self.prefs.card_theme,theme,theme.label());
                    }
                });
            ui.label("Typography");
            egui::ComboBox::from_id_salt("card-font")
                .selected_text(self.prefs.card_font.label()).show_ui(ui,|ui| {
                    for font in CardFont::ALL {
                        ui.selectable_value(&mut self.prefs.card_font,font,font.label());
                    }
                });
        });
        if old_theme!=self.prefs.card_theme || old_font!=self.prefs.card_font
            || old_template!=self.prefs.card_template {self.save();}
        self.show_card_preview(ui);
        ui.small("Live native preview of theme and statistics; exported SVG renders using GitHub-safe system font families.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export SVG + Markdown").clicked() { self.export_widget(); }
            if ui.button("Export PNG").clicked() {self.export_png();}
            if ui.button("Export session history JSON").clicked() {self.export_statistics();}
            if ui.button("Backup settings + SQLite").clicked() {self.export_full_backup();}
            if ui.button("Copy GitHub Markdown").clicked() {
                ui.ctx().copy_text(export::markdown_embed().to_owned());
                self.message = "Markdown image embed copied to clipboard.".into();
            }
        });
    }
}
impl eframe::App for Typefinch {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let focused=ctx.input(|i|i.focused);
        if focused && !self.last_window_focused {
            self.refresh_keyboards();
        }
        self.last_window_focused=focused;
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.start_test(ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F6)) {
            self.export_widget();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F7)) {
            self.export_statistics();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F8)) {
            self.export_full_backup();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.export_png();
        }
        self.accept_custom_file(ctx);
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
                    ui.add_space(16.0);
                    self.show_achievements(ui);
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
    #[test] fn key_heat_levels_preserve_readable_error_glyphs() {
        for dark in [false,true] {
            let bg=key_heat_fill(0,dark);
            assert_ne!(key_heat_fill(1,dark),bg);
            assert_ne!(key_heat_fill(4,dark),key_heat_fill(1,dark));
            for count in [1,2,4,10] {
                let fg=error_ink(dark);
                let fill=key_heat_fill(count,dark);
                assert!(contrast(fg,fill)>=4.5,"heatmap {dark} count={count}: {fg:?} vs {fill:?}");
            }
        }
    }
    #[test] fn brand_ink_and_feedback_meet_normal_text_contrast() {
        let light=Color32::WHITE;
        let dark=Color32::from_rgb(32,38,69);
        for ink in [accent_ink(false),correct_ink(false),reference_ink(false),error_ink(false)] {
            assert!(contrast(ink,light)>=4.5, "light foreground {ink:?}");
        }
        for ink in [accent_ink(true),correct_ink(true),reference_ink(true),error_ink(true)] {
            assert!(contrast(ink,dark)>=4.5, "dark foreground {ink:?}");
        }
        assert!(contrast(PERIWINKLE,light)<4.5); // Never use accent swatch as white-background text.
    }
}
