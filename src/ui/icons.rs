//! Jeu d'icônes de l'interface : grille 24 px, trait de 1,7, bouts arrondis.
//!
//! C'est LA référence graphique de hyprdesk : toute icône symbolique (barre,
//! menus, plugins) qu'il sait dessiner est prise ici avant le thème d'icônes,
//! pour que l'interface reste cohérente de bout en bout — un thème qui n'a
//! pas la variante symbolique d'une icône ne doit pas y glisser sa version en
//! couleur. Le thème ne sert qu'aux noms inconnus d'ici et aux icônes
//! d'applications — et, pour une application qui n'en a pas, à son icône de
//! remplacement (`fallback_icon`).
use iced::widget::{container, svg, text};
use iced::{Border, Color, Font};

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
    // Icônes des paquets de Réglages sans glyphe propre : le plus proche
    // (MLG-140 : entrées du menu sans icône).
    let name = match name {
        "preferences-desktop-keyboard-shortcuts" => "input-keyboard",
        "preferences-desktop-color" => "preferences-desktop-theme",
        "preferences-system-windows" => "hyprdesk-desktop-windows",
        "document-open-recent" => "preferences-system-time",
        other => other,
    };
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
        // Hors ligne (HIG ICO-08) : les arcs du Wi-Fi, barrés comme les
        // autres variantes « indisponible ».
        "network-wireless-offline" | "network-wireless-disabled" | "network-offline" => {
            owned = format!("{}<path d=\"M4 4l16 16\"/>", wifi(0));
            &owned
        }
        // Bureau masqué (no-desktop) : l'écran, barré.
        "hyprdesk-desktop-off" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4M4 4l16 16"/>"#
        }
        "user-desktop" | "computer" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4"/>"#
        }
        "preferences-system" | "emblem-system" => {
            r#"<path d="m9 3-.6 2.2-2 .9-2-.6-2 3.5 1.5 1.6v2.3L2.4 15l2 3.5 2-.6 2 .9L9 21h4l.6-2.2 2-.9 2 .6 2-3.5-1.5-1.6v-2.3l1.5-1.6-2-3.5-2 .6-2-.9L13 3Z"/><circle cx="11" cy="12" r="3"/>"#
        }
        // Configuration avancée : une fiole de laboratoire.
        "hyprdesk-advanced" => {
            r#"<path d="M9 3h6M10 3v6.2L4.8 18.1A2 2 0 0 0 6.5 21h11a2 2 0 0 0 1.7-2.9L14 9.2V3"/><path d="M7.4 15h9.2"/>"#
        }
        "preferences-system-controls" => {
            r#"<path d="M3 7h8m5 0h5M3 17h3m5 0h10"/><circle cx="13" cy="7" r="3"/><circle cx="8" cy="17" r="3"/>"#
        }
        "view-more" => {
            r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
        }
        // Cloche des notifications (repli vectoriel de la prise de main island).
        "notifications" => {
            r#"<path d="M12 3a6 6 0 0 1 6 6v3l2 4H4l2-4V9a6 6 0 0 1 6-6ZM9.5 20a2.5 2.5 0 0 0 5 0"/>"#
        }
        "audio-volume-high" => {
            r#"<path d="m11 4-5 4H3v8h3l5 4ZM15 8a6 6 0 0 1 0 8M18 4a11 11 0 0 1 0 16"/>"#
        }
        "audio-volume-medium" => r#"<path d="m11 4-5 4H3v8h3l5 4ZM16 8a6 6 0 0 1 0 8"/>"#,
        // Volume bas : une seule petite onde (HIG ICO-08).
        "audio-volume-low" => r#"<path d="m11 4-5 4H3v8h3l5 4ZM15 10a3 3 0 0 1 0 4"/>"#,
        "audio-input-microphone" => {
            r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>"#
        }
        "microphone-sensitivity-muted" => {
            r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3M4 4l16 16"/>"#
        }
        "audio-volume-muted" => r#"<path d="m11 4-5 4H3v8h3l5 4ZM16 8a6 6 0 0 1 0 8M4 4l16 16"/>"#,
        "media-playback-start" => r#"<path d="m8 4 12 8-12 8Z"/>"#,
        "media-playback-pause" => r#"<path d="M8 4v16M16 4v16"/>"#,
        "media-playback-stop" => r#"<rect x="6" y="6" width="12" height="12" rx="1.5"/>"#,
        "media-skip-backward" => r#"<path d="M5 4v16m15-16L8 12l12 8Z"/>"#,
        "media-skip-forward" => r#"<path d="M19 4v16M4 4l12 8-12 8Z"/>"#,
        "object-select" => r#"<path d="m5 12 4 4L19 6"/>"#,
        "window-close" => r#"<path d="m6 6 12 12M6 18 18 6"/>"#,
        // Bureau affiché avec les fenêtres : un écran, une fenêtre devant.
        "hyprdesk-desktop-windows" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4"/><rect x="8" y="7" width="9" height="7" rx="1.5"/><path d="M8 9.5h9"/>"#
        }
        // Fenêtre urgente : un losange plein, posé à côté du numéro d'espace.
        "hyprdesk-urgent" => {
            r##"<path d="M12 6.2 17.8 12 12 17.8 6.2 12Z" fill="#ffffff" stroke="none"/>"##
        }
        // Lanceur : une rangée d'icônes sur un socle.
        // Applications par défaut : quatre pastilles.
        // Plugins : une pièce de puzzle.
        // Assistant IA : une étoile à quatre branches et une petite.
        "ai-assistant" => {
            r#"<path d="M10 3l1.8 5.2L17 10l-5.2 1.8L10 17l-1.8-5.2L3 10l5.2-1.8z"/><path d="M18 14l.8 2.2L21 17l-2.2.8L18 20l-.8-2.2L15 17l2.2-.8z"/>"#
        }
        // À propos : un « i » dans un cercle.
        "help-about" | "dialog-information" | "document-properties" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5v.5"/>"#
        }
        // Partage : une boîte, une flèche qui sort.
        // Thème : une palette à moitié remplie.
        "preferences-desktop-theme" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18Z" fill="currentColor"/>"#
        }
        "document-send" | "send-to" => {
            r#"<path d="M4 13v6a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-6M12 3v12M7 8l5-5 5 5"/>"#
        }
        "application-x-addon" => {
            r#"<path d="M10 4a2 2 0 1 1 4 0v2h4v4h-2a2 2 0 1 0 0 4h2v4h-4v-2a2 2 0 1 0-4 0v2H6v-4h2a2 2 0 1 0 0-4H6V6h4z"/>"#
        }
        "applications-other" => {
            r#"<circle cx="7" cy="7" r="3.5"/><circle cx="17" cy="7" r="3.5"/><circle cx="7" cy="17" r="3.5"/><path d="M17 13.5v7M13.5 17h7"/>"#
        }
        "grip-vertical" => {
            r#"<g fill="currentColor" stroke="none"><circle cx="9" cy="5" r="1.3"/><circle cx="15" cy="5" r="1.3"/><circle cx="9" cy="12" r="1.3"/><circle cx="15" cy="12" r="1.3"/><circle cx="9" cy="19" r="1.3"/><circle cx="15" cy="19" r="1.3"/></g>"#
        }
        "system-search" | "edit-find" => {
            r#"<circle cx="10.5" cy="10.5" r="7.5"/><path d="m16 16 5 5"/>"#
        }
        "hyprdesk-start" => {
            // Le h plié du logo livré dans assets/greeter/logo.png.
            r##"<defs><linearGradient id="hyprdesk-mark" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#00d8b2"/><stop offset="1" stop-color="#00b9a5"/></linearGradient></defs><g fill="url(#hyprdesk-mark)" stroke="none"><path d="M2 14.8V4.8a3 3 0 0 1 3-3h2.2a3 3 0 0 1 3 3v5.8l6-2.1c2.9-1 5.8 1.1 5.8 4.2v6.5a3 3 0 0 1-3 3h-2.4a3 3 0 0 1-3-3v-3.9c0-1.8-.8-2.3-3.3-1.9-2.8.4-5.7 1.1-8.3 2z"/><path d="M2 17.2c0-1.4 1.9-1.7 3.8-2.2l4.4-1v5.2a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z"/></g>"##
        }
        "hyprdesk-task-view" => {
            r#"<rect x="3" y="7" width="14" height="14" rx="2"/><rect x="7" y="3" width="14" height="14" rx="2"/>"#
        }
        // Lanceur d'applications : la grille de ses tuiles.
        "hyprdesk-apps" => {
            r#"<rect x="3" y="3" width="5" height="5" rx="1.5"/><rect x="9.5" y="3" width="5" height="5" rx="1.5"/><rect x="16" y="3" width="5" height="5" rx="1.5"/><rect x="3" y="9.5" width="5" height="5" rx="1.5"/><rect x="9.5" y="9.5" width="5" height="5" rx="1.5"/><rect x="16" y="9.5" width="5" height="5" rx="1.5"/><rect x="3" y="16" width="5" height="5" rx="1.5"/><rect x="9.5" y="16" width="5" height="5" rx="1.5"/><rect x="16" y="16" width="5" height="5" rx="1.5"/>"#
        }
        "view-app-grid" => {
            r#"<rect x="3" y="15" width="18" height="6" rx="3"/><rect x="5" y="9" width="4" height="4" rx="1"/><rect x="10" y="9" width="4" height="4" rx="1"/><rect x="15" y="9" width="4" height="4" rx="1"/>"#
        }
        // Carte graphique : ventilateur et connecteur PCIe (widget gpu, MLG-162).
        "hyprdesk-gpu" => {
            r#"<rect x="2" y="5" width="20" height="12" rx="2"/><circle cx="9" cy="11" r="3"/><path d="M15 9h4M15 13h4M6 17v3h9v-3"/>"#
        }
        "video-display" => {
            r#"<rect x="2" y="5" width="12" height="9" rx="2"/><rect x="16" y="3" width="6" height="12" rx="1.5"/><path d="M6 19h4M8 14v5"/>"#
        }
        "workspace-switcher" => {
            r#"<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="8" rx="2"/><rect x="3" y="13" width="8" height="8" rx="2"/><path d="M14 17h6m-3-3v6"/>"#
        }
        // Panorama côte à côte : le média à gauche, la zone d'accueil (+) à droite.
        "hyprdesk-split" => {
            r#"<rect x="2" y="4" width="20" height="16" rx="3"/><path d="M12 4v16M5.5 9.5 9 12l-3.5 2.5ZM17 9.5v5M14.5 12h5"/>"#
        }
        // Inverser les deux zones.
        "hyprdesk-split-swap" => r#"<path d="M4 8h15m-4-4 4 4-4 4M20 16H5m4-4-4 4 4 4"/>"#,
        // Retour sans partage : le média seul.
        "hyprdesk-split-off" => {
            r#"<rect x="2" y="4" width="20" height="16" rx="3"/><path d="m10 9 5 3-5 3Z"/>"#
        }
        "list-add" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "go-previous" => r#"<path d="m15 5-7 7 7 7"/>"#,
        "go-next" => r#"<path d="m9 5 7 7-7 7"/>"#,
        "go-down" => r#"<path d="m5 9 7 7 7-7"/>"#,
        "user-trash" | "edit-delete" => {
            r#"<path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/>"#
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
        "bluetooth" => r#"<path d="m7 7 10 10-5 4V3l5 4L7 17"/>"#,
        // Appareil connecté : deux points de part et d'autre.
        "bluetooth-active" => r#"<path d="m7 7 10 10-5 4V3l5 4L7 17M4 12h.01M20 12h.01"/>"#,
        "bluetooth-disabled" => {
            r#"<path d="m7 7 10 10-5 4V3l5 4L7 17" stroke-opacity="0.5"/><path d="M4 4l16 16"/>"#
        }
        "system-shutdown" => r#"<path d="M12 2v10M6 5a9 9 0 1 0 12 0"/>"#,
        // Alerte : un triangle et son point d'exclamation (HIG SYS-16).
        "dialog-warning" => {
            r#"<path d="M10.4 4.4 2.9 17.6a1.9 1.9 0 0 0 1.6 2.9h15a1.9 1.9 0 0 0 1.6-2.9L13.6 4.4a1.9 1.9 0 0 0-3.2 0Z"/><path d="M12 9.5v4.5M12 17.3h.01"/>"#
        }
        // Éclairage nocturne : un croissant de lune ; barré quand il est coupé.
        "night-light" => r#"<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z"/>"#,
        "night-light-disabled" => {
            r#"<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5ZM4 4l16 16"/>"#
        }
        // Support amovible : une clé USB.
        "drive-removable-media" => {
            r#"<rect x="7" y="9" width="10" height="13" rx="2"/><path d="M9 9V3h6v6M11 5.5h.01M13 5.5h.01"/>"#
        }
        // Éjecter : un triangle sur un trait.
        "media-eject" => r#"<path d="M5 14 12 6l7 8ZM5 18h14"/>"#,
        // Capture : un appareil photo.
        "camera-photo" => {
            r#"<path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13" r="3.5"/>"#
        }
        // Enregistrement : un disque plein.
        "media-record" => r#"<circle cx="12" cy="12" r="6" fill="currentColor"/>"#,
        // Presse-papiers : une planchette et sa pince.
        // Lumières (Hue) : une ampoule.
        "lightbulb" | "hue-lights" => {
            r#"<path d="M9 18h6M10 21h4"/><path d="M12 3a6 6 0 0 0-3.5 10.9c.9.7 1.5 1.6 1.5 2.6V17h4v-.5c0-1 .6-1.9 1.5-2.6A6 6 0 0 0 12 3z"/>"#
        }
        // Calendrier : une page à anneaux.
        "x-office-calendar" | "calendar" => {
            r#"<rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 10h16M9 3v4M15 3v4"/>"#
        }
        // Notes : un crayon.
        "accessories-text-editor" | "document-edit" => {
            r#"<path d="M4 20h4L18.5 9.5a2.1 2.1 0 0 0-3-3L5 17v3z"/><path d="M14 8l3 3"/>"#
        }
        "edit-paste" => {
            r#"<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 10h6M9 14h6M9 18h3"/>"#
        }
        // Démarrage automatique : le symbole marche.
        "hyprdesk-autostart" => r#"<path d="M12 3v8M7 5.8a8 8 0 1 0 10 0"/>"#,
        // Clavier : un rectangle et ses rangées de touches.
        "input-keyboard" => {
            r#"<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M7 14h10"/>"#
        }
        // Profils d'énergie : une jauge, l'aiguille selon le profil.
        "power-profile-performance" => r#"<path d="M4 17a8 8 0 1 1 16 0M12 17l4.5-5"/>"#,
        "power-profile-balanced" => r#"<path d="M4 17a8 8 0 1 1 16 0M12 17V10"/>"#,
        "power-profile-power-saver" => r#"<path d="M4 17a8 8 0 1 1 16 0M12 17 7.5 12"/>"#,
        // Mises à jour : une flèche qui descend dans un plateau.
        "software-update-available" | "system-software-update" => {
            r#"<path d="M12 3v11m-4-4 4 4 4-4M4 15v3a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-3"/>"#
        }
        // À jour : une coche au-dessus du plateau.
        "software-update-none" => {
            r#"<path d="m7 8 3.5 3.5L17 5M4 15v3a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-3"/>"#
        }
        // Animations : un disque en mouvement, sa traînée derrière lui.
        // Date et heure : une horloge.
        "preferences-system-time" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#,
        "preferences-desktop-animation" => {
            r#"<circle cx="15.5" cy="12" r="5.5"/><path d="M2 9h5M3 12h4M2 15h5"/>"#
        }
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
        // Mot de passe : une clé.
        "dialog-password" => {
            r#"<circle cx="8" cy="15" r="4"/><path d="m11 12 8-8M16 7l3 3M14 9l2 2"/>"#
        }
        // Écran de connexion : un écran, une silhouette au milieu.
        "system-users" => {
            r#"<rect x="3" y="4" width="18" height="13" rx="2"/><circle cx="12" cy="9" r="2.5"/><path d="M8 15a4 4 0 0 1 8 0M9 21h6M12 17v4"/>"#
        }
        "preferences-system-notifications" => {
            r#"<path d="M5 17h14l-2-3V9a5 5 0 0 0-10 0v5ZM10 21h4"/>"#
        }
        // Menus du bureau, des fichiers, de la fenêtre et Panorama (HIG
        // ICO-06) : même grille, même trait que le reste du jeu.
        "document-open" => {
            r#"<path d="M3 17V7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v1M3 17l2.6-6.1A1.5 1.5 0 0 1 7 10h13a1 1 0 0 1 .9 1.4L18.4 18a1.5 1.5 0 0 1-1.4 1H5a2 2 0 0 1-2-2Z"/>"#
        }
        "edit-copy" => {
            r#"<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>"#
        }
        "edit-cut" => {
            r#"<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M8.5 7.5 20 19M8.5 16.5 20 5"/>"#
        }
        "edit-select-all" => {
            r#"<path d="M4 7V6a2 2 0 0 1 2-2h1M11 4h2M17 4h1a2 2 0 0 1 2 2v1M20 11v2M20 17v1a2 2 0 0 1-2 2h-1M13 20h-2M7 20H6a2 2 0 0 1-2-2v-1M4 13v-2"/><rect x="8" y="8" width="8" height="8" rx="1.5"/>"#
        }
        "list-remove" => r#"<path d="M5 12h14"/>"#,
        "go-jump" => {
            r#"<path d="M14 4h6v6M20 4l-9 9M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"/>"#
        }
        "mail-send" => r#"<path d="M21 3 10 14M21 3l-6.5 18-4.5-7-7-4.5Z"/>"#,
        "phone" => {
            r#"<path d="M5 3h3l2 5-2.5 1.5a11 11 0 0 0 7 7L16 14l5 2v3a2 2 0 0 1-2 2A16 16 0 0 1 3 5a2 2 0 0 1 2-2Z"/>"#
        }
        "preferences-desktop-wallpaper" => {
            r#"<rect x="3" y="4" width="18" height="16" rx="3"/><circle cx="9" cy="9.5" r="1.5"/><path d="m20.5 15.5-4.5-4.5-9.5 9"/>"#
        }
        "window-minimize" => r#"<path d="M6 17h12"/>"#,
        "window-maximize" => r#"<rect x="4" y="4" width="16" height="16" rx="3"/>"#,
        "window-new" => {
            r#"<path d="M13 5H6a3 3 0 0 0-3 3v10a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-8M3 10h10M19 2v6M16 5h6"/>"#
        }
        "view-fullscreen" => r#"<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/>"#,
        // Fenêtre flottante : une fenêtre décalée au-dessus d'une autre.
        "focus-windows" => {
            r#"<path d="M8 14H5.5A2.5 2.5 0 0 1 3 11.5v-6A2.5 2.5 0 0 1 5.5 3h8A2.5 2.5 0 0 1 16 5.5V10"/><rect x="8" y="10" width="13" height="11" rx="2.5"/>"#
        }
        "view-pin" => r#"<path d="M9 4h6M10 4v6l-3 4h10l-3-4V4M12 14v7"/>"#,
        "media-seek-backward" => r#"<path d="M10.5 6 3 12l7.5 6ZM21 6l-7.5 6 7.5 6Z"/>"#,
        "media-seek-forward" => r#"<path d="m13.5 6 7.5 6-7.5 6ZM3 6l7.5 6-7.5 6Z"/>"#,
        "audio-x-generic" => {
            r#"<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>"#
        }
        "video-x-generic" => {
            r#"<rect x="2" y="6" width="14" height="12" rx="2.5"/><path d="m16 10 6-3v10l-6-3"/>"#
        }
        "notifications-disabled" => {
            r#"<path d="M5 17h14l-2-3V9a5 5 0 0 0-10 0v5ZM10 21h4M4 4l16 16"/>"#
        }
        _ => return None,
    };
    Some(svg::Handle::from_memory(format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"##).into_bytes()))
}

/// Glyphe du jeu tel qu'il se pose dans un bouton : `size` pixels logiques de
/// côté, teinté par `color`. `None` pour un nom que le jeu ne dessine pas.
///
/// Fermer, lire, mettre en pause s'écrivent avec lui, jamais avec un caractère
/// de police (HIG ICO-07) : même trait et même graisse que le reste de
/// l'interface.
pub(crate) fn glyph<'a, Message>(
    name: &str,
    size: f32,
    color: iced::Color,
) -> Option<iced::Element<'a, Message>> {
    get(name).map(|handle| {
        svg(handle)
            .width(size)
            .height(size)
            .style(move |_, _| svg::Style { color: Some(color) })
            .into()
    })
}

/// Masque unique des icônes d'application (HIG ICO-04, décision D4) : un
/// rectangle arrondi dont le rayon vaut 22,37 % du côté, la courbure des
/// icônes d'application de macOS. L'icône de remplacement le prend aussi.
pub const APP_ICON_MASK: f32 = 0.2237;

/// Identifiant du découpage injecté dans un SVG d'application.
const MASK_ID: &str = "hyprdesk-masque-icone";

/// Découpe un SVG d'application au masque unique (HIG ICO-04) : iced ne sait
/// pas arrondir un SVG, la découpe se fait donc dans sa source, un
/// `clipPath` en rectangle arrondi autour de tout son contenu. `None` quand
/// la source ne s'y prête pas (racine introuvable ou auto-fermée, sans
/// `viewBox` ni dimensions) : l'icône reste alors telle quelle.
pub fn mask_svg(src: &str, ratio: f32) -> Option<String> {
    let start = src.find("<svg")?;
    // Fin de la balise racine : le premier `>` hors guillemets.
    let mut quote = None;
    let mut end = None;
    for (i, c) in src[start..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => {
                end = Some(start + i);
                break;
            }
            _ => {}
        }
    }
    let end = end?;
    let tag = &src[start..end];
    if tag.ends_with('/') {
        return None;
    }
    let close = src.rfind("</svg>").filter(|c| *c > end)?;
    let attr = |name: &str| -> Option<&str> {
        let mut rest = tag;
        loop {
            let i = rest.find(name)?;
            let before = rest[..i].chars().last();
            let after = rest[i + name.len()..].trim_start();
            rest = &rest[i + name.len()..];
            if before.is_some_and(char::is_whitespace)
                && let Some(v) = after.strip_prefix('=')
            {
                let v = v.trim_start();
                let q = v.chars().next().filter(|q| *q == '"' || *q == '\'')?;
                let v = &v[1..];
                return Some(&v[..v.find(q)?]);
            }
        }
    };
    let num = |s: &str| -> Option<f32> {
        s.trim()
            .trim_end_matches("px")
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
    };
    let (x, y, w, h) = match attr("viewBox") {
        Some(vb) => {
            let v: Vec<f32> = vb
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|s| !s.is_empty())
                .filter_map(num)
                .collect();
            match v[..] {
                [x, y, w, h] => (x, y, w, h),
                _ => return None,
            }
        }
        None => (0.0, 0.0, num(attr("width")?)?, num(attr("height")?)?),
    };
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = w.min(h) * ratio;
    Some(format!(
        "{head}<defs><clipPath id=\"{MASK_ID}\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" ry=\"{r}\"/></clipPath></defs><g clip-path=\"url(#{MASK_ID})\">{body}</g>{tail}",
        head = &src[..=end],
        body = &src[end + 1..close],
        tail = &src[close..],
    ))
}

/// Initiale d'une application sans icône : la première lettre de son nom,
/// en majuscule.
pub fn initial(name: &str) -> String {
    name.chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string()
}

/// Icône de remplacement d'une application sans icône : son initiale, en
/// graisse de titre (`font`), sur une tuile carrée de `side` px à l'accent,
/// encrée de `on` (une encre lisible sur cet accent). Un seul dessin, le
/// même au dock, au lanceur et au bureau : l'application s'y reconnaît
/// pareil.
pub fn fallback_icon<'a, Message: 'a>(
    name: &str,
    side: f32,
    accent: Color,
    on: Color,
    font: Font,
) -> iced::Element<'a, Message> {
    container(text(initial(name)).font(font).size(side * 0.5).color(on))
        .center(side)
        .style(move |_| container::Style {
            background: Some(accent.into()),
            border: Border::default().rounded(side * APP_ICON_MASK),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    /// Le masque des icônes d'application enveloppe tout le contenu, d'après
    /// la `viewBox` ou les dimensions ; sinon l'icône reste telle quelle
    /// (HIG ICO-04).
    #[test]
    fn masque_des_icones_d_application() {
        let m = |s: &str| super::mask_svg(s, 0.25);
        let out = m(r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><rect width="48" height="48"/></svg>"#).unwrap();
        assert!(out.starts_with(r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><defs><clipPath"#));
        assert!(out.contains(r#"<rect x="0" y="0" width="48" height="48" rx="12" ry="12"/>"#));
        assert!(out.ends_with(r#"<rect width="48" height="48"/></g></svg>"#));
        let out = m(r#"<svg width="64px" height="32" data-x="a>b"><g/></svg>"#).unwrap();
        assert!(out.contains(r#"width="64" height="32" rx="8""#), "{out}");
        assert!(
            out.contains(r#"data-x="a>b"><defs>"#),
            "le > entre guillemets"
        );
        let out = m(r#"<svg viewBox="-2,-2 20 20"><path/></svg>"#).unwrap();
        assert!(
            out.contains(r#"x="-2" y="-2" width="20" height="20" rx="5""#),
            "{out}"
        );
        assert!(
            m(r#"<svg viewBox="0 0 16 16"/>"#).is_none(),
            "racine auto-fermée"
        );
        assert!(m(r#"<svg><path/></svg>"#).is_none(), "sans dimensions");
        assert!(m("pas un svg").is_none());
    }

    use super::*;

    /// Noms d'icônes symboliques écrits dans les sources de l'interface et
    /// des plugins livrés.
    fn used_names() -> Vec<String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec![
            root.join("src/ui/plugins/bar/surface.rs"),
            root.join("src/ui/plugins/bar/popover.rs"),
            root.join("src/ui/widget_view.rs"),
            root.join("src/ui/osd.rs"),
            // Menus du bureau, de la fenêtre et Panorama (HIG ICO-06).
            root.join("src/ui/context.rs"),
            root.join("src/ui/plugins/bar/window.rs"),
            root.join("src/panorama/design.rs"),
        ];
        for e in std::fs::read_dir(root.join("assets/widgets/integres"))
            .unwrap()
            .flatten()
        {
            files.push(e.path());
        }
        let tests_last = [
            "src/ui/context.rs",
            "src/ui/plugins/bar/window.rs",
            "src/panorama/design.rs",
        ];
        let mut out = Vec::new();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            // Les noms des tests de ces fichiers sont ceux d'applications,
            // pas de l'interface ; leurs tests sont en fin de fichier.
            let src = if tests_last.iter().any(|t| f.ends_with(t)) {
                src.split("#[cfg(test)]").next().unwrap_or_default()
            } else {
                &src
            };
            for (i, _) in src.match_indices("-symbolic\"") {
                let start = src[..i].rfind('"').unwrap() + 1;
                let name = &src[start..i + "-symbolic".len()];
                // Un gabarit de `format!` (icône d'une action d'application)
                // n'est pas un nom.
                if !name.contains(['.', ' ', '{']) && !name.starts_with('-') {
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

    /// HIG ICO-08 : une variante « indisponible » porte la même barre que les
    /// autres ; volume bas et moyen ne se confondent pas.
    #[test]
    fn variantes_indisponibles_barrees_de_meme() {
        let src = |n: &str| match get(n).map(|h| h.data().clone()) {
            Some(iced::advanced::svg::Data::Bytes(b)) => String::from_utf8(b.to_vec()).unwrap(),
            other => panic!("{n} : {other:?}"),
        };
        for n in [
            "audio-volume-muted",
            "network-offline",
            "network-wireless-offline",
            "hyprdesk-desktop-off",
            "notifications-disabled",
        ] {
            assert!(src(n).contains("M4 4l16 16"), "{n}");
        }
        assert_ne!(src("audio-volume-low"), src("audio-volume-medium"));
    }

    /// HIG ICO-06 : les commandes des menus du bureau, de la fenêtre et du
    /// Panorama ont leur glyphe dans le jeu.
    #[test]
    fn glyphes_des_menus() {
        for name in [
            "audio-x-generic",
            "document-open",
            "document-properties",
            "edit-copy",
            "edit-cut",
            "edit-select-all",
            "focus-windows",
            "go-jump",
            "list-remove",
            "mail-send",
            "media-seek-backward",
            "media-seek-forward",
            "phone",
            "preferences-desktop-wallpaper",
            "send-to",
            "video-x-generic",
            "view-fullscreen",
            "view-pin",
            "window-maximize",
            "window-minimize",
            "window-new",
        ] {
            assert!(get(&format!("{name}-symbolic")).is_some(), "{name}");
        }
    }

    /// Fermer, lire, mettre en pause : les glyphes des boutons, jamais un
    /// caractère de police (HIG ICO-07).
    #[test]
    fn glyphes_des_boutons() {
        for name in [
            "window-close",
            "media-playback-start",
            "media-playback-pause",
            "battery-level-40-charging",
            "dialog-warning",
        ] {
            let g: Option<iced::Element<'_, ()>> = glyph(name, 16.0, iced::Color::BLACK);
            assert!(g.is_some(), "{name}");
        }
        let absent: Option<iced::Element<'_, ()>> = glyph("nom-hors-jeu", 16.0, iced::Color::BLACK);
        assert!(absent.is_none());
    }

    /// Initiale d'une application sans icône.
    #[test]
    fn initiale_de_replacement() {
        assert_eq!(initial("firefox"), "F");
        assert_eq!(initial("été"), "É");
        assert_eq!(initial(""), "?");
    }

    /// Dock, lanceur et bureau passent par l'unique fonction d'icône de
    /// remplacement : aucune surface ne la redessine sur place.
    #[test]
    fn icone_de_replacement_partagee() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for f in [
            "src/ui/mod.rs",
            "src/ui/plugins/dock/surface.rs",
            "src/ui/plugins/launcher/grid.rs",
        ] {
            let src = std::fs::read_to_string(root.join(f)).unwrap();
            assert!(src.contains("fallback_icon("), "{f} : dessin propre");
            assert!(!src.contains(".to_uppercase()"), "{f} : initiale propre");
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
