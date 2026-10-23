#![forbid(unsafe_code)]
mod achievements;
mod backup_verify;
mod daily;
mod goal_transfer;
mod custom_layout;
mod app;
mod devices;
mod engine;
mod history;
mod export;
mod layouts;
mod geometry;
mod lesson;
mod preferences;
mod registry;
mod qa;
mod xkb;

use eframe::egui;
use std::path::PathBuf;

#[derive(Debug,PartialEq,Eq)]
enum LaunchIntent {Gui,Version,Help,VerifyBackup(PathBuf,PathBuf),
    StageBackup(PathBuf,PathBuf,PathBuf),Invalid}
fn intent(args:&[String])->LaunchIntent {
    match args {
        []=>LaunchIntent::Gui,
        [flag] if flag=="--version" || flag=="-V"=>LaunchIntent::Version,
        [flag] if flag=="--help" || flag=="-h"=>LaunchIntent::Help,
        [flag,settings,database] if flag=="--verify-backup"=>
            LaunchIntent::VerifyBackup(PathBuf::from(settings),PathBuf::from(database)),
        [flag,settings,database,destination] if flag=="--restore-backup-to-new-dir"=>
            LaunchIntent::StageBackup(PathBuf::from(settings),PathBuf::from(database),PathBuf::from(destination)),
        _=>LaunchIntent::Invalid,
    }
}
fn main() -> eframe::Result {
    let arguments=std::env::args().skip(1).collect::<Vec<_>>();
    match intent(&arguments) {
        LaunchIntent::Gui=>{},
        LaunchIntent::Version=>{
            println!("typefinch {}",env!("CARGO_PKG_VERSION"));
            return Ok(());
        },
        LaunchIntent::Help=>{
            println!("Typefinch {} · local-first typing trainer",env!("CARGO_PKG_VERSION"));
            println!("Usage: typefinch [--version|--help]");
            println!("       typefinch --verify-backup SETTINGS.json SESSIONS.sqlite3");
            println!("       typefinch --restore-backup-to-new-dir SETTINGS.json SESSIONS.sqlite3 NEW_DIRECTORY");
            println!("Backup verification is read-only and never restores or replaces user data.");
            println!("GUI shortcuts: F1 guide · F2 focus · F3 layout layer · F4 geometry · F5 start · F6 SVG · F7 JSON · F8 backup · F9 PNG");
            return Ok(());
        },
        LaunchIntent::VerifyBackup(settings,database)=>{
            match backup_verify::verify(&settings,&database) {
                Ok(result)=>println!("Backup verified: {} profiles, {} sessions, SQLite schema {}. No files changed.",
                    result.profiles,result.sessions,result.schema),
                Err(problem)=>{
                    eprintln!("Backup verification failed: {problem}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        },
        LaunchIntent::StageBackup(settings,database,destination)=>{
            match backup_verify::stage_to_new_directory(&settings,&database,&destination) {
                Ok(summary)=>println!("Restored into new directory: {} profiles, {} sessions. Existing data unchanged; staged directory not activated.",
                    summary.profiles,summary.sessions),
                Err(problem)=>{
                    eprintln!("Restore staging refused: {problem}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        },
        LaunchIntent::Invalid=>{
            eprintln!("Unrecognized arguments. Use typefinch --help.");
            std::process::exit(2);
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Typefinch")
            .with_inner_size([1060.0, 760.0])
            .with_min_inner_size([720.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typefinch",
        options,
        Box::new(|cc| Ok(Box::new(app::Typefinch::new(cc)))),
    )
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    #[test] fn flags_never_start_graphics() {
        assert_eq!(intent(&[]),LaunchIntent::Gui);
        for x in ["--version","-V"] {
            assert_eq!(intent(&[x.to_owned()]),LaunchIntent::Version);
        }
        for x in ["--help","-h"] {
            assert_eq!(intent(&[x.to_owned()]),LaunchIntent::Help);
        }
        assert_eq!(intent(&["--unsupported".into()]),LaunchIntent::Invalid);
        assert_eq!(intent(&["--version".into(),"junk".into()]),LaunchIntent::Invalid);
        let expected=LaunchIntent::VerifyBackup(PathBuf::from("a.json"),PathBuf::from("b.sqlite3"));
        assert_eq!(intent(&["--verify-backup".into(),"a.json".into(),"b.sqlite3".into()]),expected);
        assert_eq!(intent(&["--verify-backup".into(),"a.json".into()]),LaunchIntent::Invalid);
        assert_eq!(intent(&["--restore-backup-to-new-dir".into(),"a".into(),"b".into(),"c".into()]),
            LaunchIntent::StageBackup(PathBuf::from("a"),PathBuf::from("b"),PathBuf::from("c")));
        assert_eq!(intent(&["--restore-backup-to-new-dir".into(),"a".into(),"b".into()]),LaunchIntent::Invalid);
    }
}
