//! Surface du bureau d'un écran : le fond (plugin « Fond d'écran », sinon la
//! couleur de fond), le voile du mode verre, puis — bureau relevé — les
//! icônes et les widgets disposés sans se chevaucher (`super::layout`), à
//! côté du tiroir des Réglages quand il est ouvert sur cet écran.

use iced::widget::{Space, Stack, container, mouse_area};
use iced::window::Id;
use iced::{Element, Length, Padding};

use crate::config::{Mode, Transition};
use crate::ui::{App, DesktopMode, Msg, SLIDE_PX, bg, color, kinetic};

impl App {
    /// Le bureau de la surface `id` ; rien si elle n'en porte pas.
    pub(in crate::ui) fn view_desktop(&self, id: Id) -> Element<'_, Msg> {
        let Some((m, d)) = self.desktop_of(id) else {
            return Space::new().into();
        };
        let raised = self.raised.contains(&id);
        let t = if raised { self.fade_value() } else { 1.0 };
        let mut layers: Vec<Element<'_, Msg>> = Vec::new();
        // Le bureau est sous les fenêtres : le fond reste plein, seul le contenu apparaît en fondu.
        // Paquet « Fond d'écran » coupé : la couleur de fond seule.
        layers.push(if self.cfg.plugin_on("wallpaper") {
            self.view_wall(&m.name, 1.0)
        } else {
            let c = color(self.cfg.general.background, 1.0);
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| bg(c))
                .into()
        });
        if raised && d.mode == Mode::Glass {
            // Mode verre : un voile teinté sur le fond flouté.
            let veil = color(
                self.cfg.general.theme.bg,
                self.cfg.general.glass_opacity * t,
            );
            layers.push(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| bg(veil))
                    .into(),
            );
        }
        let panel_here = self.panel.as_ref().is_some_and(|p| p.win == id);
        if raised && (!panel_here || self.size_of(id).width - self.panel_width() >= 400.0) {
            // Avec les fenêtres, seul le lanceur (dock) reste : ni icônes ni widgets.
            let with_windows = self
                .surfaces
                .get(&id)
                .is_some_and(|n| self.applied_mode(n) == DesktopMode::LauncherOnly);
            let mut content: Vec<Element<'_, Msg>> = Vec::new();
            if !with_windows {
                if self.cfg.plugin_on("icons") {
                    content.push(self.view_icons(id, t));
                }
                if self.cfg.plugin_on("widgets") && !d.widgets.is_empty() {
                    // Sans superposition : chaque widget écarté de ceux déjà posés.
                    content.push(super::layout::desk(
                        d.widgets
                            .iter()
                            .map(|w| self.view_widget(id, w, t))
                            .collect(),
                    ));
                }
            }
            // Le lanceur a sa propre surface, au-dessus des fenêtres. En
            // mouvement réduit, la remontée devient un simple fondu (HIG FON-07).
            let shift = if self.cfg.general.transition == Transition::Slide
                && !crate::ui::tokens::motion().reduced
            {
                (1.0 - t) * SLIDE_PX
            } else {
                0.0
            };
            // Le tiroir des réglages occupe la droite : le bureau reste visible
            // à côté et montre chaque réglage en direct.
            let (rl, rt, rr, rb) = self.reserved(id);
            let right = rr + if panel_here { self.panel_width() } else { 0.0 };
            // Clic droit sur le fond (pas sur une icône ni un widget, qui ont
            // le leur) : menu du bureau.
            let content: Element<'_, Msg> = if with_windows {
                Stack::with_children(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                mouse_area(
                    Stack::with_children(content)
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .on_right_press(Msg::DeskContext(id))
                .into()
            };
            layers.push(
                container(content)
                    .padding(Padding {
                        top: rt + shift,
                        right,
                        bottom: rb,
                        left: rl,
                    })
                    .into(),
            );
        }
        kinetic::probe(
            Stack::with_children(layers)
                .width(Length::Fill)
                .height(Length::Fill),
            id,
        )
    }
}
