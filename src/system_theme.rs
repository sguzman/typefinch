//! Read-only Linux desktop theme via the standardized XDG Settings portal.
//! No extra permissions, no settings writes, no shell, no new Rust crates.
//! egui/winit may not supply a Linux system theme; this is an optional
//! asynchronous fallback, not a replacement for explicit Light/Dark choices.
use eframe::egui;
use std::sync::{mpsc::{self,Receiver},
    Arc,atomic::{AtomicBool,Ordering}};

pub fn preference(portal_dark:Option<bool>)->egui::ThemePreference {
    match portal_dark {
        Some(true)=>egui::ThemePreference::Dark,
        Some(false)=>egui::ThemePreference::Light,
        None=>egui::ThemePreference::System,
    }
}

pub fn watch(ctx:egui::Context,enabled:Arc<AtomicBool>)->Receiver<Option<bool>> {
    let (tx,rx)=mpsc::channel();
    #[cfg(target_os="linux")]
    {
        let _=std::thread::Builder::new().name("typefinch-system-theme".into())
            .spawn(move || {
                let mut previous:Option<Option<bool>>=None;
                loop {
                    // No D-Bus polling when the user has explicitly pinned
                    // Light or Dark: those modes must have zero portal work.
                    if !enabled.load(Ordering::Relaxed) {
                        previous=None;
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        continue;
                    }
                    let next=read_portal();
                    // Only emit when the semantic answer changes. A missing
                    // portal reverts to egui's default, never a stale choice.
                    if previous!=Some(next) {
                        if tx.send(next).is_err(){break;}
                        previous=Some(next);
                        ctx.request_repaint();
                    }
                    std::thread::sleep(std::time::Duration::from_secs(4));
                }
            });
    }
    #[cfg(not(target_os="linux"))]
    {
        let _=(tx,ctx,enabled);
    }
    rx
}

/// Returns dark (true), light (false), or unavailable (None).
/// Zero means "no preference"; for appearance, use the conventional
/// desktop light default. Absent/unsupported portal leaves egui untouched.
fn parse_read_reply(reply:&str)->Option<bool> {
    let mut found=None;
    for line in reply.lines() {
        let words=line.split_whitespace().collect::<Vec<_>>();
        let Some(at)=words.iter().position(|part|*part=="uint32") else {continue;};
        if at==0 || words.len()!=at+2
            || !words[..at].iter().all(|part|*part=="variant") {
            return None;
        }
        let current=match words[at+1] {
            "0"|"2"=>false,
            "1"=>true,
            _=>return None,
        };
        if found.replace(current).is_some(){return None;}
    }
    found
}

#[cfg(target_os="linux")]
fn read_portal()->Option<bool> {
    use std::{io::Read,process::{Command,Stdio},time::{Duration,Instant}};
    // Avoid shell expansion, user paths, stdout logging or GUI blocking.
    // The portal read is bounded even if dbus-send or the session bus stalls.
    let mut child=Command::new("dbus-send")
        .args(["--session","--print-reply","--reply-timeout=1200",
            "--dest=org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.Read",
            "string:org.freedesktop.appearance","string:color-scheme"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().ok()?;
    let until=Instant::now()+Duration::from_secs(2);
    let status=loop {
        match child.try_wait() {
            Ok(Some(status))=>break status,
            Ok(None) if Instant::now()<until=>
                std::thread::sleep(Duration::from_millis(25)),
            _=>{
                let _=child.kill();
                let _=child.wait();
                return None;
            }
        }
    };
    if !status.success(){return None;}
    let mut bytes=Vec::new();
    child.stdout.take()?.take(4097).read_to_end(&mut bytes).ok()?;
    if bytes.len()>4096{return None;}
    parse_read_reply(std::str::from_utf8(&bytes).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PREFIX:&str="method return time=1 sender=:1.4 -> destination=:1.9 serial=2 reply_serial=2\n";
    #[test] fn read_only_portal_scheme_is_parsed_without_trusting_other_text() {
        for (n,dark) in [(0,false),(1,true),(2,false)] {
            let text=format!("{PREFIX}   variant       variant          uint32 {n}\n");
            assert_eq!(parse_read_reply(&text),Some(dark));
        }
        assert_eq!(parse_read_reply(""),None);
        assert_eq!(parse_read_reply("error: no portal"),None);
        assert_eq!(parse_read_reply("uint32 1"),None);
        assert_eq!(parse_read_reply("variant uint32 3"),None);
        assert_eq!(parse_read_reply("variant uint32 1 trailing"),None);
        assert_eq!(parse_read_reply("variant uint32 1\nvariant uint32 2"),None);
        assert_eq!(parse_read_reply("variant uint32 1\nsecret uint32 2"),None);
        assert_eq!(parse_read_reply("variant int32 1"),None);
        assert_eq!(parse_read_reply("variant uint32 4294967296"),None);
    }
    #[test] fn portal_errors_never_override_egui_default() {
        assert_eq!(preference(None),egui::ThemePreference::System);
        assert_eq!(preference(Some(true)),egui::ThemePreference::Dark);
        assert_eq!(preference(Some(false)),egui::ThemePreference::Light);
    }
}
