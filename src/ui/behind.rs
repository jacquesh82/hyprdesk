//! Ce qu'il y a derrière un menu ou les Réglages (MLG-275).
//!
//! Hyprdesk ne voit jamais les fenêtres : pour que la lentille d'une
//! surface posée sur elles réfracte quelque chose de vrai, l'écran est
//! capturé à l'ouverture (`grim`), avant que la surface ne devienne
//! visible — son fondu d'apparition attend [`DELAY`]. La place exacte de la
//! surface vient ensuite de Hyprland (`hyprctl layers`). L'image reste
//! figée tant que la surface est ouverte : une vidéo derrière un menu ne
//! bouge plus dans son verre. Seulement quand le thème a de la réfraction.

use std::sync::Arc;
use std::time::Duration;

use iced::widget::image;
use iced::{Element, Point, Size};

use crate::config::Place;

/// Attente avant le fondu d'apparition : la capture ne doit pas montrer
/// la surface elle-même.
pub const DELAY: Duration = Duration::from_millis(60);

/// Capture de l'écran sous une surface.
#[derive(Debug)]
pub struct Behind {
    pub handle: image::Handle,
    /// Coin de la surface sur l'écran (pixels logiques).
    pub origin: Point,
    /// Taille logique de l'écran.
    pub screen: Size,
}

/// La capture servira : rendu matériel, réfraction au thème pour cet
/// endroit, transparence pleine.
pub fn wanted(place: Place) -> bool {
    super::lens::available()
        && super::tokens::glass(place).refraction > 0.0
        && !super::tokens::reduced_transparency()
}

/// Image de l'écran `output` (pixels réels).
async fn shot(output: &str) -> Option<image::Handle> {
    let shot = tokio::process::Command::new("grim")
        .args(["-o", output, "-t", "png", "-l", "0", "-"])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())?;
    let png = shot.stdout;
    let rgba = tokio::task::spawn_blocking(move || {
        ::image::load_from_memory_with_format(&png, ::image::ImageFormat::Png)
            .ok()
            .map(|i| i.to_rgba8())
    })
    .await
    .ok()??;
    Some(image::Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}

/// Capture l'écran `output` (coin global `monitor`, taille logique
/// `screen`), puis situe la surface `namespace` dessus, une fois créée.
pub async fn capture(
    output: String,
    namespace: &'static str,
    monitor: Point,
    screen: Size,
) -> Option<Arc<Behind>> {
    let handle = shot(&output).await?;
    // La surface a eu le temps d'apparaître (invisible) : sa place.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let layers = tokio::process::Command::new("hyprctl")
        .args(["layers", "-j"])
        .output()
        .await
        .ok()?;
    let (x, y) = place_of(&String::from_utf8_lossy(&layers.stdout), &output, namespace)?;
    Some(Arc::new(Behind {
        handle,
        origin: Point::new(x - monitor.x, y - monitor.y),
        screen,
    }))
}

/// Capture l'écran `output` avant d'y créer une surface dont la place
/// `origin` est connue d'avance (Réglages, ancrés au bord droit).
pub async fn grab(output: String, origin: Point, screen: Size) -> Option<Arc<Behind>> {
    let handle = shot(&output).await?;
    Some(Arc::new(Behind {
        handle,
        origin,
        screen,
    }))
}

/// Place globale (pixels logiques) de la couche `namespace` sur `output`,
/// d'après `hyprctl layers -j`.
fn place_of(json: &str, output: &str, namespace: &str) -> Option<(f32, f32)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let levels = v.get(output)?.get("levels")?.as_object()?;
    levels
        .values()
        .filter_map(|l| l.as_array())
        .flatten()
        .find(|l| l.get("namespace").and_then(|n| n.as_str()) == Some(namespace))
        .and_then(|l| Some((l.get("x")?.as_f64()? as f32, l.get("y")?.as_f64()? as f32)))
}

impl super::App {
    /// Capture sous la surface `id`, peinte à sa place (fond de sa
    /// lentille).
    pub(super) fn behind_backdrop(&self, id: iced::window::Id) -> Option<Element<'_, super::Msg>> {
        let (owner, b) = self.behind.as_ref()?;
        if *owner != id {
            return None;
        }
        Some(Element::new(super::glass::Backdrop {
            wall: super::glass::Backing::Image(b.handle.clone(), iced::ContentFit::Fill),
            screen: b.screen,
            origin: b.origin,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_de_la_couche() {
        let json = r#"{"DP-1":{"levels":{"0":[{"namespace":"hyprdesk","x":0,"y":0}],
            "3":[{"namespace":"hyprdesk-popover","x":2310,"y":42,"w":240,"h":300}]}},
            "DP-2":{"levels":{"3":[{"namespace":"hyprdesk-popover","x":9,"y":9}]}}}"#;
        assert_eq!(
            place_of(json, "DP-1", "hyprdesk-popover"),
            Some((2310.0, 42.0))
        );
        assert_eq!(place_of(json, "DP-1", "hyprdesk-settings"), None);
        assert_eq!(place_of("pas du json", "DP-1", "x"), None);
    }
}
