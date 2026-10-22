//! Deterministic, local typing-session scoring. No browser/network dependency.
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

pub const WORDS: &str = "little finches practice every day with patient rhythm gentle wings make steady progress while fingers learn their keys accuracy grows through focused practice kind habits build confident typing and each new session brings another chance to improve";

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestMode { #[default] Words, Quotes, Numbers, Punctuation }
impl TestMode {
    pub const ALL: [TestMode;4] = [Self::Words, Self::Quotes, Self::Numbers, Self::Punctuation];
    pub fn label(self) -> &'static str {
        match self { Self::Words=>"Words", Self::Quotes=>"Quotes", Self::Numbers=>"Numbers", Self::Punctuation=>"Punctuation" }
    }
    pub fn key(self) -> &'static str {
        match self { Self::Words=>"words", Self::Quotes=>"quotes", Self::Numbers=>"numbers", Self::Punctuation=>"punctuation" }
    }
}
fn corpus(mode: TestMode) -> String {
    let (text, repeats) = match mode {
        TestMode::Words => (WORDS, 8),
        TestMode::Quotes => ("A small improvement, repeated carefully, becomes a remarkable skill. Every key is another opportunity to learn.", 14),
        TestMode::Numbers => ("17 42 63 85 91 24 56 78 30 49 12 75 64 83 29 50 04 68", 28),
        TestMode::Punctuation => ("Wait, is that correct? Yes! Write clearly: commas, colons; parentheses (like these), and a final period.", 13),
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
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub wpm: f64,
    pub accuracy: f64,
    pub correct: usize,
    pub entered: usize,
    pub elapsed: Duration,
}

impl Session {
    pub fn new(seconds: u64) -> Self { Self::new_with_mode(seconds, TestMode::Words) }
    pub fn new_with_mode(seconds: u64, mode: TestMode) -> Self {
        Self { mode, target:corpus(mode), typed:String::new(), duration:Duration::from_secs(seconds),
            state:State::Ready, started:None, elapsed:Duration::ZERO, corrections:0 }
    }
    pub fn begin(&mut self) {
        self.typed.clear();
        self.corrections = 0;
        self.elapsed = Duration::ZERO;
        self.started = Some(Instant::now());
        self.state = State::Running;
    }
    pub fn type_text(&mut self, text: &str) {
        if self.state != State::Running { return; }
        for ch in text.chars().filter(|c| !c.is_control()) {
            if self.typed.chars().count() >= self.target.chars().count() { break; }
            self.typed.push(ch);
        }
        if self.typed.chars().count() >= self.target.chars().count() { self.finish(); }
    }
    pub fn backspace(&mut self) {
        if self.state == State::Running && self.typed.pop().is_some() { self.corrections += 1; }
    }
    pub fn tick(&mut self) -> bool {
        if self.state != State::Running { return false; }
        self.elapsed = self.started.map_or(Duration::ZERO, |t| t.elapsed());
        if self.elapsed >= self.duration {
            self.elapsed = self.duration;
            self.state = State::Finished;
            return true;
        }
        false
    }
    fn finish(&mut self) {
        self.elapsed = self.started.map_or(Duration::ZERO, |t| t.elapsed()).min(self.duration);
        self.state = State::Finished;
    }
    pub fn score(&self) -> Score { score(&self.target, &self.typed, self.elapsed) }
    pub fn seconds_left(&self) -> f64 { self.duration.saturating_sub(self.elapsed).as_secs_f64() }
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
    Score { wpm: if minutes > 0.0 { correct as f64 / 5.0 / minutes } else { 0.0 }, accuracy, correct, entered: entered.len(), elapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn distinct_content_modes_build_different_targets() {
        let targets:Vec<_> = TestMode::ALL.iter().map(|mode|Session::new_with_mode(30,*mode).target).collect();
        let mut unique=std::collections::HashSet::new();
        for t in &targets { assert!(unique.insert(t)); assert!(t.len()>300); }
        assert!(Session::new_with_mode(30,TestMode::Numbers).target.contains("42"));
        assert!(Session::new_with_mode(30,TestMode::Punctuation).target.contains('?'));
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
