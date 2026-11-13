//! Paquet intégré « Raccourcis » (MLG-109) : tous les raccourcis actifs,
//! hors plein bureau et bureau affiché ; ceux de hyprdesk ; les essentiels
//! de fenêtres, de workspaces et d'écrans que hyprdesk pose là où
//! hyprland.lua n'en a pas (`crate::essentials`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::widget::{Space, button, column, container, row, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::essentials::{CATALOGUE, Category, Group, Line, Origin, State as Ess, pretty};

pub const INFO: PluginInfo = PluginInfo {
    id: "keys",
    label: "Raccourcis",
    icon: "preferences-desktop-keyboard-shortcuts",
    summary: "Tous les raccourcis, hors plein bureau et bureau affiché : ceux de hyprdesk, ceux de hyprland.lua, et les essentiels de fenêtres et de workspaces.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("ensemble", "Vue d'ensemble"),
    ("hyprdesk", "Raccourcis de hyprdesk"),
    ("essentiels", "Fenêtres et workspaces"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Keys(m)))
}

/// Raccourci réglable : un de hyprdesk (case de `State::keys`) ou un
/// essentiel (`swap.left`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Hyprdesk(usize),
    Essential(String),
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    /// Raccourci en cours de capture (MLG-110).
    pub capture: Option<Target>,
    /// Numéro de la capture : un délai écoulé d'une capture finie est ignoré.
    pub capture_seq: u64,
    pub keys: [String; 10],
    pub key_error: Option<String>,
    /// Commande de vue d'ensemble en cours de saisie.
    pub overview_cmd: Option<String>,
    /// Vue d'ensemble : bureau affiché (sinon hors plein bureau).
    pub desktop_view: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    SuspendBinds(bool),
    /// Saisie d'un raccourci (0 changer de mode, 1 réglages, 2 secours, 3 launcher-only, 4 no-desktop…).
    KeyInput(usize, String),
    KeySubmit(usize),
    /// Raccourci en conflit avec hyprland.lua : hyprdesk le prend (keys.take)…
    TakeKey(String),
    /// … ou le rend.
    ReleaseKey(String),
    /// Commande de vue d'ensemble : saisie, puis validée.
    OverviewCmd(String),
    OverviewCmdSubmit,
    /// Vue d'ensemble : bureau affiché, ou hors plein bureau.
    View(bool),
    /// Tous les essentiels, posés ou non.
    Essentials(bool),
    /// Un essentiel : posé ou coupé.
    Essential(&'static str, bool),
    /// Relevé des raccourcis (Hyprland, hyprland.lua), sans rien installer.
    Report(Result<crate::keymap::Report, String>),
    /// Capturer un raccourci : la prochaine combinaison pressée.
    Capture(Target),
    CaptureCancel,
    /// Touche pressée pendant la capture (sans modificateurs appliqués).
    CaptureKey(Key, Physical, Modifiers),
    /// Délai de capture écoulé (numéro de la capture).
    CaptureTimeout(u64),
    /// Revenir à la valeur par défaut.
    Reset(Target),
}

/// Valeur par défaut d'un raccourci de hyprdesk (case de `State::keys`).
pub fn default_key(i: usize) -> String {
    let k = crate::config::Keys::default();
    match i {
        0 => k.toggle,
        1 => k.settings,
        2 => k.leave,
        3 => k.launcher,
        4 => k.hide,
        5 => k.lock,
        6 => k.search,
        7 => k.switch,
        8 => k.overview,
        _ => k.search_desktop,
    }
}

/// Nom Hyprland (keysym xkb) d'un caractère sans modificateur.
fn keysym(c: char) -> Option<String> {
    if c.is_ascii_alphanumeric() {
        return Some(c.to_ascii_lowercase().to_string());
    }
    Some(
        match c {
            '&' => "ampersand",
            'é' => "eacute",
            '"' => "quotedbl",
            '\'' => "apostrophe",
            '(' => "parenleft",
            ')' => "parenright",
            '-' => "minus",
            'è' => "egrave",
            '_' => "underscore",
            'ç' => "ccedilla",
            'à' => "agrave",
            'ù' => "ugrave",
            '=' => "equal",
            '+' => "plus",
            ',' => "comma",
            ';' => "semicolon",
            ':' => "colon",
            '!' => "exclam",
            '?' => "question",
            '.' => "period",
            '/' => "slash",
            '\\' => "backslash",
            '*' => "asterisk",
            '$' => "dollar",
            '£' => "sterling",
            '%' => "percent",
            'µ' => "mu",
            '<' => "less",
            '>' => "greater",
            '[' => "bracketleft",
            ']' => "bracketright",
            '{' => "braceleft",
            '}' => "braceright",
            '#' => "numbersign",
            '@' => "at",
            '^' => "asciicircum",
            '~' => "asciitilde",
            '`' => "grave",
            '|' => "bar",
            '²' => "twosuperior",
            '§' => "section",
            '°' => "degree",
            _ => return None,
        }
        .to_string(),
    )
}

/// Touche nommée → keysym de Hyprland ; `None` pour un modificateur seul.
/// Le booléen : la touche peut servir sans modificateur (F1, Impr. écran…).
fn named(n: Named) -> Option<(String, bool)> {
    let s = |v: &str, bare: bool| Some((v.to_string(), bare));
    match n {
        Named::Shift
        | Named::Control
        | Named::Alt
        | Named::AltGraph
        | Named::Super
        | Named::Meta
        | Named::Hyper
        | Named::CapsLock
        | Named::NumLock
        | Named::Fn => None,
        Named::Enter => s("Return", false),
        Named::Tab => s("Tab", false),
        Named::Space => s("space", false),
        Named::Backspace => s("BackSpace", false),
        Named::Delete => s("Delete", false),
        Named::Insert => s("Insert", false),
        Named::Home => s("Home", false),
        Named::End => s("End", false),
        Named::PageUp => s("Page_Up", false),
        Named::PageDown => s("Page_Down", false),
        Named::ArrowLeft => s("left", false),
        Named::ArrowRight => s("right", false),
        Named::ArrowUp => s("up", false),
        Named::ArrowDown => s("down", false),
        Named::Escape => s("Escape", false),
        Named::PrintScreen => s("Print", true),
        Named::Pause => s("Pause", true),
        Named::ScrollLock => s("Scroll_Lock", true),
        Named::ContextMenu => s("Menu", true),
        Named::AudioVolumeUp => s("XF86AudioRaiseVolume", true),
        Named::AudioVolumeDown => s("XF86AudioLowerVolume", true),
        Named::AudioVolumeMute => s("XF86AudioMute", true),
        Named::MediaPlayPause => s("XF86AudioPlay", true),
        Named::MediaTrackNext => s("XF86AudioNext", true),
        Named::MediaTrackPrevious => s("XF86AudioPrev", true),
        Named::MediaStop => s("XF86AudioStop", true),
        Named::BrightnessUp => s("XF86MonBrightnessUp", true),
        Named::BrightnessDown => s("XF86MonBrightnessDown", true),
        other => {
            // F1 à F35.
            let d = format!("{other:?}");
            let f_key =
                d.len() > 1 && d.starts_with('F') && d[1..].chars().all(|c| c.is_ascii_digit());
            f_key.then_some((d, true))
        }
    }
}

/// Combinaison Hyprland d'une touche capturée. `Ok(None)` : un modificateur
/// seul, on attend la suite. La rangée des chiffres et la touche ² passent
/// par leur code physique (`code:10`), juste quelle que soit la disposition.
pub fn combo_of(key: &Key, physical: &Physical, mods: Modifiers) -> Result<Option<String>, String> {
    let code = match physical {
        Physical::Code(c) => match c {
            Code::Digit1 => Some(10),
            Code::Digit2 => Some(11),
            Code::Digit3 => Some(12),
            Code::Digit4 => Some(13),
            Code::Digit5 => Some(14),
            Code::Digit6 => Some(15),
            Code::Digit7 => Some(16),
            Code::Digit8 => Some(17),
            Code::Digit9 => Some(18),
            Code::Digit0 => Some(19),
            Code::Backquote => Some(49),
            _ => None,
        },
        _ => None,
    };
    let (name, bare) = if let Some(c) = code {
        (format!("code:{c}"), false)
    } else {
        match key {
            Key::Named(n) => match named(*n) {
                Some(v) => v,
                None => return Ok(None),
            },
            Key::Character(s) => {
                let c = s.chars().next().unwrap_or(' ');
                match keysym(c) {
                    Some(k) => (k, false),
                    None => {
                        return Err(crate::tr!(
                            "Touche « {s} » non reconnue : essayez une autre combinaison.",
                            s = s.as_str()
                        ));
                    }
                }
            }
            Key::Unidentified => {
                return Err(crate::i18n::tr(
                    "Touche non reconnue : essayez une autre combinaison.",
                )
                .to_string());
            }
        }
    };
    let mut parts: Vec<&str> = Vec::new();
    if mods.logo() {
        parts.push("SUPER");
    }
    if mods.alt() {
        parts.push("ALT");
    }
    if mods.control() {
        parts.push("CTRL");
    }
    if mods.shift() {
        parts.push("SHIFT");
    }
    // Sans SUPER, ALT ni CTRL, une touche d'écriture ne servirait plus à écrire.
    if !bare && !(mods.logo() || mods.alt() || mods.control()) {
        return Err(crate::i18n::tr(
            "Ajoutez SUPER, ALT ou CTRL : sans eux, la touche ne servirait plus à écrire.",
        )
        .to_string());
    }
    parts.push(&name);
    Ok(Some(parts.join(" + ")))
}

/// Un état d'essentiel, regroupé dans la rangée des chiffres.
type StateTest = fn(&Ess) -> bool;

/// Change la submap de Hyprland, hors du fil de l'interface.
fn submap_task(name: Option<&'static str>) -> Task<UiMsg> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || crate::hypr::set_submap(name)).await;
        },
        |_| UiMsg::Noop,
    )
}

/// Plages de chiffres : « 1 à 7, 9 ».
fn ranges(details: &[&str]) -> String {
    let nums: Vec<u32> = details.iter().filter_map(|d| d.parse().ok()).collect();
    if nums.len() != details.len() || nums.is_empty() {
        return details.join(", ");
    }
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < nums.len() {
        let mut j = i;
        while j + 1 < nums.len() && nums[j + 1] == nums[j] + 1 {
            j += 1;
        }
        out.push(if j > i + 1 {
            crate::tr!("{a} à {b}", a = nums[i], b = nums[j])
        } else if j == i + 1 {
            format!("{}, {}", nums[i], nums[j])
        } else {
            nums[i].to_string()
        });
        i = j + 1;
    }
    out.join(", ")
}

impl App {
    /// Relevé des raccourcis, hors du fil de l'interface (arrivée sur la page).
    pub(in crate::ui) fn keys_load(&self) -> Task<UiMsg> {
        let keys = self.cfg.general.keys.clone();
        let unified = self.cfg.general.apps_launcher.unified;
        let config_path = self.config_path.clone();
        let layouts = crate::tiling::in_use(&super::layouts::effective(&self.cfg));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let source = crate::keymap::load_source(&config_path);
                    crate::hypr::key_report(&keys, unified, &source, &layouts)
                        .map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| wrap(Msg::Report(r)),
        )
    }

    pub(in crate::ui) fn keys_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "hyprdesk") {
            return self.hyprdesk_keys_section();
        }
        if self.on_subtab(INFO.id, "essentiels") {
            return self.essentials_section();
        }
        self.keys_overview()
    }

    /// Bouton « Capturer » : la prochaine combinaison pressée.
    fn capture_button<'a>(&self, t: Target) -> Element<'a, UiMsg> {
        button(text(crate::i18n::tr("Capturer")).font(self.font).size(12))
            .padding([4, 10])
            .style(button::secondary)
            .on_press(wrap(Msg::Capture(t)))
            .into()
    }

    /// Bouton « Par défaut », avec la valeur rendue.
    fn reset_button<'a>(&self, t: Target, default: String) -> Element<'a, UiMsg> {
        let label = if default.trim().is_empty() {
            crate::i18n::tr("Par défaut (aucun)").to_string()
        } else {
            crate::tr!("Par défaut ({k})", k = pretty(&default))
        };
        button(text(label).font(self.font).size(12))
            .padding([4, 10])
            .style(button::text)
            .on_press(wrap(Msg::Reset(t)))
            .into()
    }

    /// Capture en cours : la consigne, l'erreur éventuelle, « Annuler ».
    fn capture_prompt<'a>(&self) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let accent = color(g.theme.accent, 1.0);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let mut col = column![
            text(crate::i18n::tr(
                "Appuyez sur la combinaison voulue… (Échap : annuler)"
            ))
            .font(self.font)
            .size(13)
            .color(accent),
        ]
        .spacing(4);
        if let Some(e) = self.panel.as_ref().and_then(|p| p.keys.key_error.clone()) {
            col = col.push(text(e).font(self.font).size(12).color(warn));
        }
        row![
            container(col)
                .padding([6, 10])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    border: Border::default().rounded(6).width(1).color(accent),
                    ..Default::default()
                }),
            button(text(crate::i18n::tr("Annuler")).font(self.font).size(12))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(wrap(Msg::CaptureCancel)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    /// Fin de capture : Hyprland retrouve sa submap (celle du bureau s'il
    /// est affiché).
    pub(in crate::ui) fn keys_capture_end(&mut self) -> Task<UiMsg> {
        if let Some(p) = self.panel.as_mut() {
            p.keys.capture = None;
        }
        let back = self.in_submap.then_some(crate::hypr::SUBMAP);
        submap_task(back)
    }

    /// Combinaison capturée : enregistrée pour sa cible.
    fn keys_apply(&mut self, t: Target, combo: String) -> Task<UiMsg> {
        match t {
            Target::Hyprdesk(i) => {
                if let Some(p) = self.panel.as_mut()
                    && let Some(slot) = p.keys.keys.get_mut(i)
                {
                    *slot = combo;
                }
                self.keys_update(Msg::KeySubmit(i))
            }
            Target::Essential(slot) => {
                let default = self
                    .key_report
                    .essentials
                    .iter()
                    .find(|l| l.slot == slot)
                    .map(|l| l.default.clone());
                let mut map = self.cfg.general.keys.essential_keys.clone();
                if default.as_deref() == Some(combo.as_str()) {
                    map.remove(&slot);
                } else {
                    map.insert(slot, combo);
                }
                self.settings.key_essential_keys = Some(map);
                self.commit()
            }
        }
    }

    /// Pastille d'une combinaison.
    fn key_chip<'a>(&self, s: String, c: Color) -> Element<'a, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        container(text(s).font(self.font).size(12).color(c))
            .padding([2, 8])
            .style(move |_| container::Style {
                background: Some(Color { a: 0.08, ..fg }.into()),
                border: Border::default()
                    .rounded(6)
                    .width(1)
                    .color(Color { a: 0.25, ..c }),
                ..Default::default()
            })
            .into()
    }

    /// Vue d'ensemble : les raccourcis actifs dans un mode, par rubrique.
    fn keys_overview(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return Space::new().into();
        };
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let desktop = p.keys.desktop_view;
        let mode = |label: &'static str, on: bool, v: bool| {
            button(text(crate::i18n::tr(label)).font(self.font).size(13))
                .padding([6, 14])
                .style(if on {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(wrap(Msg::View(v)))
        };
        let mut col = column![
            self.section("Vue d'ensemble"),
            text("Les raccourcis actifs en ce moment, lus dans Hyprland : ceux de hyprland.lua (avec leur ligne) et ceux que pose hyprdesk. Bureau affiché, ceux de hyprland.lua sont suspendus, sauf ceux marqués { submap_universal = true } ; hyprdesk y garde les siens, les workspaces et les touches multimédia.")
                .font(self.font)
                .size(12)
                .color(dim),
            row![
                mode("Hors plein bureau (no-desktop, launcher-only)", !desktop, false),
                mode("Bureau affiché (full-desktop)", desktop, true),
            ]
            .spacing(8),
        ]
        .spacing(10);
        let list = &self.key_report.hyprland;
        if list.is_empty() {
            col = col.push(
                text("Aucun raccourci lu (Hyprland injoignable, ou relevé en cours).")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for cat in Category::ALL {
            let lines: Vec<_> = list
                .iter()
                .filter(|l| l.category == cat && if desktop { l.desktop } else { l.outside })
                .collect();
            if lines.is_empty() {
                continue;
            }
            col = col.push(
                text(crate::tr!(
                    "{c} ({n})",
                    c = crate::i18n::tr(cat.label()),
                    n = lines.len()
                ))
                .font(self.font)
                .size(14)
                .color(fg),
            );
            for l in lines {
                let origin = match l.origin {
                    Origin::HyprlandLua(Some(n)) => crate::tr!("hyprland.lua, ligne {n}", n = n),
                    Origin::HyprlandLua(None) => crate::i18n::tr("hyprland.lua").to_string(),
                    Origin::Hyprdesk => crate::i18n::tr("hyprdesk").to_string(),
                };
                let c = if l.origin == Origin::Hyprdesk {
                    accent
                } else {
                    fg
                };
                col = col.push(
                    row![
                        container(self.key_chip(l.combo.clone(), c)).width(Length::FillPortion(4)),
                        text(crate::i18n::tr(&l.what).to_string())
                            .font(self.font)
                            .size(12)
                            .width(Length::FillPortion(6)),
                        text(origin)
                            .font(self.font)
                            .size(11)
                            .color(dim)
                            .width(Length::FillPortion(3)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
        }
        // Essentiels non posés : la raison est dans l'onglet voisin.
        let held = self
            .key_report
            .essentials
            .iter()
            .filter(|l| matches!(l.state, Ess::Taken { .. }))
            .count();
        if held > 0 {
            col = col.push(
                text(crate::tr!(
                    "{n} raccourci(s) essentiel(s) non posé(s), combinaison prise : onglet Fenêtres et workspaces.",
                    n = held
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        col.into()
    }

    /// Essentiels de fenêtres, de workspaces et d'écrans.
    fn essentials_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let mut col = column![
            self.section("Fenêtres, workspaces, écrans"),
            text("hyprdesk pose ces raccourcis là où hyprland.lua n'en a pas, et seulement sur une combinaison libre : hyprland.lua n'est pas modifié. Les combinaisons suivent celles de hyprland.lua (modificateur des workspaces, touches du focus). Une combinaison prise par hyprland.lua se pose une fois prise par hyprdesk.")
                .font(self.font)
                .size(12)
                .color(dim),
            self.field(
                "Poser les raccourcis essentiels",
                "keys.essentials",
                toggler(g.keys.essentials).on_toggle(|v| wrap(Msg::Essentials(v))),
            ),
        ]
        .spacing(10);
        let report = &self.key_report.essentials;
        let p_capture = self.panel.as_ref().and_then(|p| p.keys.capture.clone());
        if report.is_empty() {
            col = col.push(
                text("Relevé en cours (ou Hyprland injoignable).")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for group in Group::ALL {
            col = col.push(
                text(crate::i18n::tr(group.label()))
                    .font(self.font)
                    .size(14)
                    .color(fg),
            );
            for e in CATALOGUE.iter().filter(|e| e.group == group) {
                let lines: Vec<&Line> = report.iter().filter(|l| l.id == e.id).collect();
                let on = g.keys.essentials && !g.keys.essentials_off.iter().any(|o| o == e.id);
                let id = e.id;
                let mut head = row![
                    text(crate::i18n::tr(e.label))
                        .font(self.font)
                        .size(13)
                        .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center);
                if e.desktop {
                    head = head.push(
                        text(crate::i18n::tr("bureau affiché aussi"))
                            .font(self.font)
                            .size(11)
                            .color(dim),
                    );
                }
                head = head.push(
                    toggler(on).on_toggle_maybe(
                        g.keys
                            .essentials
                            .then_some(move |v| wrap(Msg::Essential(id, v))),
                    ),
                );
                col = col.push(head);
                let numbered = lines.len() > 4;
                if numbered {
                    // Rangée des chiffres : regroupé par état.
                    let mut parts: Vec<String> = Vec::new();
                    let kinds: [(&str, StateTest); 4] = [
                        ("dans hyprland.lua", |s| matches!(s, Ess::Present { .. })),
                        ("posés par hyprdesk", |s| s.installs()),
                        ("combinaison prise", |s| matches!(s, Ess::Taken { .. })),
                        ("coupés", |s| matches!(s, Ess::Off)),
                    ];
                    for (label, f) in kinds {
                        let d: Vec<&str> = lines
                            .iter()
                            .filter(|l| f(&l.state))
                            .map(|l| l.detail.as_str())
                            .collect();
                        if !d.is_empty() {
                            parts.push(format!("{} : {}", crate::i18n::tr(label), ranges(&d)));
                        }
                    }
                    let combo = lines
                        .iter()
                        .find(|l| l.state.installs())
                        .map(|l| pretty(&l.combo))
                        .unwrap_or_default();
                    let mut r = row![
                        text(parts.join(" · "))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                            .width(Length::Fill),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);
                    if !combo.is_empty() {
                        r = r.push(self.key_chip(crate::tr!("{c} …", c = combo), accent));
                    }
                    col = col.push(r);
                    continue;
                }
                for l in lines {
                    let (shown, c) = match &l.state {
                        Ess::Present { combo, .. } => (pretty(combo), fg),
                        Ess::Installed | Ess::TakenOver => (pretty(&l.combo), accent),
                        Ess::Taken { .. } => (pretty(&l.combo), warn),
                        Ess::Off => (pretty(&l.combo), dim),
                    };
                    let target = Target::Essential(l.slot.clone());
                    if !l.slot.is_empty() && p_capture.as_ref() == Some(&target) {
                        col = col.push(
                            row![
                                text(if l.detail.is_empty() {
                                    String::new()
                                } else {
                                    crate::i18n::tr(&l.detail).to_string()
                                })
                                .font(self.font)
                                .size(12)
                                .color(dim),
                                self.capture_prompt(),
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        );
                        continue;
                    }
                    let state = l.state.describe();
                    let what = if l.detail.is_empty() {
                        state
                    } else {
                        format!("{} — {state}", crate::i18n::tr(&l.detail))
                    };
                    let mut r = row![
                        container(self.key_chip(shown, c)).width(Length::FillPortion(4)),
                        text(what)
                            .font(self.font)
                            .size(12)
                            .color(if matches!(&l.state, Ess::Taken { .. }) {
                                warn
                            } else {
                                dim
                            })
                            .width(Length::FillPortion(8)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center);
                    match &l.state {
                        Ess::Taken { by, .. } if !by.starts_with("hyprdesk") => {
                            r = r.push(
                                button(
                                    text(crate::i18n::tr("hyprdesk le prend"))
                                        .font(self.font)
                                        .size(12),
                                )
                                .padding([4, 10])
                                .style(button::secondary)
                                .on_press(wrap(Msg::TakeKey(l.combo.clone()))),
                            );
                        }
                        Ess::TakenOver => {
                            r = r.push(
                                button(text(crate::i18n::tr("Rendre")).font(self.font).size(12))
                                    .padding([4, 10])
                                    .style(button::secondary)
                                    .on_press(wrap(Msg::ReleaseKey(l.combo.clone()))),
                            );
                        }
                        _ => {}
                    }
                    // Réglable : capturer une autre combinaison, revenir à la déduite.
                    if !l.slot.is_empty() && !matches!(l.state, Ess::Present { .. }) {
                        r = r.push(self.capture_button(target.clone()));
                        if l.combo != l.default {
                            r = r.push(self.reset_button(target, l.default.clone()));
                        }
                    }
                    col = col.push(r);
                    if let Some(n) = &l.note {
                        col = col.push(text(n.clone()).font(self.font).size(11).color(warn));
                    }
                }
            }
        }
        col.into()
    }

    pub(in crate::ui) fn hyprdesk_keys_section(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return Space::new().into();
        };
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let key_field = |i: usize, label: &'static str, cfg_key: &'static str| {
            let target = Target::Hyprdesk(i);
            let control: Element<'_, UiMsg> = if p.keys.capture.as_ref() == Some(&target) {
                self.capture_prompt()
            } else {
                let mut r = row![
                    text_input(crate::i18n::tr("aucun"), &p.keys.keys[i])
                        .font(self.font)
                        .on_input(move |v| wrap(Msg::KeyInput(i, v)))
                        .on_submit(wrap(Msg::KeySubmit(i)))
                        .padding(8)
                        .width(260),
                    self.capture_button(target.clone()),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                if p.keys.keys[i].trim() != default_key(i) {
                    r = r.push(self.reset_button(target, default_key(i)));
                }
                r.into()
            };
            self.field(label, cfg_key, control)
        };
        let mut col = column![
            self.section("Raccourcis"),
            self.key_conflicts_view(),
            self.adv(self.field(
                "Bureau affiché : suspendre les raccourcis de Hyprland",
                "suspend_binds",
                toggler(g.suspend_binds).on_toggle(|v| wrap(Msg::SuspendBinds(v))),
            )),
            self.adv(text("Les raccourcis de Hyprland n'agissent plus sur les fenêtres cachées derrière le bureau. Seuls restent actifs ceux de hyprdesk et ceux marqués { submap_universal = true } dans hyprland.lua.").font(self.font).size(12).color(dim)),
            key_field(0, "Changer de mode (no-desktop → launcher-only → full-desktop)", "keys.toggle"),
            key_field(3, "launcher-only (lanceur seul)", "keys.launcher"),
            key_field(4, "no-desktop (masquer)", "keys.hide"),
            key_field(1, crate::i18n::tr("Ouvrir les réglages"), "keys.settings"),
            key_field(6, crate::i18n::tr("Lanceur d'applications (tous les modes)"), "keys.search"),
            key_field(9, crate::i18n::tr("Lanceur d'applications en plein bureau (vide : le même)"), "keys.search_desktop"),
            key_field(7, crate::i18n::tr("Fenêtres (Alt+Tab ; avec Maj : en arrière)"), "keys.switch"),
            key_field(8, crate::i18n::tr("Vue d'ensemble des fenêtres"), "keys.overview"),
            self.adv(self.field(
                "Vue d'ensemble par un plugin de Hyprland",
                "overview_command",
                text_input(
                    crate::i18n::tr("vide : le sélecteur de hyprdesk"),
                    p.keys
                        .overview_cmd
                        .as_deref()
                        .unwrap_or(&g.overview_command),
                )
                .font(self.font)
                .on_input(|v| wrap(Msg::OverviewCmd(v)))
                .on_submit(wrap(Msg::OverviewCmdSubmit))
                .padding(8)
                .width(260),
            )),
            key_field(5, crate::i18n::tr("Verrouiller la session"), "keys.lock"),
            text(
                "Vide : celui de hyprland.lua (SUPER + L, posé à l'installation). \
                 Réglé ici, hyprdesk le remplace."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.adv(key_field(2, crate::i18n::tr("Sortie de secours (bureau affiché)"), "keys.leave")),
            text("Valider avec Entrée. Vide = aucun raccourci.").font(self.font).size(12).color(dim),
        ]
        .spacing(10);
        if let Some(e) = &p.keys.key_error {
            col = col.push(text(e.clone()).font(self.font).size(12).color(warn));
        }
        col = col.push(
            text("Tous les raccourcis actifs, hors plein bureau et bureau affiché : onglet Vue d'ensemble.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        col.into()
    }

    /// Conflits de raccourcis (`crate::keymap`) : chacun avec ses issues —
    /// hyprdesk le prend, LYRA ; puis ceux que hyprdesk a pris à hyprland.lua.
    pub(in crate::ui) fn key_conflicts_view(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let r = &self.key_report;
        let mut col = column![].spacing(10);
        if !r.issues.is_empty() {
            col = col.push(
                row![
                    text(crate::tr!(
                        "Conflits de raccourcis : {n}",
                        n = r.issues.len()
                    ))
                    .font(self.font)
                    .size(14)
                    .color(warn)
                    .width(Length::Fill),
                    self.lyra_button(crate::i18n::tr("Raccourcis"), crate::keymap::lyra_prompt(r)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        for i in &r.issues {
            let mut line = row![
                text(i.message())
                    .font(self.font)
                    .size(12)
                    .color(warn)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if i.holder().is_some() {
                line = line.push(
                    button(
                        text(crate::i18n::tr("hyprdesk le prend"))
                            .font(self.font)
                            .size(12),
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(wrap(Msg::TakeKey(i.action.combo.clone()))),
                );
            }
            col = col.push(line);
        }
        if !r.taken.is_empty() {
            col = col.push(
                text(crate::i18n::tr("Pris à hyprland.lua par hyprdesk"))
                    .font(self.font)
                    .size(14)
                    .color(fg),
            );
        }
        for h in &r.taken {
            let mut line = row![
                text(crate::tr!(
                    "{k} : « {label} », à la place de {h}",
                    k = h.action.combo,
                    label = crate::i18n::tr(h.action.label),
                    h = h.holder.describe()
                ))
                .font(self.font)
                .size(12)
                .color(dim)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            // Rendu ici seulement s'il a été pris ici (keys.take) ; le
            // lanceur unifié se règle dans Lanceur, le verrouillage plus haut.
            if g.keys
                .take
                .iter()
                .any(|t| crate::keymap::norm(t) == crate::keymap::norm(&h.action.combo))
            {
                line = line.push(
                    button(text(crate::i18n::tr("Rendre")).font(self.font).size(12))
                        .padding([4, 10])
                        .style(button::secondary)
                        .on_press(wrap(Msg::ReleaseKey(h.action.combo.clone()))),
                );
            }
            col = col.push(line);
        }
        for h in &r.left {
            col = col.push(
                text(crate::tr!(
                    "{k} : {h} hors plein bureau, « {label} » bureau affiché (lanceur non unifié).",
                    k = h.action.combo,
                    h = h.holder.describe(),
                    label = crate::i18n::tr(h.action.label)
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn keys_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::View(v) => {
                panel.keys.desktop_view = v;
                Task::none()
            }
            Msg::Report(r) => {
                match r {
                    Ok(r) => self.key_report = r,
                    Err(e) => tracing::warn!("{}", crate::tr!("raccourcis : {e}", e = e)),
                }
                Task::none()
            }
            Msg::Capture(t) => {
                panel.keys.capture = Some(t);
                panel.keys.capture_seq += 1;
                panel.keys.key_error = None;
                let seq = panel.keys.capture_seq;
                Task::batch([
                    submap_task(Some(crate::hypr::CAPTURE_SUBMAP)),
                    Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                            seq
                        },
                        |s| wrap(Msg::CaptureTimeout(s)),
                    ),
                ])
            }
            Msg::CaptureCancel => {
                panel.keys.key_error = None;
                self.keys_capture_end()
            }
            Msg::CaptureTimeout(seq) => {
                if panel.keys.capture.is_some() && panel.keys.capture_seq == seq {
                    self.keys_capture_end()
                } else {
                    Task::none()
                }
            }
            Msg::CaptureKey(key, physical, mods) => {
                let Some(target) = panel.keys.capture.clone() else {
                    return Task::none();
                };
                let plain = !(mods.logo() || mods.alt() || mods.control() || mods.shift());
                if key == Key::Named(Named::Escape) && plain {
                    panel.keys.key_error = None;
                    return self.keys_capture_end();
                }
                match combo_of(&key, &physical, mods) {
                    Ok(None) => Task::none(),
                    Err(e) => {
                        panel.keys.key_error = Some(e);
                        Task::none()
                    }
                    Ok(Some(c)) => {
                        panel.keys.key_error = None;
                        let end = self.keys_capture_end();
                        Task::batch([end, self.keys_apply(target, c)])
                    }
                }
            }
            Msg::Reset(t) => match t {
                Target::Hyprdesk(i) => {
                    if let Some(slot) = panel.keys.keys.get_mut(i) {
                        *slot = default_key(i);
                    }
                    panel.keys.key_error = None;
                    let s = &mut self.settings;
                    match i {
                        0 => s.key_toggle = None,
                        1 => s.key_settings = None,
                        2 => s.key_leave = None,
                        3 => s.key_launcher = None,
                        4 => s.key_hide = None,
                        5 => s.key_lock = None,
                        6 => s.key_search = None,
                        7 => s.key_switch = None,
                        8 => s.key_overview = None,
                        _ => s.key_search_desktop = None,
                    }
                    self.commit()
                }
                Target::Essential(slot) => {
                    let mut map = self.cfg.general.keys.essential_keys.clone();
                    map.remove(&slot);
                    self.settings.key_essential_keys = Some(map);
                    self.commit()
                }
            },
            Msg::Essentials(v) => {
                self.settings.key_essentials = Some(v);
                self.commit()
            }
            Msg::Essential(id, on) => {
                let mut off = self.cfg.general.keys.essentials_off.clone();
                off.retain(|o| o != id);
                if !on {
                    off.push(id.to_string());
                }
                self.settings.key_essentials_off = Some(off);
                self.commit()
            }
            Msg::SuspendBinds(v) => {
                self.settings.suspend_binds = Some(v);
                self.commit()
            }
            Msg::KeyInput(i, v) => {
                if let Some(slot) = panel.keys.keys.get_mut(i) {
                    *slot = v;
                }
                panel.keys.key_error = None;
                Task::none()
            }
            Msg::KeySubmit(i) => {
                let v = panel
                    .keys
                    .keys
                    .get(i)
                    .cloned()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !v.is_empty() && crate::hypr::parse_combo(&v).is_none() {
                    panel.keys.key_error = Some(crate::tr!(
                        "« {v} » : raccourci illisible (ex. SUPER + SHIFT + D)",
                        v = v
                    ));
                    return Task::none();
                }
                // Déjà la combinaison d'une autre action de hyprdesk : refusée.
                let mut keys = self.cfg.general.keys.clone();
                let (edited, target) = match i {
                    0 => ("keys.toggle", &mut keys.toggle),
                    1 => ("keys.settings", &mut keys.settings),
                    3 => ("keys.launcher", &mut keys.launcher),
                    4 => ("keys.hide", &mut keys.hide),
                    5 => ("keys.lock", &mut keys.lock),
                    6 => ("keys.search", &mut keys.search),
                    7 => ("keys.switch", &mut keys.switch),
                    8 => ("keys.overview", &mut keys.overview),
                    9 => ("keys.search_desktop", &mut keys.search_desktop),
                    _ => ("keys.leave", &mut keys.leave),
                };
                *target = v.clone();
                if let Some(other) = crate::keymap::duplicate_of(
                    &keys,
                    self.cfg.general.apps_launcher.unified,
                    edited,
                ) {
                    panel.keys.key_error = Some(crate::tr!(
                        "« {v} » sert déjà à « {other} » : choisissez une autre combinaison.",
                        v = v,
                        other = crate::i18n::tr(other)
                    ));
                    return Task::none();
                }
                match i {
                    0 => self.settings.key_toggle = Some(v),
                    1 => self.settings.key_settings = Some(v),
                    3 => self.settings.key_launcher = Some(v),
                    4 => self.settings.key_hide = Some(v),
                    5 => self.settings.key_lock = Some(v),
                    6 => self.settings.key_search = Some(v),
                    7 => self.settings.key_switch = Some(v),
                    8 => self.settings.key_overview = Some(v),
                    9 => self.settings.key_search_desktop = Some(v),
                    _ => self.settings.key_leave = Some(v),
                }
                self.commit()
            }
            Msg::TakeKey(k) => {
                let mut list = self.cfg.general.keys.take.clone();
                if !list.contains(&k) {
                    list.push(k);
                }
                self.settings.key_take = Some(list);
                self.commit()
            }
            Msg::ReleaseKey(k) => {
                let mut list = self.cfg.general.keys.take.clone();
                list.retain(|t| {
                    !crate::keymap::norm(t).is_some_and(|n| Some(n) == crate::keymap::norm(&k))
                });
                self.settings.key_take = Some(list);
                self.commit()
            }
            Msg::OverviewCmd(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.keys.overview_cmd = Some(v);
                }
                Task::none()
            }
            Msg::OverviewCmdSubmit => {
                let Some(v) = self.panel.as_mut().and_then(|p| p.keys.overview_cmd.take()) else {
                    return Task::none();
                };
                self.settings.overview_command = Some(v.trim().to_string());
                self.commit()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn capture() {
        let sup = Modifiers::LOGO;
        let none = Physical::Unidentified(iced::keyboard::key::NativeCode::Unidentified);
        // Lettre : en minuscules, avec SUPER.
        assert_eq!(
            combo_of(&ch("d"), &none, sup | Modifiers::SHIFT),
            Ok(Some("SUPER + SHIFT + d".into()))
        );
        // Rangée des chiffres (AZERTY : « & ») : par code physique.
        assert_eq!(
            combo_of(
                &ch("&"),
                &Physical::Code(Code::Digit1),
                sup | Modifiers::ALT
            ),
            Ok(Some("SUPER + ALT + code:10".into()))
        );
        // Ponctuation : son keysym.
        assert_eq!(
            combo_of(&ch(","), &none, sup),
            Ok(Some("SUPER + comma".into()))
        );
        assert_eq!(
            combo_of(&ch("é"), &none, sup),
            Ok(Some("SUPER + eacute".into()))
        );
        // Flèches, F5 seul, Impr. écran seul.
        assert_eq!(
            combo_of(&Key::Named(Named::ArrowLeft), &none, sup | Modifiers::CTRL),
            Ok(Some("SUPER + CTRL + left".into()))
        );
        assert_eq!(
            combo_of(&Key::Named(Named::F5), &none, Modifiers::empty()),
            Ok(Some("F5".into()))
        );
        assert_eq!(
            combo_of(&Key::Named(Named::PrintScreen), &none, Modifiers::empty()),
            Ok(Some("Print".into()))
        );
        // Modificateur seul : on attend.
        assert_eq!(combo_of(&Key::Named(Named::Super), &none, sup), Ok(None));
        // Lettre sans SUPER, ALT ni CTRL : refusée.
        assert!(combo_of(&ch("a"), &none, Modifiers::SHIFT).is_err());
        // Ce qui sort se relit.
        for c in ["SUPER + SHIFT + d", "SUPER + ALT + code:10", "F5", "Print"] {
            assert!(crate::hypr::parse_combo(c).is_some(), "{c}");
        }
    }

    #[test]
    fn defauts() {
        assert_eq!(default_key(0), "SUPER + D");
        assert_eq!(default_key(3), "SUPER + ALT + D");
        assert_eq!(default_key(9), "");
    }

    #[test]
    fn plages() {
        assert_eq!(ranges(&["8", "9", "10"]), "8 à 10");
        assert_eq!(ranges(&["1", "2", "4"]), "1, 2, 4");
        assert_eq!(ranges(&["gauche"]), "gauche");
    }
}
