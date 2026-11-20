//! Coût du rendu de Hyprland (paquet « Performance ») : les options de
//! Hyprland qui sollicitent le processeur et la carte graphique, et trois
//! profils qui les posent d'un coup.
//!
//! Relevé sur Hyprland 0.57.2 : wiki 0.55 (Variables, Performance ; le wiki
//! 0.57 n'est pas publié), chaque option vérifiée par `hl.get_config` dans la
//! session (`misc.vfr` n'existe plus : c'est `debug.vfr`). Écartées :
//! `debug.damage_tracking` (« ne pas toucher »), `render.cm_enabled` et
//! `render.commit_timing_enabled` (redémarrage de Hyprland), les options
//! propres à NVIDIA, et le VRR, réglé par écran (paquet Écrans).
//!
//! Les options sont celles de `crate::fx` : un profil écrit dans les mêmes
//! réglages que le paquet Animations, qui affiche donc « réglé ici » pour
//! chacune. Le profil en place n'est pas enregistré : il se lit dans ces
//! réglages (tous ceux du profil, et rien d'autre parmi les options d'ici).

use std::collections::BTreeMap;

use crate::tiling::OptValue;

/// Ce qu'une option coûte quand elle est active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    /// Carte graphique : à chaque image, pour chaque fenêtre concernée.
    Gpu,
    /// Images supplémentaires : processeur et carte graphique.
    Frames,
    /// Réglage qui économise (le couper coûte).
    Saves,
}

/// Option suivie par le paquet : chemin dans `crate::fx::OPTIONS`, coût.
pub struct Item {
    pub path: &'static str,
    pub cost: Cost,
}

const fn item(path: &'static str, cost: Cost) -> Item {
    Item { path, cost }
}

/// Options du paquet, par groupe d'affichage.
pub const GROUPS: &[(&str, &[Item])] = &[
    (
        "Effets de décoration",
        &[
            item("decoration.blur.enabled", Cost::Gpu),
            item("decoration.blur.passes", Cost::Gpu),
            item("decoration.blur.xray", Cost::Saves),
            item("decoration.blur.new_optimizations", Cost::Saves),
            item("decoration.blur.popups", Cost::Gpu),
            item("decoration.blur.special", Cost::Gpu),
            item("decoration.shadow.enabled", Cost::Gpu),
            item("decoration.glow.enabled", Cost::Gpu),
        ],
    ),
    (
        "Mouvement",
        &[
            item("animations.enabled", Cost::Frames),
            item("decoration.motion_blur.enabled", Cost::Gpu),
            item("decoration.wobble.enabled", Cost::Gpu),
            item("misc.animate_manual_resizes", Cost::Frames),
            item("misc.animate_mouse_windowdragging", Cost::Frames),
        ],
    ),
    (
        "Rendu",
        &[
            item("debug.vfr", Cost::Saves),
            item("render.direct_scanout", Cost::Saves),
            item("render.new_render_scheduling", Cost::Saves),
        ],
    ),
];

/// Toutes les options du paquet.
pub fn items() -> impl Iterator<Item = &'static Item> {
    GROUPS.iter().flat_map(|(_, list)| list.iter())
}

/// Profil de priorité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Performance,
    Balanced,
    Quality,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Performance, Profile::Balanced, Profile::Quality];

    pub fn id(self) -> &'static str {
        match self {
            Profile::Performance => "performance",
            Profile::Balanced => "balanced",
            Profile::Quality => "quality",
        }
    }

    pub fn from_id(s: &str) -> Option<Profile> {
        match s {
            "performance" | "perf" => Some(Profile::Performance),
            "equilibre" | "équilibré" | "balanced" => Some(Profile::Balanced),
            "rendu" | "rendu-graphique" | "quality" | "qualite" | "qualité" => {
                Some(Profile::Quality)
            }
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Profile::Performance => "Performance",
            Profile::Balanced => "Équilibré",
            Profile::Quality => "Rendu graphique",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Profile::Performance => {
                "Ni flou, ni ombre, ni animation : le moins de travail possible pour le \
                 processeur et la carte graphique. Pour une machine modeste ou sur batterie."
            }
            Profile::Balanced => {
                "Flou léger et ombres, animations gardées ; coupe les effets les plus \
                 coûteux (flou des menus, flou de mouvement, ondulation)."
            }
            Profile::Quality => {
                "Tous les effets, flou plus fin et étendu aux menus. Les effets de style \
                 (lueur, ondulation, flou de mouvement) restent ceux de hyprland.lua."
            }
        }
    }

    /// Valeur posée pour une option (`None` : celle de hyprland.lua).
    pub fn value(self, path: &str) -> Option<OptValue> {
        use OptValue::{Bool, Num};
        use Profile::*;
        let v = match (path, self) {
            ("animations.enabled", Performance) => Bool(false),
            ("animations.enabled", _) => Bool(true),
            ("decoration.blur.enabled", Performance) => Bool(false),
            ("decoration.blur.enabled", _) => Bool(true),
            ("decoration.blur.passes", Balanced) => Num(2.0),
            ("decoration.blur.passes", Quality) => Num(3.0),
            ("decoration.blur.xray", Quality) => Bool(false),
            ("decoration.blur.xray", _) => Bool(true),
            ("decoration.blur.new_optimizations", _) => Bool(true),
            ("decoration.blur.popups" | "decoration.blur.special", Quality) => Bool(true),
            ("decoration.blur.popups" | "decoration.blur.special", _) => Bool(false),
            ("decoration.shadow.enabled", Performance) => Bool(false),
            ("decoration.shadow.enabled", _) => Bool(true),
            ("decoration.glow.enabled", Performance) => Bool(false),
            ("decoration.motion_blur.enabled" | "decoration.wobble.enabled", Quality) => {
                return None;
            }
            ("decoration.motion_blur.enabled" | "decoration.wobble.enabled", _) => Bool(false),
            ("misc.animate_manual_resizes", Quality) => Bool(true),
            ("misc.animate_manual_resizes", _) => Bool(false),
            ("misc.animate_mouse_windowdragging", Quality) => return None,
            ("misc.animate_mouse_windowdragging", _) => Bool(false),
            ("debug.vfr", _) => Bool(true),
            ("render.direct_scanout", Performance) => Num(2.0),
            ("render.new_render_scheduling", Performance) => Bool(true),
            _ => return None,
        };
        Some(v)
    }

    /// Pose le profil dans les réglages de `crate::fx` : chaque option du
    /// paquet prend la valeur du profil, ou revient à hyprland.lua.
    pub fn apply(self, options: &mut BTreeMap<String, OptValue>) {
        for it in items() {
            match self.value(it.path) {
                Some(v) => {
                    options.insert(it.path.to_string(), v);
                }
                None => {
                    options.remove(it.path);
                }
            }
        }
    }
}

/// Profil en place : toutes les options du paquet comme il les pose.
pub fn active(options: &BTreeMap<String, OptValue>) -> Option<Profile> {
    Profile::ALL
        .into_iter()
        .find(|p| items().all(|it| options.get(it.path) == p.value(it.path).as_ref()))
}

/// Aucune option du paquet n'est réglée dans hyprdesk.
pub fn untouched(options: &BTreeMap<String, OptValue>) -> bool {
    items().all(|it| !options.contains_key(it.path))
}

/// Retire les réglages du paquet (retour à hyprland.lua).
pub fn clear(options: &mut BTreeMap<String, OptValue>) {
    for it in items() {
        options.remove(it.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_connues_de_fx() {
        for it in items() {
            let o = crate::fx::opt(it.path).unwrap_or_else(|| panic!("{}", it.path));
            for p in Profile::ALL {
                if let Some(v) = p.value(it.path) {
                    assert!(
                        o.parse(&v.display()).is_ok(),
                        "{} : {:?} hors bornes",
                        it.path,
                        v
                    );
                }
            }
        }
        let mut paths: Vec<&str> = items().map(|i| i.path).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), items().count());
    }

    #[test]
    fn profils_reconnus() {
        let mut o = BTreeMap::new();
        assert!(untouched(&o) && active(&o).is_none());
        // Une option hors du paquet ne gêne pas la lecture du profil.
        o.insert("decoration.rounding".into(), OptValue::Num(8.0));
        for p in Profile::ALL {
            p.apply(&mut o);
            assert_eq!(active(&o), Some(p), "{p:?}");
            assert_eq!(Profile::from_id(p.id()), Some(p));
        }
        // Une retouche : plus de profil.
        o.insert("decoration.shadow.enabled".into(), OptValue::Bool(false));
        assert!(active(&o).is_none() && !untouched(&o));
        clear(&mut o);
        assert!(untouched(&o));
        assert_eq!(o.get("decoration.rounding"), Some(&OptValue::Num(8.0)));
        // Changer de profil efface ce que le précédent posait seul.
        Profile::Performance.apply(&mut o);
        Profile::Quality.apply(&mut o);
        assert!(!o.contains_key("render.direct_scanout"));
        assert!(!o.contains_key("decoration.wobble.enabled"));
    }

    #[test]
    fn performance_coupe_le_couteux() {
        let p = Profile::Performance;
        for path in [
            "animations.enabled",
            "decoration.blur.enabled",
            "decoration.shadow.enabled",
            "decoration.motion_blur.enabled",
            "decoration.wobble.enabled",
        ] {
            assert_eq!(p.value(path), Some(OptValue::Bool(false)), "{path}");
        }
        assert_eq!(p.value("debug.vfr"), Some(OptValue::Bool(true)));
    }

    #[test]
    fn textes_traduits() {
        let labels = GROUPS
            .iter()
            .map(|(t, _)| *t)
            .chain(Profile::ALL.iter().flat_map(|p| [p.label(), p.summary()]));
        let missing = crate::i18n::untranslated("perf", labels);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
