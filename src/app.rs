use crate::{
    achievements,
    backup_verify,
    custom_layout::CustomLayout,
    daily,
    goal_transfer,
    devices::{self, Keyboard},
    engine::{Language, Session, State, TestMode, WordDifficulty},
    export,
    fonts,
    geometry::{self, Shape, Section, Slot, SlotKind},
    history::{self, Record},
    lesson,
    layouts::{self, Mode},
    preferences::{Preferences, Profile, Theme, CardTheme, CardFont,CardTemplate},
    registry::{self, InstalledLayout},
    xkb::Emulator,
};
use eframe::egui::{self, Color32, RichText};
use std::{sync::mpsc::{self, Receiver, TryRecvError},time::{Duration, Instant}};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum PreviewLayer {Base,Shift,AltGr,ShiftAltGr}
impl PreviewLayer {
    const ALL:[Self;4]=[Self::Base,Self::Shift,Self::AltGr,Self::ShiftAltGr];
    fn label(self)->&'static str {
        match self {Self::Base=>"Base",Self::Shift=>"Shift",Self::AltGr=>"AltGr",
            Self::ShiftAltGr=>"Shift+AltGr"}
    }
    fn shift(self)->bool {matches!(self,Self::Shift|Self::ShiftAltGr)}
    fn alt_gr(self)->bool {matches!(self,Self::AltGr|Self::ShiftAltGr)}
    fn next(self)->Self {
        match self {Self::Base=>Self::Shift,Self::Shift=>Self::AltGr,
            Self::AltGr=>Self::ShiftAltGr,Self::ShiftAltGr=>Self::Base}
    }
}
const PERIWINKLE: Color32 = Color32::from_rgb(134, 150, 245);
const KEYBOARD_POLL_INTERVAL:Duration=Duration::from_secs(20);
// Read modifier state from the *key press event*, not the frame-final
// keyboard state. Fast synthetic chords and real quick releases may have
// released Shift/Ctrl before the next GUI frame begins.
// Only a fresh emulated physical-key press can add scored text. EgGui's
// system-mode Text events are an aggregate committed stream and cannot be
// safely labeled as physical repeat or uniquely attributed to hardware.
// Keep provisional IME text *only in memory*, bounded to the visible
// candidate preview. Never mistake it for a committed scoring event.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum ReferenceFeedback {Correct,Error,Cursor,Unentered}
#[derive(Debug,PartialEq,Eq)]
struct ReferenceRun {text:String,feedback:ReferenceFeedback}
/// Group adjacent equally styled *logical* reference graphemes, without
/// reversing or slicing UTF-8. In particular, keep unentered Arabic words in
/// one shaping run rather than splitting every connected letter into a
/// separate egui text section. This is not a complete BiDi renderer.
fn reference_runs(target:&str,typed:&str,start:usize,end:usize,running:bool)
    ->Vec<ReferenceRun> {
    let entered=typed.graphemes(true).count();
    let reference=target.graphemes(true).skip(start).take(end.saturating_sub(start));
    let typed_at=typed.graphemes(true).skip(start).collect::<Vec<_>>();
    let mut runs=Vec::<ReferenceRun>::new();
    for (index,reference_grapheme) in reference.enumerate() {
        let position=start+index;
        let feedback=match typed_at.get(index) {
            Some(actual) if crate::engine::canonically_equal(actual,reference_grapheme)
                =>ReferenceFeedback::Correct,
            Some(_)=>ReferenceFeedback::Error,
            None if running && position==entered=>ReferenceFeedback::Cursor,
            None=>ReferenceFeedback::Unentered,
        };
        if let Some(last)=runs.last_mut().filter(|last|last.feedback==feedback) {
            last.text.push_str(reference_grapheme);
        } else {
            runs.push(ReferenceRun{text:reference_grapheme.to_owned(),feedback});
        }
    }
    runs
}
/// Build the *actual* colored egui reference layout from logical grapheme
/// classifications. Native egui is responsible for glyph rendering; this
/// function never reverses visual order or invents a physical caret.
fn reference_layout_job(target:&str,typed:&str,start:usize,end:usize,
    running:bool,dark:bool)->egui::text::LayoutJob {
    let mut job=egui::text::LayoutJob::default();
    for run in reference_runs(target,typed,start,end,running) {
        let color=match run.feedback {
            ReferenceFeedback::Correct=>correct_ink(dark),
            ReferenceFeedback::Error=>error_ink(dark),
            ReferenceFeedback::Cursor=>accent_ink(dark),
            ReferenceFeedback::Unentered=>reference_ink(dark),
        };
        job.append(&run.text,0.0,egui::TextFormat{
            font_id:egui::FontId::monospace(21.0),
            color,
            ..Default::default()
        });
    }
    job
}
const MAX_IME_PREEDIT_GRAPHEMES:usize=64;
const MAX_IME_PREEDIT_BYTES:usize=4096;
fn limited_ime_preedit(text:&str)->String {
    let mut visible=String::new();
    for grapheme in text.graphemes(true).take(MAX_IME_PREEDIT_GRAPHEMES) {
        if visible.len()+grapheme.len()>MAX_IME_PREEDIT_BYTES {break;}
        visible.push_str(grapheme);
    }
    visible
}
fn score_system_committed_event(session:&mut Session,preedit:&mut String,
    event:&egui::Event)->bool {
    match event {
        egui::Event::Ime(egui::ImeEvent::Enabled|egui::ImeEvent::Disabled)=>{
            preedit.clear();
            false
        },
        egui::Event::Ime(egui::ImeEvent::Preedit(text))=>{
            *preedit=limited_ime_preedit(text);
            false
        },
        egui::Event::Ime(egui::ImeEvent::Commit(text))=>{
            preedit.clear();
            if text.is_empty() {false} else {session.type_text(text);true}
        },
        egui::Event::Text(text) if !text.is_empty()=>{
            session.type_text(text);
            true
        },
        // Clipboard paste, key events and candidate edits are not scoreable.
        _=>false,
    }
}
fn emulated_key_accepted(repeat:bool,modifiers:egui::Modifiers)->bool {
    !repeat && !modifiers.command && !modifiers.ctrl && !modifiers.alt
}
fn pressed_key_modifiers(events:&[egui::Event],desired:egui::Key)->Option<egui::Modifiers> {
    events.iter().find_map(|event| match event {
        egui::Event::Key{key,pressed:true,repeat:false,modifiers,..} if *key==desired=>
            Some(*modifiers),
        _=>None,
    })
}
fn should_poll_devices(can_poll:bool,focused:bool,focus_gained:bool,elapsed:Duration)->bool {
    can_poll && focused && (focus_gained || elapsed>=KEYBOARD_POLL_INTERVAL)
}
// Manual requests and the focus/hotplug watcher share one bounded inspector.
// A slow external compositor command must not block typing or allow repeated
// clicks to create an unbounded number of detached processes.
fn take_device_query_result(
    pending:&mut Option<Receiver<Result<Vec<Keyboard>,String>>>,
)->Option<Result<Vec<Keyboard>,String>> {
    let result=match pending.as_ref()?.try_recv() {
        Ok(answer)=>Some(answer),
        Err(TryRecvError::Disconnected)=>
            Some(Err("Keyboard inspection worker disconnected; stale inventory cleared.".into())),
        Err(TryRecvError::Empty)=>None,
    };
    if result.is_some(){*pending=None;}
    result
}
fn launch_device_inspection<F>(
    pending:&mut Option<Receiver<Result<Vec<Keyboard>,String>>>,
    repaint:Option<egui::Context>,
    inspect:F,
)->bool
where F:FnOnce()->Result<Vec<Keyboard>,String>+Send+'static {
    if pending.is_some(){return false;}
    let (sender,receiver)=mpsc::channel();
    *pending=Some(receiver);
    std::thread::spawn(move || {
        let result=inspect();
        let _=sender.send(result);
        if let Some(ctx)=repaint {ctx.request_repaint();}
    });
    true
}
// Wayland normally combines physical keyboards into a seat-level event stream.
// Only tag OS input with a detected layout when all reported Hyprland keyboards
// agree. A configured X11 seat layout is *not* an active input group and must
// not be promoted to a scored layout identity.
const X11_AGGREGATE_SEAT:&str="x11:aggregate-seat";
fn inspected_target<'a>(keyboard:&Keyboard, installed:&'a [InstalledLayout])
    ->Option<&'a InstalledLayout> {
    // wl_seat only advertises aggregate capability, never active XKB
    // identity. Even a future accidental value must not turn into a target.
    if keyboard.address.starts_with("wayland:seat:") {return None;}
    if keyboard.address==X11_AGGREGATE_SEAT {
        // X11 setxkbmap -query reports configured groups, not the active
        // input group. Only a single unambiguous layout can be offered as a
        // user-requested emulation target, never as detected OS input.
        let base=keyboard.layout.trim();
        let variant=keyboard.variant.trim();
        if base.is_empty() || base.contains(',') || variant.contains(',') {
            return None;
        }
        let id=if variant.is_empty(){base.to_string()}
            else{format!("{base}({variant})")};
        let mut candidates=installed.iter().filter(|l|l.id==id);
        let first=candidates.next()?;
        if candidates.next().is_some(){None}else{Some(first)}
    } else {
        registry::resolve_active(installed,&keyboard.active_keymap)
    }
}
// Read-only inspection metadata must be freshly confirmed before users can
// adopt it as an emulation target. Stale rows remain visible for diagnosis.
fn inspection_confirmed(inventory_fresh:bool,inspection_pending:bool)->bool {
    // A previously successful inventory is not current evidence while a
    // newer inspection is running. Do not certify a new session's layout
    // or adopt a reported keymap until the refresh finishes successfully.
    inventory_fresh && !inspection_pending
}
// Inspectable rows may outlive focus and remain on screen for diagnosis.
// Distinguish a fresh read-only report from historical, unverified metadata
// without claiming that any row identifies the source of scored key events.
fn inspected_row_summary(keyboard:&Keyboard,verified:bool)->String {
    if keyboard.address.starts_with("wayland:seat:") {
        if verified {"Wayland seat reports keyboard capability.".into()}
        else {"Last observed Wayland seat (unverified): keyboard capability was reported.".into()}
    } else if keyboard.address==X11_AGGREGATE_SEAT {
        let prefix=if verified {"Configured X11 seat layout"}
            else {"Last configured X11 seat layout (unverified)"};
        format!("{prefix}: {} · variant: {}",
            keyboard.layout,if keyboard.variant.is_empty(){"(none)"}else{&keyboard.variant})
    } else {
        let prefix=if verified {"Reported active map"} else {"Last reported active map (unverified)"};
        format!("{prefix}: {} · configured: {} {}",
            keyboard.active_keymap,keyboard.layout,keyboard.variant)
    }
}
fn inspected_target_if_fresh<'a>(keyboard:&Keyboard, installed:&'a [InstalledLayout],
    inventory_fresh:bool)->Option<&'a InstalledLayout> {
    if inventory_fresh {inspected_target(keyboard,installed)} else {None}
}
fn recorded_layout(mode:Mode, target:&str, keyboards:&[Keyboard], installed:&[InstalledLayout])->String {
    if mode == Mode::Emulated { return target.to_owned(); }
    let Some(first)=keyboards.first() else { return "os:unknown".to_owned() };
    if keyboards.iter().any(|k|
        k.address==X11_AGGREGATE_SEAT || k.address.starts_with("wayland:seat:")) {
        return "os:unknown".to_owned();
    }
    if first.active_keymap.trim().is_empty() ||
        keyboards.iter().any(|k|k.active_keymap != first.active_keymap) {
        return "os:unknown".to_owned();
    }
    match registry::resolve_active(installed,&first.active_keymap) {
        Some(layout)=>format!("detected/{}",layout.id),
        None=>"os:unknown".to_owned(),
    }
}

// The detected input identity is a snapshot, not a hardware event source.
// While a session is active, a fresh read-only inventory may invalidate an
// earlier compositor-reported layout; never retroactively upgrade unknown
// attribution or rewrite an emulated target.
fn validated_session_layout(
    current:&str,mode:Mode,keyboards:&[Keyboard],installed:&[InstalledLayout]
)->String {
    if mode==Mode::System && current.starts_with("detected/")
        && recorded_layout(Mode::System,"",keyboards,installed)!=current {
        "os:unknown".into()
    } else { current.to_owned() }
}

/// The user's next OS-input session may begin before a focus-triggered
/// inventory read finishes. Only a fresh, focused inspector response may
/// contribute detected layout provenance; emulation keeps its explicit target.
fn layout_for_new_session(mode:Mode,target:&str,keyboards:&[Keyboard],
    installed:&[InstalledLayout],inventory_fresh:bool)->String {
    if mode==Mode::System && !inventory_fresh {"os:unknown".into()}
    else {recorded_layout(mode,target,keyboards,installed)}
}
// A session can finish while a periodic or manually requested inspection is
// still in flight. A stale detected/* label is never persisted as if the
// compositor had revalidated it. Keep the score; downgrade only provenance.
fn layout_at_completion(current:&str,mode:Mode,inventory_fresh:bool,
    inspection_pending:bool)->String {
    if mode==Mode::System && current.starts_with("detected/")
        && !inspection_confirmed(inventory_fresh,inspection_pending) {
        "os:unknown".into()
    } else {current.to_owned()}
}
fn layout_after_focus_change(current:&str,mode:Mode,
    previously_focused:bool,focused:bool)->String {
    if previously_focused && !focused && mode==Mode::System
        && current.starts_with("detected/") {
        "os:unknown".into()
    } else {current.to_owned()}
}

// The brand swatch is too light for text on white. Use theme-specific ink.
fn accent_ink(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(170,181,255) }
    else { Color32::from_rgb(69,89,181) }
}
fn selection_fill(dark:bool)->Color32 {
    if dark {PERIWINKLE} else {accent_ink(false)}
}
fn selection_text(dark:bool)->Color32 {
    if dark {Color32::from_rgb(32,38,69)} else {Color32::WHITE}
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


fn next_theme(theme:Theme)->Theme {
    match theme {Theme::System=>Theme::Light,Theme::Light=>Theme::Dark,
        Theme::Dark=>Theme::System}
}
fn motion_seconds(reduced:bool,normal:f32)->f32 {
    if reduced {0.0} else {normal}
}
pub struct Typefinch {
    prefs: Preferences,
    settings_read_only:bool,
    normal_animation_time:f32,
    show_keyboard_help:bool,
    unicode_fonts:fonts::Fallbacks,
    session: Session,
    mode: Mode,
    session_layout: String,
    session_input_mode: String,
    emulator: Option<Emulator>,
    composition: Option<xkbcommon::xkb::compose::State>,
    ime_preedit:String,
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
    device_poll_started:Instant,
    pending_device_query:Option<Receiver<Result<Vec<Keyboard>,String>>>,
    device_inventory_fresh:bool,
    pending_device_obsolete:bool,
    device_error: Option<String>,
    new_profile: String,
    custom_text: String,
    custom_import_path:String,
    goal_import_path:String,
    backup_settings_path:String,
    backup_check_result:String,
    preview_layer:PreviewLayer,
    preview_shape:Shape,
    preview_section:Section,
    message: String,
}
impl Typefinch {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let loaded=Preferences::load_guarded();
        let mut prefs=loaded.prefs;
        // Synthetic graphical QA may inspect installed Japanese or Spanish
        // XKB symbols without mutating an actual saved preference. Both
        // environment guards are necessary; the override is not persistent.
        if std::env::var_os("TYPEFINCH_QA_REPORT").is_some()
            && std::env::var_os("TYPEFINCH_QA_PREVIEW_SHAPE").is_some() {
            match std::env::var("TYPEFINCH_QA_PREVIEW_LAYOUT").as_deref() {
                Ok("es")=>prefs.layout="es".to_owned(),
                Ok("jp")=>prefs.layout="jp".to_owned(),
                _ if std::env::var("TYPEFINCH_QA_PREVIEW_SHAPE").as_deref()==Ok("jis")=>
                    prefs.layout="jp".to_owned(),
                _=>{},
            }
        }
        let settings_read_only=loaded.read_only;
        let startup_warning=loaded.warning.unwrap_or_default();
        if std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
            // Opt-in synthetic GUI test process only, never persisted to a real profile.
            prefs.duration_seconds = 2;
        }
        cc.egui_ctx.set_theme(match prefs.theme {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        });
        // Use only installed system Unicode fallbacks, retaining egui's
        // preferred fonts. No glyph assets are bundled or fetched.
        let unicode_fonts=fonts::install(&cc.egui_ctx);
        let normal_animation_time=cc.egui_ctx.style().animation_time;
        cc.egui_ctx.all_styles_mut(|s| {
            s.animation_time=motion_seconds(prefs.reduce_motion,normal_animation_time);
            s.spacing.item_spacing = egui::vec2(10.0, 9.0);
            s.spacing.button_padding = egui::vec2(11.0, 7.0);
            s.visuals.selection.bg_fill = selection_fill(s.visuals.dark_mode);
            s.visuals.selection.stroke.color = selection_text(s.visuals.dark_mode);
            s.visuals.hyperlink_color = accent_ink(s.visuals.dark_mode);
        });
        // Never block the initial GUI window on a compositor utility.
        // The initial read-only inventory uses the same single-flight,
        // bounded inspector as manual and focus-triggered refreshes.
        let keyboards=Vec::new();
        let device_error=None;
        let mut pending_device_query=None;
        let _=launch_device_inspection(
            &mut pending_device_query,Some(cc.egui_ctx.clone()),devices::inspect);
        let input = if prefs.test_mode == TestMode::Lesson {lesson::get(&prefs.lesson_id).passage} else {""};
        let duration = if prefs.test_mode == TestMode::Lesson {0} else {prefs.duration_seconds};
        let session = Session::new_with_settings(duration, prefs.test_mode, input, prefs.language, prefs.word_difficulty);
        let emulator = Emulator::compile(&prefs.layout).ok();
        // A temporary synthetic X11 smoke process may force installed-XKB
        // emulation without a mouse. Both guards are required; ordinary users
        // always retain the normal explicitly selected input-mode behavior.
        let synthetic_xkb = std::env::var_os("TYPEFINCH_QA_REPORT").is_some()
            && std::env::var_os("TYPEFINCH_QA_XKB_COMPOSE").is_some();
        let initial_mode=if synthetic_xkb ||
            prefs.custom_layouts.iter().any(|l|l.id==prefs.layout && l.validate().is_ok()) {
            Mode::Emulated
        } else {Mode::System};
        let installed_layouts = registry::discover().unwrap_or_default();
        // Do not open or migrate the user's existing database when the settings
        // identity is unknown. Even reading history currently initializes schema.
        let (history,completed_lessons,all_history)=if settings_read_only {
            (Vec::new(),std::collections::HashSet::<String>::new(),Vec::new())
        } else {
            let id=&prefs.profiles[prefs.active_profile].id;
            (history::recent(id).unwrap_or_default(),
             history::completed_lessons(id).unwrap_or_default(),
             history::all_for_profile(id).unwrap_or_default())
        };
        let earned_badges=achievements::earned(&all_history);
        // This override is available only to the disposable graphical smoke
        // process, never in normal use. It does not change physical input.
        let preview_shape=if std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
            match std::env::var("TYPEFINCH_QA_PREVIEW_SHAPE").as_deref() {
                Ok("iso")=>Shape::Iso, Ok("jis")=>Shape::Jis,
                Ok("split")=>Shape::Split, _=>Shape::Ansi,
            }
        } else {Shape::Ansi};
        let preview_section=if std::env::var_os("TYPEFINCH_QA_REPORT").is_some()
            && std::env::var_os("TYPEFINCH_QA_PREVIEW_SHAPE").is_some() {
            match std::env::var("TYPEFINCH_QA_PREVIEW_SECTION").as_deref() {
                Ok("navigation")=>Section::Navigation,
                Ok("keypad")=>Section::Keypad,
                _=>Section::Typing,
            }
        } else {Section::Typing};
        if std::env::var_os("TYPEFINCH_QA_PREVIEW_SHAPE").is_some()
            && std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
            let _=crate::qa::write_geometry_report(
                preview_shape.label(),&prefs.layout,preview_section.label(),
                &geometry::section_slots(preview_shape,preview_section));
        }
        Self { prefs, settings_read_only, normal_animation_time, show_keyboard_help:false, unicode_fonts, session, mode: initial_mode, session_layout: "os:unknown".into(), session_input_mode: "OS".into(), emulator, composition:None, ime_preedit:String::new(), installed_layouts, layout_search: String::new(), layout_draft:None, layout_import_path:String::new(), history, history_only_matching: false, history_metric: history::HistoryMetric::CorrectedWpm,
            earned_badges, all_history, completed_lessons, keyboards, device_error,
               device_index: 0, last_window_focused:true,
               device_poll_started:Instant::now(), pending_device_query,
               device_inventory_fresh:false, pending_device_obsolete:false,
               new_profile: String::new(), custom_text: String::new(), custom_import_path:String::new(), goal_import_path:String::new(), backup_settings_path:String::new(),
            backup_check_result:String::new(), preview_layer:PreviewLayer::Base,
            preview_shape, preview_section, message: startup_warning }
    }
    fn apply_device_result(&mut self,result:Result<Vec<Keyboard>,String>,focused:bool) {
        self.device_inventory_fresh=focused && result.is_ok();
        let selected=self.keyboards.get(self.device_index).map(|k|k.address.clone());
        let (updated,error)=match result {
            Ok(found)=>(found,None),
            Err(message)=>(Vec::new(),Some(message)),
        };
        self.device_index=devices::preserve_selection(selected.as_deref(),&updated);
        self.keyboards=updated;
        self.device_error=error;
        if self.session.state==State::Running && self.mode==Mode::System {
            let validated=validated_session_layout(
                &self.session_layout,self.mode,&self.keyboards,&self.installed_layouts);
            if validated!=self.session_layout {
                self.session_layout=validated;
                self.message="Session layout attribution downgraded to unknown after a keyboard inventory change; typing continues normally.".into();
            }
        }
    }
    fn request_device_query(&mut self,ctx:&egui::Context) {
        if launch_device_inspection(
            &mut self.pending_device_query,Some(ctx.clone()),devices::inspect) {
            self.device_poll_started=Instant::now();
            self.pending_device_obsolete=false;
        }
    }
    fn refresh_keyboards(&mut self,ctx:&egui::Context) {
        self.request_device_query(ctx);
    }
    fn poll_keyboard_hotplug(&mut self,ctx:&egui::Context,focused:bool) {
        if let Some(result)=take_device_query_result(&mut self.pending_device_query) {
            if self.pending_device_obsolete {
                // The worker was launched before a focus-loss boundary.
                // Its result cannot certify a keymap after focus returns.
                self.pending_device_obsolete=false;
                self.device_inventory_fresh=false;
                // Let Hyprland's normal periodic logic immediately issue
                // a fresh query (still only one query in flight).
                self.device_poll_started=Instant::now()-KEYBOARD_POLL_INTERVAL;
            } else {
                // A disconnected inspector invalidates the old inventory,
                // as does a failed query; neither leaves a trusted row.
                self.apply_device_result(result,focused);
            }
        }
        // Poll exactly the backends that inspect() can select; a stale
        // Hyprland signature in X11 must not activate a Wayland poll loop.
        let can_poll=devices::supports_polling();
        let focus_gained=focused && !self.last_window_focused;
        if should_poll_devices(can_poll,focused,focus_gained,self.device_poll_started.elapsed())
            && self.pending_device_query.is_none() {
            self.request_device_query(ctx);
        }
        if focused && can_poll {
            ctx.request_repaint_after(KEYBOARD_POLL_INTERVAL);
        }
    }
    fn save(&mut self) {
        if self.settings_read_only {
            self.message="Settings remain read-only because the existing settings file needs recovery. Original data is preserved.".into();
            return;
        }
        if let Err(e) = self.prefs.save() { self.message = format!("Could not save settings: {e}"); }
    }
    fn finish(&mut self) {
        if self.settings_read_only {
            let _=crate::qa::write_guarded_report();
            self.message="Session not recorded: existing settings need recovery, and the original file has been preserved.".into();
            return;
        }
        let result = self.session.score();
        if result.entered == 0 {
            self.message = "Empty session not recorded. Start another test when ready.".into();
            return;
        }
        // The UI may have finished scoring before an outstanding compositor
        // refresh was consumed. Never persist speculative detected/* identity.
        self.session_layout=layout_at_completion(&self.session_layout,self.mode,
            self.device_inventory_fresh,self.pending_device_query.is_some());
        let identity = self.prefs.profiles[self.prefs.active_profile].id.clone();
        let mut record = Record::completed(&identity, &self.session_layout, &self.session_input_mode, &self.session);
        if self.session.mode==TestMode::Lesson {
            record.test_mode=format!("lesson/{}", self.prefs.lesson_id);
        }
        // The session count, personal best, achievements and QA report must
        // never claim success if SQLite refused the record.
        if let Err(error) = history::insert(&record) {
            self.message=format!("History not saved: {error}. Profile totals were not changed.");
            return;
        }
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
            Ok(records) => self.history=records,
            Err(error) => self.message=format!("Session saved, but history refresh failed: {error}"),
        }
        let profile = &mut self.prefs.profiles[self.prefs.active_profile];
        profile.record_saved_score(self.session.mode,&result);
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
        self.ime_preedit.clear();
        let input=if self.prefs.test_mode==TestMode::Lesson {
            lesson::get(&self.prefs.lesson_id).passage
        } else { &self.custom_text };
        let duration=if self.prefs.test_mode==TestMode::Lesson {0} else {self.prefs.duration_seconds};
        self.session = Session::new_with_settings(duration, self.prefs.test_mode, input, self.prefs.language, self.prefs.word_difficulty);
    }
    fn start_test(&mut self, ctx: &egui::Context) {
        self.ime_preedit.clear();
        if self.session.target.is_empty() {
            self.message = "Enter some custom text before starting this test.".into();
            return;
        }
        self.session_layout = layout_for_new_session(self.mode,&self.prefs.layout,
            &self.keyboards,&self.installed_layouts,
            inspection_confirmed(self.device_inventory_fresh,self.pending_device_query.is_some()));
        self.session_input_mode = if self.mode == Mode::System { "OS".into() } else { "Emulated".into() };
        self.composition = if self.mode == Mode::Emulated {
            self.emulator.as_ref().and_then(Emulator::compose_state)
        } else { None };
        self.session.begin();
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("typing_surface")));
    }
    fn apply_events(&mut self, ctx: &egui::Context) {
        if self.session.state != State::Running { return; }
        if !ctx.input(|i| i.focused) {
            // A window losing focus must not collect invisible typing events.
            self.session.state = State::Ready;
            self.ime_preedit.clear();
            if let Some(compose) = self.composition.as_mut() { compose.reset(); }
            let _=crate::qa::write_focus_lost_report();
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
                    self.ime_preedit.clear();
                    if let Some(compose) = self.composition.as_mut() { compose.reset(); }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, repeat: false, .. } => {
                    if let Some(compose) = self.composition.as_mut().filter(|compose|
                        compose.status() == xkbcommon::xkb::compose::Status::Composing) {
                        // The pending accent has not entered scoring: cancel it first.
                        compose.reset();
                    } else {
                        self.session.backspace();
                    }
                },
                egui::Event::Key { key, physical_key, pressed: true, repeat, modifiers, .. }
                    if self.mode == Mode::Emulated && emulated_key_accepted(repeat,modifiers) =>
                {
                    // Crucially use the physical key, not an OS-translated logical letter.
                    // If unavailable, do not silently claim emulation works.
                    if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let mapped = if let Some(personal)=self.prefs.custom_layouts.iter()
                            .find(|l|l.id==self.prefs.layout) {
                            personal.translate(physical,modifiers.shift)
                        } else if let Some(emulator) = &self.emulator {
                            if let Some(compose) = self.composition.as_mut() {
                                emulator.translate_composed(physical, modifiers.shift, compose)
                            } else {
                                emulator.translate(physical, modifiers.shift)
                            }
                        } else {
                            layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c| c.to_string())
                        };
                        if let Some(text) = mapped { self.session.type_text(&text); }
                    }
                }
                // Explicitly accept finalized IME composition in system
                // mode, but never transient preedit or clipboard paste.
                event if self.mode==Mode::System => {
                    let _=score_system_committed_event(&mut self.session,
                        &mut self.ime_preedit,&event);
                },
                // Paste is intentionally disallowed in scored sessions.
                _ => {}
            }
        }
    }
    fn load_custom_file(&mut self,path:&std::path::Path) {
        if self.session.state==State::Running || self.prefs.test_mode!=TestMode::Custom {return;}
        match crate::engine::import_custom_file(path) {
            Ok(passage)=>{
                let count=passage.graphemes(true).count();
                self.custom_text=passage;
                self.rebuild_session();
                self.message=format!("Loaded {count} graphemes into memory. The passage is not saved.");
            },
            Err(reason)=>self.message=format!("Custom text import rejected: {reason}"),
        }
    }
    fn accept_custom_file(&mut self,ctx:&egui::Context) {
        if self.prefs.test_mode!=TestMode::Custom || self.session.state==State::Running {return;}
        let files=ctx.input(|i|i.raw.dropped_files.clone());
        let Some(drop)=files.into_iter().next() else {return;};
        if let Some(path)=drop.path {
            self.load_custom_file(&path);
        } else {
            self.message="Drop a local file with a readable filesystem path.".into();
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
            ui.add(egui::TextEdit::singleline(&mut self.new_profile)
                .id(egui::Id::new("new_profile_name"))
                .hint_text("New local profile").desired_width(190.0));
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
            if !self.settings_read_only {
                let id = self.prefs.profiles[self.prefs.active_profile].id.clone();
                self.history = history::recent(&id).unwrap_or_default();
                self.completed_lessons = history::completed_lessons(&id).unwrap_or_default();
                self.all_history=history::all_for_profile(&id).unwrap_or_default();
                self.earned_badges=achievements::earned(&self.all_history);
            }
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
        ui.add_space(8.0);
        ui.label(RichText::new("Transfer profile goals").strong());
        ui.small("Import replaces only this profile's goal targets and optional streak setting. It does not replace identity, history, test counts or records.");
        let mut export_requested=false;
        let mut import_requested=false;
        ui.add_enabled_ui(self.session.state!=State::Running,|ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Export goals JSON").clicked() {export_requested=true;}
                if ui.button("Import goals JSON").clicked() {import_requested=true;}
            });
            let response=ui.add(egui::TextEdit::singleline(&mut self.goal_import_path)
                .id(egui::Id::new("goals_import_path"))
                .hint_text("Local path to goals JSON").desired_width(ui.available_width()));
            if response.has_focus() {
                ui.scroll_to_rect(response.rect,Some(egui::Align::Center));
            }
            if (response.has_focus() || response.lost_focus())
                && ui.input(|i|i.key_pressed(egui::Key::Enter)) {
                import_requested=true;
            }
        });
        if export_requested {self.export_profile_goals();}
        if import_requested && self.settings_read_only {
            self.message="Goals cannot be imported while existing settings are protected from writes.".into();
        } else if import_requested {
            match goal_transfer::import(std::path::Path::new(self.goal_import_path.trim())) {
                Ok(goals)=>{
                    let index=self.prefs.active_profile;
                    let previous=std::mem::replace(&mut self.prefs.profiles[index].goals,goals);
                    match self.prefs.save() {
                        Ok(())=>self.message="Goals imported into the selected local profile; history and identity unchanged.".into(),
                        Err(e)=>{
                            self.prefs.profiles[index].goals=previous;
                            self.message=format!("Goals import was not saved; previous targets preserved: {e}");
                        },
                    }
                },
                Err(e)=>self.message=format!("Goals import failed: {e}"),
            }
        }
    }
    fn export_profile_goals(&mut self) {
        if self.session.state==State::Running {return;}
        if self.settings_read_only {
            self.message="Goal export disabled: fallback profile targets do not represent recovered settings.".into();
            return;
        }
        let goals=&self.prefs.profiles[self.prefs.active_profile].goals;
        self.message=match goal_transfer::export(goals) {
            Ok(path)=>format!("Local goals exported: {}",path.display()),
            Err(e)=>format!("Goals export failed: {e}"),
        };
    }
    fn import_personal_keymap(&mut self) {
        let filename=self.layout_import_path.trim();
        let result=(|| -> Result<CustomLayout,String> {
            if filename.is_empty() {return Err("Enter a local JSON file path".into());}
            let metadata=std::fs::metadata(filename).map_err(|e|e.to_string())?;
            if metadata.len()>16_384 {return Err("Layout file exceeds 16 KiB".into());}
            let bytes=std::fs::read(filename).map_err(|e|e.to_string())?;
            CustomLayout::from_json(&bytes)
        })();
        match result {
            Err(e)=>self.message=format!("Layout import rejected: {e}"),
            Ok(mut layout)=>{
                if self.prefs.custom_layouts.len()>=32 {
                    self.message="Maximum of 32 personal keymaps reached.".into();
                    return;
                }
                if self.prefs.custom_layouts.iter().any(|k|k.id==layout.id) {
                    layout.new_identity();
                }
                self.prefs.layout=layout.id.clone();
                self.prefs.custom_layouts.push(layout);
                self.layout_import_path.clear();
                self.emulator=None;
                self.mode=Mode::Emulated;
                self.save();
                self.message="Personal ANSI keymap imported and activated.".into();
            }
        }
    }
    fn export_personal_keymap(&mut self) {
        let result=(|| -> Result<std::path::PathBuf,String> {
            let layout=self.prefs.custom_layouts.iter().find(|l|l.id==self.prefs.layout)
                .ok_or("Select a personal keymap before exporting")?;
            let bytes=layout.as_json()?;
            let folder=crate::preferences::base_dir().join("exports");
            std::fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
            let id=layout.id.strip_prefix("custom/").ok_or("Invalid keymap ID")?;
            let path=folder.join(format!("typefinch-keymap-{id}.json"));
            let tmp=path.with_extension("json.tmp");
            std::fs::write(&tmp,bytes).map_err(|e|e.to_string())?;
            std::fs::rename(&tmp,&path).map_err(|e|e.to_string())?;
            Ok(path)
        })();
        self.message=match result {
            Ok(path)=>format!("Personal keymap exported: {}",path.display()),
            Err(e)=>format!("Keymap export failed: {e}"),
        };
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
                    if ui.add_enabled(enabled && self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout),egui::Button::new("Export selected JSON")).clicked() {
                        self.export_personal_keymap();
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
                ui.separator();
                ui.label("Import versioned ANSI mapping JSON (16 KiB maximum)");
                ui.add_enabled_ui(enabled,|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.layout_import_path)
                        .hint_text("Absolute path to local JSON").desired_width(270.0));
                    if ui.button("Import JSON as a new keymap").clicked() {
                        self.import_personal_keymap();
                    }
                });
                ui.small("Duplicate IDs receive a new UUID; existing layouts are never overwritten by import.");
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
                ui.small("F11 or Ctrl+O focuses the local path field; Enter imports. Drag-and-drop also works (64 KiB limit, 4096 graphemes).");
                let mut load_requested=false;
                ui.add_enabled_ui(self.session.state!=State::Running, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let response=ui.add(
                            egui::TextEdit::singleline(&mut self.custom_import_path)
                                .id(egui::Id::new("custom_import_path"))
                                .hint_text("Path to local .txt or .md file")
                                .desired_width(ui.available_width().min(230.0))
                        );
                        if response.has_focus() {
                            ui.scroll_to_rect(response.rect,Some(egui::Align::Center));
                        }
                        if (response.has_focus() || response.lost_focus())
                            && ui.input(|i|i.key_pressed(egui::Key::Enter)) {
                            load_requested=true;
                        }
                        if ui.button("Load local file").clicked() {load_requested=true;}
                    });
                });
                if load_requested {
                    let path=std::path::PathBuf::from(self.custom_import_path.trim());
                    self.load_custom_file(&path);
                }
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
            ui.small(format!("System Unicode font fallbacks: CJK {} · Arabic {} · Hebrew {}",
                if self.unicode_fonts.cjk {"available"} else {"not installed"},
                if self.unicode_fonts.arabic {"available"} else {"not installed"},
                if self.unicode_fonts.hebrew {"available"} else {"not installed"}));
            ui.small("Missing scripts need installed Noto fonts. Built-in font choices remain primary; this does not claim RTL shaping.");
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
            let was_reduced=self.prefs.reduce_motion;
            if ui.checkbox(&mut self.prefs.reduce_motion,"Reduce UI animations").changed() {
                let seconds=motion_seconds(self.prefs.reduce_motion,self.normal_animation_time);
                ctx.all_styles_mut(|style|style.animation_time=seconds);
                self.save();
            }
            if was_reduced!=self.prefs.reduce_motion {
                ui.small("Transitions update immediately. This does not change typing test timers.");
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
            // The preview must match what the next scored session could
            // actually record, including an in-flight or stale inspection.
            let fresh=inspection_confirmed(self.device_inventory_fresh,
                self.pending_device_query.is_some());
            let inferred=layout_for_new_session(self.mode,&self.prefs.layout,
                &self.keyboards,&self.installed_layouts,fresh);
            ui.small(format!("Next OS session layout: {inferred} (seat-level, not per-keyboard)"));
            if !fresh {
                ui.small("Detected layout unavailable until inspection completes successfully.");
            }
        }
        self.show_custom_layout_editor(ui);
        if self.session.state == State::Running { ui.small("Layout and input mode are locked while a test is running."); }
        if self.mode == Mode::Emulated {
            ui.colored_label(accent_ink(ui.visuals().dark_mode), "Emulation uses physical ANSI key positions inside Typefinch only; it never changes your system layout.");
            ui.small(if self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout) {
                "Personal ANSI mapping uses physical-key events. It cannot change your system layout; Compose/AltGr/IME are not supported."
            } else if self.emulator.is_some() {
                "Installed XKB ANSI keys: Base/Shift and session-local dead-key Compose where locale data exists. Physical AltGr, IME and extra keys remain pending."
            } else {
                "Using starter ANSI table; physical-key events required. Compose, AltGr, IME pending."
            });
        }
    }
    fn show_devices(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keyboard / seat inspector");
        ui.colored_label(accent_ink(ui.visuals().dark_mode),
            devices::CAPTURE_POLICY.explanation());
        ui.small("Read-only Hyprland device metadata, Wayland aggregate seat capabilities, or X11 configuration. Supported Wayland inspectors refresh on focus and every 20 seconds; no input capture.");
        let verified=inspection_confirmed(self.device_inventory_fresh,
            self.pending_device_query.is_some());
        if !verified {
            ui.small("Keyboard inventory not yet verified after startup, focus change, refresh, or failed inspection; new OS layout attribution stays unknown.");
        }
        ui.vertical(|ui| {
            let pending=self.pending_device_query.is_some();
            if ui.add_enabled(!pending,
                egui::Button::new(if pending {"Inspecting..."} else {"Refresh device list"}))
                .clicked() {
                self.refresh_keyboards(ui.ctx());
            }
            if pending {
                ui.small("One read-only keyboard / seat inspection is running; typing stays responsive.");
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
                let is_x11=k.address==X11_AGGREGATE_SEAT;
                let is_wayland=k.address.starts_with("wayland:seat:");
                ui.label(inspected_row_summary(k,verified));
                if is_wayland {
                    ui.small("Standard wl_seat gives no physical keyboard identity or active XKB layout. No key events are requested.");
                } else if is_x11 {
                    ui.small("setxkbmap reports configured groups, not the active input group.");
                    ui.small("This is one aggregate seat, never a physical keyboard.");
                }
                ui.small(format!("Inspection identifier: {}",k.address));
                let matched=inspected_target_if_fresh(k,&self.installed_layouts,
                    verified).map(|l|l.id.clone());
                if let Some(id)=matched {
                    let label=if is_x11 {
                        "Use configured X11 layout as target"
                    } else { "Use reported active layout as target" };
                    if ui.add_enabled(self.session.state != State::Running,
                        egui::Button::new(label)).clicked() {
                        self.prefs.layout=id;
                        self.emulator=Emulator::compile(&self.prefs.layout).ok();
                        self.save();
                        self.message=if is_x11 {
                            "Emulation target updated from the single configured X11 layout; the active input group is unverified and system settings are unchanged.".into()
                        } else {
                            "Emulation target updated from reported Hyprland metadata; system settings unchanged.".into()
                        };
                    }
                } else if !verified {
                    ui.small("Inspector metadata is unverified or refreshing; wait for a fresh result before adopting a layout target.");
                } else if is_wayland {
                    ui.small("This aggregate Wayland seat cannot identify or select a physical keyboard's layout.");
                } else if is_x11 {
                    ui.small("No unique installed layout for this seat; multiple X11 groups cannot be treated as one active layout.");
                } else {
                    ui.small("No unique installed XKB match for this keyboard's active map.");
                }
            } else if pending {
                ui.label("Inspecting keyboard seats; no previous inventory is available.");
            } else {
                ui.label(self.device_error.as_deref().unwrap_or("No inspected keyboard seats available."));
            }
        });
        ui.small("Inspection only. Neither Wayland seat-level typing nor X11 configuration metadata provides per-device key attribution.");
    }
    fn start_weak_drill(&mut self, drill:String, ctx:&egui::Context) {
        self.custom_text=drill;
        self.prefs.test_mode=TestMode::Custom;
        self.rebuild_session();
        self.save();
        self.start_test(ctx);
        self.message="Practice drill started using missed reference patterns. Text stays in memory only.".into();
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
                if let Some(drill)=self.session.weak_bigram_drill() {
                    if ui.button("Practice missed pairs").clicked() {
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
        if self.session.state==State::Running && self.mode==Mode::Emulated
            && self.composition.as_ref().is_some_and(|state|
                state.status()==xkbcommon::xkb::compose::Status::Composing) {
            ui.colored_label(accent_ink(ui.visuals().dark_mode),
                "Accent pending: type the next key, or press Backspace to cancel it.");
        }
        let score = self.session.score();
        ui.horizontal_wrapped(|ui| {
            if self.session.has_cjk_reference() {
                ui.label(RichText::new(format!("{:.0} CPM",score.cpm)).size(23.0)
                    .color(accent_ink(ui.visuals().dark_mode)).strong())
                    .on_hover_text("Correct committed reference graphemes per minute, not IME input keystrokes. No cross-language proficiency equivalence.");
                ui.label(format!("{:.1} standardized WPM",score.wpm))
                    .on_hover_text("Correct graphemes divided by five per minute; diagnostic for CJK custom passages, not directly comparable with English/Spanish benchmarks.");
            } else {
                ui.label(RichText::new(format!("{:.0} WPM", score.wpm)).size(23.0)
                    .color(accent_ink(ui.visuals().dark_mode)).strong())
                    .on_hover_text("Standard WPM = correct extended grapheme clusters / 5 / elapsed minutes. English and Mexican Spanish benchmarks remain separate.");
            }
            ui.label(format!("{:.1}% accuracy", score.accuracy));
            ui.label(format!("{:.0} raw WPM", score.raw_wpm))
                .on_hover_text("Raw WPM = attempted Unicode scalars / 5 / elapsed minutes. It is diagnostic, not a cross-language or cross-input-method benchmark.");
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
                let total=self.session.target.graphemes(true).count().max(1);
                (self.session.typed.graphemes(true).count() as f32 / total as f32)
                    .clamp(0.0,1.0)
            } else {
                (1.0 - self.session.seconds_left() / self.session.duration.as_secs_f64()) as f32
            }
        ).show_percentage());
        ui.add_space(8.0);
        let entered = self.session.typed.graphemes(true).count();
        let total = self.session.target.graphemes(true).count();
        let (first, last) = crate::engine::viewport(total, entered, 230);
        let job=reference_layout_job(&self.session.target,&self.session.typed,
            first,last,self.session.state==State::Running,ui.visuals().dark_mode);
        let typing_area = egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_height(135.0);
                ui.label(job);
                if !self.ime_preedit.is_empty() && self.session.state==State::Running
                    && self.mode==Mode::System {
                    ui.colored_label(accent_ink(ui.visuals().dark_mode),
                        format!("Composing (not scored): {}",self.ime_preedit));
                }
                ui.small(format!("Reference graphemes {}–{} of {} · Click here to refocus or press F5 to restart.", first.saturating_add(1), last, total));
            });
        let response = ui.interact(
            typing_area.response.rect,
            egui::Id::new("typing_surface"),
            egui::Sense::click()
        );
        if response.clicked() { response.request_focus(); }
        if self.session.state==State::Running
            && self.mode==Mode::System
            && ctx.memory(|m|m.has_focus(egui::Id::new("typing_surface"))) {
            // This focus-owned custom typing surface is an IME text target,
            // even though it is not an editable TextEdit widget. IME popup
            // positioning is approximate until caret/RTL QA is accepted.
            let rect=typing_area.response.rect;
            let cursor=egui::Rect::from_min_size(
                rect.left_top()+egui::vec2(16.0,16.0),
                egui::vec2(1.0,22.0));
            ctx.output_mut(|out|out.ime=Some(egui::output::IMEOutput{
                rect,cursor_rect:cursor
            }));
        }
        if self.session.state == State::Running { ctx.request_repaint_after(Duration::from_millis(50)); }
    }
    /// Render any supported ANSI physical position consistently, including
    /// real XKB dead keys and unavailable symbols. This is not ISO/JIS geometry.
    fn show_preview_key(&self,ui:&mut egui::Ui,key:egui::Key,shift:bool,alt_gr:bool,width:f32) {
        let custom=self.prefs.custom_layouts.iter().find(|l|l.id==self.prefs.layout);
        let mapped=if let Some(personal)=custom {
            if alt_gr {None} else {personal.translate(key,shift)}
        } else if let Some(emulator)=&self.emulator {
            emulator.translate_layer(key,shift,alt_gr)
        } else if alt_gr {
            None
        } else {
            layouts::emulate(key,&self.prefs.layout,shift).map(|c|c.to_string())
        };
        let dead=if mapped.is_none() && custom.is_none() {
            self.emulator.as_ref().and_then(|xkb|xkb.dead_key_name(key,shift,alt_gr))
        } else {None};
        let value=mapped.unwrap_or_else(||if dead.is_some() {"dk".into()} else {"–".into()});
        let value=if key==egui::Key::Space && value==" " {"Space".to_string()} else {value};
        let missed=if value.chars().count()==1 {
            value.chars().next().map(|c|self.session.missed_character_count(c)).unwrap_or(0)
        } else {0};
        let ink=if missed>0 {error_ink(ui.visuals().dark_mode)} else {ui.visuals().text_color()};
        let mut frame=egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(7,7));
        if missed>0 {frame=frame.fill(key_heat_fill(missed,ui.visuals().dark_mode));}
        let cell=frame.show(ui,|ui| {
            let finger=layouts::finger_hint(key).unwrap_or("·");
            // The miss count is legible without hovering and remains present
            // even when a user cannot distinguish the red heatmap shades.
            let note=if missed>0 {format!("{finger} · {missed}×")}
                else {finger.to_string()};
            ui.add_sized([width,43.0],egui::Label::new(
                RichText::new(format!("{value}\n{note}")).monospace().size(13.0).color(ink)
            ).selectable(false));
        });
        if let Some(name)=dead {
            cell.response.on_hover_text(format!(
                "ANSI position {key:?}: XKB dead key {name}. Composes with the next matching key when a locale Compose table is installed."
            ));
        } else if missed>0 {
            cell.response.on_hover_text(format!(
                "ANSI position {key:?}: {missed} missed attempt(s) in the current session"
            ));
        } else if let Some(ms)=value.chars().next().and_then(|ch|self.session.average_key_latency_ms(ch)) {
            cell.response.on_hover_text(format!(
                "ANSI position {key:?}: {ms:.0} ms average interval for this character; session only"
            ));
        } else {
            cell.response.on_hover_text(format!("ANSI physical position {key:?}"));
        }
    }
    fn show_preview_extra(&self,ui:&mut egui::Ui,slot:&Slot,shift:bool,alt_gr:bool,width:f32) {
        let personal=self.prefs.custom_layouts.iter().any(|l|l.id==self.prefs.layout);
        let inspect_xkb=slot.kind==SlotKind::Extra && !personal;
        let mapped=if !inspect_xkb {None} else {
            self.emulator.as_ref().and_then(|xkb|
                xkb.translate_evdev_layer(slot.evdev,shift,alt_gr))
        };
        let dead=if !inspect_xkb {None} else {
            self.emulator.as_ref().and_then(|xkb|
                xkb.dead_evdev_name(slot.evdev,shift,alt_gr))
        };
        let symbol=match slot.kind {
            SlotKind::Control|SlotKind::Modifier=>slot.caption.to_string(),
            SlotKind::Decoration=>"Space".to_string(),
            SlotKind::Extra=>mapped.unwrap_or_else(||if dead.is_some() {"dk".into()} else {"–".into()}),
            SlotKind::Character=>unreachable!("normal keys use show_preview_key"),
        };
        let frame=egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(7,7));
        let cell=frame.show(ui,|ui|{
            ui.add_sized([width,43.0],egui::Label::new(
                RichText::new(format!("{symbol}\n·")).monospace().size(13.0)
            ).selectable(false));
        });
        let note=match slot.kind {
            SlotKind::Decoration=>
                "Illustrative second space half; not a second physical input event.".to_string(),
            SlotKind::Control|SlotKind::Modifier=>
                format!("{} · Linux evdev {} · illustrated control/modifier; no physical-key capture",
                    slot.name,slot.evdev),
            SlotKind::Extra=>
                format!("{} · Linux evdev {} · XKB {} · no attributed input event{}",
                    slot.name,slot.evdev,self.prefs.layout,
                    dead.as_deref().map(|name|format!(" · dead key: {name}")).unwrap_or_default()),
            SlotKind::Character=>unreachable!("normal keys use show_preview_key"),
        };
        cell.response.on_hover_text(note);
    }
    fn show_layout_preview(&mut self,ui:&mut egui::Ui) {
        ui.heading("Keyboard mapping preview");
        ui.horizontal_wrapped(|ui|{
            ui.label("Modifier layer (F3 cycles)");
            egui::ComboBox::from_id_salt("preview-modifier-layer")
                .selected_text(self.preview_layer.label()).show_ui(ui,|ui|{
                    for layer in PreviewLayer::ALL {
                        ui.selectable_value(&mut self.preview_layer,layer,layer.label());
                    }
                });
            ui.label("Geometry (F4)");
            egui::ComboBox::from_id_salt("preview-keyboard-shape")
                .selected_text(self.preview_shape.label()).show_ui(ui,|ui|{
                    for shape in Shape::ALL {
                        ui.selectable_value(&mut self.preview_shape,shape,shape.label());
                    }
                });
            ui.label("Keys (F10)");
            egui::ComboBox::from_id_salt("preview-keyboard-section")
                .selected_text(self.preview_section.label()).show_ui(ui,|ui|{
                    for section in Section::ALL {
                        ui.selectable_value(&mut self.preview_section,section,section.label());
                    }
                });
        });
        let shift=self.preview_layer.shift();
        let alt_gr=self.preview_layer.alt_gr();
        ui.small(format!("Selected XKB target: {} · {} · {}",
            self.prefs.layout,self.preview_shape.label(),
            if self.preview_section==Section::Typing {
                self.preview_shape.note()
            } else {
                "generic Linux reference positions; not detected or routed input"
            }));
        ui.add_space(6.0);
        let slots=geometry::section_slots(self.preview_shape,self.preview_section);
        // Each row is positioned in tenth-key units without inserting
        // per-key item gaps. A horizontal scroll retains full geometry at
        // narrow viewport widths rather than silently compressing keycaps.
        egui::ScrollArea::horizontal().id_salt("geometry-preview").show(ui,|ui|{
            for row in 0..=4_u8 {
                ui.horizontal(|ui|{
                    // Geometry uses absolute tenth-key units: do not add
                    // egui's ordinary inter-widget spacing to every slot.
                    // That padding previously accumulated across the row,
                    // pushing ISO/JIS positions outside their model bounds.
                    ui.spacing_mut().item_spacing.x=0.0;
                    const UNIT:f32=geometry::PREVIEW_UNIT_PX;
                    let mut prev_end=0_u16;
                    for slot in slots.iter().filter(|s|s.row==row) {
                        let gap=slot.x.saturating_sub(prev_end) as f32*UNIT;
                        if gap>0.0 {ui.add_space(gap);}
                        let width=(slot.width as f32*UNIT-14.0).max(24.0);
                        if let Some(key)=slot.egui_key {
                            self.show_preview_key(ui,key,shift,alt_gr,width);
                        } else {
                            self.show_preview_extra(ui,slot,shift,alt_gr,width);
                        }
                        prev_end=slot.x+slot.width as u16;
                    }
                });
                ui.add_space(2.0);
            }
        });
        ui.small("Main typing keys show XKB symbols; function/navigation and keypad keys show Linux reference labels only. None are independently captured hardware events.");
        ui.small("dk = XKB dead key (hover for accent name); – = unknown/unmapped. Space and modifier captions are readable text. AltGr inspection is not physical AltGr attribution.");
        let slow=self.session.slowest_keys();
        if !slow.is_empty(){
            ui.small("Slower single-key intervals (current session only; ≥2 samples each):");
            for (ch,ms) in slow.iter().take(5){
                ui.label(format!("{} · {:.0} ms average between committed keys",ch,ms));
            }
        }
        ui.small("In-session heatmap: red keycaps mark missed reference characters; the count follows the finger hint as N×. No key events are attributed to a physical keyboard. Full ISO/JIS event routing, IMEs and live Hyprland verification remain separate work.");
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
        if self.settings_read_only {
            self.message="Profile and statistics exports are disabled while original settings are protected for recovery.".into();
            return;
        }
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
        if self.settings_read_only {
            self.message="Profile and statistics exports are disabled while original settings are protected for recovery.".into();
            return;
        }
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
        if self.settings_read_only {
            self.message="Profile and statistics exports are disabled while original settings are protected for recovery.".into();
            return;
        }
        let profile=&self.prefs.profiles[self.prefs.active_profile];
        self.message=match export::save_stats_json(profile) {
            Ok(path)=>format!("Exported all saved sessions: {}",path.display()),
            Err(err)=>format!("Statistics export failed: {err}"),
        };
    }
    fn export_full_backup(&mut self) {
        if self.settings_read_only {
            self.message="Full backup of a temporary fallback profile is disabled. Preserve the original settings file and its existing database for recovery.".into();
            return;
        }
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
    fn show_backup_verifier(&mut self,ui:&mut egui::Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("Verify private backup (read-only)").strong());
            ui.small("Paste an F8 settings-backup file path; the companion SQLite filename is inferred. Nothing is restored or overwritten.");
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.backup_settings_path)
                    .hint_text(".../typefinch-backup-...-settings.json")
                    .desired_width(ui.available_width().min(490.0)));
                if ui.button("Verify backup pair").clicked() {
                    let input=std::path::Path::new(self.backup_settings_path.trim());
                    self.backup_check_result=match backup_verify::paired_database_path(input) {
                        Ok(database)=>match backup_verify::verify(input,&database) {
                            Ok(summary)=>format!("Verified: {} profiles, {} sessions, SQLite v{}. No files changed.",
                                summary.profiles,summary.sessions,summary.schema),
                            Err(err)=>format!("Backup rejected: {err}"),
                        },
                        Err(err)=>format!("Backup rejected: {err}"),
                    };
                }
            });
            if !self.backup_check_result.is_empty() {
                ui.label(&self.backup_check_result);
            }
            ui.small("Only a matching, independently preserved settings + SQLite pair is valid; this check never repairs damaged files.");
        });
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
        if !self.settings_read_only {
            ui.add_space(10.0);
            self.show_backup_verifier(ui);
        }
    }
}
impl eframe::App for Typefinch {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let focused=ctx.input(|i|i.focused);
        if self.last_window_focused && !focused {
            self.device_inventory_fresh=false;
            self.pending_device_obsolete=self.pending_device_query.is_some();
            let safer=layout_after_focus_change(
                &self.session_layout,self.mode,self.last_window_focused,focused);
            if safer!=self.session_layout && self.session.state==State::Running {
                self.session_layout=safer;
                self.message="Session layout attribution set to unknown after focus loss; no physical keyboard is inferred.".into();
            }
        }
        self.poll_keyboard_hotplug(ctx,focused);
        self.last_window_focused=focused;
        let theme_cycle=ctx.input(|i|
            pressed_key_modifiers(&i.events,egui::Key::T)
                .is_some_and(|modifiers|modifiers.ctrl && modifiers.shift));
        if theme_cycle && self.session.state!=State::Running && !self.settings_read_only {
            self.prefs.theme=next_theme(self.prefs.theme);
            ctx.set_theme(match self.prefs.theme {
                Theme::System=>egui::ThemePreference::System,
                Theme::Light=>egui::ThemePreference::Light,
                Theme::Dark=>egui::ThemePreference::Dark,
            });
            self.save();
        }
        let f12=ctx.input(|i|pressed_key_modifiers(&i.events,egui::Key::F12));
        if let Some(modifiers)=f12.filter(|_|self.session.state!=State::Running) {
            if modifiers.shift {
                self.export_profile_goals();
            } else if !self.settings_read_only {
                ctx.memory_mut(|memory|
                    memory.request_focus(egui::Id::new("goals_import_path")));
                self.message="Enter a local goals JSON path, then press Enter to import into the selected profile.".into();
            }
        }
        let ctrl_n=ctx.input(|i|pressed_key_modifiers(&i.events,egui::Key::N)
            .is_some_and(|modifiers|modifiers.ctrl && !modifiers.shift));
        if ctrl_n && self.session.state!=State::Running && !self.settings_read_only {
            // Focus the ordinary profile editor. Creation still requires
            // Tab, then Enter/Space on its visible Create profile button.
            ctx.memory_mut(|memory|
                memory.request_focus(egui::Id::new("new_profile_name")));
        }
        let ctrl_o=ctx.input(|i|pressed_key_modifiers(&i.events,egui::Key::O)
            .is_some_and(|modifiers|modifiers.ctrl));
        if (ctrl_o || ctx.input(|i| i.key_pressed(egui::Key::F11)))
            && self.session.state!=State::Running && !self.settings_read_only {
            // Keyboard navigation to the ordinary editor, not a QA bypass.
            if self.prefs.test_mode!=TestMode::Custom {
                self.prefs.test_mode=TestMode::Custom;
                self.rebuild_session();
                self.save();
            }
            ctx.memory_mut(|memory|
                memory.request_focus(egui::Id::new("custom_import_path")));
            self.message="Enter a local UTF-8 file path and press Enter to import. The passage remains in memory only.".into();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_keyboard_help=!self.show_keyboard_help;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            ctx.memory_mut(|m|m.request_focus(egui::Id::new("typing_surface")));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            self.preview_layer=self.preview_layer.next();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F4)) {
            // Presentation-only navigation: never alters the OS layout or
            // the scoring/emulation target.
            self.preview_shape=self.preview_shape.next();
            if std::env::var_os("TYPEFINCH_QA_REPORT").is_some()
                && std::env::var_os("TYPEFINCH_QA_PREVIEW_SHAPE").is_some() {
                let _=crate::qa::write_geometry_report(
                    self.preview_shape.label(),&self.prefs.layout,
                    self.preview_section.label(),
                    &geometry::section_slots(self.preview_shape,self.preview_section));
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F10)) {
            // Presentation-only section switching. No event capture,
            // input-mode change, or saved user preference.
            self.preview_section=self.preview_section.next();
            if std::env::var_os("TYPEFINCH_QA_REPORT").is_some()
                && std::env::var_os("TYPEFINCH_QA_PREVIEW_SHAPE").is_some() {
                let _=crate::qa::write_geometry_report(
                    self.preview_shape.label(),&self.prefs.layout,
                    self.preview_section.label(),
                    &geometry::section_slots(self.preview_shape,self.preview_section));
            }
        }
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
        if before==State::Running && !focused {
            // Cancellation wins over deadline on a window that lost focus:
            // an unobserved frame must not save a session as completed.
            self.apply_events(ctx);
        } else {
            // Close the elapsed-time gate BEFORE consuming this frame's key
            // events. The old ordering accepted late input in delayed frames.
            self.session.tick();
            self.apply_events(ctx);
            self.session.tick();
        }
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
                        ui.label(RichText::new("Typefinch").size(28.0).strong().color(accent_ink(ui.visuals().dark_mode)));
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
                            .default_open(false).show(ui, |ui| {
                                self.show_profile(ui);
                                ui.add_space(12.0);
                                self.show_settings(ui, ctx);
                                ui.add_space(12.0);
                                self.show_devices(ui);
                            });
                        ui.add_space(18.0);
                    }
                    ui.add_space(8.0);
                    if self.settings_read_only {
                        ui.group(|ui| {
                            ui.colored_label(error_ink(ui.visuals().dark_mode),
                                RichText::new("⚠ Settings recovery required — original file preserved").strong());
                            ui.label("This session is temporary: preference saves and scored-session recording are disabled to avoid replacing or orphaning existing data.");
                            ui.small("Repair settings.json or restore a valid matching backup. Do not delete the existing database.");
                            self.show_backup_verifier(ui);
                        });
                        ui.add_space(12.0);
                    }
                    ui.horizontal(|ui| {
                        if ui.button(if self.show_keyboard_help {"Hide keyboard guide (F1)"} else {"Keyboard guide (F1)"}).clicked() {
                            self.show_keyboard_help=!self.show_keyboard_help;
                        }
                    });
                    if self.show_keyboard_help {
                        ui.group(|ui| {
                            ui.strong("Keyboard-only navigation");
                            ui.label("Tab / Shift+Tab: move focus through controls; Enter / Space: activate the focused control.");
                            ui.label("F1: guide · F2: typing focus · F3: modifier layer · F4: keyboard shape · F5: start/restart · Escape: cancel.");
                            ui.label("Ctrl+N: new profile name (Tab, Enter) · Ctrl+Shift+T: cycle System/Light/Dark theme.");
                            ui.label("F11 / Ctrl+O: custom passage path; F12: profile goals import; Shift+F12: export goals.");
                            ui.label("F6: SVG + Markdown · F7: history JSON · F8: private backup · F9: PNG · F10: keyboard section.");
                            ui.label("When typing, the focus must be on the typing surface. Switching window focus cancels the current test without saving.");
                            ui.small("System Light/Dark follows your desktop unless overridden in settings. Reduce animations is optional.");
                        });
                        ui.add_space(12.0);
                    }
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
    #[test] fn rtl_reference_feedback_retains_logical_order_and_clusters() {
        use unicode_normalization::UnicodeNormalization;
        // The same normalization policy applies to Arabic vowel marks and
        // Hebrew niqqud without interpreting display order as input order.
        let arabic="سَلَام بالعربية";
        let base=reference_runs(arabic,"",0,arabic.graphemes(true).count(),false);
        assert_eq!(base.len(),1,"unentered connected script must be one text section");
        assert_eq!(base[0].text,arabic);
        assert_eq!(base[0].feedback,ReferenceFeedback::Unentered);
        let hebrew="שָׁלוֹם שלום";
        let partial="שָׁלוֹם ש";
        let runs=reference_runs(hebrew,partial,0,hebrew.graphemes(true).count(),true);
        assert_eq!(runs.iter().map(|r|r.text.as_str()).collect::<String>(),hebrew);
        assert_eq!(runs.first().unwrap().feedback,ReferenceFeedback::Correct);
        assert!(runs.iter().any(|r|r.feedback==ReferenceFeedback::Cursor));
        let decomposed="שָׁ";
        let composed=decomposed.nfc().collect::<String>();
        assert!(crate::engine::canonically_equal(decomposed,&composed));
    }
    #[test] fn actual_rtl_egui_layout_retains_text_and_highlight_ranges() {
        let target="abc سَلَام שָׁלוֹם 123";
        let entered="abc سx";
        for dark in [false,true] {
            let job=reference_layout_job(target,entered,0,
                target.graphemes(true).count(),true,dark);
            assert_eq!(job.text,target,"rendered text cannot be reversed");
            assert!(!job.sections.is_empty());
            let spans=job.sections.iter().map(|s|
                (&job.text[s.byte_range.clone()],s.format.color)).collect::<Vec<_>>();
            assert_eq!(spans.iter().map(|(text,_)|*text).collect::<String>(),target);
            assert!(spans.iter().any(|(_,color)|*color==correct_ink(dark)));
            assert!(spans.iter().any(|(_,color)|*color==error_ink(dark)));
            assert!(spans.iter().any(|(_,color)|*color==accent_ink(dark)));
            assert!(spans.iter().any(|(_,color)|*color==reference_ink(dark)));
            assert_eq!(spans.last().unwrap().1,reference_ink(dark));
            // A completely unentered Arabic phrase is one shaping section
            // rather than separately formatted letters.
            let arabic="سَلَام";
            let blank=reference_layout_job(arabic,"",0,arabic.graphemes(true).count(),false,dark);
            assert_eq!(blank.text,arabic);
            assert_eq!(blank.sections.len(),1);
            assert_eq!(blank.sections[0].format.color,reference_ink(dark));
        }
    }
    #[test] fn mixed_script_errors_and_windows_never_reverse_reference() {
        let target="abc سلام שלום xyz";
        let erroneous="abc سx";
        let total=target.graphemes(true).count();
        let all=reference_runs(target,erroneous,0,total,true);
        assert_eq!(all.iter().map(|r|r.text.as_str()).collect::<String>(),target);
        assert!(all.iter().any(|r|r.feedback==ReferenceFeedback::Error));
        assert!(all.iter().any(|r|r.feedback==ReferenceFeedback::Cursor));
        assert_eq!(all.first().unwrap().feedback,ReferenceFeedback::Correct);
        let (start,end)=crate::engine::viewport(total,9,6);
        let subset=reference_runs(target,erroneous,start,end,true);
        let expected=target.graphemes(true).skip(start).take(end-start).collect::<String>();
        assert_eq!(subset.iter().map(|r|r.text.as_str()).collect::<String>(),expected);
        assert!(subset.iter().all(|r|!r.text.is_empty()));
        assert!(reference_runs(target,"",0,0,true).is_empty());
    }
    #[test] fn preview_modifier_cycle_is_deterministic_and_only_presentation() {
        let mut layer=PreviewLayer::Base;
        for expected in [PreviewLayer::Shift,PreviewLayer::AltGr,
            PreviewLayer::ShiftAltGr,PreviewLayer::Base] {
            layer=layer.next();
            assert_eq!(layer,expected);
        }
        assert_eq!(PreviewLayer::ALL.len(),4);
        assert!(PreviewLayer::AltGr.alt_gr());
        assert!(!PreviewLayer::Shift.alt_gr());
        assert!(PreviewLayer::ShiftAltGr.shift());
    }
    #[test] fn reduced_motion_preserves_normal_timing_when_disabled() {
        assert_eq!(motion_seconds(true,0.15),0.0);
        assert_eq!(motion_seconds(false,0.15),0.15);
        assert_eq!(motion_seconds(false,0.0),0.0);
    }
    #[test] fn disconnected_inspector_is_consumed_as_failure_not_stale_success() {
        let (sender,receiver)=mpsc::channel();
        let mut pending=Some(receiver);
        assert!(take_device_query_result(&mut pending).is_none());
        assert!(pending.is_some());
        drop(sender);
        let failure=take_device_query_result(&mut pending).unwrap();
        assert!(failure.unwrap_err().contains("stale inventory"));
        assert!(pending.is_none());
        let (sender,receiver)=mpsc::channel();
        let mut pending=Some(receiver);
        sender.send(Ok(vec![Keyboard{address:"valid".into(),..Default::default()}])).unwrap();
        let result=take_device_query_result(&mut pending).unwrap().unwrap();
        assert_eq!(result[0].address,"valid");
        assert!(pending.is_none());
    }
    #[test] fn keyboard_inspector_is_single_flight_and_never_blocks_the_ui_thread() {
        use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
        let counter=Arc::new(AtomicUsize::new(0));
        let (release,blocked)=mpsc::channel::<()>();
        let mut pending=None;
        let seen=Arc::clone(&counter);
        assert!(launch_device_inspection(&mut pending,None,move||{
            seen.fetch_add(1,Ordering::SeqCst);
            blocked.recv_timeout(Duration::from_secs(5))
                .map_err(|e|format!("test worker release: {e}"))?;
            Ok(vec![Keyboard{address:"first".into(),..Default::default()}])
        }));
        assert!(pending.is_some());
        let rejected=Arc::clone(&counter);
        assert!(!launch_device_inspection(&mut pending,None,move||{
            rejected.fetch_add(1,Ordering::SeqCst);
            Ok(vec![Keyboard{address:"second".into(),..Default::default()}])
        }));
        // The UI can still handle input while the first inspector is waiting.
        release.send(()).unwrap();
        let first=pending.take().unwrap().recv_timeout(Duration::from_secs(5))
            .unwrap().unwrap();
        assert_eq!(first[0].address,"first");
        assert_eq!(counter.load(Ordering::SeqCst),1);
        assert!(launch_device_inspection(&mut pending,None,||Ok(Vec::new())));
        assert!(pending.take().unwrap().recv_timeout(Duration::from_secs(5)).unwrap().unwrap().is_empty());
    }
    #[test] fn hyprland_hotplug_watch_is_bounded_focus_aware_and_opt_in_by_session() {
        assert!(!should_poll_devices(false,true,true,Duration::from_secs(30)));
        assert!(!should_poll_devices(true,false,true,Duration::from_secs(30)));
        assert!(should_poll_devices(true,true,true,Duration::ZERO));
        assert!(!should_poll_devices(true,true,false,Duration::from_secs(19)));
        assert!(should_poll_devices(true,true,false,Duration::from_secs(20)));
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
    #[test] fn wayland_seat_capability_does_not_carry_observed_layout_or_target() {
        let installed=vec![InstalledLayout{id:"us".into(),name:"English (US)".into()}];
        let seat=Keyboard{
            name:"Wayland seat seat0 (not a physical keyboard)".into(),
            address:"wayland:seat:seat0".into(),
            active_keymap:"English (US)".into(), // Malicious/stale metadata cannot promote it
            layout:"us".into(),
            ..Default::default()
        };
        assert!(inspected_target(&seat,&installed).is_none());
        assert_eq!(recorded_layout(Mode::System,"us",&[seat.clone()],&installed),
            "os:unknown");
        assert_eq!(recorded_layout(Mode::Emulated,"us",&[seat],&installed),"us");
    }
    #[test] fn ime_commits_score_once_while_noncommitted_events_do_not() {
        let mut session=Session::new_with_custom(0,TestMode::Custom,"你好世界");
        session.begin();
        let mut preedit=String::new();
        assert!(!score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Ime(egui::ImeEvent::Enabled)));
        assert!(!score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Ime(egui::ImeEvent::Preedit("ni".into()))));
        assert_eq!(preedit,"ni");
        assert_eq!(session.typed,"");
        assert!(!score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Ime(egui::ImeEvent::Preedit("你好".into()))));
        assert_eq!(preedit,"你好");
        assert!(!score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Paste("你好世界".into())));
        assert!(!score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Ime(egui::ImeEvent::Disabled)));
        assert!(preedit.is_empty());
        assert_eq!(session.attempts,0);
        assert!(score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Ime(egui::ImeEvent::Commit("你好".into()))));
        assert!(preedit.is_empty());
        assert_eq!(session.typed,"你好");
        assert_eq!(session.attempts,2);
        assert!(score_system_committed_event(&mut session,&mut preedit,
            &egui::Event::Text("世界".into())));
        assert_eq!(session.typed,"你好世界");
        assert_eq!(session.state,State::Finished);
        assert_eq!(session.score().correct,4);
    }
    #[test] fn ime_preedit_is_bounded_and_never_scores_candidates() {
        let mut session=Session::new_with_custom(0,TestMode::Custom,"你好");
        session.begin();
        let mut preview=String::new();
        let huge="你".repeat(8000);
        assert!(!score_system_committed_event(&mut session,&mut preview,
            &egui::Event::Ime(egui::ImeEvent::Preedit(huge))));
        assert_eq!(preview.graphemes(true).count(),MAX_IME_PREEDIT_GRAPHEMES);
        assert!(preview.len()<=MAX_IME_PREEDIT_BYTES);
        let huge_combining=format!("e{}", "\u{301}".repeat(10_000));
        assert!(!score_system_committed_event(&mut session,&mut preview,
            &egui::Event::Ime(egui::ImeEvent::Preedit(huge_combining))));
        assert!(preview.len()<=MAX_IME_PREEDIT_BYTES,
            "one giant combining grapheme must not bypass the preview byte cap");
        assert!(session.typed.is_empty());
        assert_eq!(session.attempts,0);
        assert!(!score_system_committed_event(&mut session,&mut preview,
            &egui::Event::Ime(egui::ImeEvent::Commit(String::new()))));
        assert!(preview.is_empty());
        assert_eq!(session.attempts,0);
    }
    #[test] fn emulated_key_repeat_and_modifier_shortcuts_are_not_scored() {
        let plain=egui::Modifiers::default();
        assert!(emulated_key_accepted(false,plain));
        assert!(!emulated_key_accepted(true,plain),
            "auto-repeat cannot inflate emulated physical-key scoring");
        assert!(emulated_key_accepted(false,egui::Modifiers{shift:true,..plain}),
            "Shift produces a legitimate new physical key position");
        assert!(!emulated_key_accepted(false,egui::Modifiers{ctrl:true,..plain}));
        assert!(!emulated_key_accepted(false,egui::Modifiers{alt:true,..plain}));
        assert!(!emulated_key_accepted(false,egui::Modifiers{command:true,..plain}));
        assert!(!emulated_key_accepted(true,egui::Modifiers{shift:true,..plain}));
    }
    #[test] fn keyboard_theme_cycle_keeps_explicit_system_light_dark_order() {
        let mut theme=Theme::System;
        for next in [Theme::Light,Theme::Dark,Theme::System] {
            theme=next_theme(theme);
            assert_eq!(theme,next);
        }
    }
    #[test] fn modified_shortcuts_preserve_keypress_modifiers_after_frame_release() {
        let held=egui::Modifiers { shift:true,..Default::default() };
        let press=egui::Event::Key {
            key:egui::Key::F12,physical_key:None,pressed:true,
            repeat:false,modifiers:held,
        };
        let release=egui::Event::Key {
            key:egui::Key::F12,physical_key:None,pressed:false,
            repeat:false,modifiers:egui::Modifiers::default(),
        };
        assert_eq!(pressed_key_modifiers(&[press.clone(),release],egui::Key::F12),Some(held));
        assert_eq!(pressed_key_modifiers(&[press.clone()],egui::Key::O),None);
        let ctrl=egui::Modifiers { ctrl:true,..Default::default() };
        let ctrl_o=egui::Event::Key{
            key:egui::Key::O,physical_key:None,pressed:true,
            repeat:false,modifiers:ctrl,
        };
        assert_eq!(pressed_key_modifiers(&[ctrl_o],egui::Key::O),Some(ctrl));
        let repeat=egui::Event::Key {
            key:egui::Key::F12,physical_key:None,pressed:true,
            repeat:true,modifiers:held,
        };
        assert_eq!(pressed_key_modifiers(&[repeat],egui::Key::F12),None);
    }
    #[test] fn pending_inspection_never_certifies_new_session_or_target() {
        let installed=vec![InstalledLayout{id:"us".into(),name:"English (US)".into()}];
        let keyboard=Keyboard{
            address:"0x1".into(),active_keymap:"English (US)".into(),
            ..Default::default()
        };
        let keyboards=vec![keyboard.clone()];
        assert!(inspection_confirmed(true,false));
        assert!(!inspection_confirmed(true,true),
            "a pending refresh invalidates adoption of old metadata");
        assert!(!inspection_confirmed(false,false));
        assert!(!inspection_confirmed(false,true));
        assert_eq!(layout_for_new_session(Mode::System,"us",&keyboards,&installed,
            inspection_confirmed(true,true)),"os:unknown");
        assert!(inspected_target_if_fresh(&keyboard,&installed,
            inspection_confirmed(true,true)).is_none());
        assert_eq!(layout_for_new_session(Mode::System,"us",&keyboards,&installed,
            inspection_confirmed(true,false)),"detected/us");
        assert_eq!(layout_for_new_session(Mode::Emulated,"us",&keyboards,&installed,
            inspection_confirmed(false,true)),"us");
    }
    #[test] fn stale_inspector_rows_are_never_displayed_as_current_reports() {
        let hyprland=Keyboard{
            name:"USB keyboard".into(),address:"0x1".into(),
            active_keymap:"English (US)".into(),layout:"us".into(),
            ..Default::default()
        };
        assert!(inspected_row_summary(&hyprland,true).starts_with("Reported active map:"));
        assert!(inspected_row_summary(&hyprland,false)
            .starts_with("Last reported active map (unverified):"));
        let x11=Keyboard{
            address:X11_AGGREGATE_SEAT.into(),layout:"us".into(),
            ..Default::default()
        };
        assert!(inspected_row_summary(&x11,true).starts_with("Configured X11 seat layout:"));
        assert!(inspected_row_summary(&x11,false)
            .starts_with("Last configured X11 seat layout (unverified):"));
        let wayland=Keyboard{
            address:"wayland:seat:42".into(),name:"seat0".into(),..Default::default()
        };
        assert!(inspected_row_summary(&wayland,true)
            .starts_with("Wayland seat reports keyboard capability."));
        assert!(inspected_row_summary(&wayland,false).contains("(unverified)"));
        assert!(!inspected_row_summary(&wayland,true).contains("physical keyboard"));
    }
    #[test] fn layout_preview_matches_new_session_provenance_when_refresh_pending() {
        let installed=vec![InstalledLayout{id:"us".into(),name:"English (US)".into()}];
        let found=vec![Keyboard{
            address:"0x1".into(),active_keymap:"English (US)".into(),
            ..Default::default()
        }];
        for (fresh,pending,expected) in [
            (true,false,"detected/us"),
            (true,true,"os:unknown"),
            (false,false,"os:unknown"),
            (false,true,"os:unknown")
        ] {
            let preview=layout_for_new_session(Mode::System,"us",&found,&installed,
                inspection_confirmed(fresh,pending));
            assert_eq!(preview,expected);
            assert_eq!(preview,layout_for_new_session(Mode::System,"us",&found,&installed,
                inspection_confirmed(fresh,pending)),
                "the displayed next-session layout must use scoring provenance");
        }
        assert_eq!(layout_for_new_session(Mode::Emulated,"us",&found,&installed,
            inspection_confirmed(false,true)),"us");
    }
    #[test] fn completing_a_session_during_refresh_cannot_persist_stale_layout() {
        assert_eq!(layout_at_completion("detected/us",Mode::System,true,false),
            "detected/us");
        assert_eq!(layout_at_completion("detected/us",Mode::System,true,true),
            "os:unknown","pending refresh may contradict active input layout");
        assert_eq!(layout_at_completion("detected/us",Mode::System,false,false),
            "os:unknown");
        assert_eq!(layout_at_completion("os:unknown",Mode::System,true,false),
            "os:unknown","an unknown label cannot be upgraded after finish");
        assert_eq!(layout_at_completion("us",Mode::Emulated,false,true),
            "us","explicit software emulation is not derived from inspection");
    }
    #[test] fn stale_inspector_inventory_cannot_supply_emulation_target() {
        let installed=vec![InstalledLayout{id:"us".into(),name:"English (US)".into()}];
        let configured=Keyboard{
            address:X11_AGGREGATE_SEAT.into(),layout:"us".into(),
            ..Default::default()
        };
        assert!(inspected_target_if_fresh(&configured,&installed,false).is_none());
        assert_eq!(inspected_target_if_fresh(&configured,&installed,true).unwrap().id,"us");
        let hyprland=Keyboard{
            address:"0x1".into(),active_keymap:"English (US)".into(),
            ..Default::default()
        };
        assert!(inspected_target_if_fresh(&hyprland,&installed,false).is_none());
        assert_eq!(inspected_target_if_fresh(&hyprland,&installed,true).unwrap().id,"us");
        let wayland=Keyboard{address:"wayland:seat:10".into(),..hyprland};
        assert!(inspected_target_if_fresh(&wayland,&installed,true).is_none());
    }
    #[test] fn x11_configured_groups_are_never_claimed_as_observed_session_input() {
        let xkb=vec![
            InstalledLayout{id:"us".into(),name:"English (US)".into()},
            InstalledLayout{id:"de(nodeadkeys)".into(),name:"German (no dead keys)".into()},
        ];
        let single=Keyboard {
            name:"X11 aggregate seat (not a physical keyboard)".into(),
            address:X11_AGGREGATE_SEAT.into(),
            layout:"us".into(),active_keymap:"English (US)".into(),..Default::default()
        };
        // Even when the installed registry matches the configured ID, X11
        // does not report which group actually generated the key events.
        assert_eq!(recorded_layout(Mode::System,"us",&[single.clone()],&xkb),"os:unknown");
        assert_eq!(inspected_target(&single,&xkb).unwrap().id,"us");
        let variant=Keyboard{layout:"de".into(),variant:"nodeadkeys".into(),..single.clone()};
        assert_eq!(inspected_target(&variant,&xkb).unwrap().id,"de(nodeadkeys)");
        for (layout,variant) in [("us,de",""),("us",",nodeadkeys"),("missing",""),("","")] {
            let seat=Keyboard{layout:layout.into(),variant:variant.into(),..single.clone()};
            assert!(inspected_target(&seat,&xkb).is_none());
            assert_eq!(recorded_layout(Mode::System,"us",&[seat],&xkb),"os:unknown");
        }
        assert_eq!(recorded_layout(Mode::Emulated,"us",&[single],&xkb),"us");
    }
    #[test] fn focus_changes_never_promote_stale_keyboard_metadata_to_session_provenance() {
        let installed=vec![InstalledLayout{id:"us".into(),name:"English (US)".into()}];
        let keyboards=vec![Keyboard{
            address:"0x1".into(),active_keymap:"English (US)".into(),
            ..Default::default()
        }];
        assert_eq!(layout_for_new_session(Mode::System,"us",
            &keyboards,&installed,true),"detected/us");
        assert_eq!(layout_for_new_session(Mode::System,"us",
            &keyboards,&installed,false),"os:unknown");
        assert_eq!(layout_for_new_session(Mode::Emulated,"us",
            &keyboards,&installed,false),"us");
        assert_eq!(layout_after_focus_change(
            "detected/us",Mode::System,true,false),"os:unknown");
        assert_eq!(layout_after_focus_change(
            "detected/us",Mode::System,true,true),"detected/us");
        assert_eq!(layout_after_focus_change(
            "os:unknown",Mode::System,true,false),"os:unknown");
        assert_eq!(layout_after_focus_change(
            "us",Mode::Emulated,true,false),"us");
    }
    #[test] fn active_session_layout_is_downgraded_not_relabelled_on_hotplug() {
        let installed=vec![
            InstalledLayout{id:"us".into(),name:"English (US)".into()},
            InstalledLayout{id:"de".into(),name:"German".into()},
        ];
        let us=Keyboard{
            address:"first".into(),active_keymap:"English (US)".into(),
            ..Default::default()
        };
        let de=Keyboard{
            address:"second".into(),active_keymap:"German".into(),
            ..Default::default()
        };
        // Adding an identical mapping or changing inspection order alone
        // does not invalidate the original aggregate layout identity.
        let us2=Keyboard{address:"second".into(),..us.clone()};
        assert_eq!(validated_session_layout("detected/us",Mode::System,
            &[us2,us.clone()],&installed),"detected/us");
        // A newly attached second keyboard with a different keymap makes
        // the original seat-level claim unsafe, even if one device was selected.
        assert_eq!(validated_session_layout("detected/us",Mode::System,
            &[us.clone(),de.clone()],&installed),"os:unknown");
        assert_eq!(validated_session_layout("detected/us",Mode::System,
            &[],&installed),"os:unknown");
        // Unknown layout cannot be opportunistically upgraded mid-session.
        assert_eq!(validated_session_layout("os:unknown",Mode::System,
            &[us.clone()],&installed),"os:unknown");
        // An explicit per-session emulation target does not follow device
        // metadata or a selected inspection row.
        assert_eq!(validated_session_layout("us(colemak)",Mode::Emulated,
            &[us,de],&installed),"us(colemak)");
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
        assert!(contrast(selection_text(false),selection_fill(false))>=4.5);
        assert!(contrast(selection_text(true),selection_fill(true))>=4.5);
        assert!(contrast(PERIWINKLE,light)<4.5); // Never use accent swatch as white-background text.
    }
}
