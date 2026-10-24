//! Deterministic, local typing-session scoring. No browser/network dependency.
use std::{collections::BTreeMap, time::{Duration, Instant}};
use serde::{Serialize, Deserialize};
use unicode_normalization::{UnicodeNormalization, char::canonical_combining_class};
use unicode_segmentation::UnicodeSegmentation;

// A standard typing WPM unit is five extended grapheme clusters, not a
// natural-language lexical word. Apply this identically to en and es-MX,
// and partition benchmark histories by language/mode instead of mixing them.
pub const STANDARD_GRAPHEMES_PER_WORD:f64=5.0;

/// Compare extended grapheme clusters after Unicode canonical normalization.
pub fn canonically_equal(a:&str,b:&str)->bool {a.nfc().eq(b.nfc())}
fn canonical_prefix(expected:&str,entered:&str)->bool {
    let normalized=expected.nfd().collect::<String>();
    let prefix=entered.nfd().collect::<String>();
    normalized.starts_with(&prefix)
}
fn grapheme_count(text:&str)->usize {text.graphemes(true).count()}
fn extends_previous_grapheme(ch:char, typed:&str)->bool {
    let value=ch as u32;
    canonical_combining_class(ch)>0
        || matches!(value,0x200D|0x20E3|0xFE0E|0xFE0F)
        || (0x1F3FB..=0x1F3FF).contains(&value)
        || typed.ends_with('\u{200D}')
        || ((0x1F1E6..=0x1F1FF).contains(&value)
            && typed.chars().last().is_some_and(|c|(0x1F1E6..=0x1F1FF).contains(&(c as u32))))
}

/// Parse a local custom passage in bounded memory; the bytes are never logged.
/// Both drag-and-drop and the keyboard-operated path field use the same
/// strict local file import route. No selected path or content is persisted.
pub fn import_custom_file(path:&std::path::Path)->Result<String,String> {
    use std::io::Read;
    let metadata=std::fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !metadata.file_type().is_file() {
        return Err("Choose a regular file (not a directory, device or symbolic link)".into());
    }
    if metadata.len()>65_536 {return Err("Text file exceeds 64 KiB".into());}
    let mut bytes=Vec::new();
    std::fs::File::open(path).map_err(|e|e.to_string())?
        .take(65_537).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    import_custom_passage(&bytes)
}
pub fn import_custom_passage(bytes:&[u8])->Result<String,String> {
    if bytes.len()>65_536 {return Err("Text file exceeds 64 KiB".into());}
    let text=std::str::from_utf8(bytes).map_err(|_|"Only UTF-8 text files are supported")?;
    if text.chars().any(|c|c.is_control() && !matches!(c,'\n'|'\r'|'\t')) {
        return Err("File contains unsupported control characters".into());
    }
    let result=text.graphemes(true).take(4096).collect::<String>();
    if result.split_whitespace().next().is_none() {
        return Err("Text file is empty or whitespace-only".into());
    }
    Ok(result)
}

pub const WORDS: &str = "little finches practice every day with patient rhythm gentle wings make steady progress while fingers learn their keys accuracy grows through focused practice kind habits build confident typing and each new session brings another chance to improve";
pub const WORDS_ES_MX: &str = "casa camino tiempo palabra ritmo teclado aprender práctica esfuerzo paciencia sonrisa alegría mañana corazón canción rápido lento montaña jardín sueño pequeño grande trabajo comunidad familia amistad libro ventana pueblo memoria historia ciudad México español niña niño café azúcar limón día noche tarde cielo lluvia viento nueva meta claridad futuro";
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum WordDifficulty { Easy, #[default] Standard, Challenge }
impl WordDifficulty {
    pub const ALL:[Self;3]=[Self::Easy,Self::Standard,Self::Challenge];
    pub fn label(self)->&'static str {
        match self {Self::Easy=>"Easy",Self::Standard=>"Standard",Self::Challenge=>"Challenge"}
    }
    pub fn key(self)->&'static str {
        match self {Self::Easy=>"easy",Self::Standard=>"standard",Self::Challenge=>"challenge"}
    }
}
pub const WORDS_EN_EASY:&str="cat dog sun moon lake bird tree home game book fish day play warm cool fast calm word note kind time rain road blue green run walk look see read";
pub const WORDS_ES_MX_EASY:&str="casa mesa gato perro pato agua cielo campo mano libro amigo juego flor sol luna patio joven leer día tarde noche comer caminar";
pub const WORDS_EN_CHALLENGE:&str="synchronization perseverance extraordinary pronunciation configuration responsibility telecommunications architectural ambiguity interdisciplinary experimentation mathematical";
pub const WORDS_ES_MX_CHALLENGE:&str="extraordinariamente responsabilidad pronunciación comunicación configuración investigación interdisciplinario arquitectura interpretación colaboración organización comprensión";
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename="en")]
    English,
    #[serde(rename="es-MX")]
    MexicanSpanish,
}
impl Language {
    pub const ALL:[Language;2]=[Self::English,Self::MexicanSpanish];
    pub fn label(self)->&'static str {
        match self {Self::English=>"English",Self::MexicanSpanish=>"Español (México)"}
    }
    pub fn code(self)->&'static str {
        match self {Self::English=>"en",Self::MexicanSpanish=>"es-MX"}
    }
}


#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestMode { #[default] Words, Quotes, Numbers, Punctuation, Paragraphs, Custom, Lesson }
impl TestMode {
    pub const ALL: [TestMode;7] = [Self::Words, Self::Quotes, Self::Numbers, Self::Punctuation, Self::Paragraphs, Self::Custom, Self::Lesson];
    pub fn label(self) -> &'static str {
        match self { Self::Words=>"Words", Self::Quotes=>"Quotes", Self::Numbers=>"Numbers", Self::Punctuation=>"Punctuation", Self::Paragraphs=>"Paragraphs", Self::Custom=>"Custom text", Self::Lesson=>"Practice lesson" }
    }
    pub fn counts_for_benchmark(self) -> bool {
        !matches!(self, Self::Custom | Self::Lesson)
    }
    pub fn key(self) -> &'static str {
        match self { Self::Words=>"words", Self::Quotes=>"quotes", Self::Numbers=>"numbers", Self::Punctuation=>"punctuation", Self::Paragraphs=>"paragraphs", Self::Custom=>"custom", Self::Lesson=>"lesson" }
    }
}
fn corpus(mode: TestMode, language:Language, difficulty:WordDifficulty) -> String {
    let (text, repeats) = match mode {
        TestMode::Words => (match (language,difficulty) {
            (Language::English,WordDifficulty::Easy)=>WORDS_EN_EASY,
            (Language::English,WordDifficulty::Standard)=>WORDS,
            (Language::English,WordDifficulty::Challenge)=>WORDS_EN_CHALLENGE,
            (Language::MexicanSpanish,WordDifficulty::Easy)=>WORDS_ES_MX_EASY,
            (Language::MexicanSpanish,WordDifficulty::Standard)=>WORDS_ES_MX,
            (Language::MexicanSpanish,WordDifficulty::Challenge)=>WORDS_ES_MX_CHALLENGE,
        }, 8),
        TestMode::Quotes => (
            if language==Language::MexicanSpanish {
                "Con práctica constante, cada palabra encuentra su ritmo. Una mejora pequeña también cuenta."
            } else {
                "A small improvement, repeated carefully, becomes a remarkable skill. Every key is another opportunity to learn."
            },14),
        // Digits are locale-neutral in this starter corpus, but sessions remain
        // separate by the chosen language context for honest comparisons.
        TestMode::Numbers => ("17 42 63 85 91 24 56 78 30 49 12 75 64 83 29 50 04 68", 28),
        TestMode::Punctuation => (
            if language==Language::MexicanSpanish {
                "¡Hola, qué tal! ¿Todo bien? Sí: escribe comas, puntos; paréntesis (así), y preguntas."
            } else {
                "Wait, is that correct? Yes! Write clearly: commas, colons; parentheses (like these), and a final period."
            },13),
        TestMode::Paragraphs => (
            if language==Language::MexicanSpanish {
                "El ritmo al escribir se construye con paciencia. Cada intento ofrece una oportunidad para conocer mejor el teclado.\n\nLas palabras tienen acentos, signos y sonidos propios. Practicar sin prisa ayuda a reconocerlos con claridad."
            } else {
                "A bird learns to fly through careful repetition, just as a typist learns rhythm from patient practice. Every attempt records a small improvement.\n\nThe best progress is steady and deliberate: watch each symbol, keep the hands comfortable, and let consistency grow."
            },10),
        TestMode::Custom | TestMode::Lesson => ("",0),
    };
    std::iter::repeat_n(text, repeats).collect::<Vec<_>>().join(" ")
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum State { Ready, Running, Finished }

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct KeyTiming {
    pub count:u32,
    total_ms:u64,
}
impl KeyTiming {
    fn add(&mut self,interval:Duration) {
        let ms=interval.as_millis();
        if !(25..=2000).contains(&ms) {return;}
        self.count=self.count.saturating_add(1);
        self.total_ms=self.total_ms.saturating_add(ms as u64);
    }
    pub fn average_ms(self)->Option<f64> {
        (self.count>=2).then(||self.total_ms as f64/self.count as f64)
    }
}
pub struct Session {
    pub mode: TestMode,
    pub language: Language,
    pub difficulty: WordDifficulty,
    pub target: String,
    pub typed: String,
    pub duration: Duration,
    pub state: State,
    started: Option<Instant>,
    elapsed: Duration,
    pub corrections: u32,
    pub attempts: u32,
    pub mistakes: u32,
    weak_keys: BTreeMap<char,u32>,
    key_timings:BTreeMap<char,KeyTiming>,
    last_key_at:Option<Instant>,
    pending_completion: Option<Instant>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub wpm: f64,
    pub accuracy: f64,
    pub raw_wpm: f64,
    pub raw_accuracy: f64,
    pub attempts: usize,
    pub mistakes: usize,
    pub correct: usize,
    pub entered: usize,
    pub elapsed: Duration,
}

impl Session {
    pub fn new(seconds: u64) -> Self { Self::new_with_mode(seconds, TestMode::Words) }
    pub fn new_with_mode(seconds: u64, mode: TestMode) -> Self {
        Self::new_with_custom(seconds, mode, "")
    }
    pub fn new_with_custom(seconds: u64, mode: TestMode, custom: &str) -> Self {
        Self::new_with_language(seconds,mode,custom,Language::English)
    }
    pub fn new_with_language(seconds:u64, mode:TestMode, custom:&str, language:Language)->Self {
        Self::new_with_settings(seconds,mode,custom,language,WordDifficulty::Standard)
    }
    pub fn new_with_settings(seconds:u64, mode:TestMode, custom:&str,
        language:Language, difficulty:WordDifficulty)->Self {
        // Newline/whitespace are normalized to spaces so the target remains
        // typable by the existing committed-character pipeline.
        let source = if matches!(mode, TestMode::Custom | TestMode::Lesson) {
            custom.to_owned()
        } else {
            corpus(mode,language,difficulty)
        };
        let target = source.graphemes(true).take(4096).collect::<String>()
            .split_whitespace().collect::<Vec<_>>().join(" ");
        Self { mode, language, difficulty, target, typed:String::new(), duration:Duration::from_secs(seconds),
            state:State::Ready, started:None, elapsed:Duration::ZERO,
            corrections:0, attempts:0, mistakes:0, weak_keys:BTreeMap::new(),key_timings:BTreeMap::new(),last_key_at:None,pending_completion:None }
    }
    pub fn history_mode(&self)->String {
        if !self.mode.counts_for_benchmark() {return self.mode.key().to_owned();}
        let mut result=self.mode.key().to_owned();
        if self.language!=Language::English {
            result.push('/');
            result.push_str(self.language.code());
        }
        if self.mode==TestMode::Words && self.difficulty!=WordDifficulty::Standard {
            result.push('/');
            result.push_str(self.difficulty.key());
        }
        // The standard English context keeps its legacy ID in SQLite.
        result
    }
    pub fn begin(&mut self) {
        self.typed.clear();
        self.corrections = 0;
        self.attempts = 0;
        self.mistakes = 0;
        self.weak_keys.clear();
        self.key_timings.clear();
        self.last_key_at=None;
        self.pending_completion=None;
        self.elapsed = Duration::ZERO;
        self.started = Some(Instant::now());
        self.state = State::Running;
    }
    fn observe_interval(&mut self,key:char,correct:bool,now:Instant) {
        let last=self.last_key_at.replace(now);
        if correct && !key.is_control() && !key.is_whitespace() {
            if let Some(interval)=last.and_then(|when|now.checked_duration_since(when)) {
                self.key_timings.entry(key).or_default().add(interval);
            }
        }
    }
    /// Only display timing from at least two valid single-commit intervals.
    pub fn average_key_latency_ms(&self,key:char)->Option<f64> {
        self.key_timings.get(&key).and_then(|t|t.average_ms())
    }
    pub fn slowest_keys(&self)->Vec<(char,f64)> {
        let mut keys=self.key_timings.iter().filter_map(|(ch,t)|t.average_ms().map(|ms|(*ch,ms)))
            .collect::<Vec<_>>();
        keys.sort_by(|a,b|b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        keys
    }
    pub fn type_text(&mut self, text: &str) {
        if self.state != State::Running { return; }
        let single=text.chars().filter(|c|!c.is_control()).count()==1;
        if !single {self.last_key_at=None;}
        let total=grapheme_count(&self.target);
        for ch in text.chars().filter(|c|!c.is_control()) {
            let cursor=grapheme_count(&self.typed);
            // Allow a combining mark to extend the final grapheme, including
            // when its base character arrived in a separate Text event.
            let extension=extends_previous_grapheme(ch,&self.typed);
            if cursor>=total && !extension {break;}
            let position=if extension && cursor>0 {cursor-1} else {cursor};
            let expected=self.target.graphemes(true).nth(position).unwrap_or("");
            self.typed.push(ch);
            self.attempts=self.attempts.saturating_add(1);
            let actual=self.typed.graphemes(true).nth(position).unwrap_or("");
            if !canonical_prefix(expected,actual) {
                self.mistakes=self.mistakes.saturating_add(1);
                if let Some(key)=expected.chars().next().filter(|c|!c.is_control() && !c.is_whitespace()) {
                    let errors=self.weak_keys.entry(key).or_default();
                    *errors=errors.saturating_add(1);
                }
            }
            if single {
                let correct=canonically_equal(expected,actual);
                if let Some(key)=expected.chars().next() {
                    self.observe_interval(key,correct,Instant::now());
                }
            }
        }
        if grapheme_count(&self.typed)>=total {
            let expected=self.target.graphemes(true).last().unwrap_or("");
            let actual=self.typed.graphemes(true).last().unwrap_or("");
            if canonically_equal(expected,actual) || !canonical_prefix(expected,actual) {
                self.finish();
            } else {
                // Delay only the final *incomplete combining sequence*.
                self.pending_completion.get_or_insert_with(Instant::now);
            }
        }
    }
    /// Create a temporary drill targeting commonly mistyped reference characters.
    /// Does not persist target text, error positions, or the generated drill.
    pub fn missed_character_count(&self,key:char)->u32 {
        self.weak_keys.get(&key).copied().unwrap_or(0)
    }
    pub fn weak_character_drill(&self)->Option<String> {
        let mut candidates=self.weak_keys.iter().map(|(ch,n)|(*ch,*n)).collect::<Vec<_>>();
        candidates.sort_by(|a,b|b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut syllables=Vec::new();
        for (ch,_) in candidates.into_iter().take(8) {
            for _ in 0..8 {syllables.push(ch.to_string());}
        }
        if syllables.is_empty() {None} else {Some(syllables.join(" "))}
    }
    pub fn backspace(&mut self) {
        if self.state!=State::Running {return;}
        if let Some((start,_))=self.typed.grapheme_indices(true).last() {
            self.typed.truncate(start);
            self.pending_completion=None;
            self.last_key_at=None;
            self.corrections=self.corrections.saturating_add(1);
        }
    }
    pub fn tick(&mut self) -> bool { self.tick_at(Instant::now()) }
    // One clock snapshot per frame; keeping the boundary deterministic lets
    // CI prove deadline behavior without sleeping or relying on scheduling.
    fn tick_at(&mut self,now:Instant)->bool {
        if self.state!=State::Running {return false;}
        self.elapsed=self.started.map_or(Duration::ZERO,
            |start|now.saturating_duration_since(start));
        // A timed cutoff takes precedence over the incomplete final combining
        // sequence grace period: do not accept post-deadline key events.
        if !self.duration.is_zero() && self.elapsed>=self.duration {
            self.elapsed=self.duration;
            self.pending_completion=None;
            self.state=State::Finished;
            return true;
        }
        if self.pending_completion.is_some_and(|at|
            now.saturating_duration_since(at)>=Duration::from_millis(250)) {
            self.finish_at(now);
            return true;
        }
        false
    }
    fn finish(&mut self) {self.finish_at(Instant::now());}
    fn finish_at(&mut self,now:Instant) {
        self.pending_completion=None;
        let elapsed=self.started.map_or(Duration::ZERO,
            |start|now.saturating_duration_since(start));
        self.elapsed=if self.duration.is_zero(){elapsed}else{elapsed.min(self.duration)};
        self.state=State::Finished;
    }
    pub fn score(&self) -> Score {
        let mut result=score(&self.target, &self.typed, self.elapsed);
        let minutes=self.elapsed.as_secs_f64() / 60.0;
        result.attempts=self.attempts as usize;
        result.mistakes=self.mistakes as usize;
        result.raw_wpm=if minutes>0.0 {self.attempts as f64 / STANDARD_GRAPHEMES_PER_WORD / minutes} else {0.0};
        result.raw_accuracy=if self.attempts>0 {
            100.0*(self.attempts-self.mistakes) as f64 / self.attempts as f64
        } else {0.0};
        result
    }
    pub fn seconds_left(&self) -> f64 { self.duration.saturating_sub(self.elapsed).as_secs_f64() }
}


/// A stable, bounded typing viewport. Advancing in 80-character steps prevents
/// long tests from silently continuing beyond the visible reference passage.
pub fn viewport(total: usize, cursor: usize, width: usize) -> (usize, usize) {
    if width == 0 || total == 0 { return (0,0); }
    let width = width.min(total);
    let cursor = cursor.min(total);
    let anchor = width / 3;
    let step = (width / 3).max(1);
    let start = if cursor <= anchor { 0 } else {
        ((cursor - anchor) / step) * step
    }.min(total.saturating_sub(width));
    (start, (start + width).min(total))
}

/// Final WPM and accuracy operate on canonically normalized extended
/// grapheme clusters. Raw input attempts are still scalar input events;
/// their denominator is deliberately separate from final grapheme accuracy.
pub fn score(target: &str, typed: &str, elapsed: Duration) -> Score {
    let expected: Vec<&str> = target.graphemes(true).collect();
    let entered: Vec<&str> = typed.graphemes(true).collect();
    let correct = entered.iter().zip(expected.iter()).filter(|(a,b)|canonically_equal(a,b)).count();
    let accuracy = if entered.is_empty() { 0.0 } else { 100.0 * correct as f64 / entered.len() as f64 };
    let minutes = elapsed.as_secs_f64() / 60.0;
    let wpm=if minutes>0.0 {correct as f64 / STANDARD_GRAPHEMES_PER_WORD / minutes} else {0.0};
    let raw_wpm=if minutes>0.0 {entered.len() as f64 / STANDARD_GRAPHEMES_PER_WORD / minutes} else {0.0};
    Score { wpm, accuracy, raw_wpm, raw_accuracy:accuracy, attempts:entered.len(),
        mistakes:entered.len().saturating_sub(correct), correct, entered:entered.len(), elapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn nfc_equivalent_graphemes_score_identically() {
        let scores=score("café mañana", "cafe\u{301} man\u{303}ana",Duration::from_secs(60));
        assert_eq!(scores.entered,11);
        assert_eq!(scores.correct,11);
        assert_eq!(scores.accuracy,100.0);
        assert!(canonically_equal("é","e\u{301}"));
        assert!(!canonically_equal("é","e"));
    }
    #[test] fn accent_composition_can_arrive_in_separate_input_events() {
        let mut session=Session::new_with_custom(0,TestMode::Custom,"é");
        session.begin();
        session.type_text("e");
        assert_eq!(session.state,State::Running);
        session.type_text("\u{301}");
        assert_eq!(session.state,State::Finished);
        assert_eq!(session.score().entered,1);
        assert_eq!(session.score().correct,1);
        assert_eq!(session.mistakes,0);
    }
    #[test] fn composed_emoji_and_flags_remain_single_graphemes() {
        for glyph in ["👩‍💻","🇲🇽","👍🏽"] {
            let mut session=Session::new_with_custom(0,TestMode::Custom,glyph);
            session.begin();
            session.type_text(glyph);
            assert_eq!(session.state,State::Finished,"failed emoji {glyph}");
            assert_eq!(session.score().entered,1);
            assert_eq!(session.score().correct,1);
        }
        let mut session=Session::new_with_custom(30,TestMode::Custom,"👩‍💻 abc");
        session.begin();
        session.type_text("👩‍💻");
        session.backspace();
        assert!(session.typed.is_empty());
    }
    #[test] fn a_backspace_erases_one_whole_grapheme() {
        let mut session=Session::new_with_custom(30,TestMode::Custom,"é a");
        session.begin();
        session.type_text("e\u{301}");
        assert_eq!(session.typed,"e\u{301}");
        session.backspace();
        assert!(session.typed.is_empty());
        assert_eq!(session.corrections,1);
    }
    #[test] fn local_custom_passage_rejects_links_and_preserves_unicode_clusters() {
        #[cfg(unix)] {
            use std::os::unix::fs::symlink;
            let dir=std::env::temp_dir().join(format!("typefinch-custom-symlink-{}",uuid::Uuid::new_v4()));
            std::fs::create_dir(&dir).unwrap();
            let source=dir.join("passage.txt");
            let link=dir.join("alias.txt");
            std::fs::write(&source,"mañana café".as_bytes()).unwrap();
            symlink(&source,&link).unwrap();
            assert!(import_custom_file(&link).is_err());
            assert_eq!(import_custom_file(&source).unwrap(),"mañana café");
            std::fs::remove_dir_all(dir).unwrap();
        }
        // Truncation is by extended grapheme, not byte or scalar; ZWJ
        // emoji must remain intact and never become a broken partial glyph.
        let text="👩‍💻".repeat(4100);
        let truncated=import_custom_passage(text.as_bytes()).unwrap();
        assert_eq!(truncated.graphemes(true).count(),4096);
        assert_eq!(truncated,"👩‍💻".repeat(4096));
    }
    #[test] fn keyboard_path_and_drop_import_share_a_bounded_file_reader() {
        let path=std::env::temp_dir().join(format!("typefinch-import-{}.txt",uuid::Uuid::new_v4()));
        {
            use std::io::Write;
            let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&path).unwrap();
            file.write_all("mañana café".as_bytes()).unwrap();
        }
        assert_eq!(import_custom_file(&path).unwrap(),"mañana café");
        std::fs::write(&path,vec![b'a';65_537]).unwrap();
        assert!(import_custom_file(&path).unwrap_err().contains("64 KiB"));
        std::fs::write(&path,[0xFF]).unwrap();
        assert!(import_custom_file(&path).unwrap_err().contains("UTF-8"));
        std::fs::remove_file(&path).unwrap();
        assert!(import_custom_file(&path).is_err());
        assert!(import_custom_file(&std::env::temp_dir()).is_err());
    }
    #[test] fn private_text_file_import_is_utf8_and_bounded() {
        assert_eq!(import_custom_passage(b"hello\nworld").unwrap(),"hello\nworld");
        assert!(import_custom_passage(&vec![b'a';65_537]).is_err());
        assert_eq!(import_custom_passage(&vec![b'a';65_536]).unwrap().len(),4096);
        assert!(import_custom_passage(b" \t \n ").is_err());
        assert!(import_custom_passage(&[0xFF]).is_err());
        assert!(import_custom_passage(b"hi\0there").is_err());
    }
    #[test] fn custom_and_lesson_scores_never_inflate_standard_records() {
        for mode in TestMode::ALL {
            assert_eq!(mode.counts_for_benchmark(), !matches!(mode,TestMode::Custom|TestMode::Lesson));
        }
    }
    #[test] fn word_difficulty_passages_are_distinct_and_context_safe() {
        for language in Language::ALL {
            let sessions=WordDifficulty::ALL.map(|difficulty|
                Session::new_with_settings(30,TestMode::Words,"",language,difficulty));
            assert!(sessions.iter().all(|s|s.target.chars().count()>90));
            assert_ne!(sessions[0].target,sessions[1].target);
            assert_ne!(sessions[1].target,sessions[2].target);
            assert_ne!(sessions[0].history_mode(),sessions[1].history_mode());
            assert_ne!(sessions[1].history_mode(),sessions[2].history_mode());
            assert_eq!(sessions[1].history_mode(),if language==Language::English {"words"} else {"words/es-MX"});
        }
        assert_eq!(serde_json::to_string(&WordDifficulty::Easy).unwrap(),"\"easy\"");
        assert_eq!(Session::new_with_settings(30,TestMode::Words,"",Language::MexicanSpanish,WordDifficulty::Challenge).history_mode(),"words/es-MX/challenge");
        // Difficulty applies only to Words; quotes are not silently split.
        let quotes=Session::new_with_settings(30,TestMode::Quotes,"",Language::English,WordDifficulty::Easy);
        assert_eq!(quotes.history_mode(),"quotes");
    }
    #[test] fn english_and_mexican_spanish_word_sessions_are_distinct() {
        let en=Session::new_with_language(30,TestMode::Words,"",Language::English);
        let es=Session::new_with_language(30,TestMode::Words,"",Language::MexicanSpanish);
        assert_ne!(en.target,es.target);
        assert!(es.target.contains("México") && es.target.contains("mañana"));
        assert_eq!(en.history_mode(),"words");
        assert_eq!(es.history_mode(),"words/es-MX");
        assert_eq!(serde_json::to_string(&Language::MexicanSpanish).unwrap(),"\"es-MX\"");
    }
    #[test] fn english_and_mexican_spanish_share_explicit_five_grapheme_wpm_units() {
        assert_eq!(STANDARD_GRAPHEMES_PER_WORD,5.0);
        let en="a".repeat(10);
        let es="á".repeat(10);
        let decomposed="a\u{301}".repeat(10);
        for typed in [&en,&es,&decomposed] {
            let result=score(typed,typed,Duration::from_secs(60));
            assert_eq!(result.correct,10,"combining marks cannot inflate corrected WPM");
            assert_eq!(result.wpm,2.0,"ten correct graphemes per minute is two standard words");
            assert_eq!(result.accuracy,100.0);
        }
        let english=Session::new_with_language(60,TestMode::Words,"",Language::English);
        let mexican=Session::new_with_language(60,TestMode::Words,"",Language::MexicanSpanish);
        assert_eq!(english.history_mode(),"words");
        assert_eq!(mexican.history_mode(),"words/es-MX",
            "benchmark histories must remain separate by supported language");
    }
    #[test] fn raw_attempt_speed_is_diagnostic_not_equivalent_to_corrected_graphemes() {
        let mut s=Session::new_with_custom(60,TestMode::Custom,"é");
        s.begin();
        s.type_text("e");
        s.type_text("\u{301}");
        s.elapsed=Duration::from_secs(60);
        let score=s.score();
        assert_eq!(score.correct,1);
        assert_eq!(score.entered,1);
        assert_eq!(score.attempts,2,"two Unicode scalar input attempts");
        assert_eq!(score.wpm,1.0/5.0);
        assert_eq!(score.raw_wpm,2.0/5.0,
            "raw attempt rate must not be compared directly between input methods");
    }
    #[test] fn benchmark_languages_are_not_merged_in_history() {
        for mode in [TestMode::Words,TestMode::Quotes,TestMode::Punctuation,TestMode::Paragraphs,TestMode::Numbers] {
            let en=Session::new_with_language(30,mode,"",Language::English);
            let es=Session::new_with_language(30,mode,"",Language::MexicanSpanish);
            assert_eq!(en.history_mode(),mode.key());
            assert_eq!(es.history_mode(),format!("{}/es-MX",mode.key()));
            if mode!=TestMode::Numbers {
                assert_ne!(en.target,es.target,"missing Spanish corpus for {}",mode.key());
            }
        }
        let punctuation=Session::new_with_language(30,TestMode::Punctuation,"",Language::MexicanSpanish);
        assert!(punctuation.target.contains('¿') && punctuation.target.contains('¡'));
        let custom=Session::new_with_language(30,TestMode::Custom,"texto",Language::MexicanSpanish);
        assert_eq!(custom.history_mode(),"custom");
        let lesson=Session::new_with_language(0,TestMode::Lesson,"lesson",Language::MexicanSpanish);
        assert_eq!(lesson.history_mode(),"lesson");
    }
    #[test] fn distinct_content_modes_build_different_targets() {
        let targets:Vec<_> = TestMode::ALL.iter()
            .filter(|mode|!matches!(mode,TestMode::Custom | TestMode::Lesson))
            .map(|mode|Session::new_with_mode(30,*mode).target).collect();
        let mut unique=std::collections::HashSet::new();
        for t in &targets { assert!(unique.insert(t)); assert!(t.len()>300); }
        assert!(Session::new_with_mode(30,TestMode::Numbers).target.contains("42"));
        assert!(Session::new_with_mode(30,TestMode::Punctuation).target.contains('?'));
    }
    #[test] fn decomposed_unicode_accent_has_one_progress_position() {
        let mut session=Session::new_with_custom(0,TestMode::Custom,"éé");
        session.begin();
        session.type_text("e\u{301}");
        assert_eq!(session.typed.graphemes(true).count(),1);
        assert_eq!(session.target.graphemes(true).count(),2);
        assert_eq!(session.score().correct,1);
        assert_eq!(session.state,State::Running);
        session.type_text("e\u{301}");
        assert_eq!(session.typed.graphemes(true).count(),2);
        assert_eq!(session.state,State::Finished);
    }
    #[test] fn canceled_session_never_accepts_more_input_or_auto_finishes() {
        let mut session=Session::new_with_custom(15,TestMode::Custom,"abcdefghijklmnop");
        session.begin();
        session.type_text("ab");
        assert_eq!(session.state,State::Running);
        let typed=session.typed.clone();
        let attempts=session.attempts;
        // This is the same cancellation state used by Escape and focus loss,
        // deliberately NOT a pause that can later resume hidden input.
        session.state=State::Ready;
        session.type_text("defg");
        session.backspace();
        assert!(!session.tick());
        assert_eq!(session.state,State::Ready);
        assert_eq!(session.typed,typed);
        assert_eq!(session.attempts,attempts);
        session.begin();
        assert!(session.typed.is_empty());
        assert_eq!(session.attempts,0);
    }
    #[test] fn timed_deadline_is_exact_and_rejects_late_input() {
        let mut s=Session::new_with_custom(2,TestMode::Custom,"abcdef");
        s.begin();
        let start=s.started.unwrap();
        s.type_text("ab");
        assert!(!s.tick_at(start+Duration::from_millis(1999)));
        assert_eq!(s.state,State::Running);
        assert!(s.seconds_left()>0.0);
        assert!(s.tick_at(start+Duration::from_secs(2)));
        assert_eq!(s.state,State::Finished);
        assert_eq!(s.seconds_left(),0.0);
        assert_eq!(s.score().elapsed,Duration::from_secs(2));
        s.type_text("cdef");
        s.backspace();
        assert_eq!(s.typed,"ab","input after the exact cutoff is ignored");
        assert!(!s.tick_at(start+Duration::from_secs(200)));
        assert_eq!(s.score().elapsed,Duration::from_secs(2));
    }
    #[test] fn timed_pending_grapheme_does_not_extend_past_deadline() {
        let mut s=Session::new_with_custom(2,TestMode::Custom,"é");
        s.begin();
        let start=s.started.unwrap();
        s.type_text("e");
        assert_eq!(s.state,State::Running);
        assert!(s.pending_completion.is_some());
        // Even with the incomplete-grapheme timeout outstanding, the
        // session never extends beyond the configured timed cutoff.
        assert!(s.tick_at(start+Duration::from_secs(2)));
        assert_eq!(s.state,State::Finished);
        assert!(s.pending_completion.is_none());
        s.type_text("\u{301}");
        assert_eq!(s.typed,"e");
        assert_eq!(s.score().elapsed,Duration::from_secs(2));
        // Untimed lessons are exempt from this deadline.
        let mut lesson=Session::new_with_custom(0,TestMode::Lesson,"asdf");
        lesson.begin();
        let started=lesson.started.unwrap();
        assert!(!lesson.tick_at(started+Duration::from_secs(120)));
        assert_eq!(lesson.state,State::Running);
    }
    #[test] fn untimed_lesson_finishes_on_passage_completion_not_deadline() {
        let mut s=Session::new_with_custom(0,TestMode::Lesson,"asdf");
        s.begin();
        assert!(!s.tick());
        assert_eq!(s.state,State::Running);
        s.type_text("asd");
        assert!(!s.tick());
        assert_eq!(s.state,State::Running);
        s.type_text("f");
        assert_eq!(s.state,State::Finished);
        assert_eq!(s.typed,"asdf");
        assert!(s.score().elapsed < Duration::from_secs(10));
    }
    #[test] fn custom_text_is_ephemeral_normalized_and_bounded() {
        let source = format!("  Hello,\nworld!  {} ", "x".repeat(5000));
        let session=Session::new_with_custom(60, TestMode::Custom, &source);
        assert!(session.target.starts_with("Hello, world!"));
        assert!(session.target.chars().count()<=4096);
        assert!(!session.target.contains('\n'));
        assert!(Session::new_with_mode(60,TestMode::Custom).target.is_empty());
    }
    #[test] fn viewport_follows_cursor_and_stays_in_range() {
        assert_eq!(viewport(0,0,230), (0,0));
        assert_eq!(viewport(50,50,230), (0,50));
        assert_eq!(viewport(1000,0,230), (0,230));
        assert_eq!(viewport(1000,450,230), (304,534));
        let (start,end)=viewport(1000,999,230);
        assert!(start<=999 && end==1000);
        for cursor in 0..=1000 {
            let (s,e)=viewport(1000,cursor,230);
            assert!(s<=cursor && cursor<=e && e-s<=230);
        }
    }
    #[test] fn scoring_is_position_based() {
        let a = score("cat", "cut", Duration::from_secs(60));
        assert_eq!(a.correct, 2);
        assert!((a.accuracy - 66.66666667).abs() < 0.001);
        assert!((a.wpm - 0.4).abs() < 0.001);
    }
    #[test] fn empty_test_has_no_accuracy_or_wpm() {
        let a=score("abc", "", Duration::from_secs(30));
        assert_eq!(a.accuracy,0.0);
        assert_eq!(a.wpm,0.0);
        assert_eq!(a.entered,0);
    }
    #[test] fn per_key_latency_is_bounded_requires_samples_and_resets() {
        let mut session=Session::new_with_custom(30,TestMode::Custom,"abc abc abc");
        session.begin();
        let t=Instant::now();
        session.observe_interval('a',true,t);
        session.observe_interval('b',true,t+Duration::from_millis(100));
        session.observe_interval('b',true,t+Duration::from_millis(200));
        assert_eq!(session.average_key_latency_ms('b'),Some(100.0));
        assert_eq!(session.average_key_latency_ms('a'),None);
        session.observe_interval('c',false,t+Duration::from_millis(300));
        assert_eq!(session.average_key_latency_ms('c'),None);
        session.observe_interval('a',true,t+Duration::from_millis(8000));
        assert_eq!(session.average_key_latency_ms('a'),None);
        assert_eq!(session.slowest_keys(),vec![('b',100.0)]);
        session.begin();
        assert!(session.slowest_keys().is_empty());
    }
    #[test] fn adaptive_drill_is_deterministic_ephemeral_and_resettable() {
        let mut s=Session::new_with_custom(30,TestMode::Custom,"cat");
        s.begin();
        assert!(s.weak_character_drill().is_none());
        s.type_text("x");
        s.backspace();
        s.type_text("c");
        s.type_text("a");
        s.type_text("x");
        assert_eq!(s.missed_character_count('c'),1);
        assert_eq!(s.missed_character_count('t'),1);
        assert_eq!(s.missed_character_count('a'),0);
        let drill=s.weak_character_drill().unwrap();
        assert_eq!(drill.split_whitespace().collect::<Vec<_>>(),
            vec!["c","c","c","c","c","c","c","c","t","t","t","t","t","t","t","t"]);
        s.begin();
        assert!(s.weak_character_drill().is_none());
        assert_eq!(s.missed_character_count('c'),0);
    }
    #[test] fn raw_metrics_preserve_corrected_mistakes() {
        let mut s=Session::new_with_custom(30,TestMode::Custom,"cat");
        s.begin();
        s.type_text("cx");
        s.backspace();
        s.type_text("at");
        let r=s.score();
        assert_eq!(r.entered,3);
        assert_eq!(r.correct,3);
        assert_eq!(r.attempts,4);
        assert_eq!(r.mistakes,1);
        assert!((r.raw_accuracy-75.0).abs()<0.01);
        assert_eq!(r.accuracy,100.0);
        s.begin();
        let again=s.score();
        assert_eq!(again.attempts,0);
        assert_eq!(again.mistakes,0);
    }
    #[test] fn zero_duration_is_safe() {
        assert_eq!(score("abc", "abc", Duration::ZERO).wpm, 0.0);
    }
    #[test] fn entry_limits_and_backspace() {
        let mut s = Session::new(30);
        s.begin();
        s.type_text("finch");
        assert_eq!(s.typed, "finch");
        s.backspace();
        assert_eq!(s.typed, "finc");
        assert_eq!(s.corrections, 1);
    }
    #[test] fn unicode_scalars_are_scored_without_byte_counts() {
        let s = score("café", "café", Duration::from_secs(60));
        assert_eq!(s.correct, 4);
    }
}
