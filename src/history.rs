//! Versioned local SQLite history, no raw typed text or device IDs.
use crate::{engine::Session, preferences::base_dir};
use rusqlite::{params, Connection};
use std::{fs, time::{SystemTime, UNIX_EPOCH}};

pub const SCORING_VERSION: i64 = 1;

#[derive(Clone, Debug)]
pub struct Record {
    pub id: i64,
    pub profile_id: String,
    pub finished_at_ms: i64,
    pub layout: String,
    pub input_mode: String,
    pub test_mode: String,
    pub duration_ms: i64,
    pub elapsed_ms: i64,
    pub entered: i64,
    pub correct: i64,
    pub wpm: f64,
    pub accuracy: f64,
    pub corrections: i64,
    pub scoring_version: i64,
}
impl Record {
    pub fn completed(profile_id: &str, layout: &str, input_mode: &str, session: &Session) -> Self {
        let score = session.score();
        let at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
        Self {
            id:0, profile_id:profile_id.into(), finished_at_ms:at,
            layout:layout.into(), input_mode:input_mode.into(), test_mode:session.mode.key().into(),
            duration_ms:session.duration.as_millis() as i64,
            elapsed_ms:score.elapsed.as_millis() as i64,
            entered:score.entered as i64, correct:score.correct as i64,
            wpm:score.wpm, accuracy:score.accuracy, corrections:session.corrections as i64,
            scoring_version:SCORING_VERSION,
        }
    }
}

fn init(db: &Connection) -> Result<(), rusqlite::Error> {
    let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match version {
        0 => db.execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE IF NOT EXISTS sessions (
               id INTEGER PRIMARY KEY,
               profile_id TEXT NOT NULL,
               finished_at_ms INTEGER NOT NULL,
               layout TEXT NOT NULL,
               input_mode TEXT NOT NULL,
               test_mode TEXT NOT NULL DEFAULT 'words',
               duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
               elapsed_ms INTEGER NOT NULL CHECK (elapsed_ms >= 0),
               entered INTEGER NOT NULL CHECK (entered >= 0),
               correct INTEGER NOT NULL CHECK (correct >= 0),
               wpm REAL NOT NULL CHECK (wpm >= 0),
               accuracy REAL NOT NULL CHECK (accuracy >= 0 AND accuracy <= 100),
               corrections INTEGER NOT NULL CHECK (corrections >= 0),
               scoring_version INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS sessions_profile_time
               ON sessions(profile_id, finished_at_ms DESC, id DESC);
             PRAGMA user_version=2;
             COMMIT;"
        ),
        1 => db.execute_batch("BEGIN IMMEDIATE; ALTER TABLE sessions ADD COLUMN test_mode TEXT NOT NULL DEFAULT 'words'; PRAGMA user_version=2; COMMIT;"),
        2 => Ok(()),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn connect() -> Result<Connection, String> {
    let base = base_dir();
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let db = Connection::open(base.join("sessions.sqlite3")).map_err(|e| e.to_string())?;
    init(&db).map_err(|e| format!("Session history schema: {e}"))?;
    Ok(db)
}
fn insert_with(db: &Connection, item: &Record) -> Result<i64, rusqlite::Error> {
    db.execute("INSERT INTO sessions
       (profile_id, finished_at_ms, layout, input_mode, duration_ms, elapsed_ms,
        entered, correct, wpm, accuracy, corrections, scoring_version, test_mode)
       VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
       params![item.profile_id, item.finished_at_ms, item.layout, item.input_mode,
       item.duration_ms, item.elapsed_ms, item.entered, item.correct, item.wpm,
       item.accuracy, item.corrections, item.scoring_version, item.test_mode])?;
    Ok(db.last_insert_rowid())
}
fn recent_with(db: &Connection, profile_id: &str, limit: usize) -> Result<Vec<Record>, rusqlite::Error> {
    let mut stmt = db.prepare(
        "SELECT id, profile_id, finished_at_ms, layout, input_mode, duration_ms, elapsed_ms,
          entered, correct, wpm, accuracy, corrections, scoring_version, test_mode
         FROM sessions WHERE profile_id = ?1 ORDER BY finished_at_ms DESC, id DESC LIMIT ?2"
    )?;
    stmt.query_map(params![profile_id, limit.min(100) as i64], |row| Ok(Record {
        id:row.get(0)?, profile_id:row.get(1)?, finished_at_ms:row.get(2)?,
        layout:row.get(3)?, input_mode:row.get(4)?, test_mode:row.get(13)?, duration_ms:row.get(5)?,
        elapsed_ms:row.get(6)?, entered:row.get(7)?, correct:row.get(8)?,
        wpm:row.get(9)?, accuracy:row.get(10)?, corrections:row.get(11)?,
        scoring_version:row.get(12)?,
    }))?.collect()
}
pub fn insert(record: &Record) -> Result<i64, String> {
    let db=connect()?;
    insert_with(&db, record).map_err(|e| e.to_string())
}
pub fn recent(profile_id: &str) -> Result<Vec<Record>, String> {
    let db=connect()?;
    recent_with(&db, profile_id, 8).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: &str) -> Record {
        Record {
            id:0, profile_id:id.into(), finished_at_ms:123456789,
            layout:"us(colemak)".into(), input_mode:"Emulated".into(), test_mode:"words".into(),
            duration_ms:30000, elapsed_ms:30000, entered:120,
            correct:114, wpm:45.6, accuracy:95.0, corrections:3, scoring_version:1
        }
    }
    #[test] fn history_is_profile_isolated_and_ordered() {
        let db=Connection::open_in_memory().unwrap();
        init(&db).unwrap();
        insert_with(&db,&item("a")).unwrap();
        insert_with(&db,&item("b")).unwrap();
        let id=insert_with(&db,&item("a")).unwrap();
        let rows=recent_with(&db,"a",8).unwrap();
        assert_eq!(rows.len(),2);
        assert_eq!(rows[0].id,id);
        assert_eq!(rows[0].correct,114);
        assert_eq!(recent_with(&db,"b",8).unwrap().len(),1);
        assert!(recent_with(&db,"c",8).unwrap().is_empty());
    }
    #[test] fn v1_sessions_migrate_to_words_without_data_loss() {
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE sessions (
            id INTEGER PRIMARY KEY, profile_id TEXT NOT NULL, finished_at_ms INTEGER NOT NULL,
            layout TEXT NOT NULL, input_mode TEXT NOT NULL, duration_ms INTEGER NOT NULL,
            elapsed_ms INTEGER NOT NULL, entered INTEGER NOT NULL, correct INTEGER NOT NULL,
            wpm REAL NOT NULL, accuracy REAL NOT NULL, corrections INTEGER NOT NULL,
            scoring_version INTEGER NOT NULL);
            INSERT INTO sessions (profile_id,finished_at_ms,layout,input_mode,duration_ms,elapsed_ms,entered,correct,wpm,accuracy,corrections,scoring_version)
            VALUES ('legacy',1,'us','OS',30000,30000,10,10,4.0,100.0,0,1);
            PRAGMA user_version=1;").unwrap();
        init(&db).unwrap();
        let found=recent_with(&db,"legacy",8).unwrap();
        assert_eq!(found.len(),1);
        assert_eq!(found[0].test_mode,"words");
        init(&db).unwrap();
    }
    #[test] fn migration_is_idempotent_and_detects_future_schema() {
        let db=Connection::open_in_memory().unwrap();
        init(&db).unwrap();
        init(&db).unwrap();
        db.pragma_update(None,"user_version",999_i64).unwrap();
        assert!(init(&db).is_err());
    }
    #[test] fn history_stores_metrics_only() {
        let db=Connection::open_in_memory().unwrap();
        init(&db).unwrap();
        insert_with(&db,&item("a")).unwrap();
        let cols: Vec<String> = {
            let stmt=db.prepare("SELECT * FROM sessions LIMIT 0").unwrap();
            stmt.column_names().iter().map(|s|s.to_string()).collect()
        };
        assert!(!cols.iter().any(|c| c.contains("text") || c.contains("typed")));
        assert!(cols.contains(&"profile_id".to_owned()));
        assert!(cols.contains(&"scoring_version".to_owned()));
    }
    #[test] fn rejects_invalid_derived_accuracy() {
        let db=Connection::open_in_memory().unwrap();
        init(&db).unwrap();
        let mut t=item("a");t.accuracy=120.0;
        assert!(insert_with(&db,&t).is_err());
    }
}
