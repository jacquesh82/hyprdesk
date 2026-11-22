//! Gabarits (§7 de la spécification) : placeholders `{{chemin|filtre|filtre}}`
//! et filtres builtin en liste close.
//!
//! Pas de contrôle de flux (ni `for`, ni `if`) : un pack qui veut du
//! conditionnel écrit un gabarit par cas. Cette restriction est ce qui garantit
//! « un pack = des données » : un gabarit ne peut ni exécuter du code, ni lire
//! l'environnement, ni toucher au système de fichiers.
//!
//! Chemins : `palette.<slot>`, `effects.<surface>.<opacity|blur>`,
//! `meta.<id|name|version|description>`, `variant`. Filtres : `alpha`,
//! `lighten`, `darken`, `saturate`, `hue`, `mix`, `contrast`, `pick`, `fmt`.

use crate::config::{Place, Rgba};

use super::color::{self, Fmt};
use super::resolve::Resolved;
use super::{Error, Result};

/// Chemin résolu d'un placeholder.
#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    Palette(String),
    Effect(String, String),
    Meta(String),
    Variant,
}

/// Filtre appliqué à la valeur d'un chemin, dans l'ordre de la chaîne.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Alpha(f64),
    Lighten(f64),
    Darken(f64),
    Saturate(f64),
    Hue(f64),
    /// Mélange vers un autre slot (`palette.bg`) ou une couleur littérale.
    Mix(String, f64),
    /// Ajuste la luminosité jusqu'au rapport WCAG `min` contre le fond.
    Contrast(f64),
    /// Valeur selon la variante : `pick(si_sombre, si_clair)`.
    Pick(String, String),
    /// Format de sortie de la couleur (surcharge le défaut de la cible).
    Fmt(Fmt),
}

/// Contexte de rendu : palette résolue et format par défaut de la cible.
pub struct Context<'a> {
    pub resolved: &'a Resolved,
    pub default_fmt: Fmt,
}

/// Valeur d'un chemin : couleur, texte, nombre ou booléen.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Color(Rgba),
    Text(String),
    Num(f64),
    Bool(bool),
}

/// Rend un gabarit : chaque `{{…}}` est remplacé, tout le reste passe tel quel.
pub fn render(text: &str, ctx: &Context) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| Error::Template("placeholder non fermé".to_string()))?;
        let inner = &after[..end];
        let parsed = parse_placeholder(inner)?;
        let (val, fmt) = eval(&parsed, ctx)?;
        out.push_str(&finish(val, fmt.unwrap_or(ctx.default_fmt)));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Placeholder analysé : chemin + filtres.
#[derive(Debug, Clone, PartialEq)]
struct Placeholder {
    path: Path,
    filters: Vec<Filter>,
}

fn parse_placeholder(inner: &str) -> Result<Placeholder> {
    let mut parts = inner.split('|').map(str::trim);
    let path_part = parts
        .next()
        .ok_or_else(|| Error::Template("placeholder vide".to_string()))?;
    let path = parse_path(path_part)?;
    let mut filters = Vec::new();
    for f in parts {
        if f.is_empty() {
            return Err(Error::Template(format!("filtre vide dans « {inner} »")));
        }
        filters.push(parse_filter(f)?);
    }
    Ok(Placeholder { path, filters })
}

/// `palette.accent`, `effects.bar.opacity`, `meta.name`, `variant`.
fn parse_path(path: &str) -> Result<Path> {
    let segments: Vec<&str> = path.split('.').map(str::trim).collect();
    match segments.as_slice() {
        ["palette", slot] => Ok(Path::Palette((*slot).to_string())),
        ["effects", surface, key] => Ok(Path::Effect((*surface).to_string(), (*key).to_string())),
        ["meta", field] => Ok(Path::Meta((*field).to_string())),
        ["variant"] => Ok(Path::Variant),
        _ => Err(Error::Template(format!(
            "chemin inconnu « {path} » (attendu : palette.<slot>, effects.<surface>.<clé>, meta.<champ>, variant)"
        ))),
    }
}

fn parse_filter(f: &str) -> Result<Filter> {
    let (name, args) = match f.split_once('(') {
        Some((n, rest)) => {
            let inner = rest.strip_suffix(')').ok_or_else(|| {
                Error::Template(format!("filtre « {f} » : parenthèse non fermée"))
            })?;
            (n.trim(), inner)
        }
        None => (f.trim(), ""),
    };
    let number = |arg: &str, suffixes: &[&str]| -> Result<f64> {
        let s = arg.trim();
        let stripped = suffixes
            .iter()
            .find_map(|sfx| s.strip_suffix(sfx))
            .unwrap_or(s);
        stripped
            .trim()
            .parse::<f64>()
            .map_err(|_| Error::Template(format!("filtre « {name} » : nombre attendu, « {arg} »")))
    };
    let mut parts = args.split(',');
    let mut next = || parts.next().unwrap_or("").trim().to_string();
    let arg1 = next();
    let arg2 = next();
    match name {
        "alpha" => Ok(Filter::Alpha(number(&arg1, &[])?)),
        "lighten" => Ok(Filter::Lighten(number(&arg1, &["%"])?)),
        "darken" => Ok(Filter::Darken(number(&arg1, &["%"])?)),
        "saturate" => Ok(Filter::Saturate(number(&arg1, &["%"])?)),
        "mix" => {
            // `mix(palette.bg 20%)` (forme à espace) ou `mix(palette.bg, 20%)`.
            let (target, pct_str) = if arg2.is_empty() {
                let (a, b) = arg1.split_once(' ').ok_or_else(|| {
                    Error::Template("mix : « slot pourcentage » attendu".to_string())
                })?;
                (a.trim(), b.trim())
            } else {
                (arg1.as_str(), arg2.as_str())
            };
            let pct = number(pct_str, &["%"])?;
            Ok(Filter::Mix(target.to_string(), pct / 100.0))
        }
        "contrast" => Ok(Filter::Contrast(number(&arg1, &[])?)),
        "pick" => Ok(Filter::Pick(arg1, arg2)),
        "fmt" => Fmt::parse(&arg1)
            .map(Filter::Fmt)
            .ok_or_else(|| Error::Template(format!("fmt : format inconnu « {arg1} »"))),
        "hue" | "rotate_hue" => Ok(Filter::Hue(number(&arg1, &["deg"])?)),
        _ => Err(Error::Template(format!("filtre inconnu « {name} »"))),
    }
}

/// Résout le chemin puis applique les filtres ; rend la valeur et le format
/// demandé (`fmt`, sinon celui de la cible).
fn eval(p: &Placeholder, ctx: &Context) -> Result<(Val, Option<Fmt>)> {
    let mut val = resolve_path(&p.path, ctx)?;
    let mut fmt = None;
    for filter in &p.filters {
        match filter {
            Filter::Fmt(f) => fmt = Some(*f),
            Filter::Pick(dark, light) => {
                val = Val::Text(
                    if ctx.resolved.variant == "dark" {
                        dark
                    } else {
                        light
                    }
                    .clone(),
                );
            }
            _ => {
                let Val::Color(c) = val else {
                    return Err(Error::Template(format!(
                        "filtre « {} » attend une couleur, « {} » n'en rend pas",
                        filter_name(filter),
                        describe(&p.path)
                    )));
                };
                val = Val::Color(apply_color_filter(filter, c, ctx)?);
            }
        }
    }
    Ok((val, fmt))
}

fn filter_name(f: &Filter) -> &'static str {
    match f {
        Filter::Alpha(_) => "alpha",
        Filter::Lighten(_) => "lighten",
        Filter::Darken(_) => "darken",
        Filter::Saturate(_) => "saturate",
        Filter::Hue(_) => "hue",
        Filter::Mix(..) => "mix",
        Filter::Contrast(_) => "contrast",
        Filter::Pick(..) => "pick",
        Filter::Fmt(_) => "fmt",
    }
}

fn describe(p: &Path) -> String {
    match p {
        Path::Palette(s) => format!("palette.{s}"),
        Path::Effect(s, k) => format!("effects.{s}.{k}"),
        Path::Meta(m) => format!("meta.{m}"),
        Path::Variant => "variant".to_string(),
    }
}

/// `mix` : slot `palette.x` ou couleur littérale.
fn mix_target(arg: &str, ctx: &Context) -> Result<Rgba> {
    if let Some(slot) = arg.strip_prefix("palette.") {
        return ctx
            .resolved
            .palette
            .get(slot.trim())
            .copied()
            .ok_or_else(|| Error::Template(format!("mix : slot inconnu « {arg} »")));
    }
    color::parse(arg.trim())
        .ok_or_else(|| Error::Template(format!("mix : couleur attendue, « {arg} »")))
}

fn apply_color_filter(f: &Filter, c: Rgba, ctx: &Context) -> Result<Rgba> {
    Ok(match f {
        Filter::Alpha(a) => color::with_alpha(c, *a),
        Filter::Lighten(p) => color::lighten(c, *p),
        Filter::Darken(p) => color::darken(c, *p),
        Filter::Saturate(p) => color::saturate(c, *p),
        Filter::Hue(d) => color::rotate_hue(c, *d),
        Filter::Mix(target, t) => color::mix(c, mix_target(target, ctx)?, *t),
        Filter::Contrast(min) => color::ensure_contrast(c, ctx.resolved.palette["bg"], *min),
        // Traité plus haut : pick ne requiert pas de couleur.
        Filter::Pick(..) | Filter::Fmt(_) => c,
    })
}

fn resolve_path(path: &Path, ctx: &Context) -> Result<Val> {
    Ok(match path {
        Path::Palette(slot) => Val::Color(
            ctx.resolved
                .palette
                .get(slot)
                .copied()
                .ok_or_else(|| Error::Template(format!("slot inconnu « {slot} »")))?,
        ),
        Path::Effect(surface, key) => {
            let place = Place::parse(surface).ok_or_else(|| {
                Error::Template(format!("surface d'effet inconnue « {surface} »"))
            })?;
            let effect =
                ctx.resolved.effects.get(&place).ok_or_else(|| {
                    Error::Template(format!("surface d'effet absente « {surface} »"))
                })?;
            match key.as_str() {
                "opacity" => Val::Num((f64::from(effect.opacity) * 10_000.0).round() / 10_000.0),
                "blur" => Val::Bool(effect.blur),
                _ => {
                    return Err(Error::Template(format!(
                        "clé d'effet inconnue « {key} » (attendu : opacity ou blur)"
                    )));
                }
            }
        }
        Path::Meta(field) => {
            let meta = &ctx.resolved;
            let text = match field.as_str() {
                "id" => meta.id.clone(),
                "name" => meta.name.clone(),
                "version" => meta.version.clone(),
                _ => {
                    return Err(Error::Template(format!(
                        "champ meta inconnu « {field} » (attendu : id, name, version)"
                    )));
                }
            };
            Val::Text(text)
        }
        Path::Variant => Val::Text(ctx.resolved.variant.clone()),
    })
}

/// Valeur finale en texte, dans le format demandé pour les couleurs.
fn finish(val: Val, fmt: Fmt) -> String {
    match val {
        Val::Color(c) => color::fmt(c, fmt),
        Val::Text(s) => s,
        Val::Num(n) => {
            if n == n.trunc() && n.abs() < 1e15 {
                format!("{}", n as i64)
            } else {
                format!("{n}")
            }
        }
        Val::Bool(b) => if b { "true" } else { "false" }.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::resolve;
    use super::*;

    /// Palette ardoise posée à la main : pas de dépendance à un pack.
    fn ardoise() -> resolve::Resolved {
        let mut palette = std::collections::BTreeMap::new();
        for (k, v) in [
            ("bg", "#171b27"),
            ("fg", "#f2f4fc"),
            ("accent", "#a69cff"),
            ("background", "#131724"),
        ] {
            palette.insert(k.to_string(), color::parse(v).unwrap());
        }
        resolve::from_adjusted(
            palette["fg"],
            palette["bg"],
            palette["accent"],
            palette["background"],
        )
    }

    fn ctx(r: &resolve::Resolved) -> Context<'_> {
        Context {
            resolved: r,
            default_fmt: Fmt::Hex,
        }
    }

    #[test]
    fn texte_sans_placeholder_intact() {
        let r = ardoise();
        let ctx = ctx(&r);
        assert_eq!(
            render("rien à remplacer", &ctx).unwrap(),
            "rien à remplacer"
        );
        // Accolades simples (CSS) ne sont pas des placeholders.
        assert_eq!(render("a { b }", &ctx).unwrap(), "a { b }");
    }

    #[test]
    fn couleur_au_format_defaut() {
        let r = ardoise();
        let ctx = ctx(&r);
        assert_eq!(render("{{palette.accent}}", &ctx).unwrap(), "#a69cff");
    }

    #[test]
    fn filtres_en_chaines() {
        let r = ardoise();
        let ctx = ctx(&r);
        let out = render("{{palette.accent|alpha(0.4)|fmt(hypr)}}", &ctx).unwrap();
        assert_eq!(out, "rgba(a69cff66)", "{out}");
        let out = render("{{palette.accent|lighten(50%)|fmt(rgb)}}", &ctx).unwrap();
        assert!(out.starts_with("rgb("), "{out}");
    }

    #[test]
    fn mix_vers_slot_ou_litteral() {
        let r = ardoise();
        let ctx = ctx(&r);
        let out = render("{{palette.bg|mix(palette.fg 100%)}}", &ctx).unwrap();
        assert_eq!(out, "#f2f4fc");
        let out = render("{{palette.bg|mix(#000000 100%)}}", &ctx).unwrap();
        assert_eq!(out, "#000000");
    }

    #[test]
    fn pick_selon_la_variante() {
        let mut r = ardoise();
        r.variant = "dark".to_string();
        assert_eq!(
            render("{{variant|pick(sombre, clair)}}", &ctx(&r)).unwrap(),
            "sombre"
        );
        r.variant = "light".to_string();
        assert_eq!(
            render("{{variant|pick(sombre, clair)}}", &ctx(&r)).unwrap(),
            "clair"
        );
    }

    #[test]
    fn effets_meta_variant() {
        let r = ardoise();
        let ctx = ctx(&r);
        let out = render(
            "{{effects.bar.opacity}} {{effects.settings.blur}} {{meta.name}} ({{variant}})",
            &ctx,
        )
        .unwrap();
        assert!(out.contains("0.72") && out.contains("true"), "{out}");
        assert!(
            out.contains("Thème réglé") && out.contains("(dark)"),
            "{out}"
        );
    }

    #[test]
    fn erreurs_explicites() {
        let r = ardoise();
        let ctx = ctx(&r);
        assert!(render("{{palette.inconnu}}", &ctx).is_err());
        assert!(render("{{palette.accent|inconnu(1)}}", &ctx).is_err());
        assert!(render("{{palette.accent", &ctx).is_err());
        assert!(render("{{effects.rien.opacity}}", &ctx).is_err());
        // Filtre couleur sur une valeur texte : refusé.
        assert!(render("{{variant|lighten(10%)}}", &ctx).is_err());
    }

    #[test]
    fn gabarit_kitty_complet() {
        let r = ardoise();
        let ctx = ctx(&r);
        let out = render(include_str!("templates/kitty.conf.tmpl"), &ctx).unwrap();
        assert!(out.contains("foreground               #f2f4fc"), "{out}");
        assert!(out.contains("background               #171b27"), "{out}");
        assert!(out.contains("selection_background     #a69cff"), "{out}");
        assert_eq!(out.lines().filter(|l| l.starts_with("color")).count(), 4);
    }

    #[test]
    fn gabarits_gtk_et_thunar_en_css_valide() {
        let r = ardoise();
        let ctx = Context {
            resolved: &r,
            default_fmt: Fmt::Rgb,
        };
        let gtk = render(include_str!("templates/gtk.css.tmpl"), &ctx).unwrap();
        assert!(gtk.contains("@define-color hyprdesk_accent rgb("), "{gtk}");
        assert!(gtk.contains("@define-color window_bg_color"), "{gtk}");
        let thunar = render(include_str!("templates/thunar.css.tmpl"), &ctx).unwrap();
        // Chaque règle reste limitée à .thunar : rien d'autre ne change.
        for line in thunar
            .lines()
            .filter(|l| l.contains('{') || l.trim_end().ends_with(','))
        {
            for sel in line.trim_end_matches(['{', ' ', ',']).split(',') {
                assert!(
                    sel.trim().starts_with(".thunar")
                        || sel.trim().starts_with("background-color")
                        || sel.trim().starts_with("color")
                        || sel.trim().starts_with("caret-color"),
                    "{line}"
                );
            }
        }
    }

    #[test]
    fn gabarits_hyprland_et_hyprlock() {
        let r = ardoise();
        let ctx = Context {
            resolved: &r,
            default_fmt: Fmt::Hypr,
        };
        let hypr = render(include_str!("templates/hyprland.conf.tmpl"), &ctx).unwrap();
        assert!(hypr.contains("general:col.active_border = rgba("), "{hypr}");
        assert!(!hypr.contains("rgba(rgba("), "{hypr}");
        let lock = render(include_str!("templates/hyprlock.conf.tmpl"), &ctx).unwrap();
        assert!(lock.contains("$hd_fg = rgba("), "{lock}");
    }
}
