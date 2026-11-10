//! Touches de volume, de luminosité et de lecture : `hyprdesk osd …` et
//! `hyprdesk media …`, liées aux touches XF86 par hyprdesk (`keys.media`).
//!
//! L'action se fait ici, dans le processus client : elle marche même sans
//! démon. Volume et micro par `wpctl` (PipeWire), luminosité par logind
//! (`crate::brightness`), lecture par MPRIS.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Volume,
    Mic,
    Brightness,
    Kbd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Pas en pour cent, signé.
    By(i32),
    Toggle,
}

/// Niveau après l'action, pour l'affichage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub kind: Kind,
    /// 0..1 (le volume peut dépasser 1, jusqu'à 1,5).
    pub value: f32,
    pub muted: bool,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "volume" => Kind::Volume,
            "mic" => Kind::Mic,
            "brightness" => Kind::Brightness,
            "kbd" => Kind::Kbd,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Volume => "volume",
            Kind::Mic => "mic",
            Kind::Brightness => "brightness",
            Kind::Kbd => "kbd",
        }
    }
}

impl Step {
    /// `+5`, `-5` ou `toggle`.
    pub fn parse(s: &str) -> Option<Step> {
        if s == "toggle" {
            return Some(Step::Toggle);
        }
        let n: i32 = s.parse().ok()?;
        (s.starts_with(['+', '-']) && n != 0 && n.abs() <= 100).then_some(Step::By(n))
    }
}

/// `osd <kind> <step>` → action.
pub fn parse(args: &[String]) -> Result<(Kind, Step), String> {
    let usage =
        || crate::i18n::tr("hyprdesk osd <volume|mic|brightness|kbd> <+N|-N|toggle>").to_string();
    let kind = args
        .first()
        .and_then(|a| Kind::parse(a))
        .ok_or_else(usage)?;
    let step = args.get(1).and_then(|a| Step::parse(a)).ok_or_else(usage)?;
    if step == Step::Toggle && kind == Kind::Brightness {
        return Err(
            crate::i18n::tr("la luminosité de l'écran ne se bascule pas : +N ou -N").to_string(),
        );
    }
    Ok((kind, step))
}

/// Arguments de `wpctl` pour une action sur le son.
fn wpctl_args(kind: Kind, step: Step) -> Vec<String> {
    let target = if kind == Kind::Mic {
        "@DEFAULT_AUDIO_SOURCE@"
    } else {
        "@DEFAULT_AUDIO_SINK@"
    };
    match step {
        Step::Toggle => vec!["set-mute".into(), target.into(), "toggle".into()],
        Step::By(n) => vec![
            "set-volume".into(),
            "-l".into(),
            "1.5".into(),
            target.into(),
            format!("{}%{}", n.abs(), if n > 0 { "+" } else { "-" }),
        ],
    }
}

async fn wpctl(args: &[String]) -> Result<String, String> {
    let o = crate::processes::output(
        tokio::process::Command::new("wpctl").args(args),
        Duration::from_secs(2),
    )
    .await
    .map_err(|e| crate::tr!("wpctl : {e}", e = e))?;
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Fait l'action et rend le niveau obtenu.
pub async fn apply(kind: Kind, step: Step) -> Result<Level, String> {
    match kind {
        Kind::Volume | Kind::Mic => {
            wpctl(&wpctl_args(kind, step)).await?;
            let target = if kind == Kind::Mic {
                "@DEFAULT_AUDIO_SOURCE@"
            } else {
                "@DEFAULT_AUDIO_SINK@"
            };
            let out = wpctl(&["get-volume".into(), target.into()]).await?;
            let (value, muted) = crate::ui::bar::parse_volume(&out)
                .ok_or_else(|| crate::i18n::tr("volume illisible (wpctl)").to_string())?;
            Ok(Level { kind, value, muted })
        }
        Kind::Brightness | Kind::Kbd => {
            let dev = if kind == Kind::Kbd {
                crate::brightness::keyboard()
                    .ok_or_else(|| crate::i18n::tr("aucun rétroéclairage de clavier").to_string())?
            } else {
                crate::brightness::screen()
                    .ok_or_else(|| crate::i18n::tr("aucun rétroéclairage d'écran").to_string())?
            };
            let value = match step {
                Step::By(n) => crate::brightness::next_value(&dev, n),
                // Clavier : allumé ↔ éteint.
                Step::Toggle => {
                    if dev.current > 0 {
                        0
                    } else {
                        dev.max
                    }
                }
            };
            crate::brightness::set(&dev, value)
                .await
                .map_err(|e| crate::tr!("luminosité (logind) : {e}", e = e))?;
            Ok(Level {
                kind,
                value: crate::brightness::Device {
                    current: value,
                    ..dev
                }
                .level(),
                muted: false,
            })
        }
    }
}

/// Méthode MPRIS d'une commande `hyprdesk media …`.
pub fn media_method(s: &str) -> Option<&'static str> {
    Some(match s {
        "play-pause" | "toggle" => "PlayPause",
        "next" => "Next",
        "previous" | "prev" => "Previous",
        _ => return None,
    })
}

/// Lecture : agit sur le lecteur qui joue, sinon le premier trouvé.
pub async fn media(method: &'static str) -> Result<(), String> {
    match crate::widgets::media::media("*").await {
        crate::widgets::Value::Media(Some(m)) => {
            crate::widgets::media::media_control(m.bus, method).await;
            Ok(())
        }
        crate::widgets::Value::Media(None) => {
            Err(crate::i18n::tr("aucun lecteur (MPRIS)").to_string())
        }
        _ => Err(crate::i18n::tr("lecteurs injoignables (MPRIS)").to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn arguments() {
        assert_eq!(
            parse(&s(&["volume", "+5"])),
            Ok((Kind::Volume, Step::By(5)))
        );
        assert_eq!(parse(&s(&["mic", "toggle"])), Ok((Kind::Mic, Step::Toggle)));
        assert_eq!(parse(&s(&["kbd", "-10"])), Ok((Kind::Kbd, Step::By(-10))));
        assert!(parse(&s(&["brightness", "toggle"])).is_err());
        assert!(parse(&s(&["volume", "5"])).is_err(), "signe obligatoire");
        assert!(parse(&s(&["volume", "+0"])).is_err());
        assert!(parse(&s(&["bass", "+5"])).is_err());
        assert!(parse(&s(&["volume"])).is_err());
    }

    #[test]
    fn commandes_wpctl() {
        assert_eq!(
            wpctl_args(Kind::Volume, Step::By(5)),
            s(&["set-volume", "-l", "1.5", "@DEFAULT_AUDIO_SINK@", "5%+"])
        );
        assert_eq!(
            wpctl_args(Kind::Mic, Step::Toggle),
            s(&["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
        );
        assert_eq!(wpctl_args(Kind::Volume, Step::By(-3))[4], "3%-");
    }

    #[test]
    fn lecture() {
        assert_eq!(media_method("play-pause"), Some("PlayPause"));
        assert_eq!(media_method("prev"), Some("Previous"));
        assert_eq!(media_method("stop"), None);
    }
}
