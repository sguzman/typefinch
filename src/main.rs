#![forbid(unsafe_code)]
mod achievements;
mod backup_verify;
mod daily;
mod goal_transfer;
mod custom_layout;
mod app;
mod devices;
mod wayland_seat;
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
enum LaunchIntent {Gui,Version,Help,InspectKeyboards,ProbeWaylandSeats,VerifyBackup(PathBuf,PathBuf),
    StageBackup(PathBuf,PathBuf,PathBuf),Invalid}
fn intent(args:&[String])->LaunchIntent {
    match args {
        []=>LaunchIntent::Gui,
        [flag] if flag=="--version" || flag=="-V"=>LaunchIntent::Version,
        [flag] if flag=="--help" || flag=="-h"=>LaunchIntent::Help,
        [flag] if flag=="--inspect-keyboards"=>LaunchIntent::InspectKeyboards,
        [flag] if flag=="--_probe_wayland_seats"=>LaunchIntent::ProbeWaylandSeats,
        [flag,settings,database] if flag=="--verify-backup"=>
            LaunchIntent::VerifyBackup(PathBuf::from(settings),PathBuf::from(database)),
        [flag,settings,database,destination] if flag=="--restore-backup-to-new-dir"=>
            LaunchIntent::StageBackup(PathBuf::from(settings),PathBuf::from(database),PathBuf::from(destination)),
        _=>LaunchIntent::Invalid,
    }
}
// Explicit, read-only JSON snapshot for terminal diagnostics and automated
// compositor QA. Inspection IDs are NOT physical device identifiers.
fn inspection_snapshot(rows:&[devices::Keyboard])->serde_json::Value {
    fn reported(value:&str)->Option<&str> {
        if value.is_empty() {None} else {Some(value)}
    }
    let rows=rows.iter().map(|keyboard|serde_json::json!({
        "label":keyboard.name,
        "inspection_id":keyboard.address,
        "reported_active_keymap":reported(&keyboard.active_keymap),
        "configured_layout":reported(&keyboard.layout),
        "configured_variant":reported(&keyboard.variant)
    })).collect::<Vec<_>>();
    serde_json::json!({
        "schema":1,
        "capture_policy":"inspection-only",
        "input_attribution":"aggregate seat; no per-device key-event attribution",
        "rows":rows
    })
}
fn main() -> eframe::Result {
    let arguments=std::env::args().skip(1).collect::<Vec<_>>();
    match intent(&arguments) {
        LaunchIntent::Gui=>{},
        LaunchIntent::Version=>{
            println!("typefinch {}",env!("CARGO_PKG_VERSION"));
            return Ok(());
        },
        LaunchIntent::InspectKeyboards=>{
            match devices::inspect() {
                Ok(rows)=>println!("{}",inspection_snapshot(&rows)),
                Err(err)=>{
                    eprintln!("Keyboard inspection unavailable: {err}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        },
        LaunchIntent::ProbeWaylandSeats=>{
            match wayland_seat::read_only_seats() {
                Ok(seats)=>println!("{}",serde_json::to_string(&seats).expect("seat metadata serializable")),
                Err(err)=>{
                    eprintln!("Wayland seat inspection failed: {err}");
                    std::process::exit(2);
                },
            }
            return Ok(());
        },
        LaunchIntent::Help=>{
            println!("Typefinch {} · local-first typing trainer",env!("CARGO_PKG_VERSION"));
            println!("Usage: typefinch [--version|--help|--inspect-keyboards]");
            println!("       typefinch --verify-backup SETTINGS.json SESSIONS.sqlite3");
            println!("       typefinch --restore-backup-to-new-dir SETTINGS.json SESSIONS.sqlite3 NEW_DIRECTORY");
            println!("Keyboard inspection prints read-only JSON: seats/layout metadata, never key events or physical input attribution.");
            println!("Backup verification is read-only and never restores or replaces user data.");
            println!("GUI shortcuts: F1 guide · F2 focus · F3 layout layer · F4 geometry · F5 start · F6 SVG · F7 JSON · F8 backup · F9 PNG · F10 reference keys · F11 / Ctrl+O custom file import");
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
    #[test] fn keyboard_snapshot_discloses_only_reported_seat_metadata() {
        let hyprland=devices::Keyboard {
            name:"Keyboard A".into(),address:"0x1".into(),
            active_keymap:"English (US)".into(),layout:"us".into(),
            ..Default::default()
        };
        let wayland=devices::Keyboard {
            name:"Wayland seat seat0".into(),address:"wayland:seat:10".into(),
            ..Default::default()
        };
        let snap=inspection_snapshot(&[hyprland,wayland]);
        assert_eq!(snap["schema"],1);
        assert_eq!(snap["capture_policy"],"inspection-only");
        assert_eq!(snap["rows"][0]["inspection_id"],"0x1");
        assert_eq!(snap["rows"][0]["reported_active_keymap"],"English (US)");
        assert_eq!(snap["rows"][1]["inspection_id"],"wayland:seat:10");
        assert!(snap["rows"][1]["reported_active_keymap"].is_null());
        assert!(snap["rows"][1]["configured_layout"].is_null());
        assert!(snap["rows"][0]["physical_device"].is_null());
        assert!(snap["rows"][0]["key_events"].is_null());
        assert!(snap["rows"][1]["key_events"].is_null());
        assert_eq!(inspection_snapshot(&[])["rows"].as_array().unwrap().len(),0);
    }
    #[test] fn flags_never_start_graphics() {
        assert_eq!(intent(&[]),LaunchIntent::Gui);
        for x in ["--version","-V"] {
            assert_eq!(intent(&[x.to_owned()]),LaunchIntent::Version);
        }
        for x in ["--help","-h"] {
            assert_eq!(intent(&[x.to_owned()]),LaunchIntent::Help);
        }
        assert_eq!(intent(&["--unsupported".into()]),LaunchIntent::Invalid);
        assert_eq!(intent(&["--_probe_wayland_seats".into()]),LaunchIntent::ProbeWaylandSeats);
        assert_eq!(intent(&["--inspect-keyboards".into()]),LaunchIntent::InspectKeyboards);
        assert_eq!(intent(&["--version".into(),"junk".into()]),LaunchIntent::Invalid);
        let expected=LaunchIntent::VerifyBackup(PathBuf::from("a.json"),PathBuf::from("b.sqlite3"));
        assert_eq!(intent(&["--verify-backup".into(),"a.json".into(),"b.sqlite3".into()]),expected);
        assert_eq!(intent(&["--verify-backup".into(),"a.json".into()]),LaunchIntent::Invalid);
        assert_eq!(intent(&["--restore-backup-to-new-dir".into(),"a".into(),"b".into(),"c".into()]),
            LaunchIntent::StageBackup(PathBuf::from("a"),PathBuf::from("b"),PathBuf::from("c")));
        assert_eq!(intent(&["--restore-backup-to-new-dir".into(),"a".into(),"b".into()]),LaunchIntent::Invalid);
    }
}
