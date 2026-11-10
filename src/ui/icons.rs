//! Jeu d'icônes de l'interface : grille 24 px, trait de 1,7, bouts arrondis.
//!
//! C'est LA référence graphique de hyprdesk : toute icône symbolique (barre,
//! menus, plugins) qu'il sait dessiner est prise ici avant le thème d'icônes,
//! pour que l'interface reste cohérente de bout en bout — un thème qui n'a
//! pas la variante symbolique d'une icône ne doit pas y glisser sa version en
//! couleur. Le thème ne sert qu'aux noms inconnus d'ici et aux icônes
//! d'applications.
use iced::widget::svg;

/// Trait estompé : arcs Wi-Fi absents, par exemple (l'opacité survit à la teinte).
const FAINT: &str = r#"stroke-opacity="0.3""#;

/// Batterie horizontale ; niveau rempli, ou éclair en charge.
fn battery(name: &str) -> String {
    let rest = name.trim_start_matches("battery-");
    let charging = rest.contains("charging");
    let level: u32 = match rest.trim_start_matches("level-").split('-').next() {
        Some("full") => 100,
        Some("good") => 70,
        Some("low") => 30,
        Some("caution") => 10,
        Some("empty") => 0,
        Some(n) => n.parse().map(|n: u32| n.min(100)).unwrap_or(50),
        None => 50,
    };
    let body = r#"<rect x="2" y="7" width="17" height="10" rx="2.5"/><path d="M22 10.5v3"/>"#;
    if charging {
        return format!(
            r##"{body}<path d="M11.5 8.8 8.3 12.6h3.2l-1 2.6 3.2-3.8h-3.2Z" fill="#ffffff" stroke-width="1"/>"##
        );
    }
    let w = 12.0 * level as f32 / 100.0;
    if w < 0.5 {
        return body.to_string();
    }
    format!(
        r##"{body}<rect x="4.5" y="9.5" width="{w:.1}" height="5" rx="1" fill="#ffffff" stroke="none"/>"##
    )
}

/// Wi-Fi : trois arcs et un point, les arcs au-delà du signal estompés.
fn wifi(bars: usize) -> String {
    const ARCS: [&str; 3] = [
        "M8.5 15.5a5 5 0 0 1 7 0",
        "M5 12a10.5 10.5 0 0 1 14 0",
        "M2 8.5a15 15 0 0 1 20 0",
    ];
    let mut out = String::from(r#"<path d="M12 19.5h.01"/>"#);
    for (i, d) in ARCS.iter().enumerate() {
        let faint = if i < bars { "" } else { FAINT };
        out.push_str(&format!(r#"<path d="{d}" {faint}/>"#));
    }
    out
}

pub fn get(name: &str) -> Option<svg::Handle> {
    let name = name.strip_suffix("-symbolic").unwrap_or(name);
    let owned: String;
    let path: &str = match name {
        _ if name.starts_with("battery") => {
            owned = battery(name);
            &owned
        }
        "network-wireless" | "network-wireless-signal-excellent" => {
            owned = wifi(3);
            &owned
        }
        "network-wireless-signal-good" => {
            owned = wifi(2);
            &owned
        }
        "network-wireless-signal-ok" => {
            owned = wifi(1);
            &owned
        }
        "network-wireless-signal-weak" | "network-wireless-signal-none" => {
            owned = wifi(0);
            &owned
        }
        "network-wireless-offline" | "network-wireless-disabled" => {
            owned = format!("{}<path d=\"M4 4l16 16\"/>", wifi(0));
            &owned
        }
        "user-desktop" | "computer" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4"/>"#
        }
        "preferences-system" | "emblem-system" => {
            r#"<path d="m9 3-.6 2.2-2 .9-2-.6-2 3.5 1.5 1.6v2.3L2.4 15l2 3.5 2-.6 2 .9L9 21h4l.6-2.2 2-.9 2 .6 2-3.5-1.5-1.6v-2.3l1.5-1.6-2-3.5-2 .6-2-.9L13 3Z"/><circle cx="11" cy="12" r="3"/>"#
        }
        "audio-volume-high" => {
            r#"<path d="m11 4-5 4H3v8h3l5 4ZM15 8a6 6 0 0 1 0 8M18 4a11 11 0 0 1 0 16"/>"#
        }
        "audio-volume-medium" | "audio-volume-low" => {
            r#"<path d="m11 4-5 4H3v8h3l5 4ZM16 8a6 6 0 0 1 0 8"/>"#
        }
        "audio-input-microphone" => {
            r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>"#
        }
        "microphone-sensitivity-muted" => {
            r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3M4 4l16 16"/>"#
        }
        "audio-volume-muted" => r#"<path d="m11 4-5 4H3v8h3l5 4ZM16 9l6 6m0-6-6 6"/>"#,
        "media-playback-start" => r#"<path d="m8 4 12 8-12 8Z"/>"#,
        "media-playback-pause" => r#"<path d="M8 4v16M16 4v16"/>"#,
        "media-skip-backward" => r#"<path d="M5 4v16m15-16L8 12l12 8Z"/>"#,
        "media-skip-forward" => r#"<path d="M19 4v16M4 4l12 8-12 8Z"/>"#,
        "object-select" => r#"<path d="m5 12 4 4L19 6"/>"#,
        "window-close" => r#"<path d="m6 6 12 12M6 18 18 6"/>"#,
        // Bureau affiché avec les fenêtres : un écran, une fenêtre devant.
        "hyprdesk-desktop-windows" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4"/><rect x="8" y="7" width="9" height="7" rx="1.5"/><path d="M8 9.5h9"/>"#
        }
        // Lanceur : une rangée d'icônes sur un socle.
        // Applications par défaut : quatre pastilles.
        // Plugins : une pièce de puzzle.
        // Assistant IA : une étoile à quatre branches et une petite.
        "ai-assistant" => {
            r#"<path d="M10 3l1.8 5.2L17 10l-5.2 1.8L10 17l-1.8-5.2L3 10l5.2-1.8z"/><path d="M18 14l.8 2.2L21 17l-2.2.8L18 20l-.8-2.2L15 17l2.2-.8z"/>"#
        }
        // Partage : une boîte, une flèche qui sort.
        // Thème : une palette à moitié remplie.
        "preferences-desktop-theme" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18Z" fill="currentColor"/>"#
        }
        "document-send" => {
            r#"<path d="M4 13v6a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-6M12 3v12M7 8l5-5 5 5"/>"#
        }
        "application-x-addon" => {
            r#"<path d="M10 4a2 2 0 1 1 4 0v2h4v4h-2a2 2 0 1 0 0 4h2v4h-4v-2a2 2 0 1 0-4 0v2H6v-4h2a2 2 0 1 0 0-4H6V6h4z"/>"#
        }
        "applications-other" => {
            r#"<circle cx="7" cy="7" r="3.5"/><circle cx="17" cy="7" r="3.5"/><circle cx="7" cy="17" r="3.5"/><path d="M17 13.5v7M13.5 17h7"/>"#
        }
        "view-app-grid" => {
            r#"<rect x="3" y="15" width="18" height="6" rx="3"/><rect x="5" y="9" width="4" height="4" rx="1"/><rect x="10" y="9" width="4" height="4" rx="1"/><rect x="15" y="9" width="4" height="4" rx="1"/>"#
        }
        "video-display" => {
            r#"<rect x="2" y="5" width="12" height="9" rx="2"/><rect x="16" y="3" width="6" height="12" rx="1.5"/><path d="M6 19h4M8 14v5"/>"#
        }
        "workspace-switcher" => {
            r#"<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="8" rx="2"/><rect x="3" y="13" width="8" height="8" rx="2"/><path d="M14 17h6m-3-3v6"/>"#
        }
        "view-grid" => {
            r#"<rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/>"#
        }
        "image-x-generic" => {
            r#"<rect x="3" y="3" width="18" height="18" rx="4"/><circle cx="8" cy="8" r="1.5"/><path d="m3 16 5-5 4 4 4-6 5 7"/>"#
        }
        "preferences-desktop-font" => r#"<path d="m3 20 6-16 6 16M5 15h8m3-5h6m-3 0v10"/>"#,
        "applications-graphics" => {
            r#"<path d="M12 3a9 9 0 1 0 0 18h2a2 2 0 0 0 1.4-3.4 1.5 1.5 0 0 1 1.1-2.6H18a3 3 0 0 0 3-3 9 9 0 0 0-9-9Z"/><path d="M7 9h.01M11 6h.01M16 8h.01M6 14h.01"/>"#
        }
        "network-wired" => {
            r#"<rect x="5" y="3" width="14" height="10" rx="2"/><path d="M12 13v5M3 21v-3h18v3M8 7h8"/>"#
        }
        "network-offline" => {
            r#"<path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 17l8 6m0-6-8 6"/>"#
        }
        "bluetooth" => r#"<path d="m7 7 10 10-5 4V3l5 4L7 17"/>"#,
        // Appareil connecté : deux points de part et d'autre.
        "bluetooth-active" => r#"<path d="m7 7 10 10-5 4V3l5 4L7 17M4 12h.01M20 12h.01"/>"#,
        "bluetooth-disabled" => {
            r#"<path d="m7 7 10 10-5 4V3l5 4L7 17" stroke-opacity="0.5"/><path d="M4 4l16 16"/>"#
        }
        "system-shutdown" => r#"<path d="M12 2v10M6 5a9 9 0 1 0 12 0"/>"#,
        // Luminosité de l'écran : un soleil.
        "display-brightness" | "brightness-display" => {
            r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#
        }
        // Rétroéclairage du clavier : un clavier, rayons au-dessus.
        "keyboard-brightness" => {
            r#"<rect x="2" y="11" width="20" height="10" rx="2.5"/><path d="M6 15h.01M10 15h.01M14 15h.01M18 15h.01M8 18h8M12 3v3M6.5 5l1.5 2M17.5 5 16 7"/>"#
        }
        "system-reboot" => r#"<path d="M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5"/>"#,
        "system-log-out" => {
            r#"<path d="M14 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4M9 8l-4 4 4 4M5 12h11"/>"#
        }
        "weather-clear-night" => {
            r#"<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z"/>"#
        }
        "application-x-executable" => {
            r#"<rect x="3" y="4" width="18" height="16" rx="3"/><path d="m7 9 3 3-3 3M12 15h5"/>"#
        }
        "folder" | "folder-open" | "system-file-manager" | "user-home" => {
            r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>"#
        }
        "system-lock-screen" => {
            r#"<rect x="5" y="10" width="14" height="11" rx="3"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>"#
        }
        "preferences-system-notifications" => {
            r#"<path d="M5 17h14l-2-3V9a5 5 0 0 0-10 0v5ZM10 21h4"/>"#
        }
        "notifications-disabled" => {
            r#"<path d="M5 17h14l-2-3V9a5 5 0 0 0-10 0v5ZM10 21h4M4 4l16 16"/>"#
        }
        _ => return None,
    };
    Some(svg::Handle::from_memory(format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"##).into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Noms d'icônes symboliques écrits dans les sources de l'interface et
    /// des plugins livrés.
    fn used_names() -> Vec<String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec![
            root.join("src/ui/bar.rs"),
            root.join("src/ui/popover.rs"),
            root.join("src/ui/widget_view.rs"),
            root.join("src/ui/osd.rs"),
        ];
        for e in std::fs::read_dir(root.join("assets/widgets/integres"))
            .unwrap()
            .flatten()
        {
            files.push(e.path());
        }
        let mut out = Vec::new();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            for (i, _) in src.match_indices("-symbolic\"") {
                let start = src[..i].rfind('"').unwrap() + 1;
                let name = &src[start..i + "-symbolic".len()];
                if !name.contains(['.', ' ']) && !name.starts_with('-') {
                    out.push(name.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Chaque icône symbolique de l'interface a son glyphe ici : aucune ne
    /// dépend du thème d'icônes (cohérence graphique de bout en bout).
    #[test]
    fn interface_entierement_dessinee() {
        let names = used_names();
        assert!(names.len() > 15, "{names:?}");
        let missing: Vec<&String> = names.iter().filter(|n| get(n).is_none()).collect();
        assert!(missing.is_empty(), "sans glyphe : {missing:?}");
        // Noms produits par les plugins (batterie : niveau et charge).
        for n in [
            "battery-level-0-symbolic",
            "battery-level-50-symbolic",
            "battery-level-100-charging-symbolic",
            "battery-caution-symbolic",
        ] {
            assert!(get(n).is_some(), "{n}");
        }
    }

    #[test]
    fn batterie_et_wifi_graduees() {
        assert_eq!(battery("battery-level-0"), battery("battery-empty"));
        assert!(battery("battery-level-50").contains(r#"width="6.0""#));
        assert!(battery("battery-level-100").contains(r#"width="12.0""#));
        assert!(battery("battery-level-40-charging").contains("M11.5"));
        assert_eq!(wifi(3).matches(FAINT).count(), 0);
        assert_eq!(wifi(1).matches(FAINT).count(), 2);
    }
}
