# Intégrer une application au thème de hyprdesk (hyprdesk-theme-engine)

Tu intègres l'application **<NOM>** au moteur de thème de hyprdesk, pour
qu'elle prenne les couleurs du thème en place et les suive quand il change.
C'est le moteur qui fait foi : son pack actif et sa palette résolue sont la
vérité. Ton application les **lit** ; elle n'écrit jamais dans l'état du
moteur et ne recalcule pas de couleurs à sa façon.

Spécification complète du moteur : [THEME-ENGINE.md](THEME-ENGINE.md).

## 1. Choisir le canal (par ordre de préférence)

| L'application est… | Canal |
|---|---|
| en Rust (iced ou autre) | crate `hyprdesk-theme-client` (§3) |
| écrite dans n'importe quel langage, avec une boucle d'événements | lecture et suivi de `palette.json` (§2) |
| un script, un TUI, un prompt de shell | `hyprdesk-theme-engine palette --format env\|css\|json\|hex` (§4), ou les variables `HYPRDESK_*` au lancement (§5) |
| une application GTK 3/4 ou Qt | rien à coder : elle suit déjà (§6) |
| un logiciel tiers avec un fichier de config et sans API | cible générique dans un pack (§7) |

## 2. Le fichier `palette.json` (canal canonique)

Emplacement : `$XDG_STATE_HOME/hyprdesk/theme-engine/palette.json`
(par défaut `~/.local/state/hyprdesk/theme-engine/palette.json`).

```json
{
  "schema": 1,
  "pack": "verre",
  "variant": "dark",
  "revision": 3,
  "colors": {
    "bg": "#1b1f33", "fg": "#f6f7ff", "accent": "#ff80bf",
    "surface": "#292e43", "surface_raised": "#33384e",
    "fg_muted": "#898a91", "border": "#f6f7ff2e",
    "accent_fg": "#000000", "accent_muted": "#ff80bf59",
    "success": "#54bf5c", "warning": "#f9ad26", "danger": "#f44747",
    "background": "#121629"
  },
  "effects": { "bar": { "opacity": 0.72, "blur": true }, "menus": { "...": "..." } },
  "fonts": { "family": "…", "mono_family": "…", "size": 10.5 },
  "updated": "2026-11-22T11:01:21+00:00"
}
```

Règles à respecter :
- **`schema`** : refuser tout schéma autre que `1`, avec un message clair. Ne
  rien deviner.
- **`colors`** : `#rrggbb` ou `#rrggbbaa`. Les 13 slots ci-dessus sont
  toujours présents. Un pack peut en ajouter : ignorer les inconnus.
  `success`, `warning` et `danger` sont déjà lisibles en texte sur `bg`
  (4,5:1, en clair comme en sombre) : les utiliser tels quels pour les états.
- **`variant`** : `dark` ou `light`.
- **`effects`** : surfaces `bar`, `menus`, `launcher`, `settings`,
  `widgets` ; `opacity` entre 0 et 1, `blur` booléen. Surface absente :
  opacité 1, pas de flou.
- **`fonts`** : facultatif, et chaque champ peut manquer.
- **`revision`** ne bouge que si le contenu change. Redessiner **seulement**
  quand elle change.
- **Suivi** : le moteur publie par renommage atomique. Il faut donc surveiller
  le **dossier** et filtrer sur le nom `palette.json`. Surveiller le fichier
  lui-même perd l'inode remplacé dès la première publication. Regrouper les
  événements (environ 100 ms), relire, comparer `revision`. Pas de boucle de
  sondage.
- **Fichier absent** (hyprdesk pas encore lancé) : garder les couleurs par
  défaut de l'application, et appliquer celles du thème dès que le fichier
  apparaît (le dossier se surveille de la même façon).
- Ne jamais écrire dans ce dossier, ni dans `active.json`.

## 3. Rust : la crate `hyprdesk-theme-client`

```toml
[dependencies]
hyprdesk-theme-client = { git = "https://github.com/jacquesh82/hyprdesk", package = "hyprdesk-theme-client" }
```
(Édition 2024, Rust ≥ 1.99. Dépendances : serde, serde_json, notify. Elle ne
lie ni le démon, ni iced.)

```rust
use hyprdesk_theme_client::{ThemeClient, Palette};

let client = ThemeClient::new();               // emplacement XDG par défaut
match client.palette() {                       // Result<Palette, Error>
    Ok(pal) => apply(&pal),
    Err(e) => eprintln!("thème hyprdesk indisponible : {e}"), // couleurs par défaut
}
// Le rappel tourne hors du fil de l'interface, une fois par révision
// nouvelle. Garder le Watcher en vie tant que l'abonnement est voulu
// (l'abandonner arrête le suivi).
let _watcher = client.subscribe(move |pal: &Palette| {
    // envoyer un message à l'application (iced : Task / Message),
    // puis redessiner seulement les surfaces touchées.
})?;
```
API : `pal.slot("accent") -> Option<Color{r,g,b,a}>`, `pal.is_dark()`,
`pal.opacity("bar")`, `pal.blur("bar")`, `pal.fonts`, `pal.revision`,
`pal.pack`, `pal.variant`. Pour les essais ou une démo :
`ThemeClient::from_path(chemin)`.

## 4. Ligne de commande (scripts)

```sh
hyprdesk-theme-engine palette --format env   # HYPRDESK_BG=… HYPRDESK_THEME_ID=…
hyprdesk-theme-engine palette --format css   # :root { --hd-<slot>: #…; }
hyprdesk-theme-engine palette --format json  # le document du §2
hyprdesk-theme-engine palette --format hex   # « slot #hex », une ligne par slot
hyprdesk-theme-engine list --json            # packs disponibles (lecture)
hyprdesk-theme-engine verify --json          # contrastes de la palette publiée
```
Code de sortie 4 : aucune palette publiée. Une application ne doit **pas**
appeler `use`, `apply`, `install` ou `remove` : c'est l'utilisateur qui
choisit son thème.

## 5. Variables au lancement

Une application lancée par hyprdesk (lanceur, dock, démarrage) reçoit
`HYPRDESK_THEME_ID`, `HYPRDESK_THEME_VARIANT`, `HYPRDESK_BG`, `HYPRDESK_FG`,
`HYPRDESK_ACCENT`, `HYPRDESK_SURFACE` et `HYPRDESK_BORDER`. Elles sont lues
une fois, au démarrage, et ne suivent pas les changements à chaud (utiliser
le §2 ou le §3 pour ça). Elles sont absentes si l'application n'a pas été
lancée par hyprdesk.

## 6. GTK et Qt : déjà couverts

- **GTK 3/4** : le moteur installe le thème GTK de base de l'utilisateur,
  recoloré (`~/.local/share/themes/hyprdesk-<pack>`), et le sélectionne par
  `gtk-theme`. Les applications ouvertes se recolorent à chaud. Ne pas forcer
  `GTK_THEME` ni embarquer de CSS qui fige les couleurs.
- **libadwaita** : il suit `color-scheme` et `accent-color` (gsettings).
- **Qt** : schéma `~/.config/qt6ct/colors/hyprdesk.conf`, à choisir dans
  qt6ct. Il est pris au redémarrage de l'application.
- Le portail `org.freedesktop.appearance` n'est **pas encore** servi par
  hyprdesk (ticket MLG-236) : ne pas en dépendre.

## 7. Logiciel sans API : une cible générique dans un pack

Un pack est fait de **données** : un manifeste `theme.toml` et des gabarits.
Jamais de code.

```toml
[targets.<nom>]                    # identifiant [a-z0-9_-]
template = "templates/<nom>.tmpl"  # relatif au dossier du pack, 1 Mio au plus
output   = "~/.config/<nom>/hyprdesk-colors.css"
reload   = "signal:<processus>:SIGUSR1"   # ou none | socket:kitty | gsettings | hyprctl | daemon
```
Gabarit : placeholders `{{chemin|filtre|…}}`, **sans** `if` ni `for`.
- Chemins : `palette.<slot>`, `effects.<surface>.opacity|blur`,
  `meta.id|name|version|description`, `variant`.
- Filtres : `alpha(0.4)`, `lighten(8%)`, `darken(8%)`, `saturate(±%)`,
  `hue(±deg)`, `mix(palette.bg 20%)`, `contrast(4.5)`, `pick(sombre, clair)`,
  `fmt(hex|rgb|hsl|hypr|argb)`.
- Exemple : `@define-color accent {{palette.accent|fmt(rgb)}};`

Contraintes que le moteur fait respecter :
- **Sortie** sous `~/.config` ou `~/.local/share` uniquement, sans `..` et
  sans lien qui en sorte.
- **Dossiers refusés** : `autostart`, `systemd`, `environment.d`, `uwsm`,
  `hypr`, `kitty`, `fish`, `nushell`, `plasma-workspace`,
  `xdg-desktop-portal`, `Thunar`, `hyprdesk` (et sous `~/.local/share` :
  `applications`, `dbus-1`, `systemd`, `flatpak`, `nautilus`, `hyprdesk`).
- **Extensions refusées** : `.desktop`, `.service`, `.timer`, `.socket`,
  `.path`, `.sh`, `.bash`, `.zsh`, `.fish`, `.nu`, `.lua`, `.py`, `.pl`,
  `.rb`, `.js`, `.vim`, `.el`, `.so`.
- **Rechargement** : uniquement les formes listées, jamais de commande
  shell. Le fichier de sortie doit être **inclus** par la config de
  l'application (`include`, `@import`…) ; le moteur n'édite pas la config
  principale.

Limite actuelle : une cible générique n'est appliquée que si son pack (ou un
pack qui en hérite) est actif. Il n'existe pas de registre global des
applications. Le pack `perso` étant réécrit par hyprdesk à chaque retouche
dans les Réglages, ne pas y déclarer de cible.

## 8. Critères de réussite

- Au lancement, l'application prend les couleurs de `palette.json`. Sans
  fichier, elle garde son thème par défaut, sans erreur.
- `hyprdesk-theme-engine use ambre`, puis `use clair` : l'application se
  recolore sans redémarrer, une seule fois par révision. Clair et sombre
  restent lisibles (pas de couleur codée en dur).
- Un `schema` inconnu ou une couleur invalide est signalé, jamais deviné.
- Aucune écriture dans `~/.local/state/hyprdesk/`, aucun appel à `use`,
  `apply`, `install` ou `remove`.
- Essais : lecture d'un `palette.json` de test, et changement de révision
  publié **par renommage** (fichier temporaire puis `rename`), comme le fait
  le moteur.
