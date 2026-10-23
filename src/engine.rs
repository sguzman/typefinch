//! Deterministic, local typing-session scoring. No browser/network dependency.
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

pub const WORDS: &str = "little finches practice every day with patient rhythm gentle wings make steady progress while fingers learn their keys accuracy grows through focused practice kind habits build confident typing and each new session brings another chance to improve";

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestMode { #[default] Words, Quotes, Numbers, Punctuation, Paragraphs, Custom, Lesson }
impl TestMode {
    pub const ALL: [TestMode;7] = [Self::Words, Self::Quotes, Self::Numbers, Self::Punctuation, Self::Paragraphs, Self::Custom, Self::Lesson];
    pub fn label(self) -> &'static str {
        match self { Self::Words=>"Words", Self::Quotes=>"Quotes", Self::Numbers=>"Numbers", Self::Punctuation=>"Punctuation", Self::Paragraphs=>"Paragraphs", Self::Custom=>"Custom text", Self::Lesson=>"Practice lesson" }
    }
    pub fn key(self) -> &'static str {
        match self { Self::Words=>"words", Self::Quotes=>"quotes", Self::Numbers=>"numbers", Self::Punctuation=>"punctuation", Self::Paragraphs=>"paragraphs", Self::Custom=>"custom", Self::Lesson=>"lesson" }
    }
}
fn corpus(mode: TestMode) -> String {
    let (text, repeats) = match mode {
        TestMode::Words => (WORDS, 8),
        TestMode::Quotes => ("A small improvement, repeated carefully, becomes a remarkable skill. Every key is another opportunity to learn.", 14),
        TestMode::Numbers => ("17 42 63 85 91 24 56 78 30 49 12 75 64 83 29 50 04 68", 28),
        TestMode::Punctuation => ("Wait, is that correct? Yes! Write clearly: commas, colons; parentheses (like these), and a final period.", 13),
        TestMode::Paragraphs => ("A bird learns to fly through careful repetition, just as a typist learns rhythm from patient practice. Every attempt records a small improvement.\n\nThe best progress is steady and deliberate: watch each symbol, keep the hands comfortable, and let consistency grow.", 10),
        TestMode::Custom | TestMode::Lesson => ("",0),
    };
    std::iter::repeat_n(text, repeats).collect::<Vec<_>>().join(" ")
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum State { Ready, Running, Finished }

pub struct Session {
    pub mode: TestMode,
    pub target: String,
    pub typed: String,
    pub duration: Duration,
    pub state: State,
    started: Option<Instant>,
    elapsed: Duration,
    pub corrections: u32,
    pub attempts: u32,
    pub mistakes: u32,
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
        // Newline/whitespace are normalized to spaces so the target remains
        // typable by the existing committed-character pipeline.
        let source = if matches!(mode, TestMode::Custom | TestMode::Lesson) { custom.to_owned() } else { corpus(mode) };
        let target = source.chars().take(4096).collect::<String>()
            .split_whitespace().collect::<Vec<_>>().join(" ");
        Self { mode, target, typed:String::new(), duration:Duration::from_secs(seconds),
            state:State::Ready, started:None, elapsed:Duration::ZERO, corrections:0, attempts:0, mistakes:0 }
    }
    pub fn begin(&mut self) {
        self.typed.clear();
        self.corrections = 0;
        self.attempts = 0;
        self.mistakes = 0;
        self.elapsed = Duration::ZERO;
        self.started = Some(Instant::now());
        self.state = State::Running;
    }
    pub fn type_text(&mut self, text: &str) {
        if self.state != State::Running { return; }
        let total = self.target.chars().count();
        let mut cursor = self.typed.chars().count();
        for ch in text.chars().filter(|c| !c.is_control()) {
            if cursor >= total { break; }
            self.attempts = self.attempts.saturating_add(1);
            if self.target.chars().nth(cursor) != Some(ch) {
                self.mistakes = self.mistakes.saturating_add(1);
            }
            self.typed.push(ch);
            cursor += 1;
        }
        if self.typed.chars().count() >= self.target.chars().count() { self.finish(); }
    }
    pub fn backspace(&mut self) {
        if self.state == State::Running && self.typed.pop().is_some() { self.corrections += 1; }
    }
    pub fn tick(&mut self) -> bool {
        if self.state != State::Running { return false; }
        self.elapsed = self.started.map_or(Duration::ZERO, |t| t.elapsed());
        if self.duration.is_zero() { return false; }
        if self.elapsed >= self.duration {
            self.elapsed = self.duration;
            self.state = State::Finished;
            return true;
        }
        false
    }
    fn finish(&mut self) {
        let elapsed = self.started.map_or(Duration::ZERO, |t| t.elapsed());
        self.elapsed = if self.duration.is_zero() { elapsed } else { elapsed.min(self.duration) };
        self.state = State::Finished;
    }
    pub fn score(&self) -> Score {
        let mut result=score(&self.target, &self.typed, self.elapsed);
        let minutes=self.elapsed.as_secs_f64() / 60.0;
        result.attempts=self.attempts as usize;
        result.mistakes=self.mistakes as usize;
        result.raw_wpm=if minutes>0.0 {self.attempts as f64 / 5.0 / minutes} else {0.0};
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

/// Gross errors include uncorrected mismatches, including mistyped Unicode scalars.
/// WPM = correctly positioned characters / 5 / elapsed minutes.
/// Each session defines its metric version; corrected-error accounting is separate.
pub fn score(target: &str, typed: &str, elapsed: Duration) -> Score {
    let expected: Vec<char> = target.chars().collect();
    let entered: Vec<char> = typed.chars().collect();
    let correct = entered.iter().zip(expected.iter()).filter(|(a,b)| a==b).count();
    let accuracy = if entered.is_empty() { 0.0 } else { 100.0 * correct as f64 / entered.len() as f64 };
    let minutes = elapsed.as_secs_f64() / 60.0;
    let wpm=if minutes>0.0 {correct as f64 / 5.0 / minutes} else {0.0};
    let raw_wpm=if minutes>0.0 {entered.len() as f64 / 5.0 / minutes} else {0.0};
    Score { wpm, accuracy, raw_wpm, raw_accuracy:accuracy, attempts:entered.len(),
        mistakes:entered.len().saturating_sub(correct), correct, entered:entered.len(), elapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn distinct_content_modes_build_different_targets() {
        let targets:Vec<_> = TestMode::ALL.iter()
            .filter(|mode|!matches!(mode,TestMode::Custom | TestMode::Lesson))
            .map(|mode|Session::new_with_mode(30,*mode).target).collect();
        let mut unique=std::collections::HashSet::new();
        for t in &targets { assert!(unique.insert(t)); assert!(t.len()>300); }
        assert!(Session::new_with_mode(30,TestMode::Numbers).target.contains("42"));
        assert!(Session::new_with_mode(30,TestMode::Punctuation).target.contains('?'));
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
