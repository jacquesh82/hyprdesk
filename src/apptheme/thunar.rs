//! Cible « thunar » : une feuille GTK 3 (`~/.config/gtk-3.0/hyprdesk-thunar.css`
//! par défaut) dont toutes les règles sont limitées à la classe `.thunar` que
//! Thunar pose sur ses fenêtres : les autres applications GTK ne changent pas.
//! gtk.css l'importe (`@import`, en tête : GTK ignore un import placé après
//! d'autres règles). GTK ne relit pas gtk.css à chaud : le thème vaut pour les
//! fenêtres ouvertes ensuite (ou après `thunar -q`). Les trois se règlent
//! (fichier, feuille qui l'importe — vide : ne pas y toucher —, commande de
//! rechargement, vide par défaut : `thunar -q` fermerait les fenêtres).

use std::path::PathBuf;

use super::{
    Opt, Opts, Target, config_dir, hex, in_path, include_first, mix, opt, run_reload,
    write_if_changed,
};
use crate::config::Theme;

const OPTIONS: &[Opt] = &[
    Opt {
        key: "file",
        label: "Fichier de couleurs",
        help: "Écrit par hyprdesk à chaque changement de thème.",
        default: || {
            config_dir("gtk-3.0")
                .join("hyprdesk-thunar.css")
                .display()
                .to_string()
        },
    },
    Opt {
        key: "conf",
        label: "Feuille GTK qui l'importe",
        help: "hyprdesk y ajoute une fois la ligne @import, en tête. Vide : ne pas la modifier (à importer soi-même).",
        default: || config_dir("gtk-3.0").join("gtk.css").display().to_string(),
    },
    Opt {
        key: "reload",
        label: "Commande de rechargement",
        help: "GTK ne relit pas ses feuilles à chaud : vide, le thème vaut pour les prochaines fenêtres ; « thunar -q » ferme Thunar pour qu'il reparte à jour.",
        default: String::new,
    },
];

pub const TARGET: Target = Target {
    id: "thunar",
    label: "Thunar",
    summary: "Gestionnaire de fichiers : fond, texte, sélection, panneau latéral, barres, \
              champs. Seul Thunar change, pas les autres applications GTK.",
    installed,
    options: OPTIONS,
    file,
    files,
    apply,
    clear,
};

fn file(o: &Opts) -> PathBuf {
    let f = opt(o, "thunar", &OPTIONS[0]);
    PathBuf::from(if f.is_empty() {
        (OPTIONS[0].default)()
    } else {
        f
    })
}

fn conf_path(o: &Opts) -> Option<PathBuf> {
    Some(opt(o, "thunar", &OPTIONS[1]))
        .filter(|c| !c.is_empty())
        .map(PathBuf::from)
}

fn files(o: &Opts) -> Vec<PathBuf> {
    std::iter::once(file(o)).chain(conf_path(o)).collect()
}

fn installed() -> bool {
    in_path("thunar")
}

/// Feuille GTK de Thunar d'après le thème, limitée à `.thunar`.
pub fn css(t: &Theme) -> String {
    let (fg, bg, ac) = (t.fg, t.bg, t.accent);
    let side = hex(mix(bg, fg, 0.06));
    let bar = hex(mix(bg, fg, 0.10));
    let field = hex(mix(bg, fg, 0.14));
    let hover = hex(mix(bg, ac, 0.25));
    let dim = hex(mix(fg, bg, 0.35));
    let (fg, bg, ac) = (hex(fg), hex(bg), hex(ac));
    format!(
        "/* Généré par hyprdesk (Réglages → Thème des applications) — thème « {name} ».\n   \
         Réécrit à chaque changement de thème ; importé par gtk.css. Règles limitées\n   \
         à Thunar (.thunar) : les autres applications GTK ne changent pas. */\n\n\
         .thunar, .thunar .view, .thunar treeview.view, .thunar iconview, .thunar scrolledwindow {{\n  \
         background-color: {bg};\n  color: {fg};\n}}\n\n\
         .thunar .sidebar, .thunar .shortcuts-pane, .thunar placessidebar,\n\
         .thunar .sidebar .view, .thunar .shortcuts-pane treeview.view {{\n  \
         background-color: {side};\n  color: {fg};\n}}\n\n\
         .thunar .view:selected, .thunar treeview.view:selected, .thunar iconview:selected,\n\
         .thunar .sidebar .view:selected, .thunar row:selected {{\n  \
         background-color: {ac};\n  color: {bg};\n}}\n\n\
         .thunar headerbar, .thunar toolbar, .thunar menubar, .thunar .path-bar,\n\
         .thunar .location-bar, .thunar statusbar {{\n  \
         background-color: {bar};\n  color: {fg};\n}}\n\n\
         .thunar statusbar label {{\n  color: {dim};\n}}\n\n\
         .thunar entry {{\n  background-color: {field};\n  color: {fg};\n  caret-color: {ac};\n}}\n\n\
         .thunar button:hover, .thunar .path-bar button:hover {{\n  background-color: {hover};\n}}\n\n\
         .thunar .path-bar button:checked {{\n  background-color: {ac};\n  color: {bg};\n}}\n",
        name = t.name
    )
}

fn apply(t: &Theme, o: &Opts) -> Result<String, String> {
    let file = file(o);
    let changed = write_if_changed(&file, &css(t))?;
    let reload = opt(o, "thunar", &OPTIONS[2]);
    let Some(gtk) = conf_path(o) else {
        if changed {
            run_reload(&reload);
        }
        return Ok(crate::tr!(
            "Thunar : thème « {name} » écrit dans {f} ; à importer dans votre gtk.css ({line}).",
            name = t.name,
            f = file.display(),
            line = format!("@import url(\"{}\");", file.display())
        ));
    };
    // Chemin relatif si la feuille est à côté de gtk.css, sinon complet.
    let target = if file.parent() == gtk.parent() {
        file.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        file.display().to_string()
    };
    let line = format!("@import url(\"{target}\");");
    let included = include_first(
        &gtk,
        &line,
        &[&format!("@import url(\"{}\");", file.display())],
        "/* Thème de Thunar par hyprdesk (Réglages → Thème des applications). */",
    )?;
    if changed || included {
        run_reload(&reload);
    }
    Ok(if included {
        crate::tr!(
            "Thunar : thème « {name} » appliqué ; {c} l'importe désormais (copie : {c}.avant-hyprdesk.bak). Il vaut pour les prochaines fenêtres de Thunar.",
            name = t.name,
            c = gtk.display()
        )
    } else {
        crate::tr!(
            "Thunar : thème « {name} » appliqué (prochaines fenêtres de Thunar).",
            name = t.name
        )
    })
}

fn clear(o: &Opts) -> Result<(), String> {
    let file = file(o);
    if !file.is_file() {
        return Ok(());
    }
    // Feuille vidée, pas supprimée : l'import de gtk.css reste valide.
    let empty = "/* Thème de Thunar par hyprdesk coupé (Réglages → Thème des applications). */\n";
    if write_if_changed(&file, empty)? {
        run_reload(&opt(o, "thunar", &OPTIONS[2]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regles_limitees_a_thunar() {
        let c = css(&Theme::default());
        // Chaque sélecteur commence par .thunar : rien d'autre ne change.
        for l in c.lines().filter(|l| l.contains('{') || l.ends_with(',')) {
            for sel in l.trim_end_matches(['{', ' ', ',']).split(',') {
                assert!(
                    sel.trim().starts_with(".thunar"),
                    "sélecteur hors de Thunar : {sel}"
                );
            }
        }
        assert!(c.contains("#171b27") && c.contains("#a69cff"), "{c}");
    }

    #[test]
    fn import_en_tete_de_gtk_css() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-thunar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let gtk = dir.join("gtk.css");
        std::fs::write(&gtk, "@define-color accent_color #9580ff;\n").unwrap();
        let mut o = Opts::new();
        o.insert(
            "thunar.file".into(),
            dir.join("hyprdesk-thunar.css").display().to_string(),
        );
        o.insert("thunar.conf".into(), gtk.display().to_string());
        apply(&Theme::default(), &o).unwrap();
        let g = std::fs::read_to_string(&gtk).unwrap();
        let first_rule = g
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with("/*"))
            .unwrap();
        assert_eq!(first_rule, "@import url(\"hyprdesk-thunar.css\");", "{g}");
        assert!(g.contains("@define-color accent_color"));
        // Deuxième passage : pas de second import.
        apply(&Theme::default(), &o).unwrap();
        assert_eq!(
            std::fs::read_to_string(&gtk)
                .unwrap()
                .matches("@import")
                .count(),
            1
        );
        clear(&o).unwrap();
        assert!(
            !std::fs::read_to_string(dir.join("hyprdesk-thunar.css"))
                .unwrap()
                .contains(".thunar")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
