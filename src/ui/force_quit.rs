//! Forcer une application à quitter (HIG ENT-10, `hyprdesk force-quit`) : une
//! carte à clavier exclusif sur le calque overlay, comme celle du commutateur.
//! Une ligne par application — les fenêtres de `self.clients` groupées par
//! classe —, l'application active choisie d'abord. « Forcer à quitter » se
//! confirme en deux temps et nomme le nombre de fenêtres : sous Hyprland, une
//! classe rassemble souvent plusieurs processus, tous tués.

use crate::hypr::Client;
use crate::ui::tokens::Shown as _;
use iced::widget::{button, column, container, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, IconImg, Msg, color};

pub const NAMESPACE: &str = "hyprdesk-force-quit";
/// Lignes affichées au plus : au-delà, la liste défile par pages.
const SHOWN: usize = 10;
const WIDTH: f32 = 480.0;
const ROW: f32 = 44.0;
/// Marge de la surface autour de la carte, pour son ombre.
const SHADOW: f32 = 16.0;

/// Une application de la boîte : sa classe et le nombre de ses fenêtres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRow {
    pub class: String,
    pub windows: usize,
}

pub struct ForceQuit {
    pub win: Id,
    /// Applications, dans l'ordre de la liste à l'ouverture ; comptes tenus à
    /// jour à chaque lecture des fenêtres — une application disparaît avec
    /// sa dernière.
    pub apps: Vec<AppRow>,
    /// Ligne choisie ; aucune : « Forcer à quitter » est sans effet.
    pub selected: Option<usize>,
    /// « Forcer à quitter » armé : la commande nomme l'application, l'action
    /// suivante tue.
    pub confirm: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FqMsg {
    /// Ligne choisie à la souris.
    Pick(usize),
    /// Flèche bas (`true`) ou haut.
    Move(bool),
    /// « Forcer à quitter » : arme la confirmation, puis tue.
    Force,
    /// « Annuler » ou Échap : désarme, puis referme la boîte.
    Cancel,
}

/// Les fenêtres que le démon ouvre lui-même (l'aperçu du Panorama) : les tuer
/// tuerait hyprdesk. Elles ne sont jamais proposées.
fn own(class: &str) -> bool {
    class == crate::ui::plugins::panorama::surface::PREVIEW_APP_ID
}

/// Les fenêtres groupées par classe, dans leur ordre d'apparition ; la classe
/// `active` en tête, si elle en a. Les fenêtres du démon n'y sont pas.
pub fn group(clients: &[Client], active: Option<&str>) -> Vec<AppRow> {
    let mut apps: Vec<AppRow> = Vec::new();
    for c in clients.iter().filter(|c| !own(&c.class)) {
        match apps.iter_mut().find(|a| a.class == c.class) {
            Some(a) => a.windows += 1,
            None => apps.push(AppRow {
                class: c.class.clone(),
                windows: 1,
            }),
        }
    }
    if let Some(i) = active.and_then(|c| apps.iter().position(|a| a.class == c))
        && i > 0
    {
        // En tête, les autres gardant leur ordre.
        let a = apps.remove(i);
        apps.insert(0, a);
    }
    apps
}

/// Les adresses à tuer pour l'application `class` : les siennes seulement.
pub fn targets(clients: &[Client], class: &str) -> Vec<String> {
    clients
        .iter()
        .filter(|c| c.class == class && !own(&c.class))
        .map(|c| c.address.clone())
        .collect()
}

/// Nom d'une application d'après sa classe : celui de son entrée `.desktop`
/// (`StartupWMClass`, ou l'identifiant), à défaut la classe.
pub fn app_name(class: &str, apps: &[crate::launch::AppInfo]) -> String {
    let lc = class.to_lowercase();
    apps.iter()
        .find(|a| {
            a.wm_class
                .as_deref()
                .is_some_and(|w| w.eq_ignore_ascii_case(class))
                || a.id.to_lowercase() == lc
        })
        .map_or_else(|| class.to_string(), |a| a.name.clone())
}

/// Le nom dans la confirmation, borné : la commande tient sur sa ligne.
fn short(name: &str) -> String {
    const MAX: usize = 24;
    if name.chars().count() > MAX {
        let mut s: String = name.chars().take(MAX - 1).collect();
        s.push('…');
        s
    } else {
        name.to_string()
    }
}

fn windows_label(n: usize) -> String {
    if n == 1 {
        crate::tr!("1 fenêtre").to_string()
    } else {
        crate::tr!("{n} fenêtres", n = n)
    }
}

/// Hauteur de la surface : la carte la plus haute (titre, `SHOWN` lignes,
/// commandes) et sa marge d'ombre. La carte réelle, plus courte, y est
/// centrée.
fn surface_height() -> f32 {
    SHOWN as f32 * (ROW + 4.0) + 28.0 + 36.0 + 12.0 + 2.0 * 16.0 + 2.0 * SHADOW
}

impl App {
    /// `hyprdesk force-quit` (raccourci `keys.force_quit`) : ouvre la boîte, ou
    /// la referme. Le commutateur se referme : deux cartes à clavier exclusif
    /// ne se superposent pas. Le mode du bureau ne change pas.
    pub(super) fn force_quit_toggle(&mut self) -> Task<Msg> {
        if self.force_quit.is_some() {
            return self.close_force_quit();
        }
        // L'application active, lue avant que la carte ne prenne le clavier.
        let active = self.active_window.as_ref().map(|(class, _)| class.as_str());
        let apps = group(&self.clients, active);
        let selected = (!apps.is_empty()).then_some(0);
        let win = Id::unique();
        self.force_quit = Some(ForceQuit {
            win,
            apps,
            selected,
            confirm: false,
        });
        Task::batch([
            self.close_switcher(),
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(((WIDTH + 2.0 * SHADOW) as u32, surface_height() as u32)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
        ])
    }

    pub(super) fn close_force_quit(&mut self) -> Task<Msg> {
        match self.force_quit.take() {
            Some(f) => iced::window::close(f.win),
            None => Task::none(),
        }
    }

    /// La boîte suit les fenêtres : comptes tenus à jour, une application sans
    /// fenêtre retirée, une application apparue ajoutée à la suite ; la ligne
    /// choisie reste sur la même application.
    pub(super) fn force_quit_refresh(&mut self, clients: &[Client]) {
        let Some(f) = self.force_quit.as_mut() else {
            return;
        };
        let kept = f
            .selected
            .and_then(|i| f.apps.get(i))
            .map(|a| a.class.clone());
        let mut apps: Vec<AppRow> = f
            .apps
            .iter()
            .filter_map(|a| {
                let windows = targets(clients, &a.class).len();
                (windows > 0).then(|| AppRow {
                    class: a.class.clone(),
                    windows,
                })
            })
            .collect();
        for a in group(clients, None) {
            if !apps.iter().any(|k| k.class == a.class) {
                apps.push(a);
            }
        }
        f.selected = kept
            .as_deref()
            .and_then(|c| apps.iter().position(|a| a.class == c));
        if f.selected.is_none() {
            // La confirmation nommait une application : elle ne tient plus.
            f.confirm = false;
        }
        f.apps = apps;
    }

    pub(super) fn force_quit_update(&mut self, m: FqMsg) -> Task<Msg> {
        match m {
            FqMsg::Pick(k) => {
                if let Some(f) = self.force_quit.as_mut()
                    && k < f.apps.len()
                    && f.selected != Some(k)
                {
                    // La confirmation nomme une application : on la désarme.
                    f.selected = Some(k);
                    f.confirm = false;
                }
                Task::none()
            }
            FqMsg::Move(down) => {
                if let Some(f) = self.force_quit.as_mut()
                    && !f.apps.is_empty()
                {
                    let n = f.apps.len();
                    let to = match f.selected {
                        None if down => 0,
                        None => n - 1,
                        Some(i) if down => (i + 1) % n,
                        Some(i) => (i + n - 1) % n,
                    };
                    if f.selected != Some(to) {
                        f.selected = Some(to);
                        f.confirm = false;
                    }
                }
                Task::none()
            }
            FqMsg::Force => {
                // Deux temps (HIG ENT-10) : le premier arme et nomme
                // l'application, le second tue. Sans ligne choisie, sans effet.
                let Some(f) = self.force_quit.as_mut() else {
                    return Task::none();
                };
                let Some(i) = f.selected.filter(|i| *i < f.apps.len()) else {
                    return Task::none();
                };
                if !f.confirm {
                    f.confirm = true;
                    return Task::none();
                }
                match f.apps.get(i).map(|a| a.class.clone()) {
                    Some(class) => self.force_quit_kill(&class),
                    None => Task::none(),
                }
            }
            FqMsg::Cancel => {
                // La confirmation d'abord, la boîte ensuite.
                if let Some(f) = self.force_quit.as_mut()
                    && f.confirm
                {
                    f.confirm = false;
                    Task::none()
                } else {
                    self.close_force_quit()
                }
            }
        }
    }

    /// Tue chaque fenêtre de l'application `class`, puis referme la boîte.
    pub(super) fn force_quit_kill(&mut self, class: &str) -> Task<Msg> {
        let addresses = targets(&self.clients, class);
        let close = self.close_force_quit();
        if addresses.is_empty() {
            return close;
        }
        let kill = Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || crate::hypr::kill_windows(&addresses))
                    .await;
            },
            |_| Msg::Noop,
        );
        Task::batch([close, kill])
    }

    pub(in crate::ui) fn view_force_quit(&self) -> Element<'_, Msg> {
        let Some(f) = &self.force_quit else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let ts = crate::ui::tokens::ts;
        let mut list = column![
            crate::i18n::text("Forcer à quitter")
                .font(self.font)
                .size(ts(12.0))
                .color(dim)
                .height(28)
        ]
        .spacing(4);
        if f.apps.is_empty() {
            list = list.push(
                crate::i18n::text("Aucune application ouverte.")
                    .font(self.font)
                    .size(ts(13.0))
                    .color(dim),
            );
        }
        // Ligne choisie toujours visible : on fait défiler par pages.
        let first = f.selected.map_or(0, |i| (i / SHOWN) * SHOWN);
        for (k, a) in f.apps.iter().enumerate().skip(first).take(SHOWN) {
            let img = self
                .class_imgs
                .get(&a.class)
                .copied()
                .unwrap_or(IconImg::None);
            let icon: Element<'_, Msg> = self.img_element(img, 26.0, 1.0).unwrap_or_else(|| {
                // Sans image : la pastille de repli du dock.
                crate::ui::icons::fallback_icon(
                    &a.class,
                    26.0,
                    accent,
                    super::design::on_accent(accent, fg, color(th.bg, 1.0)),
                    self.title_font(),
                )
            });
            let selected = Some(k) == f.selected;
            list = list.push(
                button(
                    row![
                        icon,
                        text(app_name(&a.class, &self.apps))
                            .font(self.font)
                            .size(ts(14.0))
                            .color(fg)
                            .width(Length::Fill)
                            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                        text(windows_label(a.windows))
                            .font(self.font)
                            .size(ts(11.0))
                            .color(dim),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([4, 12])
                .width(Length::Fill)
                .style(move |_, st| button::Style {
                    background: (selected || matches!(st, button::Status::Hovered)).then(|| {
                        Color {
                            a: if selected { 0.24 } else { 0.1 },
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(super::design::SMALL_RADIUS),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::ForceQuit(FqMsg::Pick(k))),
            );
        }
        // Deux temps (HIG ENT-10) : armée, la commande nomme l'application et
        // le nombre de ses fenêtres ; sans ligne choisie, elle est sans effet.
        // Aucune application : la boîte le dit et n'a qu'« Annuler ».
        let chosen = f.selected.and_then(|i| f.apps.get(i));
        let cancel = {
            button(crate::i18n::text("Annuler").font(self.font).size(ts(13.0)))
                .padding([6, 14])
                .style(button::secondary)
                .on_press(Msg::ForceQuit(FqMsg::Cancel))
        };
        let commands: Element<'_, Msg> = if f.apps.is_empty() {
            cancel.into()
        } else {
            let force_label = match (f.confirm, chosen) {
                (true, Some(a)) => crate::tr!(
                    "Forcer « {nom} » à quitter ({n}) ?",
                    nom = short(&app_name(&a.class, &self.apps)),
                    n = windows_label(a.windows)
                ),
                _ => crate::tr("Forcer à quitter").to_string(),
            };
            row![
                cancel,
                button(text(force_label).font(self.font).size(ts(13.0)))
                    .padding([6, 14])
                    .style(button::danger)
                    .on_press_maybe(chosen.is_some().then(|| Msg::ForceQuit(FqMsg::Force))),
            ]
            .spacing(8)
            .into()
        };
        let body = column![
            list,
            container(commands)
                .width(Length::Fill)
                .align_x(Alignment::End)
        ]
        .spacing(12)
        .width(Length::Fill);
        let card = container(body)
            .padding(16)
            .width(WIDTH)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card)
            .padding(SHADOW)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(class: &str, address: &str) -> Client {
        Client {
            address: address.into(),
            class: class.into(),
            title: String::new(),
            workspace: "1".into(),
            monitor: "DP-1".into(),
        }
    }

    #[test]
    fn applications_groupees_active_en_tete() {
        // HIG ENT-10 : une ligne par application, l'active d'abord ; sans
        // fenêtre active (ou d'une classe absente), l'ordre des fenêtres.
        let clients = [
            client("foot", "0x1"),
            client("firefox", "0x2"),
            client("foot", "0x3"),
            client("org.gnome.Nautilus", "0x4"),
        ];
        assert_eq!(
            group(&clients, Some("firefox")),
            vec![
                AppRow {
                    class: "firefox".into(),
                    windows: 1
                },
                AppRow {
                    class: "foot".into(),
                    windows: 2
                },
                AppRow {
                    class: "org.gnome.Nautilus".into(),
                    windows: 1
                },
            ]
        );
        // L'active passe en tête, les autres gardent leur ordre.
        let order: Vec<String> = group(&clients, Some("org.gnome.Nautilus"))
            .into_iter()
            .map(|a| a.class)
            .collect();
        assert_eq!(order, ["org.gnome.Nautilus", "foot", "firefox"]);
        assert_eq!(group(&clients, None)[0].class, "foot");
        assert_eq!(group(&clients, Some("absente"))[0].class, "foot");
        assert!(group(&[], Some("foot")).is_empty());
    }

    #[test]
    fn jamais_les_fenetres_du_demon() {
        // L'aperçu du Panorama est une fenêtre du démon : la tuer tuerait
        // hyprdesk. Ni ligne, ni cible.
        let own = crate::ui::plugins::panorama::surface::PREVIEW_APP_ID;
        let clients = [client(own, "0x9"), client("foot", "0x1")];
        let apps = group(&clients, Some(own));
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].class, "foot");
        assert!(targets(&clients, own).is_empty());
    }

    #[test]
    fn nom_de_l_application() {
        // Le nom de son entrée `.desktop`, par `StartupWMClass` ou par
        // l'identifiant ; à défaut, la classe. Borné dans la confirmation.
        let app = |id: &str, name: &str, wm: Option<&str>| crate::launch::AppInfo {
            id: id.into(),
            name: name.into(),
            icon: None,
            wm_class: wm.map(Into::into),
            mime_types: Vec::new(),
            categories: Vec::new(),
            available: true,
        };
        let apps = [
            app("org.gnome.Nautilus", "Fichiers", None),
            app("firefox", "Firefox", Some("firefox-esr")),
        ];
        assert_eq!(app_name("org.gnome.Nautilus", &apps), "Fichiers");
        assert_eq!(app_name("Firefox-ESR", &apps), "Firefox");
        assert_eq!(app_name("foot", &apps), "foot");
        assert_eq!(short("foot"), "foot");
        let long = short("org.gnome.TextEditor.Nightly.Build");
        assert_eq!(long.chars().count(), 24);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn cibles_de_l_application_choisie() {
        // Tuer « firefox » ne touche pas « foot » ; sans fenêtre, rien à tuer.
        let clients = [
            client("foot", "0x1"),
            client("firefox", "0x2"),
            client("foot", "0x3"),
        ];
        assert_eq!(targets(&clients, "foot"), vec!["0x1", "0x3"]);
        assert_eq!(targets(&clients, "firefox"), vec!["0x2"]);
        assert!(targets(&clients, "absente").is_empty());
    }
}
