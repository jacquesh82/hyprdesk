//! Module Météo : le temps qu'il fait et les jours qui suivent, par wttr.in
//! (JSON, sans compte ni clé). Les requêtes passent par `curl`, comme la
//! webradio et le pont Hue : pas de client HTTP embarqué.
//!
//! Option : `meteo.city`, la ville telle que wttr.in la cherche (un nom, un
//! code postal, « lat,lon »…) ; vide : la position déduite de l'adresse IP.
//!
//! Jour ou nuit : le service le décide pour le lieu, par l'icône qu'il
//! propose (`_night` dans son adresse ; les éclaircies, elles, y perdent leur
//! soleil) — juste dans tous les fuseaux, sans supposer celui de la machine.

use crate::i18n::Lang;
use serde_json::Value;

/// Ville réglée (vide : la position de l'adresse IP).
pub fn city(cfg: &crate::config::Config) -> String {
    super::option(cfg, "meteo", "city")
}

/// Adresse du bulletin de `city`.
pub fn url(city: &str) -> String {
    format!(
        "https://wttr.in/{}?format=j1",
        crate::panorama::radio::encode(city)
    )
}

/// Un bulletin en une ligne : « Lyon, France — 7°, Couvert ».
pub fn summary(w: &Weather) -> String {
    crate::tr!(
        "{place} — {temp}°, {label}",
        place = w.place.clone(),
        temp = w.now.temp,
        label = crate::i18n::tr(w.now.kind.label(w.now.night)).to_string()
    )
}

/// Ce que répond le service pour `city` : le bulletin en une ligne, ou la
/// raison pour laquelle il ne la connaît pas (vérification du réglage de la
/// ville). Bloquant (requête HTTP).
pub(super) fn check_city(city: &str) -> Result<String, String> {
    fetch(city).map(|w| summary(&w))
}

/// Famille de temps ; son icône et son libellé suivent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Clear,
    FewClouds,
    Cloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    HeavyRain,
    Showers,
    FreezingRain,
    Sleet,
    Snow,
    HeavySnow,
    Blizzard,
    Storm,
    Hail,
}

impl Kind {
    /// Famille d'après le code Météo de la réponse (worldweatheronline) ; un
    /// code inconnu vaut temps couvert.
    pub fn of(code: u16) -> Self {
        match code {
            113 => Self::Clear,
            116 => Self::FewClouds,
            119 => Self::Cloudy,
            122 => Self::Overcast,
            143 | 248 | 260 => Self::Fog,
            263 | 266 => Self::Drizzle,
            176 | 353 | 356 => Self::Showers,
            293 | 296 | 299 | 302 => Self::Rain,
            305 | 308 | 359 => Self::HeavyRain,
            185 | 282 | 311 | 314 => Self::FreezingRain,
            182 | 317 | 320 | 362 | 365 => Self::Sleet,
            179 | 227 | 323 | 326 | 329 | 332 | 368 => Self::Snow,
            335 | 338 | 371 => Self::HeavySnow,
            230 => Self::Blizzard,
            200 | 386 | 389 | 392 | 395 => Self::Storm,
            350 | 374 | 377 => Self::Hail,
            _ => Self::Overcast,
        }
    }

    /// Glyphe de `ui::icons` : soleil ou lune, suivant la nuit.
    pub fn icon(self, night: bool) -> &'static str {
        match self {
            Self::Clear => {
                if night {
                    "weather-clear-night"
                } else {
                    "weather-clear"
                }
            }
            Self::FewClouds => {
                if night {
                    "weather-few-clouds-night"
                } else {
                    "weather-few-clouds"
                }
            }
            Self::Cloudy => "weather-cloudy",
            Self::Overcast => "weather-overcast",
            Self::Fog => "weather-fog",
            Self::Drizzle => "weather-drizzle",
            // Averses de nuit : le nuage et la pluie, sans soleil.
            Self::Rain | Self::HeavyRain | Self::Showers if night => "weather-rain",
            Self::Rain | Self::HeavyRain => "weather-rain",
            Self::Showers => "weather-showers",
            Self::FreezingRain | Self::Sleet => "weather-sleet",
            Self::Snow | Self::HeavySnow | Self::Blizzard => "weather-snow",
            Self::Storm => "weather-storm",
            Self::Hail => "weather-hail",
        }
    }

    /// Libellé affiché (traduit à l'écran).
    pub fn label(self, night: bool) -> &'static str {
        match self {
            Self::Clear if night => "Nuit claire",
            Self::Clear => "Ensoleillé",
            Self::FewClouds => "Peu nuageux",
            Self::Cloudy => "Nuageux",
            Self::Overcast => "Couvert",
            Self::Fog => "Brouillard",
            Self::Drizzle => "Bruine",
            Self::Rain => "Pluie",
            Self::HeavyRain => "Pluie forte",
            Self::Showers => "Averses",
            Self::FreezingRain => "Pluie verglaçante",
            Self::Sleet => "Grésil",
            Self::Snow => "Neige",
            Self::HeavySnow => "Fortes chutes de neige",
            Self::Blizzard => "Blizzard",
            Self::Storm => "Orage",
            Self::Hail => "Grêle",
        }
    }
}

/// Bulletin : le temps qu'il fait, puis les jours qui suivent.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    /// Lieu rendu par wttr.in (le quartier ou la ville, et son pays).
    pub place: String,
    pub now: Now,
    /// Du jour (sur place) aux suivants.
    pub days: Vec<Day>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Now {
    /// Température et température ressentie, en °C.
    pub temp: i16,
    pub feels: i16,
    pub kind: Kind,
    /// Nuit sur place : lune au lieu de soleil.
    pub night: bool,
    /// Vent en km/h, et sa direction (`winddir16Point` : « NNW »).
    pub wind: u8,
    pub wind_dir: String,
    /// Humidité, en %.
    pub humidity: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Day {
    /// `2026-11-26`.
    pub date: String,
    /// Températures mini et maxi, en °C.
    pub min: i16,
    pub max: i16,
    /// Temps de la journée (midi).
    pub kind: Kind,
    /// Plus forte probabilité de pluie de la journée, en %.
    pub rain: u8,
    /// Un relevé toutes les trois heures, de 0 h à 21 h (heure sur place).
    pub hours: Vec<Hour>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hour {
    /// Heure du relevé, sur place (0, 3, … 21).
    pub hour: u8,
    pub temp: i16,
    pub kind: Kind,
    /// Nuit sur place à cette heure.
    pub night: bool,
    /// Probabilité de pluie, en %.
    pub rain: u8,
}

impl Weather {
    /// Relevés des heures à venir, depuis celui qui contient `minute`
    /// (minute du jour, 0 à 1439) : la fin du jour, puis les jours suivants.
    pub fn hours_from(&self, minute: u16) -> Vec<&Hour> {
        let from = (minute.min(1439) / 180) as u8 * 3;
        self.days
            .iter()
            .enumerate()
            .flat_map(|(i, d)| d.hours.iter().filter(move |h| i > 0 || h.hour >= from))
            .collect()
    }

    /// Jours suivants, sans celui-ci (déjà dans l'en-tête).
    pub fn next_days(&self) -> &[Day] {
        self.days.get(1..).unwrap_or(&[])
    }
}

/// Point du compas de wttr.in (anglais : « NNW »), en abrégé dans la langue
/// de l'interface. Hors du catalogue à dessein : des clés d'une ou deux
/// lettres (« O », « NO ») traduiraient tout texte affiché qui leur ressemble.
pub fn wind_dir(en: &str, lang: Lang) -> String {
    match (en, lang) {
        ("SSW", Lang::Fr) => "SSO",
        ("SW", Lang::Fr) => "SO",
        ("WSW", Lang::Fr) => "OSO",
        ("W", Lang::Fr) => "O",
        ("WNW", Lang::Fr) => "ONO",
        ("NW", Lang::Fr) => "NO",
        ("NNW", Lang::Fr) => "NNO",
        // Ceux qui s'écrivent pareil (N, NE, E…), et l'inconnu.
        (other, _) => other,
    }
    .to_string()
}

/// Heure d'un relevé, en clair : « 3 h », ou « 3 AM » en anglais.
pub fn hour_label(hour: u8, lang: Lang) -> String {
    match lang {
        Lang::Fr => format!("{hour} h"),
        Lang::En => format!(
            "{} {}",
            match hour % 12 {
                0 => 12,
                h => h,
            },
            if hour < 12 { "AM" } else { "PM" }
        ),
    }
}

/// Ce que la tuile montre, selon la place qu'elle a.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Côté de l'icône du moment, taille de sa température.
    pub icon: f32,
    pub temp: f32,
    /// Côté des icônes des rangées (heures, jours).
    pub small: f32,
    /// Le lieu et le jour tiennent à droite de l'en-tête ; sinon le lieu
    /// descend sur la ligne en dessous.
    pub beside: bool,
    /// Une ligne sous l'en-tête : les détails (ressentie, vent, humidité),
    /// ou le lieu quand il n'est pas à droite.
    pub line: bool,
    /// Relevés des heures à venir, jours suivants (0 : la rangée ne vient pas).
    pub hours: usize,
    pub days: usize,
}

/// Marge intérieure de la tuile, écart entre deux de ses parties, écart de
/// l'en-tête (icône, température, lieu) : la tuile les pose, `plan` en tient
/// compte.
pub const PAD: f32 = 14.0;
pub const GAP: f32 = 10.0;
pub const EDGE: f32 = 16.0;
/// Largeur d'une colonne (relevé d'heure, jour) et hauteur d'une rangée,
/// hors l'icône, à l'échelle 1.
const HOUR_W: f32 = 62.0;
const DAY_W: f32 = 84.0;
const ROW_H: f32 = 64.0;
/// Largeur d'une température (« 12° », en part de sa taille) et celle qu'il
/// faut à un lieu pour rester à droite de l'en-tête.
const TEMP_W: f32 = 1.55;
const PLACE_W: f32 = 64.0;

/// Composition de la tuile `w`×`h` (à l'échelle `scale` du texte du
/// Panorama) : ce qui tient, les jours suivants d'abord, puis les heures, puis
/// la ligne sous l'en-tête.
pub fn plan(w: f32, h: f32, scale: f32) -> Plan {
    let scale = scale.max(0.1);
    let (w, h) = (w.max(1.0), h.max(1.0));
    let icon = (h * 0.3).clamp(40.0, 200.0);
    let mut p = Plan {
        temp: (icon * 0.62).round(),
        icon,
        small: (h * 0.06).clamp(22.0, 44.0),
        beside: false,
        line: false,
        hours: 0,
        days: 0,
    };
    // En largeur : le lieu à droite de l'en-tête s'il y a la place ; et
    // combien de colonnes entrent, au plus.
    let inner_w = (w - 2.0 * PAD).max(1.0);
    p.beside = inner_w >= icon + 2.0 * EDGE + TEMP_W * p.temp + PLACE_W * scale;
    let hours = ((inner_w / (HOUR_W * scale)) as usize).clamp(1, 8);
    let days = ((inner_w / (DAY_W * scale)) as usize).clamp(1, 4);
    // En hauteur : chaque partie prise dans l'ordre, tant qu'elle tient (une
    // marge de 4 px : les lignes de texte n'ont pas partout la même hauteur).
    let mut left = h - 2.0 * PAD - 4.0 - icon - 6.0;
    let mut fits = |need: f32| {
        if left >= need {
            left -= need;
            true
        } else {
            false
        }
    };
    let row = p.small + ROW_H * scale + GAP;
    if fits(row) {
        p.days = days;
    }
    if fits(row) {
        p.hours = hours;
    }
    p.line = fits(18.0 * scale + GAP);
    p
}

/// Relevé de wttr.in pour `city`. Bloquant (requête HTTP).
pub fn fetch(city: &str) -> Result<Weather, String> {
    // Sans `-f` : le corps d'une réponse en erreur dit pourquoi (lieu
    // introuvable) ; la dernière ligne est le code HTTP.
    let out = std::process::Command::new("curl")
        .args([
            "-sSL",
            "--max-time",
            "10",
            "--max-filesize",
            "2000000",
            "-A",
            concat!("hyprdesk/", env!("CARGO_PKG_VERSION")),
            "-w",
            "\n%{http_code}",
        ])
        .arg(url(city))
        .output()
        .map_err(|e| format!("curl : {e}"))?;
    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if e.is_empty() {
            crate::i18n::tr("wttr.in injoignable").to_string()
        } else {
            e
        });
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, status) = text.trim_end().rsplit_once('\n').unwrap_or(("", ""));
    match status.trim() {
        "200" => parse(body),
        // Lieu que le service ne trouve pas : il répond 500, « location not found ».
        _ if body.contains("location not found") => Err(if city.trim().is_empty() {
            crate::i18n::tr("wttr.in n'a pas trouvé votre position (adresse IP).").to_string()
        } else {
            crate::tr!(
                "Lieu introuvable : wttr.in ne connaît pas « {city} ».",
                city = city.trim().to_string()
            )
        }),
        s => Err(format!("wttr.in : HTTP {s}")),
    }
}

fn unexpected() -> String {
    crate::i18n::tr("réponse inattendue de wttr.in").to_string()
}

/// Nombre de la réponse (wttr.in l'écrit en texte : « 7 »).
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Degrés Celsius de la réponse, entiers.
fn celsius(n: f64) -> i16 {
    n.round().clamp(-100.0, 100.0) as i16
}

/// Pourcentage de la réponse (0 à 100).
fn percent(v: &Value) -> u8 {
    number(v).map_or(0, |n| n.clamp(0.0, 100.0).round() as u8)
}

/// Code Météo d'un relevé ; absent ou illisible : temps couvert.
fn code(v: &Value) -> u16 {
    number(v).map_or(122, |n| n.clamp(0.0, 65535.0).round() as u16)
}

/// Nuit sur place, d'après l'icône que propose le service : `_night` la
/// nuit — sauf pour les éclaircies (116), qu'il dessine alors sans soleil
/// (`black_low_cloud`) : sans soleil dans l'icône, c'est la nuit. Les autres
/// temps (nuage, pluie, neige) ne montrent ni soleil ni lune : peu importe.
fn night(v: &Value) -> bool {
    let Some(icon) = v["weatherIconUrl"][0]["value"].as_str() else {
        return false;
    };
    icon.ends_with("_night.png") || (code(&v["weatherCode"]) == 116 && !icon.contains("sunny"))
}

fn hour(h: &Value) -> Option<Hour> {
    Some(Hour {
        hour: (number(&h["time"])? / 100.0).round().clamp(0.0, 23.0) as u8,
        temp: celsius(number(&h["tempC"])?),
        kind: Kind::of(code(&h["weatherCode"])),
        night: night(h),
        rain: percent(&h["chanceofrain"]),
    })
}

fn day(d: &Value) -> Option<Day> {
    let (min, max) = (number(&d["mintempC"])?, number(&d["maxtempC"])?);
    let hours: Vec<Hour> = d["hourly"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(hour)
        .collect();
    // Temps du jour : midi, sinon un relevé de la journée, sinon le premier.
    let noon = hours
        .iter()
        .find(|h| h.hour == 12)
        .or_else(|| hours.iter().find(|h| (9..=15).contains(&h.hour)))
        .or_else(|| hours.first());
    Some(Day {
        date: d["date"].as_str().unwrap_or("").to_string(),
        min: celsius(min),
        max: celsius(max),
        kind: noon.map_or(Kind::Overcast, |h| h.kind),
        rain: hours.iter().map(|h| h.rain).max().unwrap_or(0),
        hours,
    })
}

/// Bulletin lu dans la réponse JSON de wttr.in (`?format=j1`).
pub fn parse(json: &str) -> Result<Weather, String> {
    let v: Value = serde_json::from_str(json).map_err(|_| unexpected())?;
    let c = &v["current_condition"][0];
    let Some(temp) = number(&c["temp_C"]) else {
        return Err(unexpected());
    };
    let days: Vec<Day> = v["weather"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(day)
        .collect();
    if days.is_empty() {
        return Err(unexpected());
    }
    let a = &v["nearest_area"][0];
    let place = |k: &str| {
        a[k][0]["value"]
            .as_str()
            .map(str::trim)
            .unwrap_or("")
            .to_string()
    };
    let (name, country) = (place("areaName"), place("country"));
    let place = match (name.is_empty(), country.is_empty()) {
        (true, true) => String::new(),
        (true, false) => country,
        (false, true) => name,
        (false, false) => format!("{name}, {country}"),
    };
    let temp = celsius(temp);
    Ok(Weather {
        place,
        now: Now {
            temp,
            feels: number(&c["FeelsLikeC"]).map_or(temp, celsius),
            kind: Kind::of(code(&c["weatherCode"])),
            night: night(c),
            wind: number(&c["windspeedKmph"]).map_or(0, |n| n.clamp(0.0, 250.0).round() as u8),
            wind_dir: c["winddir16Point"].as_str().unwrap_or("").to_string(),
            humidity: percent(&c["humidity"]),
        },
        days,
    })
}

/// Bulletin d'exemple, pour l'aperçu (`hyprdesk preview`, scène
/// `panorama-meteo`) : fin novembre à Lyon, la pluie du matin se dégage.
pub fn sample() -> Weather {
    /// Un jour de ce bulletin : ses huit relevés (températures, temps,
    /// pluie), de 0 h à 21 h.
    fn jour(date: &str, temps: [i16; 8], kinds: [Kind; 8], rains: [u8; 8]) -> Day {
        let hours: Vec<Hour> = (0..8)
            .map(|i| Hour {
                hour: i as u8 * 3,
                temp: temps[i],
                kind: kinds[i],
                // Nuit hors de 9 h à 18 h.
                night: !(3..6).contains(&i),
                rain: rains[i],
            })
            .collect();
        // Temps du jour : celui de midi, comme dans un bulletin lu.
        Day {
            date: date.into(),
            min: temps.iter().copied().min().unwrap_or(0),
            max: temps.iter().copied().max().unwrap_or(0),
            kind: hours[4].kind,
            rain: rains.iter().copied().max().unwrap_or(0),
            hours,
        }
    }
    #[rustfmt::skip]
    let days = vec![
        jour("2026-11-26", [7, 6, 5, 7, 9, 11, 10, 8],
             [Kind::Rain, Kind::Rain, Kind::Cloudy, Kind::FewClouds, Kind::Clear, Kind::FewClouds, Kind::Cloudy, Kind::Rain],
             [60, 55, 40, 20, 10, 5, 15, 35]),
        jour("2026-11-27", [6, 5, 4, 7, 10, 13, 12, 9],
             [Kind::FewClouds, Kind::Clear, Kind::Clear, Kind::Clear, Kind::Clear, Kind::Clear, Kind::FewClouds, Kind::FewClouds],
             [10, 5, 5, 5, 5, 5, 10, 10]),
        jour("2026-11-28", [9, 8, 7, 10, 13, 14, 13, 11],
             [Kind::Rain, Kind::Rain, Kind::Showers, Kind::Showers, Kind::Rain, Kind::Storm, Kind::Rain, Kind::Showers],
             [70, 65, 60, 55, 70, 80, 70, 60]),
    ];
    Weather {
        place: "Lyon, France".into(),
        now: Now {
            temp: 9,
            feels: 7,
            kind: Kind::FewClouds,
            night: false,
            wind: 14,
            wind_dir: "NW".into(),
            humidity: 68,
        },
        days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Relevé d'une heure : heure, température, code, pluie ; de nuit, le
    /// service propose son icône de nuit.
    fn h(time: i32, temp: &str, code: i32, rain: i32, night: bool) -> Value {
        let icon = if night {
            "https://cdn.example/wsymbol_0008_clear_sky_night.png"
        } else {
            "https://cdn.example/wsymbol_0001_sunny.png"
        };
        json!({
            "time": time.to_string(), "tempC": temp, "weatherCode": code.to_string(),
            "chanceofrain": rain.to_string(),
            "weatherIconUrl": [{"value": icon}],
        })
    }

    fn reponse() -> Value {
        json!({
            "current_condition": [{
                "temp_C": "7", "FeelsLikeC": "6", "weatherCode": "122", "humidity": "91",
                "windspeedKmph": "7", "winddir16Point": "NNW",
                "weatherIconUrl": [{"value": "https://cdn.example/wsymbol_0033_cloudy_with_light_rain_night.png"}],
            }],
            "nearest_area": [{
                "areaName": [{"value": "Fourviere "}],
                "country": [{"value": "France"}],
                "region": [{"value": "Rhone-Alpes"}],
            }],
            "weather": [
                { "date": "2026-11-25", "mintempC": "7", "maxtempC": "10", "hourly": [
                    h(0, "8", 266, 71, true), h(300, "8", 296, 77, true),
                    h(900, "8", 176, 31, false), h(1200, "9", 113, 12, false),
                    h(1800, "9", 122, 27, true), h(2100, "7", 122, 30, true),
                ]},
                { "date": "2026-11-26", "mintempC": "-0", "maxtempC": "7", "hourly": [
                    h(0, "7", 122, 31, true), h(300, "6", 176, 43, true),
                    h(1200, "6", 176, 28, false),
                ]},
            ],
        })
    }

    #[test]
    fn bulletin() {
        let w = parse(&reponse().to_string()).unwrap();
        assert_eq!(w.place, "Fourviere, France");
        let n = &w.now;
        assert_eq!((n.temp, n.feels), (7, 6));
        assert!(matches!(n.kind, Kind::Overcast) && n.night);
        assert_eq!((n.wind, n.wind_dir.as_str(), n.humidity), (7, "NNW", 91));
        assert_eq!(w.days.len(), 2);
        let d = &w.days[0];
        assert_eq!((d.date.as_str(), d.min, d.max), ("2026-11-25", 7, 10));
        // Temps du jour : celui de midi ; pluie : la plus forte de la journée.
        assert!(matches!(d.kind, Kind::Clear));
        assert_eq!(d.rain, 77);
        assert_eq!(d.hours.len(), 6);
        let first = &d.hours[0];
        assert_eq!(
            (first.hour, first.temp, first.night, first.rain),
            (0, 8, true, 71)
        );
        assert!(matches!(first.kind, Kind::Drizzle) && !d.hours[2].night);
        // Sans midi : un relevé de la journée.
        assert!(matches!(w.days[1].kind, Kind::Showers));
        assert_eq!((w.days[1].min, w.days[1].rain), (0, 43));
    }

    #[test]
    fn bulletin_d_apercu() {
        // De quoi remplir la tuile de l'aperçu : toutes ses colonnes.
        let w = sample();
        assert_eq!((w.days.len(), w.next_days().len()), (3, 2));
        assert!(w.days.iter().all(|d| d.hours.len() == 8 && d.min < d.max));
        assert!(w.hours_from(0).len() >= 8);
        assert!(!w.now.night && !w.place.is_empty());
    }

    /// Lecture réelle du service (réseau) : `cargo test --lib -- --ignored meteo`.
    #[test]
    #[ignore = "appelle wttr.in (réseau)"]
    fn lecture_du_service() {
        let w = fetch("Lyon").unwrap();
        assert!(!w.place.is_empty(), "{}", w.place);
        assert!((-90..=60).contains(&w.now.temp), "{}", w.now.temp);
        assert!(w.now.humidity <= 100);
        assert!(matches!(
            w.now.wind_dir.as_str(),
            "N" | "NNE"
                | "NE"
                | "ENE"
                | "E"
                | "ESE"
                | "SE"
                | "SSE"
                | "S"
                | "SSW"
                | "SW"
                | "WSW"
                | "W"
                | "WNW"
                | "NW"
                | "NNW"
        ));
        // Trois jours au moins, des relevés toutes les trois heures, sur place.
        assert!(w.days.len() >= 3, "{}", w.days.len());
        for d in &w.days {
            assert!(!d.hours.is_empty() && d.min <= d.max, "{:?}", d.date);
            for h in &d.hours {
                assert!(h.hour < 24 && h.hour % 3 == 0, "{}", h.hour);
            }
        }
        // Le relevé en cours existe à toute heure du jour.
        assert!(!w.hours_from(0).is_empty() && !w.hours_from(1439).is_empty());
        // Pour l'œil, à l'exécution manuelle (`--nocapture`).
        let n = &w.now;
        eprintln!(
            "{} : {}° {}, {} km/h {}, {} %",
            w.place,
            n.temp,
            n.kind.label(n.night),
            n.wind,
            wind_dir(&n.wind_dir, Lang::Fr),
            n.humidity
        );
        for d in &w.days {
            let hours: Vec<String> = d
                .hours
                .iter()
                .map(|h| {
                    format!(
                        "{}h {}° {}{}",
                        h.hour,
                        h.temp,
                        h.kind.label(h.night),
                        if h.night { " (nuit)" } else { "" }
                    )
                })
                .collect();
            eprintln!(
                "  {} : {}°..{}° {}, pluie {} % — {}",
                d.date,
                d.min,
                d.max,
                d.kind.label(false),
                d.rain,
                hours.join(" | ")
            );
        }
        // Accents et tiret dans une ville ; sans ville : la position de
        // l'adresse IP ; un lieu inconnu : une erreur, pas un bulletin.
        let e = fetch("Saint-Étienne").unwrap();
        assert!(e.place.contains("tienne"), "{}", e.place);
        fetch("").unwrap();
        let err = fetch("Qzxvjkwnope").unwrap_err();
        assert!(!err.is_empty(), "lieu inconnu lu comme un bulletin");
    }

    /// Les vrais noms d'icônes du service, relevés sur dix lieux : `_night`
    /// la nuit — sauf pour les éclaircies, qui y perdent leur soleil.
    #[test]
    fn nuit_selon_l_icone_du_service() {
        let at = |time: u16, code: u16, icon: &str| {
            night(&json!({
                "time": time.to_string(),
                "weatherCode": code.to_string(),
                "weatherIconUrl": [{"value": format!("https://cdn.example/{icon}.png")}],
            }))
        };
        // Dégagé.
        assert!(!at(1200, 113, "wsymbol_0001_sunny"));
        assert!(at(0, 113, "wsymbol_0008_clear_sky_night"));
        // Éclaircies : la nuit, l'icône est neutre (pas de `_night`).
        assert!(!at(1200, 116, "wsymbol_0002_sunny_intervals"));
        assert!(at(2100, 116, "wsymbol_0004_black_low_cloud"));
        // Couvert : neutre, et sans importance (ni soleil ni lune au glyphe).
        assert!(!at(2100, 122, "wsymbol_0004_black_low_cloud"));
        // Pluie : le jour et la nuit ont chacun leur icône.
        assert!(!at(1200, 296, "wsymbol_0017_cloudy_with_light_rain"));
        assert!(at(300, 296, "wsymbol_0033_cloudy_with_light_rain_night"));
        // Sans icône : le jour.
        assert!(!night(&json!({ "weatherCode": "116" })));
    }

    #[test]
    fn familles_du_code() {
        for (code, kind) in [
            (113, Kind::Clear),
            (116, Kind::FewClouds),
            (119, Kind::Cloudy),
            (176, Kind::Showers),
            (230, Kind::Blizzard),
            (282, Kind::FreezingRain),
            (305, Kind::HeavyRain),
            (317, Kind::Sleet),
            (335, Kind::HeavySnow),
            (350, Kind::Hail),
            (389, Kind::Storm),
        ] {
            assert_eq!(Kind::of(code), kind, "{code}");
        }
        // Un code hors liste (149, vu dans une réponse) : temps couvert.
        assert_eq!(Kind::of(149), Kind::Overcast);
    }

    #[test]
    fn libelles_et_glyphes() {
        let all = [
            Kind::Clear,
            Kind::FewClouds,
            Kind::Cloudy,
            Kind::Overcast,
            Kind::Fog,
            Kind::Drizzle,
            Kind::Rain,
            Kind::HeavyRain,
            Kind::Showers,
            Kind::FreezingRain,
            Kind::Sleet,
            Kind::Snow,
            Kind::HeavySnow,
            Kind::Blizzard,
            Kind::Storm,
            Kind::Hail,
        ];
        for k in all {
            for night in [false, true] {
                assert!(
                    crate::i18n::has_english(k.label(night)),
                    "{} : sans traduction",
                    k.label(night)
                );
                assert!(
                    crate::ui::icons::get(k.icon(night)).is_some(),
                    "{} : sans glyphe",
                    k.icon(night)
                );
            }
        }
        assert_eq!(Kind::Clear.icon(true), "weather-clear-night");
        assert_eq!(Kind::Showers.icon(true), "weather-rain");
        assert_ne!(Kind::Clear.label(true), Kind::Clear.label(false));
        // La pluie et le soleil ne se lisent pas pareil.
        assert_ne!(Kind::Rain.icon(false), Kind::Clear.icon(false));
    }

    #[test]
    fn heures_a_venir() {
        let w = parse(&reponse().to_string()).unwrap();
        // 23 h 52 : le relevé de 21 h, puis le lendemain depuis minuit.
        let times = |minute: u16| {
            w.hours_from(minute)
                .iter()
                .map(|h| h.hour)
                .collect::<Vec<u8>>()
        };
        // 23 h 52 : le relevé de 21 h, puis le lendemain depuis minuit.
        assert_eq!(times(23 * 60 + 52), [21, 0, 3, 12]);
        // 10 h 05 : depuis le relevé de 9 h.
        assert_eq!(times(10 * 60 + 5), [9, 12, 18, 21, 0, 3, 12]);
        // Minuit : toute la journée, puis le lendemain.
        assert_eq!(times(0), [0, 3, 9, 12, 18, 21, 0, 3, 12]);
        // Une valeur au-delà du jour vaut sa dernière minute.
        assert_eq!(times(5000), times(1439));
        assert_eq!(w.next_days().len(), 1);
        assert_eq!(w.days[1].date, "2026-11-26");
    }

    #[test]
    fn reponses_inattendues() {
        assert!(parse("location not found: upstream error").is_err());
        assert!(parse("{}").is_err());
        let mut v = reponse();
        v["current_condition"][0]["temp_C"] = json!("");
        assert!(parse(&v.to_string()).is_err());
        v["current_condition"][0]["temp_C"] = json!("7");
        v["weather"] = json!([]);
        assert!(parse(&v.to_string()).is_err());
    }

    #[test]
    fn adresse_du_bulletin() {
        assert_eq!(url(""), "https://wttr.in/?format=j1");
        assert_eq!(url(" Lyon "), "https://wttr.in/Lyon?format=j1");
        assert_eq!(
            url("Aix-en-Provence"),
            "https://wttr.in/Aix-en-Provence?format=j1"
        );
        assert_eq!(
            url("Saint-Étienne"),
            "https://wttr.in/Saint-%C3%89tienne?format=j1"
        );
        assert_eq!(url("New York"), "https://wttr.in/New%20York?format=j1");
        // Virgule et barre oblique : encodées, sans découper l'adresse.
        assert_eq!(url("45.76,4.83"), "https://wttr.in/45.76%2C4.83?format=j1");
        assert_eq!(url("../admin"), "https://wttr.in/..%2Fadmin?format=j1");
    }

    #[test]
    fn composition() {
        let shown = |w: f32, h: f32, s: f32| {
            let p = plan(w, h, s);
            (p.beside, p.line, p.hours, p.days)
        };
        // Bandeau 2560×720 : tout, et toutes les colonnes.
        let p = plan(666.0, 666.0, 1.0);
        assert_eq!((p.beside, p.line, p.hours, p.days), (true, true, 8, 4));
        assert!((p.icon - 199.8).abs() < 0.01 && (p.small - 39.96).abs() < 0.01);
        assert_eq!(p.temp, 124.0);
        // Bandeau 1280×400 : sans la ligne sous l'en-tête, moins de colonnes.
        assert_eq!(shown(346.0, 346.0, 1.0), (true, false, 5, 3));
        // Zone d'accueil, quatre modules : l'en-tête, les jours et la ligne
        // (une ligne, elle tient) ; la rangée des heures, elle, ne vient pas.
        assert_eq!(shown(604.0, 324.0, 1.0), (true, true, 0, 4));
        // Tuile minuscule : l'en-tête seul, à tailles plancher.
        let p = plan(120.0, 100.0, 1.0);
        assert_eq!(
            (p.icon, p.small, p.beside, p.line, p.hours, p.days),
            (40.0, 22.0, false, false, 0, 0)
        );
        // Texte plus grand : moins de colonnes, la ligne tient encore.
        assert_eq!(shown(666.0, 666.0, 1.6), (true, true, 6, 4));
        // Large mais bas : les colonnes suivent la largeur, pas la hauteur.
        assert_eq!(shown(1200.0, 346.0, 1.0), (true, false, 8, 4));
        // Étroit : une colonne au moins, à chaque rangée qui vient.
        assert_eq!(shown(150.0, 666.0, 1.0), (false, true, 1, 1));
        // Tailles et échelle nulles : rien ne divise par zéro.
        let p = plan(0.0, 0.0, 0.0);
        assert_eq!(
            (p.icon, p.small, p.beside, p.hours, p.days),
            (40.0, 22.0, false, 0, 0)
        );
    }

    /// Les tailles des écrans et des zones du Panorama, d'étroites en larges
    /// : (largeur, hauteur) → lieu à droite, ligne sous l'en-tête, heures, jours.
    #[test]
    fn composition_des_ecrans() {
        let shown = |w: f32, h: f32| {
            let p = plan(w, h, 1.0);
            (p.beside, p.line, p.hours, p.days)
        };
        for ((w, h), want) in [
            // Écran bandeau 2560×720 à l'échelle 1,6 (1600×450).
            ((384.0, 384.0), (true, true, 5, 4)),
            // Bande 1280×400.
            ((332.0, 332.0), (true, false, 4, 3)),
            // Bande 1000×320 : le lieu n'a plus la place à droite, il descend.
            ((252.0, 252.0), (false, true, 0, 2)),
            // Tuile de 146 px : l'en-tête et le lieu.
            ((146.0, 146.0), (false, true, 0, 0)),
        ] {
            assert_eq!(shown(w, h), want, "{w}×{h}");
        }
    }

    #[test]
    fn heure_en_clair() {
        assert_eq!(hour_label(3, Lang::Fr), "3 h");
        assert_eq!(hour_label(21, Lang::Fr), "21 h");
        assert_eq!(hour_label(0, Lang::En), "12 AM");
        assert_eq!(hour_label(3, Lang::En), "3 AM");
        assert_eq!(hour_label(12, Lang::En), "12 PM");
        assert_eq!(hour_label(21, Lang::En), "9 PM");
    }

    #[test]
    fn vent() {
        assert_eq!(wind_dir("NNW", Lang::Fr), "NNO");
        assert_eq!(wind_dir("W", Lang::Fr), "O");
        assert_eq!(wind_dir("SSW", Lang::Fr), "SSO");
        // En anglais, ce que rend le service.
        assert_eq!(wind_dir("NNW", Lang::En), "NNW");
        // Ceux qui s'écrivent pareil, et l'inconnu, restent tels quels.
        assert_eq!(wind_dir("ESE", Lang::Fr), "ESE");
        assert_eq!(wind_dir("?", Lang::Fr), "?");
    }
}
