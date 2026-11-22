# hyprdesk

[English version](README.md)

[![Vérifications](https://github.com/jacquesh82/hyprdesk/actions/workflows/ci.yml/badge.svg)](https://github.com/jacquesh82/hyprdesk/actions/workflows/ci.yml)
[![Licence : GPL v3+](https://img.shields.io/badge/licence-GPL--3.0--or--later-blue)](LICENSE)
![Hyprland 0.57+](https://img.shields.io/badge/Hyprland-0.57%2B-58e1ff)
![Rust 1.99+](https://img.shields.io/badge/Rust-1.99%2B-dea584)

**Un bureau complet pour [Hyprland](https://hyprland.org), qui reste en
retrait jusqu'à ce que vous l'appeliez.**

[![Connexion sur l'écran greetd de hyprdesk devant une vidéo en boucle, SUPER + D vers le dock puis le bureau complet, un fichier trouvé par la recherche :f, LYRA qui corrige un raccourci en double après accord, SmartUI qui propose de nouvelles icônes, un code myMFA saisi dans un formulaire, puis le carton final](assets/screenshots/hyprdesk-demo.gif)](assets/screenshots/hyprdesk-demo.mp4)

<sub>Film de 70 secondes, en anglais ; cliquer dessus pour la vidéo avec le son.</sub>

hyprdesk fait de Hyprland un bureau complet sans renoncer aux tuiles. Il
réunit barre, dock, lanceur, icônes et widgets du bureau, notifications,
écrans de connexion et de verrouillage, et un tiroir de réglages. Tout vient
d'un seul programme natif en Rust, réglé depuis le tiroir ou en Lua.

Vos fenêtres restent en mosaïque. `SUPER + D` fait monter le bureau
par-dessus ; quittez-le, elles reprennent leur place. hyprdesk écrit aussi
la configuration de Hyprland à votre place : raccourcis, clavier, écrans,
layouts. Hyprland vérifie chaque écriture, et tout changement peut être
annulé.

[Points forts](#points-forts) · [Installation](#installation) · [Prise en main](#prise-en-main) · [Configuration](#configuration) · [Guide d'utilisation](docs/GUIDE.fr.md) · [Contribuer](#contribuer)

![bureau hyprdesk : barre en haut, raccourcis d'applications, horloge, calendrier, widgets système, et un dock](assets/screenshots/desktop.png)

## Points forts

### Un bureau à la demande

`SUPER + D` passe par trois modes : masqué, dock seul, et bureau complet
avec fond d'écran, icônes et widgets. En bureau complet, les fenêtres
flottent par-dessus et retrouvent leur mosaïque quand vous le quittez.
Chaque workspace garde son mode d'une session à l'autre, et chaque écran
suit le workspace qu'il affiche. Les fonds (images, diaporamas, **vidéos en
boucle**) se règlent par écran ou par workspace, et un thème se tire de
n'importe quelle image.

### Hyprland réglé depuis les Réglages

Disposez les écrans, réglez clavier et pavé tactile, changez un raccourci en
appuyant sur la nouvelle combinaison, choisissez un layout par workspace ou
découpez un écran en zones (de 1×2 à 3×3). Animations, bordures, règles de
fenêtres, profil de rendu et carte graphique qui dessine chaque image se
règlent aussi. hyprdesk écrit les fichiers que charge `hyprland.lua`
(`binds.lua`, `inputs.lua`, `monitors.lua`, `autostart.lua`…) et les fait
vérifier par Hyprland. Si Hyprland signale une nouvelle erreur, le fichier
d'avant revient. Un changement d'écrans s'annule seul au bout de
12 secondes si vous ne le gardez pas, et 30 jours d'instantanés de la
configuration permettent de revenir à n'importe quel point.

### Panorama : une seconde vie pour les écrans bandeau et tactiles

Un écran très allongé (2560×720, 1920×480…), tactile ou non, devient un
plein écran à pages qu'on fait glisser. Il peut afficher un lecteur, des
services vidéo et musique, des webradios, des graphiques système avec un
gestionnaire de tâches, des horloges et des raccourcis tactiles. Les
compositions sont libres (forest, telemetry, studio, cinématique) et
l'écran se découpe jusqu'à neuf zones. Pas d'écran bandeau ? Réservez une
bande Panorama sur un écran ordinaire, ou essayez-le en fenêtre avec
`hyprdesk panorama preview`.

| ![Panorama sur un bandeau 2560×720 en trois zones : lecteur, horloge et cartes système](assets/screenshots/panorama-split.png) | ![Composition Forest : graphiques du processeur, de la mémoire, du disque et de la charge à côté d'une horloge à aiguilles](assets/screenshots/panorama-forest.png) |
| --- | --- |

### LYRA, une assistante qui connaît votre session

LYRA est facultative. Elle explique un réglage ou corrige un conflit de
raccourcis avec ce que hyprdesk sait de votre session : écrans, workspaces,
raccourcis, applications. Vous approuvez chaque commande qu'elle propose,
souvent après un aperçu en direct. Elle passe par votre abonnement Claude
Code, Codex ou Zcode. Dites « Lyra » depuis le Panorama pour lui parler ;
la reconnaissance et la voix tournent hors ligne. **SmartUI** redessine les
icônes des applications dans un même style.

### Chaque fichier à une frappe

La loupe du dock, `:f` dans le lanceur ou `hyprdesk files` cherchent dans un
index de votre home et des volumes montés. Cherchez par nom, chemin ou
catégorie : la réponse arrive en quelques millisecondes. inotify tient
l'index à jour en arrière-plan.

### Connexion, verrouillage et trousseau

hyprdesk propose un écran de connexion greetd facultatif avec fond vidéo, et
un verrouillage natif avec PAM et empreintes, à enregistrer depuis les
Réglages. L'écran de verrouillage et l'écran de connexion sont de petits
binaires dédiés : redémarrer ou mettre à jour hyprdesk ne les emporte
jamais. Le trousseau facultatif **myMFA** ajoute mots de passe et codes 2FA
(`:2fa`, `:mdp` dans le lanceur), tient le Secret Service et s'ouvre à la
connexion.

### Lua de bout en bout

Les Réglages s'enregistrent dans `settings.lua`. Pour vos dotfiles,
`hyprdesk.lua` passe avant et se recharge à l'enregistrement ; une ligne
fautive est signalée avec son fichier et son numéro. Widgets et modules de
barre sont de simples fichiers Lua. Une douzaine de plugins sont livrés :
réseau, Bluetooth, batterie, mises à jour avec progression, captures,
éclairage nocturne, bascule de carte graphique en passthrough, et d'autres.

### Sans risque à essayer

Avant de toucher à quoi que ce soit, l'installateur garde l'état d'origine.
`./scripts/uninstall.sh --restore` remet chaque fichier, service et paquet
comme avant. Chaque page des Réglages a son bouton **Par défaut…**, et
`hyprdesk export` emballe tout votre setup (hyprdesk, Hyprland, thèmes,
images) dans une archive à importer sur une autre machine.

<details>
<summary>Plus de captures</summary>

| Bureau et dock | Réglages et apparence |
| --- | --- |
| ![Disposition et réglages des écrans](assets/screenshots/screens.png) | ![Réglages des widgets, avec horloge, calendrier et système sur le bureau](assets/screenshots/widgets.png) |
| ![Mode lanceur seul](assets/screenshots/mode-launcher.png) | ![Réglages → Bureau](assets/screenshots/settings-desktop.png) |
| ![Mode bureau masqué](assets/screenshots/mode-hidden.png) | ![Réglages → Clavier et souris](assets/screenshots/settings-input.png) |
| ![Réglages → Barre](assets/screenshots/bar.png) | ![Layouts par workspace](assets/screenshots/layouts.png) |
| ![Lanceur d'applications](assets/screenshots/launcher.png) | ![Réglages → Animations](assets/screenshots/settings-animations.png) |
| ![Icônes du bureau](assets/screenshots/icons.png) | ![Réglages → Lanceur (dock)](assets/screenshots/settings-launcher.png) |
| ![Réglages → Fond d'écran](assets/screenshots/wallpaper.png) | ![Réglages → Thème](assets/screenshots/theme.png) |
| ![Transitions du bureau et fondu entre les fonds](assets/screenshots/effects.png) | ![À propos et informations système](assets/screenshots/settings-about.png) |
| ![Composition Telemetry : horloge à volets, lecteur et jauges circulaires](assets/screenshots/panorama-telemetry.png) | ![Composition Studio : grande horloge et lecteur](assets/screenshots/panorama-studio.png) |
| ![Tiroir de LYRA : aperçu en direct et demande d'approbation d'une commande](assets/screenshots/assistant-chat.png) | ![Génération des icônes SmartUI](assets/screenshots/settings-smartui.png) |

</details>

## Installation

Lancez l'installateur depuis un terminal, dans votre session Hyprland. Il
vérifie le système et montre les options retenues avant de modifier quoi que
ce soit.

| Critère | Détails |
| --- | --- |
| Système | Linux x86_64 ou aarch64, systemd, une session utilisateur ordinaire, et un accès `sudo` |
| Bureau | Hyprland 0.57+ et un pilote Vulkan ou GL |
| Gestionnaire de paquets | pacman (Arch), apt (Debian/Ubuntu), dnf (Fedora) ou zypper (openSUSE) |
| Construction | Rust et ~4 Gio libres, seulement pour compiler le dépôt ou LYRA et myMFA |

```bash
git clone https://github.com/jacquesh82/hyprdesk
cd hyprdesk
./scripts/install.sh
```

`scripts/install.sh` télécharge le paquet de la
[dernière release](https://github.com/jacquesh82/hyprdesk/releases/latest),
l'installe avec votre gestionnaire de paquets et met en place le reste. Pour
compiler le dépôt en l'état, lancez plutôt
`scripts/install_nightly_build.sh`. L'assistant demande la langue, le
clavier et les intégrations. Par défaut, il n'installe que le bureau :
l'écran de connexion, LYRA et myMFA restent décochés tant que vous ne les
choisissez pas. Au premier démarrage, un assistant visuel passe en revue
écrans, clavier, thème, barre et applications par défaut.

| Distribution | Paquet de la release 0.1.9 |
| --- | --- |
| Debian, Ubuntu (amd64) | [hyprdesk_0.1.9-1_amd64.deb](https://github.com/jacquesh82/hyprdesk/releases/download/v0.1.9/hyprdesk_0.1.9-1_amd64.deb) |
| Fedora, openSUSE (x86_64) | [hyprdesk-0.1.9-1.x86_64.rpm](https://github.com/jacquesh82/hyprdesk/releases/download/v0.1.9/hyprdesk-0.1.9-1.x86_64.rpm) |
| Arch Linux (x86_64) | [hyprdesk-0.1.9-1-x86_64.pkg.tar.zst](https://github.com/jacquesh82/hyprdesk/releases/download/v0.1.9/hyprdesk-0.1.9-1-x86_64.pkg.tar.zst), ou le [paquet AUR](https://aur.archlinux.org/packages/hyprdesk) |

### Mettre à jour et désinstaller

```bash
./scripts/update.sh                # relève puis applique les mises à jour (--check, --yes)
./scripts/uninstall.sh --dry-run   # ce qui changerait, sans rien toucher
./scripts/uninstall.sh             # récapitulatif, confirmation, puis désinstallation
./scripts/uninstall.sh --restore   # tout remettre exactement comme avant hyprdesk
```

Les options d'installation, les intégrations et ce que rend la
désinstallation sont dans le
[guide d'utilisation](docs/GUIDE.fr.md#installation-en-détail).

## Prise en main

| Raccourci | Action |
| --- | --- |
| `SUPER + D` | Mode suivant : masqué → dock → bureau complet → masqué |
| `SUPER + ALT + D` | Fait apparaître ou disparaître le dock, fenêtres en place |
| `SUPER + CTRL + D` | Masque le bureau et le dock |
| `SUPER + SHIFT + D` | Ouvre les Réglages |
| `SUPER + SHIFT + L` | Ouvre ou referme LYRA, l'assistant IA |
| `SUPER + R` | Recherche des applications. `:f` fouille les fichiers, `:clip` l'historique du presse-papiers, `:2fa` et `:mdp` le trousseau myMFA |
| `ALT + Tab` | Change de fenêtre |
| `SUPER + Tab` | Vue des workspaces : glisser les fenêtres d'un workspace à l'autre |
| `Impr` / `SUPER + Impr` / `ALT + Impr` | Capture une zone, l'écran ou une fenêtre |
| `SUPER + L` | Verrouille la session |
| `SUPER + Échap` | Sortie de secours |

hyprdesk respecte les raccourcis déjà présents dans `hyprland.lua`. Là où il
en manque un essentiel (workspaces 1 à 10, maximiser, épingler, écran
suivant…), il le pose sur une combinaison libre. **Réglages → Raccourcis** et
`hyprdesk keys` listent toutes les liaisons actives avec leur ligne
d'origine.

## Configuration

Ouvrez les Réglages depuis le dock, ou sautez à une page avec
`hyprdesk settings screens`. Les changements s'appliquent en direct et
s'enregistrent dans `settings.lua`. La bascule **Configuration avancée** de
l'en-tête révèle toutes les pages.

Pour un setup versionné dans vos dotfiles, éditez
`~/.config/hypr/hyprdesk.lua`. Il est livré tout en commentaires, passe avant
les Réglages, et reste facultatif.

```lua
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.bar({ left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })
hd.icon({ app = "firefox" })
hd.widget("clock", { anchor = "top-right", format = "%H:%M" })
```

La [référence de configuration](docs/ADVANCED.md#configuration-reference)
(en anglais) liste toutes les clés, et [Écrire un plugin](docs/WRITING_PLUGINS.md)
(en anglais) couvre widgets et modules de barre.

### Ligne de commande

| Commande | À quoi elle sert |
| --- | --- |
| `hyprdesk show` / `launcher-only` / `hide` | Règle le mode du bureau du workspace |
| `hyprdesk settings [page]` | Ouvre les Réglages, ou saute à une page |
| `hyprdesk files` / `index query <mot>` | Cherche des fichiers depuis la loupe du dock, ou au terminal |
| `hyprdesk panorama [show\|hide\|preview]` | Pilote le Panorama, ou l'essaie en fenêtre |
| `hyprdesk assistant` / `ask <titre> <texte>` | Ouvre LYRA, ou lance une conversation depuis un script |
| `hyprdesk config get\|set\|list\|history\|reset` | Lit et change les réglages, parcourt les instantanés, revient aux valeurs par défaut |
| `hyprdesk export` / `import <archive>` | Partage ou restaure un setup entier (`--theme` pour le thème seul) |
| `hyprdesk check [fichier]` | Vérifie une configuration Lua sans toucher au démon |
| `hyprdesk status` / `doctor` | État de la session, ou diagnostic des dépendances |
| `hyprdesk --help` | Toutes les commandes |

## Documentation

- [Guide d'utilisation](docs/GUIDE.fr.md) : chaque fonction en détail,
  options d'installation, dépannage
- [Composer un bandeau Panorama](docs/PANORAMA.md)
- [Guide avancé](docs/ADVANCED.md) (anglais) : architecture et référence de
  configuration
- [Écrire un plugin](docs/WRITING_PLUGINS.md) (anglais) et
  [plugins, widgets et lanceur](docs/PLUGINS.md) (conception interne)
- [Journal des versions](CHANGELOG.md) (anglais)

## Contribuer

Pour un rapport de bogue, indiquez ce que vous attendiez, ce qui s'est passé,
et le plus court chemin pour le reproduire. **Réglages → À propos** copie les
versions de hyprdesk, du système, de Hyprland et de LYRA pour votre rapport.
[Ouvrir un ticket](https://github.com/jacquesh82/hyprdesk/issues).

Utilisez **Rust 1.99 ou plus** et des builds verrouillés sur `Cargo.lock`.

```bash
cargo test --locked --workspace
bash scripts/verify.sh
cargo run --locked -- preview
```

Le dépôt est un espace de travail Cargo : le paquet `hyprdesk`
(bibliothèque, binaire principal et binaires dédiés de `src/bin/`), plus
`crates/hyprdesk-auth` (PAM, fprintd) et `crates/hyprdesk-i18n`
(traductions). Le [guide de développement](docs/ADVANCED.md#development) et
le [guide de contribution](CONTRIBUTING.md) (en anglais) couvrent build,
vérifications et pull requests.

## Crédits et licence

Copyright © 2026 Jacques HULLU. hyprdesk est un logiciel libre, sous
[licence publique générale GNU, version 3 ou toute version ultérieure](LICENSE)
(GPL-3.0-or-later).

- [Hyprland](https://hyprland.org), le compositeur dans lequel vit hyprdesk ;
- [iced](https://github.com/iced-rs/iced), la boîte à outils d'interface, avec
  des copies patchées de `layershellev`, `iced_layershell` et `sessionlockev`
  ([exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)) dans
  `vendor/` (MIT) ;
- la police [Mindlog](https://github.com/jacquesh82/mindlog.fonts), embarquée
  sous [SIL Open Font License 1.1](assets/fonts/mindlog/OFL.txt) ;
- [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools),
  maison de LYRA et de l'IPC Hyprland `hyprpanel-core` ;
- le trousseau myMFA, de
  [mindlog.mymfa.clients](https://github.com/lelabnet-creator/mindlog.mymfa.clients).

La liste complète des dépendances vit dans `Cargo.lock`.
