//! `hyprdesk-lock` : écran de verrouillage de hyprdesk (ext-session-lock-v1,
//! PAM, empreinte). Binaire dédié : il ne lie que le verrouillage et ce dont
//! il se sert, pas le démon ni Lua. Voir `hyprdesk::lock`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(code) = hyprdesk::companion::start(hyprdesk::companion::LOCK, &mut args) {
        return code;
    }
    hyprdesk::lock::run()
}
