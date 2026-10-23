//! Small, license-free, deterministic starter curriculum for untimed lessons.
//! All passages are original Typefinch text, not third-party copyrighted quotes.
pub struct Lesson {
    pub id: &'static str,
    pub name: &'static str,
    pub level: &'static str,
    pub hint: &'static str,
    pub passage: &'static str,
}
pub const LESSONS: &[Lesson] = &[
    Lesson { id:"home",name:"Home row",level:"Beginner",hint:"Build an even rhythm with the home-row keys.",
        passage:"asdf jkl; asdf jkl; fj dk sl a; sad flask; jill asks; asdf jkl;" },
    Lesson { id:"top",name:"Top row",level:"Beginner",hint:"Find the top-row positions without watching your hands.",
        passage:"qwerty uiop qwer tyui op we try quiet power pure type write your query" },
    Lesson { id:"bottom",name:"Bottom row",level:"Beginner",hint:"Practice reaches into the lower alphabet row.",
        passage:"zxcv bnm zxcv bnm zoom mix box cave vex nimble move calm" },
    Lesson { id:"pairs",name:"Common English pairs",level:"Intermediate",hint:"Practice frequent combinations rather than isolated letters.",
        passage:"the there their then this that think thing when what where which with would other" },
    Lesson { id:"symbols",name:"Symbols and punctuation",level:"Intermediate",hint:"Mix letter rhythm with commas, semicolons and question marks.",
        passage:"Hello, finch! Can you type this? Yes: clean, careful; quick, steady." },
];
pub fn get(id:&str)->&'static Lesson {
    LESSONS.iter().find(|l|l.id==id).unwrap_or(&LESSONS[0])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn unique_and_bounded_lesson_passages() {
        let mut ids=std::collections::HashSet::new();
        for l in LESSONS {
            assert!(ids.insert(l.id));
            assert!(!l.passage.is_empty());
            assert!(l.passage.chars().count()<=4096);
            assert!(!l.name.is_empty() && !l.hint.is_empty());
        }
    }
}
