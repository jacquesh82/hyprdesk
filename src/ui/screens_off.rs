//! Veille des écrans (MLG-174) : tous les écrans s'éteignent (DPMS), la
//! machine reste allumée — la musique continue, les téléchargements aussi.
//! Un changement de volume les rallume (`idle_wake_volume`) : sur un poste
//! sans clavier à portée (Panorama), la molette du son suffit. La souris et
//! le clavier suivent les réglages de Hyprland (`misc.*_enables_dpms`).

use std::time::Duration;

use iced::Task;

use super::{App, Msg, bar};

/// Délai avant d'éteindre : la main quitte l'écran tactile ou la souris.
const SETTLE: Duration = Duration::from_millis(600);

fn dpms(action: &'static str) -> Task<Msg> {
    Task::perform(
        async move {
            if action == "disable" {
                tokio::time::sleep(SETTLE).await;
            }
            let _ = tokio::task::spawn_blocking(move || {
                hyprpanel_core::ipc::dispatch(&format!("hl.dsp.dpms({{ action = \"{action}\" }})"))
            })
            .await;
        },
        |_| Msg::Noop,
    )
}

/// Le volume relu rallume-t-il les écrans ? (départ inconnu : il le devient).
pub fn wakes(base: &mut Option<(f32, bool)>, now: Option<(f32, bool)>) -> bool {
    let Some(now) = now else {
        return false;
    };
    match base {
        None => {
            *base = Some(now);
            false
        }
        Some(b) => (b.0 - now.0).abs() > 0.004 || b.1 != now.1,
    }
}

impl App {
    pub(super) fn screens_off(&mut self) -> Task<Msg> {
        // Suivi par PipeWire déjà en cours : le volume connu est le bon ;
        // sinon le premier relevé sert de départ.
        let watched = bar::VOLUME_WATCH.load(std::sync::atomic::Ordering::Relaxed);
        self.screens_off = Some(self.volume.filter(|_| watched));
        let read = if watched {
            Task::none()
        } else {
            Task::perform(bar::read_volume(), Msg::VolumeValue)
        };
        Task::batch([read, dpms("disable")])
    }

    /// Volume relu pendant la veille des écrans : changé, ils se rallument.
    pub(super) fn screens_wake(&mut self, v: Option<(f32, bool)>) -> Task<Msg> {
        let Some(base) = self.screens_off.as_mut() else {
            return Task::none();
        };
        if !self.cfg.general.idle.wake_volume || !wakes(base, v) {
            return Task::none();
        }
        self.screens_off = None;
        dpms("enable")
    }
}

#[cfg(test)]
mod tests {
    use super::wakes;

    #[test]
    fn le_volume_rallume() {
        let mut base = None;
        assert!(!wakes(&mut base, None));
        assert!(!wakes(&mut base, Some((0.5, false))), "départ relevé");
        assert!(!wakes(&mut base, Some((0.5, false))), "même volume");
        assert!(wakes(&mut base, Some((0.55, false))));
        assert!(wakes(&mut base, Some((0.5, true))), "coupé : rallume aussi");
    }
}
