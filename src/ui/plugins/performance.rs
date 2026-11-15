//! Paquet intégré « Performance » : ce que le rendu de Hyprland coûte au
//! processeur et à la carte graphique, option par option, et trois profils
//! de priorité (Performance, Équilibré, Rendu graphique) qui les posent d'un
//! coup. Domaine : `crate::perf` ; les réglages sont ceux de `crate::fx`
//! (partagés avec le paquet Animations). Onglet « hyprdesk » : le moteur de
//! l'interface de hyprdesk lui-même (`crate::render`) — rendu matériel ou
//! logiciel, carte graphique, fils d'exécution.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, radio, row};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::perf::{self, Cost, Profile};
use crate::render::{self, Gpu, Renderer};
use crate::tiling::{Kind, OptValue};

pub const INFO: PluginInfo = PluginInfo {
    id: "performance",
    label: "Performance",
    icon: "power-profile-performance",
    summary: "Effets de Hyprland qui sollicitent le processeur et la carte graphique, \
              profils Performance, Équilibré, Rendu graphique, et moteur de hyprdesk \
              (rendu, carte graphique, fils).",
    builtin: true,
};

/// Onglets : effets de Hyprland, moteur de hyprdesk.
pub const TABS: &[super::SubTab] = &[
    ("hyprland", "Effets de Hyprland"),
    ("hyprdesk", "Moteur de hyprdesk"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Performance(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Pose un profil.
    Profile(Profile),
    /// Retire les réglages du paquet : tout revient à hyprland.lua.
    Clear,
    /// Ouvre Énergie (profil du processeur).
    Energy,
    /// Moteur de rendu de hyprdesk (pris en compte à la relance).
    Renderer(Renderer),
    /// Carte graphique préférée (à la relance).
    Gpu(Gpu),
    /// Fils d'exécution (0 : un par cœur ; à la relance).
    Workers(u16),
    /// Relance hyprdesk sur le moteur choisi.
    Restart,
}

/// Valeur lisible d'un profil pour une option (`None` : hyprland.lua).
fn shown(path: &str, v: Option<&OptValue>) -> String {
    match v {
        None => "hyprland.lua".into(),
        Some(OptValue::Bool(b)) => crate::i18n::tr(if *b { "oui" } else { "non" }).into(),
        Some(OptValue::Num(n)) => match crate::fx::opt(path).map(|o| &o.kind) {
            Some(Kind::IntChoice(list)) => list
                .iter()
                .find(|(x, _)| *x as f64 == *n)
                .map(|(_, l)| crate::i18n::tr(l).to_string())
                .unwrap_or_else(|| n.to_string()),
            _ => OptValue::Num(*n).display(),
        },
        Some(OptValue::Str(s)) => s.clone(),
        Some(other) => other.display(),
    }
}

/// Ce que coûte l'option, selon sa nature (interrupteur, curseur, choix).
fn cost_caption(path: &str, c: Cost) -> &'static str {
    match (crate::fx::opt(path).map(|o| &o.kind), c) {
        (Some(Kind::Num { .. }), _) => "Plus élevé, plus coûteux pour la carte graphique",
        (Some(Kind::IntChoice(_)), _) => "Hors « Jamais », économise en plein écran",
        (_, Cost::Gpu) => "Activé, sollicite la carte graphique",
        (_, Cost::Frames) => "Activé, fait calculer plus d'images",
        (_, Cost::Saves) => "Activé, économise",
    }
}

impl App {
    fn perf_card(&self, p: Profile, on: bool) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let fg = ink.fg;
        button(
            column![
                text(p.label())
                    .font(self.title_font())
                    .size(15)
                    .color(if on { ink.accent } else { fg }),
                text(p.summary()).font(self.font).size(12).color(ink.dim),
            ]
            .spacing(6),
        )
        .padding(12)
        .width(Length::FillPortion(1))
        .on_press(wrap(Msg::Profile(p)))
        .style(move |_, status| button::Style {
            background: Some(
                if on {
                    Color { a: 0.16, ..accent }
                } else if matches!(status, button::Status::Hovered) {
                    Color { a: 0.06, ..fg }
                } else {
                    Color::TRANSPARENT
                }
                .into(),
            ),
            text_color: fg,
            border: iced::Border::default()
                .rounded(12)
                .width(if on { 2 } else { 1 })
                .color(if on { accent } else { Color { a: 0.15, ..fg } }),
            ..Default::default()
        })
        .into()
    }

    /// Moteur de hyprdesk (onglet « hyprdesk ») : rendu, carte graphique,
    /// fils d'exécution ; ce qui tourne, et la relance quand ça diffère.
    fn engine_page(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let (dim, warn) = (ink.dim, ink.accent);
        let g = &self.cfg.general;
        let small = |s: String, c: Color| text(s).font(self.font).size(12).color(c);
        let mut col = column![small(
            crate::i18n::tr(
                "Ce que hyprdesk demande lui-même à la machine pour dessiner sa barre, son \
                     dock et ses bureaux. Ces réglages prennent effet à la relance de hyprdesk."
            )
            .into(),
            dim
        ),]
        .spacing(14);

        // État : ce qui tourne, ce qui est imposé, la relance à faire.
        let renderer = render::running_summary()
            .unwrap_or_else(|| crate::i18n::tr("rendu pas encore relevé").to_string());
        let workers = render::running_workers()
            .map(|n| crate::tr!("{n} fils d'exécution", n = n))
            .unwrap_or_else(|| crate::i18n::tr("fils non relevés").to_string());
        col = col.push(small(
            crate::tr!("En cours : {r} · {w}.", r = renderer, w = workers),
            dim,
        ));
        let started = render::started();
        for v in started.map(|s| s.forced.as_slice()).unwrap_or_default() {
            col = col.push(small(
                crate::tr!(
                    "Imposé par la variable d'environnement {v} : le réglage correspondant \
                     ne s'applique pas tant qu'elle est posée.",
                    v = v
                ),
                dim,
            ));
        }
        if g.renderer == Renderer::Hardware
            && started.is_some_and(|s| s.renderer == Renderer::Hardware)
            && render::running_hardware() == Some(false)
        {
            col = col.push(small(
                crate::i18n::tr(
                    "Aucune carte graphique utilisable : hyprdesk dessine en logiciel. \
                     Vérifier le pilote Vulkan ou OpenGL (vulkaninfo, glxinfo).",
                )
                .into(),
                warn,
            ));
        }
        if render::restart_pending(g.renderer, g.gpu, g.worker_threads) {
            col = col.push(
                row![
                    small(
                        crate::i18n::tr("Réglages modifiés : ils prennent effet à la relance.")
                            .into(),
                        warn
                    )
                    .width(Length::Fill),
                    button(text("Relancer hyprdesk").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::Restart)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }

        col = col.push(self.section("Rendu"));
        col = col.push(
            self.field(
                "Moteur de rendu",
                "renderer",
                row(Renderer::ALL.map(|r| {
                    radio(crate::i18n::tr(r.label()), r, Some(g.renderer), |v| {
                        wrap(Msg::Renderer(v))
                    })
                    .font(self.font)
                    .into()
                }))
                .spacing(16),
            ),
        );
        col = col.push(small(
            crate::i18n::tr(
                "Matériel : la carte graphique dessine la barre, le dock et le bureau (wgpu, \
                 Vulkan ou OpenGL), et le logiciel prend le relais sans carte utilisable. \
                 Logiciel : le processeur dessine tout (tiny-skia) — à réserver à un pilote \
                 graphique défaillant, le flou, la transparence et le fond vidéo y coûtent \
                 nettement plus.",
            )
            .into(),
            dim,
        ));

        col = col.push(self.section("Carte graphique"));
        col = col.push(
            self.field(
                "Carte préférée",
                "gpu",
                row(Gpu::ALL.map(|v| {
                    radio(crate::i18n::tr(v.label()), v, Some(g.gpu), |v| {
                        wrap(Msg::Gpu(v))
                    })
                    .font(self.font)
                    .into()
                }))
                .spacing(16),
            ),
        );
        col = col.push(small(
            crate::i18n::tr(
                "Économe : la carte intégrée dessine, et la carte dédiée d'un portable \
                 hybride reste en veille. Puissante : la carte dédiée est réveillée pour \
                 hyprdesk — utile seulement si les écrans y sont branchés. Sans effet avec \
                 une seule carte ou en rendu logiciel.",
            )
            .into(),
            dim,
        ));

        col = col.push(self.section("Exécution"));
        let cores = render::cores();
        col = col.push(
            self.field(
                "Fils d'exécution",
                "worker_threads",
                row(render::WORKER_CHOICES.map(|n| {
                    let label = match n {
                        0 => crate::tr!("Un par cœur ({n})", n = cores),
                        n if n == render::DEFAULT_WORKERS => {
                            crate::tr!("{n} (recommandé)", n = n)
                        }
                        n => n.to_string(),
                    };
                    radio(label, n, Some(g.worker_threads), |v| wrap(Msg::Workers(v)))
                        .font(self.font)
                        .into()
                }))
                .spacing(16),
            ),
        );
        if !render::WORKER_CHOICES.contains(&g.worker_threads) {
            col = col.push(small(
                crate::tr!("Réglé à {n} dans settings.lua.", n = g.worker_threads),
                dim,
            ));
        }
        col = col.push(small(
            crate::i18n::tr(
                "Fils qui traitent les tâches de fond de l'interface (Hyprland, D-Bus, \
                 relevés). Un par cœur, le comportement d'origine d'iced, laisse surtout \
                 des fils inactifs et de la mémoire réservée.",
            )
            .into(),
            dim,
        ));
        col.into()
    }

    pub(in crate::ui) fn performance_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "hyprdesk") {
            return self.engine_page();
        }
        self.hyprland_page()
    }

    /// Effets de Hyprland (onglet « Hyprland ») : profils et options.
    fn hyprland_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let options = &self.cfg.general.fx.options;
        let active = perf::active(options);
        let mut col = column![
            text(
                "Chaque effet de Hyprland se paie à chaque image : le flou et l'ombre sur la \
                 carte graphique, les animations en images à calculer. Un profil pose tous \
                 les réglages ci-dessous d'un coup ; chacun se retouche ensuite à la main."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Profil"),
            row(Profile::ALL.map(|p| self.perf_card(p, active == Some(p))))
                .spacing(10)
                .width(Length::Fill),
        ]
        .spacing(14);
        let status = match active {
            Some(p) => crate::tr!("Profil en place : {p}.", p = crate::i18n::tr(p.label())),
            None if perf::untouched(options) => {
                crate::i18n::tr("Aucun profil : tout vient de hyprland.lua.").to_string()
            }
            None => crate::i18n::tr("Réglages retouchés à la main (aucun profil).").to_string(),
        };
        let mut line = row![text(status).font(self.font).size(12).color(dim)]
            .spacing(12)
            .align_y(Alignment::Center);
        if !perf::untouched(options) {
            line = line.push(
                button(text("Revenir à hyprland.lua").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::Clear)),
            );
        }
        col = col.push(line);
        if self.hypr_blur_off() {
            col = col.push(
                text("Sans flou, le tiroir des Réglages devient opaque pour rester lisible.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for (title, items) in perf::GROUPS {
            col = col.push(self.section(title));
            for it in *items {
                let profiles: Vec<String> = Profile::ALL
                    .iter()
                    .map(|p| {
                        crate::tr!(
                            "{p} : {v}",
                            p = crate::i18n::tr(p.label()),
                            v = shown(it.path, p.value(it.path).as_ref())
                        )
                    })
                    .collect();
                col = col.push(
                    column![
                        self.fx_option(it.path),
                        text(format!(
                            "{} · {}",
                            crate::i18n::tr(cost_caption(it.path, it.cost)),
                            profiles.join(" · ")
                        ))
                        .font(self.font)
                        .size(11)
                        .color(dim),
                    ]
                    .spacing(4),
                );
            }
        }
        col = col.push(self.section("Processeur"));
        col = col.push(
            row![
                text(
                    "La fréquence du processeur suit le profil d'énergie du système \
                     (power-profiles-daemon)."
                )
                .font(self.font)
                .size(12)
                .color(dim)
                .width(Length::Fill),
                button(text("Profil d'énergie…").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::Energy)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
        col.into()
    }

    pub(in crate::ui) fn performance_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Profile(p) => {
                self.edit_fx(|f| p.apply(&mut f.options));
                self.commit()
            }
            Msg::Clear => {
                self.edit_fx(|f| perf::clear(&mut f.options));
                self.commit()
            }
            Msg::Energy => self.panel_update(PanelMsg::Tab("energy")),
            Msg::Renderer(r) => {
                self.settings.renderer = Some(r);
                self.commit()
            }
            Msg::Gpu(g) => {
                self.settings.gpu = Some(g);
                self.commit()
            }
            Msg::Workers(n) => {
                self.settings.worker_threads = Some(n);
                self.commit()
            }
            Msg::Restart => {
                if let Err(e) = render::restart_daemon() {
                    crate::hypr::notify(
                        0,
                        8000,
                        &crate::tr!("hyprdesk : relance impossible ({e})", e = e),
                    );
                }
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_lisibles() {
        assert_eq!(shown("debug.vfr", None), "hyprland.lua");
        assert_eq!(
            shown("decoration.blur.passes", Some(&OptValue::Num(3.0))),
            "3"
        );
        let missing = crate::i18n::untranslated(
            "performance",
            [
                ("debug.vfr", Cost::Gpu),
                ("debug.vfr", Cost::Frames),
                ("debug.vfr", Cost::Saves),
                ("decoration.blur.passes", Cost::Gpu),
                ("render.direct_scanout", Cost::Saves),
            ]
            .map(|(p, c)| cost_caption(p, c)),
        );
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
