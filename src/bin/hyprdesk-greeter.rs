//! `hyprdesk-greeter` : écran de connexion pour greetd. Binaire dédié, lancé
//! avant toute connexion sous le compte `greeter` : il ne lie que l'écran de
//! connexion. La mise en place reste `hyprdesk greeter setup` (en root).
//!
//!   hyprdesk-greeter                 écran de connexion (Hyprland, layer-shell)
//!   hyprdesk-greeter --window        en fenêtre ordinaire (sous cage)
//!   hyprdesk-greeter --demo [--window]  essai, sans greetd
//!   hyprdesk-greeter --tty           au terminal (secours)

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(code) = hyprdesk::companion::start(hyprdesk::companion::GREETER, &mut args) {
        return code;
    }
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        [] => hyprdesk::greeter_ui::run(false, false),
        ["--window"] => hyprdesk::greeter_ui::run(false, true),
        ["--demo"] => hyprdesk::greeter_ui::run(true, false),
        ["--demo", "--window"] => hyprdesk::greeter_ui::run(true, true),
        ["--tty"] => match hyprdesk::greeter::run_tty() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", hyprdesk::tr!("hyprdesk greeter : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "{}",
                hyprdesk::tr!("usage : hyprdesk-greeter [--window | --demo [--window] | --tty]")
            );
            ExitCode::from(2)
        }
    }
}
