#![forbid(unsafe_code)]
mod achievements;
mod daily;
mod custom_layout;
mod app;
mod devices;
mod engine;
mod history;
mod export;
mod layouts;
mod lesson;
mod preferences;
mod registry;
mod qa;
mod xkb;

use eframe::egui;

#[derive(Debug,PartialEq,Eq)]
enum LaunchIntent {Gui,Version,Help,Invalid}
fn intent(args:&[String])->LaunchIntent {
    match args {
        []=>LaunchIntent::Gui,
        [flag] if flag=="--version" || flag=="-V"=>LaunchIntent::Version,
        [flag] if flag=="--help" || flag=="-h"=>LaunchIntent::Help,
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
            println!("GUI shortcuts: F1 guide · F2 focus typing · F5 start · F6 SVG · F7 JSON · F8 backup · F9 PNG");
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
    }
}
