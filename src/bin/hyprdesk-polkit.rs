//! `hyprdesk-polkit` : agent d'authentification polkit de la session.
//! Binaire dédié, lancé et supervisé par le démon : il ne lie que l'agent
//! et sa carte.
//!
//!   hyprdesk-polkit          agent avec sa carte (Wayland)
//!   hyprdesk-polkit --tty    au terminal, comme pkttyagent

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let config = match hyprdesk::companion::start(hyprdesk::companion::POLKIT, &mut args) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match args.first().map(String::as_str) {
        None => hyprdesk::ui::polkit::run(config),
        Some("--tty") => hyprdesk::polkit::tty_agent(),
        Some(_) => {
            eprintln!("{}", hyprdesk::tr!("usage : hyprdesk-polkit [--tty]"));
            ExitCode::from(2)
        }
    }
}
