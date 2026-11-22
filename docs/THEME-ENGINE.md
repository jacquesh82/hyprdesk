# hyprdesk-theme-engine — spécification

Statut : spécification à implémenter (v1). Le binaire `hyprdesk-theme-engine` devient le moteur de thème unique de hyprdesk : découverte, résolution, génération et application des thèmes pour Hyprland, hyprdesk et les applications. Il remplace progressivement la logique dispersée de `src/theme.rs` (préréglages) et `src/apptheme/` (cibles kitty, Thunar) tout en conservant leurs contrats et leurs clés de réglage.

Références au code existant : `src/theme.rs`, `src/apptheme/mod.rs` (contrat `Target`), `src/colors.rs`, `src/share.rs`, `src/history.rs`, `src/doctor.rs`, `src/genfiles.rs`, `src/portal.rs`, `src/config/model.rs`.

---

## 1. But et périmètre

GTK dispose d'un moteur de thème : découverte dans des dossiers standards, `index.theme` avec héritage, variantes clair/sombre, commutation à chaud, distribution par des tiers, icônes, curseurs, polices, sons. hyprdesk-theme-engine vise la même complétude, avec des ajouts propres (OKLCH, contraste garanti, palette depuis le fond d'écran, effets par surface, transaction avec retour arrière).

### Objectifs

1. **Un binaire dédié** hors du démon : `hyprdesk-theme-engine`, conforme à la règle du dépôt « un binaire ne lie que ce qu'il atteint » — ni iced, ni mlua, ni l'index.
2. **Une bibliothèque réutilisable** : le cœur (`src/theme_engine/`) est appelé en cours de processus par le démon (Réglages → Thème) et en ligne de commande par le binaire.
3. **Des thèmes installables par des tiers** : un pack est un dossier de données (manifeste + gabarits), sans aucun code exécutable.
4. **Styles transmis aux applications tierces** : iced, GTK et Qt consomment le thème résolu sans connaître hyprdesk — palette publiée, crate cliente, portail freedesktop, schémas Qt (§9).
5. **LYRA au pupitre** : l'assistante pilote le moteur par une surface d'outils schématisés et confirmés — elle traduit le langage, le moteur fait les maths ; jamais d'écriture de fichier ni de shell (§17).
6. **Parité GTK puis dépassement** : tableau de parité en §18, ajouts en §19.
7. **Continuité avec l'existant** : les clés `theme.*`, `general.app_themes` et `plugin_options["apptheme"]` restent la surface de réglage ; le moteur en devient l'implémentation.

### Non-goals

- Aucun rendu d'interface dans le binaire (pas d'iced) ; l'aperçu visuel reste dans le démon (`hyprdesk preview`).
- Aucune édition de `hyprland.lua` à la main : le moteur écrit des fragments générés, le démon ou l'utilisateur les inclut.
- Aucun réseau en v1 : installation depuis un fichier ou un dossier local. Les dépôts signés sont en phase 3.
- Aucun code utilisateur dans un pack : pas de Lua, pas de shell, pas d'inclusions de fichiers arbitraires.

---

## 2. Vue d'ensemble

```
                     ┌─────────────────────────────┐
  pack(s) installés  │        RÉSOLUTION           │  réglages actifs
  + héritage         │  variante + héritage + slots │  (theme.* si aucun pack)
  + variantes        └──────────────┬──────────────┘
                                    ▼
                     ┌─────────────────────────────┐
  gabarits           │        GÉNÉRATION           │  filtres couleur OKLCH
  (builtin + pack)   │  placeholders → fichiers     │  contraste garanti
                                    ▼
                     ┌─────────────────────────────┐
                     │        COMMIT               │  staging → rename atomique
                     │  par cible, par fichier     │  copie de sauvegarde unique
                                    ▼
                     ┌─────────────────────────────┐
  apps cibles        │        RELOAD               │  par cible : signal, socket,
  (hyprland, kitty…) │                             │  gsettings, événement démon
                                    ▼
                     ┌─────────────────────────────┐
                     │        RAPPORT              │  JSON par cible, journal,
                     │                             │  Historique, rollback si échec
                     └─────────────────────────────┘
```

Cinq étapes, toutes journalisées. À chaque application réussie, la palette résolue est en outre publiée pour les applications tierces (§9). `apply --mode dry-run` exécute les deux premières et affiche le rapport sans rien écrire ; `apply --mode hot` applique Hyprland uniquement en `hyprctl keyword` (non persistant), comme un aperçu.

---

## 3. Le pack de thème

Un **pack** est un dossier :

```
<id>/
├── theme.toml          # manifeste (obligatoire, ≤ 64 Kio)
├── templates/          # gabarits optionnels, surchargent ceux du moteur
│   ├── kitty.conf.tmpl
│   └── waybar.css.tmpl
├── fonds/              # fonds d'écran du pack (optionnel)
│   └── mur.jpg
└── capture.png         # aperçu pour la galerie (optionnel)
```

Emplacements de découverte, par priorité décroissante :

1. `~/.local/share/hyprdesk/themes/<id>/`
2. `$XDG_DATA_DIRS/hyprdesk/themes/<id>/`
3. Packs embarqués (ressources `include_str!` du binaire, exposés par `list` avec `origin = "bundled"` ; installés dans le dossier utilisateur à la sélection, sans écraser une modification locale).

Deux packs avec le même `id` : l'utilisateur gagne. Un pack local incomplet (manifeste invalide) est signalé par `doctor` et ignoré de la découverte.

### Archive de distribution

Extension `.hyprtheme` : tar+gzip dont la racine contient le dossier du pack (manifeste à la racine). Limite 50 Mio, manifeste ≤ 64 Kio, gabarit ≤ 1 Mio, 2000 fichiers max. Somme de contrôle sha256 optionnelle adjacente (`<archive>.sha256`). Signature en phase 3 (minisign, clé hyprdesk).

---

## 4. Manifeste `theme.toml`

TOML 1.0, clés en anglais (distribution tierce), texte d'affichage libre. Champs inconnus : ignorés avec avertissement (compatibilité ascendante des packs).

```toml
[meta]
id = "ardoise"                    # requis, ^[a-z0-9-]{2,32}$
name = "Ardoise"                  # requis
version = "1.0.0"                 # requis
description = "Verre fumé, accent iris."
authors = ["Jacques Hullu"]
license = "MIT"
screenshot = "capture.png"
default_variant = "dark"          # "dark" | "light" (défaut "dark")
min_engine = "0.2.0"              # version de hyprdesk-theme-engine requise

[inherits]
themes = ["graphite-base"]        # ordre = priorité croissante (le dernier gagne)

# ─── Palette : slots sémantiques (voir §5) ───
[palette]
bg = "#171b27"
surface = "#171b27"
fg = "#f2f4fc"
accent = "#a69cff"
background = "#131724"

# ─── Variantes : surcharges de [palette], [effects], [hyprland.options] ───
[variant.dark]  # section optionnelle : valeurs par défaut du pack
[variant.light]
palette.fg = "#171b27"
palette.bg = "#f2f4fc"

# ─── Effets par surface (specifique hyprdesk) ───
[effects.bar]
opacity = 0.72
blur = true
[effects.menus]
opacity = 0.94
blur = true
[effects.launcher]
opacity = 0.88
blur = true
[effects.settings]
opacity = 0.97
blur = true
[effects.widgets]
opacity = 0.84
blur = true

# ─── Options Hyprland (liste blanche, voir §8 cible hyprland) ───
[hyprland.options]
"decoration:rounding" = "10"
"general:gaps_in" = "4"

[fonts]
family = "Mindlog"
mono_family = "Mindlog Mono"
size = 10.5

[icons]
theme = "Papirus"

[cursors]
theme = "Bibata-Modern-Ice"
size = 24

[sounds]
theme = "ocean"

[wallpaper]
dir = "fonds/"
mode = "static"                   # "static" | "slideshow"

# ─── Gabarits additionnels du pack (cible générique ou surcharge) ───
[targets.waybar]
template = "templates/waybar.css.tmpl"
output = "~/.config/waybar/hyprdesk.css"
reload = "signal:waybar:USR1"     # langage de rechargement borné, voir §8
enabled = true

[targets.kitty]                   # surcharge du gabarit intégré de la cible
template = "templates/kitty.conf.tmpl"
```

Champ `enabled` (défaut `true`) : une cible désactivée est ignorée, même si `general.app_themes` l'active — le pack a le dernier mot pour sa propre distribution.

---

## 5. Palette : slots sémantiques

Slots internes OKLCH ; entrées manifestes acceptées en `#rrggbb`, `#rrggbbaa`, `rgb()`, `hsl()`. La liste couvre les besoins de `Theme` (fg, bg, accent, background) et va plus loin :

| Slot | Défaut dérivé si absent | Usage |
|---|---|---|
| `bg` | — (requis, ou hérité) | fond des surfaces |
| `surface` | `bg` éclairci de 6 % | cartes, bulles |
| `surface_raised` | `surface` éclairci de 4 % | éléments posés sur surface |
| `fg` | — (requis, ou hérité) | texte courant |
| `fg_muted` | `fg` à 65 % de luminosité perçue | texte secondaire |
| `border` | `fg` à 18 % d'opacité | filets |
| `accent` | — (requis, ou hérité) | sélection, actions |
| `accent_fg` | couleur de contraste maximal contre `accent` | texte sur accent |
| `accent_muted` | `accent` alpha 0,35 | traits, focus discrets |
| `success` | vert OKLCH par défaut | états positifs |
| `warning` | ambre OKLCH par défaut | avertissements |
| `danger` | rouge OKLCH par défaut | erreurs, urgent |
| `background` | `bg` assombri de 4 % | fond du bureau sans image |

Règle de résolution d'un slot : surcharge de variante > palette du pack > chaîne d'héritage (le plus proche d'abord) > défaut dérivé. Un slot requis absent partout : erreur de résolution explicite (`resolve` échoue, jamais de valeur inventée).

L'aliasing vers les clés de réglage existantes est bijectif : `theme.fg` ↔ `fg`, `theme.bg` ↔ `bg`, `theme.accent` ↔ `accent`, `background` ↔ `background`. Quand aucun pack n'est actif (« thème réglé »), le moteur construit une palette implicite depuis ces quatre clés et les défauts dérivés — le comportement actuel de hyprdesk devient un cas particulier du moteur.

---

## 6. Héritage et variantes

- **Héritage** : `[inherits].themes`, ordre = priorité croissante (le dernier de la liste gagne pour chaque slot ou gabarit non défini localement). Profondeur maximale 4, cycle détecté → erreur `circular_inheritance`. Un parent manquant : avertissement, résolution continue.
- **Variantes** : sections `[variant.dark]` / `[variant.light]` (noms réservés) qui surchargent palette, effets et options Hyprland. Une variante ne peut hériter que de l'autre si le pack le déclare (`variant.light.inherits = "dark"` est refusé : les variantes sont plates, l'héritage est l'affaire des packs).
- **`theme.variant`** : réglage utilisateur `dark | light | auto`. En `auto`, suit `org.freedesktop.appearance color-scheme` (portail), relu à chaque événement du portail. La variante effective fait partie du rapport d'application.

---

## 7. Gabarits : le DSL

Un gabarit est du texte avec des placeholders. **Pas de contrôle de flux** (pas de `for`, pas de `if`) : un pack qui veut du conditionnel écrit une variante ou un gabarit par cas. Cette restriction est ce qui garantit « un pack = des données ».

```
placeholder  ::= "{{" ws path ws ( "|" ws filter ws )* "}}"
path         ::= ns ( "." ident )+        ns ::= "palette" | "effects" | "meta" | "variant"
filter       ::= ident ( "(" arg ( "," arg )* ")" )?
arg          ::= nombre | pourcentage | couleur | ident | texte
```

Espaces de noms : `palette.*` (couleurs), `effects.<surface>.<opacity|blur>`, `meta.*` (id, name, version), `variant` (valeur effective `dark`/`light`). Aucun accès à la machine (pas de `env`, pas de `file`) — c'est un écart volontaire avec la plupart des moteurs de gabarits, assumé pour la sécurité.

### Filtres (builtins, liste close)

| Filtre | Signature | Effet |
|---|---|---|
| `alpha` | `alpha(f64)` | opacité : `{{palette.accent|alpha(0.4)}}` |
| `lighten` / `darken` | `(pct)` | luminosité OKLCH, bornée |
| `saturate` | `(pct signé)` | chroma OKLCH |
| `hue` | `(deg signé)` | rotation de teinte |
| `mix` | `(slot_ou_couleur, pct)` | mélange : `{{palette.accent|mix(palette.bg 20%)}}` |
| `contrast` | `(min)` | ajuste la luminosité jusqu'au ratio WCAG `min` contre le fond de son rôle |
| `pick` | `(si_sombre, si_clair)` | valeur selon la variante effective |
| `fmt` | `(hex\|rgb\|hsl\|hypr)` | format de sortie ; `hypr` = `rgba(aabbccdd)` hyprlang |

Format par défaut par cible (kitty → `hex`, GTK CSS → `rgb()`, hyprlang → `hypr`), toujours surchargeable par `fmt`. Les filtres se composent : `{{palette.accent|lighten(8%)|alpha(0.6)|fmt(hypr)}}`.

### Gabarits intégrés

Le moteur embarque un gabarit par cible (rendu identique au comportement actuel de hyprdesk). Un pack les surcharge par `[targets.<cible>].template`. Un gabarit rendu identique en octets à la sortie précédente ne déclenche ni réécriture ni rechargement (hash SHA-256, §11).

---

## 8. Cibles (adaptateurs)

La cible généralise le contrat `apptheme::Target` (`src/apptheme/mod.rs`) : identifiant stable, `installed()`, options réglables, `apply(theme, opts) → compte rendu`, `clear(opts)`. Le registre devient `theme_engine::TARGETS`.

| Cible | Sorties | Rechargement | Dépendances | Phase |
|---|---|---|---|---|
| `hyprland` | `~/.config/hypr/theme/hyprland.conf` (fragment généré) | `hyprctl reload` ; mode `hot` = batch `hyprctl keyword` sans écriture | hyprctl | 1 |
| `hyprlock` | fragment sourcé par `~/.config/hypr/hyprlock.conf` (pattern `source=` déjà en place) | rien (rechargé au verrouillage) | hyprlock | 1 |
| `kitty` | `~/.config/kitty/hyprdesk-theme.conf` (fichier actuel) | SIGUSR1, ou socket `kitty @ load_config` si contrôle distant | kitty | 1 |
| `gtk` | thème GTK installé `~/.local/share/themes/hyprdesk-<pack>/` (gtk-3.0, gtk-4.0, index.theme) + override `~/.config/gtk-3.0/gtk.css` et `gtk-4.0/gtk.css` avec `@define-color` ; `gsettings` pour `gtk-theme` et `color-scheme` | gsettings → rechargement des apps GTK | gsettings | 1 |
| `thunar` | `~/.config/gtk-3.0/hyprdesk-thunar.css` (fichier actuel) | inclus dans `gtk` | thunar | 1 |
| `qt` | `~/.config/qt6ct/colors/hyprdesk.conf` (miroir qt5ct), Kvantum `.kvconfig` si installé, gabarit `.qss` optionnel | relance de l'application (`restart_required`) ; accent via portail | qt6ct, kvantum | 2 |
| `fonts` | clés hyprdesk + `gsettings font-name`, gabarit kitty | selon cibles touchées | — | 2 |
| `icons` | `gsettings icon-theme` + paquet icônes hyprdesk | événement démon | — | 2 |
| `cursors` | `HYPRCURSOR_THEME` + `XCURSOR_THEME` (env.lua) + `gsettings cursor-theme` | nouvelle session | hyprcursor | 2 |
| `sounds` | thème de sons freedesktop + paquet son hyprdesk | événement démon | — | 2 |
| `wallpaper` | pose le fond du pack (délégue au paquet Fond d'écran) | démon hyprdesk | — | 2 |
| `<id> générique` | sortie déclarée dans `[targets.*]` du pack | langage borné | — | 2 |

### Langage de rechargement (borné)

Valeurs autorisées pour `reload` : `none`, `signal:<nom_de_processus>:<SIGUSR1|SIGUSR2|SIGHUP>`, `socket:kitty`, `gsettings`, `hyprctl`, `daemon`. Tout le reste (commande shell arbitraire, script) est **refusé à la validation du pack** : un pack ne peut pas exécuter de code.

### Cible `hyprland` en détail

- Le fragment généré ne touche que la liste blanche d'options décoratives : `decoration:*`, `general:gaps_*`, `general:border*`, `animation:*` par nom d'animation, `misc:*` pour le flou. Toute autre option dans `[hyprland.options]` → erreur de validation.
- L'inclusion du fragment est faite une fois dans `hyprland.lua` par le démon (pattern existant `hyprsrc.rs`/`genfiles.rs` : écriture idempotente, inclusion unique avec copie de sauvegarde). Le moteur n'édite jamais `hyprland.lua` lui-même : il écrit le fragment et signale l'inclusion manquante dans le rapport (`warning: include_missing`).
- Mode `hot` : batch `hyprctl keyword` des mêmes options, sans écrire de fichier — c'est l'aperçu en direct, aligné sur le comportement de `hypr_set_option_live`.

---

## 9. Transport des styles vers les applications

Une application tierce — iced, GTK, Qt, ou tout autre toolkit — doit pouvoir suivre le thème sans rien connaître de hyprdesk ni dépendre du démon. Quatre transports, du plus simple au plus intégré, tous produits par la même passe `apply` (§10) et soumis au contrat commun (§9.7).

### 9.1 Palette publiée (transport canonique)

À chaque application réussie, le moteur publie la palette résolue dans `~/.local/state/hyprdesk/theme-engine/palette.json` (mode 0644, écriture par rename atomique) :

```json
{
  "schema": 1,
  "pack": "ardoise",
  "variant": "dark",
  "revision": 12,
  "colors": { "bg": "#171b27", "fg": "#f2f4fc", "accent": "#a69cff" },
  "effects": { "bar": { "opacity": 0.72, "blur": true } },
  "fonts": { "family": "Mindlog", "mono_family": "Mindlog Mono", "size": 10.5 },
  "updated": "2026-11-22T10:12:00Z"
}
```

- `colors` contient **tous** les slots du §5, dérivés compris — c'est la source de vérité pour tout client.
- `revision` : compteur monotone persisté dans l'état, incrémenté seulement quand le contenu change ; les clients comparent avant de redessiner.
- La commande `palette` (§12) imprime la même donnée en `json`, `env`, `css` ou `hex`.

### 9.2 Environnement de lancement

Pour les applications lancées par hyprdesk (lanceur, dock, autostart), le démon expose un jeu d'environnement minimal et stable : `HYPRDESK_THEME_ID`, `HYPRDESK_THEME_VARIANT`, `HYPRDESK_BG`, `HYPRDESK_FG`, `HYPRDESK_ACCENT`, `HYPRDESK_SURFACE`, `HYPRDESK_BORDER`. Réservé aux lectures au démarrage (prompts shell, TUI, scripts) ; suivre le thème à chaud exige 9.1 ou 9.3.

### 9.3 Crate cliente iced — `hyprdesk-theme-client`

Nouvelle crate du workspace (`crates/hyprdesk-theme-client`, même modèle que hyprdesk-i18n et hyprdesk-auth) : elle ne lie ni le démon, ni iced, ni mlua — serde et inotify suffisent.

```rust
use hyprdesk_theme_client::ThemeClient;

let client = ThemeClient::new()?;              // lit palette.json (schema 1)
let pal = client.palette();                    // slots typés en Rgba + effets
client.subscribe(move |pal| /* callbacks hors thread UI */);
```

- `Palette` expose les slots du §5, les effets et la révision ; méthodes utilitaires (`is_dark()`, `contrast_ratio(a, b)`).
- Intégration iced : le callback hors thread UI est converti par l'application en `Task`/`Message` (pattern hyprdesk : rechargement local des seules surfaces touchées, `transition = "none"` respecté). L'application mappe `Palette` vers ses propres couleurs ; un helper `hyprdesk_theme_client::iced_palette(&pal)` fournit un point de départ (`iced::Color` par slot).
- mindlog et les autres applications du dépôt l'adoptent comme implémentation de référence ; une application tierce ne dépend que de cette crate, jamais du démon.

### 9.4 GTK — deux couches

- **Thème installé** (couche forte) : le pack est rendu comme un vrai thème GTK dans `~/.local/share/themes/hyprdesk-<pack>/` — `gtk-3.0/gtk.css`, `gtk-4.0/gtk.css` (named colors `@define-color` + règles légères), `index.theme` minimal — puis sélectionné par `gsettings gtk-theme`. Les applications qui honorent le réglage système suivent le pack, y compris à leur prochain démarrage, sans override utilisateur.
- **Override utilisateur** (couche douce) : les `gtk.css` de `~/.config` portent les mêmes `@define-color` pour les applications qui écrasent ou ignorent le thème choisi.
- **libadwaita** : ces applications ne suivent pas `gtk-theme` pour les couleurs ; elles consomment le portail (9.5) pour `color-scheme` et, quand le toolkit l'implémente, `accent-color`. Le rapport marque ces applications `restart_required` tant que leur version n'applique pas les changements à chaud.
- **GTK3** : le rechargement à chaud des CSS n'est pas garanti ; le signal du portail est émis, et le rapport note `restart_required`.

### 9.5 Portail freedesktop (`org.freedesktop.appearance`)

`src/portal.rs` publie déjà le service ; le moteur l'alimente :

- `color-scheme` : déjà publié, piloté par `theme.variant` ;
- `accent-color` (portail ≥ 1.19) : couleur d'accent en rgb flottant, dérivée de la palette publiée ;
- `contrast` : 0, 1 ou 2 selon le ratio fg/bg mesuré (`verify`, §12).

Consommateurs : libadwaita récent, Qt ≥ 6.5 pour `color-scheme` (l'accent progresse toolkit par toolkit), applications des autres bureaux. Aucun nouveau processus : hyprdesk tient déjà le portail.

### 9.6 Qt

- **qt5ct / qt6ct** : schéma de couleurs généré (`~/.config/qt6ct/colors/hyprdesk.conf`, sérialisation QPalette des slots ; miroir qt5ct), sélectionné si l'utilisateur a déjà `QT_QPA_PLATFORMTHEME` — le moteur n'impose pas la plateforme.
- **Kvantum** (si installé) : gabarit `.kvconfig` (palette + rayons) dans `~/.config/Kvantum/hyprdesk-<pack>/`, activable par `[targets.kvantum]` dans le pack.
- **QSS** : gabarit `.qss` générique pour les applications qui chargent leur propre stylesheet.
- Le changement Qt prend effet au relancement de l'application (`restart_required`) ; l'accent et le schéma clair/sombre progressent via le portail (9.5).

### 9.7 Contrat commun des transports

| Propriété | Engagement |
|---|---|
| Atomicité | toute publication par rename atomique, jamais de fichier à moitié écrit |
| Versionnement | `schema` du fichier + `revision` monotone du contenu |
| Détection de changement | les clients comparent `revision` ; le moteur ne l'incrémente que sur changement effectif |
| Fraîcheur annoncée | le rapport d'application note chaque transport `live` ou `restart_required` |
| Sécurité | un pack ne publie rien hors des transports déclarés ; les variables d'environnement ne sont injectées que par le lanceur de hyprdesk |

Aucun transport n'est obligatoire pour une application : la palette JSON seule (9.1) suffit ; la crate cliente (9.3) est le raccourci recommandé côté iced, le portail (9.5) côté GTK/Qt.

## 10. Application transactionnelle

1. **Résoudre** : pack actif + variante + héritage → palette complète, effets, options. (Échec : rapport `resolve_failed`, rien d'écrit.)
2. **Générer** : rendu de chaque gabarit dans le staging `~/.cache/hyprdesk/theme-engine/staging/<uuid>/`.
3. **Valider** : par cible, contrôle best-effort (équilibre des accolades CSS, paire de couleurs contrastée signalée, options Hyprland contre liste blanche).
4. **Commettre** : par fichier, écriture temporaire dans le dossier cible puis `rename` atomique ; **copie de sauvegarde une seule fois** si le fichier a été modifié hors du moteur (pattern `.bak` d'`apptheme`, jamais écrasée par le moteur). Hash SHA-256 : fichier identique → inchangé, pas de reload.
5. **Recharger** : par cible, selon son mécanisme ; échec de reload ≠ échec d'écriture.
6. **Rapporter** : JSON (annexe D), écrit dans `~/.local/state/hyprdesk/theme-engine/last-applied.json`, entrée dans l'Historique (paquet `theme`).

Rollback : si la validation d'une cible échoue après des écritures locales, les fichiers de cette cible sont restaurés depuis les `.bak` et le contenu précédent (conservé dans le staging) ; les autres cibles restent appliquées. Échec de résolution ou de génération : rien n'est touché, par construction.

---

## 11. Modes d'application

| Mode | Effet |
|---|---|
| `persist` (défaut) | fichiers générés + reload, état persistant |
| `hot` | cible `hyprland` uniquement en `hyprctl keyword`, rien d'écrit ; autres cibles en persist |
| `dry-run` | résolution + génération + rapport, aucune écriture, aucun reload |

### `watch` (mode veille)

`hyprdesk-theme-engine watch` lance un processus résident, socket Unix `$XDG_RUNTIME_DIR/hyprdesk-theme-engine.sock` (une ligne JSON par événement) :

- **déclencheurs** : changement de fond (paquet Fond d'écran du démon → socket), événement `color-scheme` du portail (`theme.variant = auto`), changement de réglage `theme.*` signalé par le démon, minuteurs planifiés (§20 heure du jour) ;
- **actions** : re-résolution, re-génération différentielle (hash), rechargement des cibles touchées uniquement.

Un seul `watch` à la fois : prise de socket exclusive, deuxième instance → `apply` one-shot puis sortie.

---

## 12. CLI

Binaire : `hyprdesk-theme-engine`. Tout supporte `--json` (sortie machine, une valeur par ligne pour les listes) et `--no-color`.

```
hyprdesk-theme-engine list [--all] [--json]
hyprdesk-theme-engine info <id|chemin|archive> [--json]
hyprdesk-theme-engine use <id> [--variant dark|light|auto] [--targets a,b]
                            [--mode persist|hot|dry-run]
hyprdesk-theme-engine apply [--targets a,b] [--mode …]        # réapplique l'actif
hyprdesk-theme-engine generate [--variant …] [--out <dir>]    # rend les gabarits sans appliquer
hyprdesk-theme-engine palette [--format json|env|css|hex] [--variant …]
hyprdesk-theme-engine verify [<id>] [--wcag] [--json]         # contraste, dépendances, cohérence
hyprdesk-theme-engine preview <id> [--variant …] --out <png>  # mire de palette (style assets/colors/mire.png)
hyprdesk-theme-engine recolor --from <image> [--scheme vibrant|material|pastel]
                              [--hue-lock <deg>] [--out <dossier pack>]
hyprdesk-theme-engine install <chemin|.hyprtheme>
hyprdesk-theme-engine remove <id>
hyprdesk-theme-engine export <id> -o <archive.hyprtheme>
hyprdesk-theme-engine import <archive.hyprtheme>
hyprdesk-theme-engine diff <id|chemin> <id|chemin> [--json]   # slots et gabarits divergents
hyprdesk-theme-engine tools [--json]                          # manifeste d'outils (schémas JSON), surface LYRA
hyprdesk-theme-engine build --intent <intent.json> [--out <dir>]
hyprdesk-theme-engine watch
hyprdesk-theme-engine doctor [--json]                         # deps + packs invalides
```

Codes de sortie : `0` succès ; `1` usage ; `2` pack ou manifeste invalide ; `3` échec d'au moins une cible (le rapport détaille) ; `4` dépendance absente (doctor liste) ; `5` rollback effectué.

`--origin user|lyra|daemon|cli` sur les commandes qui modifient l'état (`use`, `apply`, `install`, `remove`, `import`, `build`) : étiquette d'origine portée par `last-applied.json`, le journal et l'Historique (« modifié par LYRA »). Défaut `cli`.

Variable d'environnement `HYPRDESK_THEME=<id>[:<variante>]` : surcharge globale de l'équivalent GTK `GTK_THEME`, respectée par toutes les commandes. Priorité : `HYPRDESK_THEME` > réglages du démon > défaut.

---

## 13. Emplacements

| Rôle | Chemin |
|---|---|
| Packs utilisateur | `~/.local/share/hyprdesk/themes/<id>/` |
| Packs système | `$XDG_DATA_DIRS/hyprdesk/themes/<id>/` |
| Packs embarqués | ressources du binaire |
| Staging | `~/.cache/hyprdesk/theme-engine/staging/<uuid>/` |
| État | `~/.local/state/hyprdesk/theme-engine/` (`palette.json`, `last-applied.json`, `journal.log`) |
| Socket | `$XDG_RUNTIME_DIR/hyprdesk-theme-engine.sock` |
| Fragment Hyprland | `~/.config/hypr/theme/hyprland.conf` |
| Thème GTK installé | `~/.local/share/themes/hyprdesk-<pack>/` |
| Packs construits par LYRA | `~/.cache/hyprdesk/theme-engine/builds/` |

---

## 14. Sécurité

- **Un pack est des données** : pas de Lua, pas de shell, pas d'inclusion de fichiers, pas d'accès environnement dans les gabarits. La liste des filtres est close (§7), le langage de rechargement est borné (§8).
- **Chemins de sortie en liste blanche** par cible ; la cible générique est confinée à `~/.config/` et `~/.local/share/`, sans symlink sortant (les symlinks rencontrés en chemin de sortie sont refusés).
- **Limites** : manifeste 64 Kio, pack 50 Mio, gabarit 1 Mio, 2000 fichiers, héritage profondeur 4, recolor sur image ≤ 100 Mio décodée.
- **Aucun réseau** en v1 ; signature minisign des archives en phase 3.
- **LYRA** : l'assistante passe par la même surface (`tools`, même liste blanche, mêmes confirmations) ; jamais d'écriture directe de fichier, jamais de shell ; une clé de `policy.toml` peut tout couper (§17).
- Décodage d'image avec budget mémoire borné (protecteur déjà en place côté démon, à réutiliser).

---

## 15. Performance

- Résolution + génération : ≤ 30 ms pour un pack typique (12 gabarits).
- `apply` complet persist : ≤ 300 ms hors reloads des applications.
- Différentiel par hash : fichier inchangé → ni réécriture ni reload ; cible sans changement → pas de reload.
- `watch` : inotify sur le pack actif, socket du démon pour les fonds ; aucune boucle de sondage.

---

## 16. Intégration hyprdesk

- **Démon** : Réglages → Thème appelle la bibliothèque en cours de processus (pas de sous-process) ; chaque `hyprdesk config set theme.*` est suivi d'un `apply` debouncé (300 ms). Les clés `theme.*`, `general.app_themes`, `plugin_options["apptheme"]` gardent leur sens exact.
- **Surfaces iced du démon** : la barre, les réglages et les widgets rechargent leurs couleurs au changement de `revision` de `palette.json` via la crate cliente (§9.3) — même règle que le reste : redessin local des seules surfaces touchées.
- **Portail** (`src/portal.rs`) : publie `accent-color` et `contrast` d'après la palette publiée, en plus de `color-scheme` (§9.5).
- **`src/apptheme/`** : les modules `kitty` et `thunar` migrent vers des cibles du moteur, clés d'options conservées ; le contrat `Target` devient `theme_engine::Target`. La page Réglages → Thème des applications devient la liste des cibles du moteur.
- **Préréglages** : ceux de `src/theme.rs` (Ardoise…) deviennent des packs embarqués équivalents bit-à-bit ; `theme.preset` reste valable (alias de `use <id>`), `theme.pack` désigne un pack installé.
- **Partage** (`src/share.rs`) : « exporter le thème seul » → `export` du pack actif + valeurs des clés `theme.*` ; « importer » → `import` + `use`.
- **Historique** (`src/history.rs`) : chaque `apply` journalisé ; « retour à une version précédente » rejoue le `last-applied.json` correspondant.
- **doctor** (`src/doctor.rs`) : `theme-engine doctor` alimente `DEPS` (gsettings, kitty, hyprctl, thunar…).
- **Aperçu** : scène de preview `theme-palette` (mire des slots en clair et sombre) ; `HYPRDESK_PREVIEW_THEME=light` lit la variante du moteur.
- **LYRA** : au pupitre du moteur, par la surface d'outils et les workflows du §17.
- **verrouillage/connexion** : `hyprdesk-lock` et `hyprdesk-greeter` lisent la palette résolue via la bibliothèque (aucun lien vers le démon, conforme à leur règle de liaison).

---

## 17. LYRA au pupitre du moteur

Principe : LYRA traduit le langage, le moteur fait les maths. L'assistante ne manipule jamais le moteur en écrivant des fichiers ni en passant par un shell : elle appelle une surface d'outils schématisés, validée par le démon, avec les confirmations déjà en place du chat (`Tool` + `confirm_request`, `src/chat.rs`). Le modèle sous LYRA répond en un tour par demande (abonnement, `src/ai_services.rs`) ; les workflows multi-étapes sont orchestrés par le démon — chaque étape est un appel d'outil, le démon reconsulte le modèle avec le résultat si besoin. Le moteur n'héberge aucun agent.

Le démon exécute ces outils par la bibliothèque en cours de processus (pas de sous-process, pas de shell) ; les mêmes opérations restent disponibles en CLI pour un humain.

### 17.1 Surface d'outils

`hyprdesk-theme-engine tools --json` imprime le manifeste : chaque outil, son schéma d'arguments (JSON Schema), sa classe de permission, son effet. Le démon le passe au modèle (function calling) ; tout appel est vérifié contre le schéma avant exécution.

| Outil | Arguments | Classe | Effet |
|---|---|---|---|
| `theme_list` | — | lecture | packs, origine, variante active |
| `theme_info` | id | lecture | manifeste résolu |
| `theme_palette` | format | lecture | palette publiée + révision |
| `theme_verify` | id ? | lecture | contraste, dépendances, cohérence |
| `theme_diff` | a, b | lecture | slots et gabarits divergents |
| `theme_preview` | id, variante | préparation | mire + scène d'aperçu, rien d'appliqué |
| `theme_build` | intent (§17.3) | préparation | construit un pack dans `builds/`, sans l'installer |
| `theme_use` | id, variante, mode | application | installe et pose le thème |
| `theme_apply` | cibles ?, mode | application | réapplique l'actif |
| `theme_install` | chemin | application | promeut un pack de `builds/` ou une archive |
| `theme_remove` | id | application | retire un pack |

### 17.2 Classes de permission

| Classe | Confirmation | Policy |
|---|---|---|
| `lecture` | jamais | toujours autorisée, sauf coupure totale |
| `préparation` | jamais (rien n'est visible ni appliqué) | idem |
| `application` | **toujours** : carte de confirmation dans le chat (ce qui change, aperçu joint) | clé dédiée |

`/etc/hyprdesk/policy.toml` (pattern `src/policy.rs`, fichier root, non contournable par les Réglages) reçoit deux clés :

```toml
theme_engine_ai = true        # false : LYRA n'a aucun outil de thème
theme_engine_ai_apply = true  # false : LYRA peut lire et préparer, jamais appliquer
```

### 17.3 Palette par intention (`build --intent`)

LYRA ne choisit pas de hex : elle traduit une demande en **intent** structuré, le moteur calcule la palette (maths OKLCH, contraste garanti par le plancher choisi). Vocabulaire fermé = résultats déterministes et rejouables.

```json
{
  "name": "sauge",
  "variant": "dark",
  "hue": "sauge",
  "mood": "calme",
  "contrast": "aa",
  "accent": "auto",
  "inherit": "hyprdesk-base"
}
```

- `hue` : couleur nommée d'une liste close (iris, sauge, ambre, rubis, océan, mousse, graphite…) ou degrés ; `mood` : `calme | vif | contraste` — bornes de chroma et de luminosité ; `contrast` : `aa | aaa` — plancher WCAG pour les paires du §5 ; `accent` : `auto` (dérivée) ou hex imposée.
- `build --intent intent.json --out <dir>` : écrit un pack complet (manifeste, gabarits intégrés, mire) dans `~/.cache/hyprdesk/theme-engine/builds/` — ni installé ni appliqué.
- `theme_use` installe (promotion du build) et pose, après confirmation.

Workflow complet : demande (« un thème calme, vert sauge, sombre ») → intent → `theme_build` → `theme_preview` joint au chat → confirmation → `theme_use` → Historique avec `origin = lyra`. Retouche (« rends l'accent plus doux ») : LYRA relit la palette, ajuste l'intent (chroma relatif), nouveau build et nouvel aperçu — l'utilisateur voit avant de décider.

### 17.4 Attribution et traçabilité

- `--origin lyra` sur les commandes qui modifient l'état ; `last-applied.json`, le journal et l'Historique la portent (« modifié par LYRA » dans Réglages → Historique).
- Chaque appel d'outil est journalisé (outil, arguments, résultat, classe) — fil d'audit du démon.
- La voix du Panorama suit le même chemin : demande vocale → mêmes outils, confirmation parlée explicite (« je pose le thème ? »), jamais d'application sans oui.

### 17.5 Limites

- LYRA ne voit que ce que les outils renvoient : aucun accès aux fichiers de configuration, aucun accès réseau côté moteur.
- Le moteur reste déterministe : aucune décision esthétique cachée dans le moteur ; l'intent est le contrat, révisable et rejouable.
- Un thème construit par LYRA est un pack ordinaire : il se partage, s'exporte et se retire comme les autres ; rien de privilégié.

## 18. Parité GTK

| Capacité du moteur GTK | hyprdesk-theme-engine |
|---|---|
| Découverte `~/.themes`, XDG dirs | §3 : XDG `hyprdesk/themes` + embarqués |
| `index.theme` + `Inherits=` | §4/§6 : `[inherits.themes]`, profondeur 4, cycles détectés |
| Variantes clair/sombre | §6 : `[variant.*]`, `theme.variant`, `auto` par portail |
| Commutation à chaud (gsettings) | §11 : `use`/`apply` + reloads par cible + événements démon |
| Surcharge par application `GTK_THEME` | §12 : `HYPRDESK_THEME` + désactivation par cible (`app_themes`) |
| Thèmes d'icônes | §8 : cible `icons` (gsettings + paquet hyprdesk) |
| Curseurs | §8 : cible `cursors` (hyprcursor + X cursor) |
| Polices | §8 : cible `fonts` |
| Thèmes de sons (freedesktop) | §8 : cible `sounds` |
| Distribution tierce (GNOME look…) | §3 : archives `.hyprtheme`, `install`/`import` |
| Recoloration (icônes symboliques) | §7 : filtres `alpha`, `contrast`, `mix` |
| Outil de sélection (nwg-look) | §16 : Réglages → Thème (galerie + aperçu) |
| Récupération après thème cassé | §10 : rollback + `.bak` + Historique (GTK n'a rien d'équivalent) |
| Intégration des applications tierces (iced, Qt) | §9 : palette JSON versionnée, crate cliente, portail `accent-color`, schémas Qt (GTK n'a rien de transversal aux autres toolkits) |

---

## 19. Au-delà de GTK

1. **Effets par surface** (opacité/flou de la barre, menus, lanceur, réglages, widgets) : première classe dans le manifeste, déjà le modèle de `theme.effects.*`.
2. **Palette depuis le fond d'écran** (`recolor`) : extraction des dominantes en OKLCH, trois schémas (`vibrant`, `material` tonal, `pastel`), verrou de teinte ; en `watch`, re-coloration automatique au changement de fond du diaporama — le superset de matugen, sans outil externe.
3. **Heure du jour** : plan `theme.schedule` (variante claire au lever, sombre au coucher, intégrée au module nightlight) ; exécution par `watch`.
4. **Par écran** : accent ou variante distincte pour le Panorama et la barre d'un écran donné (les tables gamma par écran existent déjà) — phase 3, clés `theme.<écran>.*`.
5. **`verify --wcag`** : rapport de ratios fg/bg, accent_fg/accent, fg_muted/surface, avec seuils AA/AAA — aucun moteur GTK ne garantit le contraste.
6. **`diff`** : comparaison de deux packs (slots, effets, gabarits) — utile avec l'Historique.
7. **Import de matugen/wallust** : convertisseur de leurs formats vers un pack (phase 3).
8. **Thème décrit par LYRA** : « un thème calme, vert sauge, sombre » → intent structuré, pack construit par le moteur, prévisualisé avant application (§17).
9. **Dépôts signés** : catalogues minisign, mise à jour des packs (phase 3).

---

## 20. Jalons

### Phase 1 — cœur et cibles existantes (MVP)

- [ ] `src/theme_engine/` : manifeste, découverte, résolution (héritage, variantes, dérivés), DSL de gabarits + filtres, maths OKLCH.
- [ ] Cibles `hyprland`, `hyprlock`, `kitty`, `gtk`, `thunar` (kitty/thunar migrés depuis `apptheme`, clés conservées).
- [ ] Application transactionnelle : staging, hash, `.bak`, rollback, rapport JSON, `last-applied.json`.
- [ ] Binaire `src/bin/hyprdesk-theme-engine.rs` : `list`, `info`, `use`, `apply`, `generate`, `palette`, `--json`.
- [ ] Publication `palette.json` (révision) et crate `crates/hyprdesk-theme-client` ; les surfaces iced du démon suivent la révision.
- [ ] Préréglages `src/theme.rs` convertis en packs embarqués ; palette implicite « thème réglé ».
- [ ] Démon : `hyprdesk config set theme.*` → apply debouncé ; fragment `hyprland` inclus par `hyprsrc.rs`.
- Acceptance : `use ardoise --variant light` en session commute Hyprland, kitty, GTK et Thunar sans reconnexion ; une application iced tierce (liée à `hyprdesk-theme-client` seule) suit la variante à chaud ; `--mode hot` ne persiste rien ; `--mode dry-run` n'écrit rien ; retour arrière Historique fonctionnel.

### Phase 2 — distribution et couverture

- [ ] Archives `.hyprtheme` : `install`, `remove`, `export`, `import` ; branchement Partage.
- [ ] Cibles `fonts`, `icons`, `cursors`, `sounds`, `wallpaper`, cible générique.
- [ ] `watch` + socket + re-coloration depuis le fond d'écran (`recolor`).
- [ ] Transports GTK/Qt complets : portail `accent-color`/`contrast`, cible `qt` (qt5/6ct, Kvantum, QSS), variables d'environnement de lancement.
- [ ] `verify --wcag`, `diff`, `doctor`, `preview`.
- [ ] Surface d'outils LYRA : `tools`, `build --intent`, `--origin`, confirmation dans le chat, clés de policy (`theme_engine_ai`, `theme_engine_ai_apply`).
- Acceptance : un pack tiers importé d'une archive s'applique sans accès shell ; diaporama → re-coloration automatique ; Qt suit le schéma au relancement ; l'accent est publié par le portail ; « crée un thème sauge sombre » chez LYRA produit un aperçu puis, après confirmation, un pack appliqué avec `origin = lyra` dans l'Historique ; `verify` signale tout ratio < AA.

### Phase 3 — au-delà

- [ ] Heure du jour, thème par écran, import matugen, signature des archives, catalogue, atelier de thème conversationnel multi-tours (affinage itératif avec aperçu).

---

## Annexe A — pack complet d'exemple

```toml
[meta]
id = "ardoise"
name = "Ardoise"
version = "1.1.0"
description = "Verre fumé, accent iris, clair et sombre."
authors = ["Jacques Hullu"]
license = "MIT"
default_variant = "dark"

[inherits]
themes = ["hyprdesk-base"]

[palette]
bg = "#171b27"
fg = "#f2f4fc"
accent = "#a69cff"
background = "#131724"

[variant.light]
palette.bg = "#eef0f8"
palette.fg = "#171b27"
palette.accent = "#6c58e6"
palette.background = "#e4e7f2"

[effects.bar]
opacity = 0.72
blur = true

[hyprland.options]
"decoration:rounding" = "10"
"general:gaps_in" = "4"

[targets.waybar]
template = "templates/waybar.css.tmpl"
output = "~/.config/waybar/hyprdesk.css"
reload = "signal:waybar:USR1"
```

## Annexe B — gabarit kitty (intégré, simplifié)

```text
# Généré par hyprdesk-theme-engine — pack {{meta.name}} v{{meta.version}} ({{variant}})
# Modifié à la main : ne sera pas écrasé (copie .bak).
foreground {{palette.fg}}
background {{palette.bg}}
color8     {{palette.fg_muted|fmt(hex)}}
selection_foreground {{palette.accent_fg}}
selection_background {{palette.accent|alpha(0.9)|fmt(hex)}}
url_color  {{palette.accent|fmt(hex)}}
```

## Annexe C — session type

```console
$ hyprdesk-theme-engine list
ardoise   Ardoise   1.1.0   bundled
nord      Nord      0.3.2   user

$ hyprdesk-theme-engine use ardoise --variant light
hyprland   écrit  ~/.config/hypr/theme/hyprland.conf   reload ok
hyprlock   écrit  hyprlock.conf (source)                ok
kitty      écrit  kitty/hyprdesk-theme.conf             reload ok (SIGUSR1)
gtk        écrit  gtk-3.0 + gtk-4.0 css                 gsettings ok
thunar     inchangé (hash identique)

$ hyprdesk-theme-engine apply --mode dry-run --json
{"variant":"light","targets":{"hyprland":"changed","kitty":"unchanged"},"warnings":[]}
```

## Annexe D — rapport d'application (JSON)

```json
{
  "pack": "ardoise", "variant": "light", "mode": "persist",
  "started": "2026-11-22T10:12:00Z", "duration_ms": 184,
  "targets": [
    {"id": "hyprland", "status": "ok", "files": [{"path": "~/.config/hypr/theme/hyprland.conf", "result": "written"}], "reload": "ok"},
    {"id": "kitty", "status": "ok", "files": [{"path": "~/.config/kitty/hyprdesk-theme.conf", "result": "unchanged"}], "reload": "skipped"},
    {"id": "gtk", "status": "ok", "files": [{"path": "~/.config/gtk-4.0/gtk.css", "result": "written"}], "reload": "gsettings"}
  ],
  "warnings": ["hyprland: include_missing dans hyprland.lua"],
  "errors": [], "rolled_back": []
}
```
