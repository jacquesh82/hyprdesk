//! Icônes vectorielles de secours : grille 24 px, trait cohérent et sans police d'icônes.
use iced::widget::svg;

pub fn get(name: &str) -> Option<svg::Handle> {
    let name = name.strip_suffix("-symbolic").unwrap_or(name);
    let path = match name {
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
        "network-wireless"
        | "network-wireless-signal-excellent"
        | "network-wireless-signal-good"
        | "network-wireless-signal-ok"
        | "network-wireless-signal-weak" => {
            r#"<path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0m-10 4a5 5 0 0 1 6 0M12 20h.01"/>"#
        }
        "network-wired" => {
            r#"<rect x="5" y="3" width="14" height="10" rx="2"/><path d="M12 13v5M3 21v-3h18v3M8 7h8"/>"#
        }
        "network-offline" => {
            r#"<path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 17l8 6m0-6-8 6"/>"#
        }
        "bluetooth" | "bluetooth-active" | "bluetooth-disabled" => {
            r#"<path d="m7 7 10 10-5 4V3l5 4L7 17"/>"#
        }
        "system-shutdown" | "system-reboot" => r#"<path d="M12 2v10M6 5a9 9 0 1 0 12 0"/>"#,
        "system-lock-screen" => {
            r#"<rect x="5" y="10" width="14" height="11" rx="3"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>"#
        }
        "preferences-system-notifications" | "notifications-disabled" => {
            r#"<path d="M5 17h14l-2-3V9a5 5 0 0 0-10 0v5ZM10 21h4"/>"#
        }
        _ if name.starts_with("battery-") => {
            r#"<rect x="2" y="6" width="18" height="12" rx="3"/><path d="M23 10v4M6 10v4m4-4v4m4-4v4"/>"#
        }
        _ => return None,
    };
    Some(svg::Handle::from_memory(format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"##).into_bytes()))
}
