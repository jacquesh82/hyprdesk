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
use iced::widget::{Space, button, column, radio, row};
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

/// Onglets : effets de Hyprland, carte de rendu, moteur de hyprdesk.
pub const TABS: &[super::SubTab] = &[
    ("hyprland", "Effets de Hyprland"),
    ("gpu", "Carte graphique"),
    ("hyprdesk", "Moteur de hyprdesk"),
];

/// Cartes graphiques relevées (onglet « Carte graphique »).
#[derive(Debug, Clone, Default)]
pub struct State {
    pub cards: Vec<crate::render_gpu::Card>,
    /// Carte de rendu de la session Hyprland en cours (adresse PCI).
    pub current: Option<String>,
    pub loaded: bool,
    /// « Fermer la session pour appliquer » cliqué : confirmation affichée.
    pub confirm_exit: bool,
}

/// Relève les cartes et la carte de rendu de Hyprland (hors du fil de l'interface).
pub fn load() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| State {
                cards: crate::render_gpu::cards(),
                current: crate::render_gpu::hyprland_current(),
                loaded: true,
                confirm_exit: false,
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Cards(s)),
    )
}

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
    Cards(State),
    /// Carte de rendu de Hyprland (adresse PCI ; vide : son choix).
    HyprlandGpu(String),
    /// Appliquer la carte de Hyprland : demander à fermer la session…
    AskExit,
    /// … y renoncer…
    CancelExit,
    /// … ou fermer la session (Hyprland relit env.lua au démarrage).
    Exit,
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
                    .size(crate::ui::tokens::ts(15.0))
                    .color(if on { ink.accent } else { fg }),
                text(p.summary())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(ink.dim),
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
        let small = |s: String, c: Color| {
            text(s)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(c)
        };
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
                    button(
                        text("Relancer hyprdesk")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                    )
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
        col = col.push(small(
            crate::tr!(
                "{g} — réglée dans l'onglet « Carte graphique », avec celle de Hyprland.",
                g = crate::i18n::tr(g.gpu.label())
            ),
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

    /// Carte de rendu de Hyprland et de hyprdesk (onglet « Carte graphique »).
    fn gpu_page(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let (fg, dim, warn) = (ink.fg, ink.dim, ink.accent);
        let g = &self.cfg.general;
        let small = |s: String, c: Color| {
            text(s)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(c)
        };
        let Some(st) = self
            .panel
            .as_ref()
            .map(|p| &p.performance)
            .filter(|s| s.loaded)
        else {
            return small(crate::i18n::tr("Lecture des cartes…").into(), dim).into();
        };
        let mut col = column![small(
            crate::i18n::tr(
                "Hyprland dessine toutes les images sur une seule carte. Si les écrans sont \
                 branchés sur une autre, chaque image y est recopiée : la carte de rendu \
                 travaille pour rien. Choisir la carte où sont branchés les écrans."
            )
            .into(),
            dim
        )]
        .spacing(14);
        let name_of = |slot: &str| {
            st.cards
                .iter()
                .find(|c| c.slot == slot)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| slot.to_string())
        };
        // État : carte de rendu en cours, écrans branchés ailleurs.
        match &st.current {
            Some(cur) => {
                col = col.push(small(
                    crate::tr!("Hyprland rend en ce moment sur : {c}.", c = name_of(cur)),
                    fg,
                ));
                let elsewhere: Vec<&crate::render_gpu::Card> = st
                    .cards
                    .iter()
                    .filter(|c| &c.slot != cur && !c.outputs.is_empty())
                    .collect();
                if !elsewhere.is_empty()
                    && st
                        .cards
                        .iter()
                        .find(|c| &c.slot == cur)
                        .is_some_and(|c| c.outputs.is_empty())
                {
                    col = col.push(small(
                        crate::tr!(
                            "Les écrans sont branchés sur {c}, mais Hyprland rend sur {r} : \
                             chaque image est recopiée d'une carte à l'autre. Choisir {c} \
                             ci-dessous, puis fermer et rouvrir la session.",
                            c = elsewhere[0].name.clone(),
                            r = name_of(cur)
                        ),
                        warn,
                    ));
                }
            }
            None => {
                col = col.push(small(
                    crate::i18n::tr("Carte de rendu de Hyprland inconnue (journal illisible).")
                        .into(),
                    dim,
                ));
            }
        }
        col = col.push(self.section("Rendu de Hyprland"));
        let chosen = g.hyprland_gpu.clone();
        // Choix par rang (iced veut une valeur Copy) : 0 automatique, puis les cartes.
        let slots: Vec<String> = std::iter::once(String::new())
            .chain(st.cards.iter().map(|c| c.slot.clone()))
            .collect();
        let selected = slots.iter().position(|s| *s == chosen);
        let pick =
            move |i: usize| wrap(Msg::HyprlandGpu(slots.get(i).cloned().unwrap_or_default()));
        col = col.push(
            radio(
                crate::i18n::tr("Automatique (choix de Hyprland)"),
                0usize,
                selected,
                pick.clone(),
            )
            .font(self.font),
        );
        for (i, c) in st.cards.iter().enumerate() {
            let mut what = vec![c.driver.clone()];
            if c.outputs.is_empty() {
                what.push(crate::i18n::tr("aucun écran").to_string());
            } else {
                what.push(crate::tr!("écrans : {o}", o = c.outputs.join(", ")));
            }
            if st.current.as_deref() == Some(c.slot.as_str()) {
                what.push(crate::i18n::tr("en cours").to_string());
            }
            col = col.push(
                radio(
                    format!("{} ({}) — {}", c.name, c.slot, what.join(" · ")),
                    i + 1,
                    selected,
                    pick.clone(),
                )
                .font(self.font),
            );
        }
        if !chosen.is_empty() && !st.cards.iter().any(|c| c.slot == chosen) {
            col = col.push(small(
                crate::tr!(
                    "Carte choisie absente ({s}) : réservée aux machines virtuelles (vfio-pci) \
                     ou débranchée ; Hyprland rend sur une autre au démarrage.",
                    s = chosen.clone()
                ),
                warn,
            ));
        }
        // Carte choisie, pas celle de la session : Hyprland ne la lit qu'au démarrage.
        let hypr_pending = st
            .current
            .as_deref()
            .is_some_and(|c| !chosen.is_empty() && c != chosen);
        col = col.push(small(
            crate::i18n::tr(
                "Écrit dans env.lua (AQ_DRM_DEVICES) : la carte est retrouvée par son adresse \
                 PCI à chaque démarrage, les autres restent utilisables à la suite.",
            )
            .into(),
            dim,
        ));
        col = col.push(self.section("Rendu de hyprdesk"));
        col = col.push(
            self.field(
                "Carte de la barre, du dock et des bureaux",
                "gpu",
                column(Gpu::ALL.map(|v| {
                    radio(crate::i18n::tr(v.label()), v, Some(g.gpu), |v| {
                        wrap(Msg::Gpu(v))
                    })
                    .font(self.font)
                    .into()
                }))
                .spacing(8),
            ),
        );
        // Un seul bouton pour appliquer : ce que demande le réglage changé.
        // La carte de Hyprland demande une nouvelle session (Hyprland ne se
        // relance pas sans fermer les fenêtres) ; hyprdesk y repart avec ses
        // propres réglages. Sinon, relancer hyprdesk suffit.
        let desk_pending = render::restart_pending(g.renderer, g.gpu, g.worker_threads);
        if hypr_pending && st.confirm_exit {
            col = col.push(
                column![
                    small(
                        crate::i18n::tr(
                            "Les fenêtres ouvertes vont être fermées : enregistrez votre travail. \
                             À la reconnexion, Hyprland et hyprdesk démarrent sur les cartes choisies.",
                        )
                        .into(),
                        warn
                    ),
                    row![
                        Space::new().width(Length::Fill),
                        button(text("Annuler").font(self.font).size(crate::ui::tokens::ts(12.0)))
                            .padding([4, 10])
                            .style(button::secondary)
                            .on_press(wrap(Msg::CancelExit)),
                        button(text("Fermer la session").font(self.font).size(crate::ui::tokens::ts(12.0)))
                            .padding([4, 10])
                            .style(button::danger)
                            .on_press(wrap(Msg::Exit)),
                    ]
                    .spacing(8),
                ]
                .spacing(8),
            );
        } else if hypr_pending {
            col = col.push(
                row![
                    small(
                        crate::i18n::tr(
                            "Pour appliquer : fermer puis rouvrir la session (Hyprland ne lit sa \
                             carte qu'au démarrage ; hyprdesk repart avec la sienne).",
                        )
                        .into(),
                        warn
                    )
                    .width(Length::Fill),
                    button(
                        text("Fermer la session pour appliquer…")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                    )
                    .padding([4, 10])
                    .on_press(wrap(Msg::AskExit)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        } else if desk_pending {
            col = col.push(
                row![
                    small(
                        crate::i18n::tr(
                            "Pour appliquer : relancer hyprdesk (quelques secondes, les fenêtres restent ouvertes)."
                        )
                        .into(),
                        warn
                    )
                    .width(Length::Fill),
                    button(text("Relancer hyprdesk pour appliquer").font(self.font).size(crate::ui::tokens::ts(12.0)))
                        .padding([4, 10])
                        .on_press(wrap(Msg::Restart)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn performance_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "hyprdesk") {
            return self.engine_page();
        }
        if self.on_subtab(INFO.id, "gpu") {
            return self.gpu_page();
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
            .size(crate::ui::tokens::ts(12.0))
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
        let mut line = row![
            text(status)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if !perf::untouched(options) {
            line = line.push(
                button(
                    text("Revenir à hyprland.lua")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                )
                .padding([4, 10])
                .on_press(wrap(Msg::Clear)),
            );
        }
        col = col.push(line);
        if self.hypr_blur_off() {
            col = col.push(
                text("Sans flou, le tiroir des Réglages devient opaque pour rester lisible.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
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
                        .size(crate::ui::tokens::ts(11.0))
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
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
                .width(Length::Fill),
                button(
                    text("Profil d'énergie…")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
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
            Msg::Cards(s) => {
                if let Some(p) = self.panel.as_mut() {
                    let confirm = p.performance.confirm_exit;
                    p.performance = s;
                    p.performance.confirm_exit = confirm;
                }
                Task::none()
            }
            Msg::AskExit | Msg::CancelExit => {
                if let Some(p) = self.panel.as_mut() {
                    p.performance.confirm_exit = matches!(msg, Msg::AskExit);
                }
                Task::none()
            }
            Msg::Exit => {
                if let Some(p) = self.panel.as_mut() {
                    p.performance.confirm_exit = false;
                }
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(crate::hypr::exit_session)
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| {
                        if let Err(e) = r {
                            crate::hypr::notify(
                                0,
                                8000,
                                &crate::tr!(
                                    "hyprdesk : fermeture de session impossible ({e})",
                                    e = e
                                ),
                            );
                        }
                        UiMsg::Noop
                    },
                )
            }
            Msg::HyprlandGpu(slot) => {
                self.settings.hyprland_gpu = Some(slot);
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
