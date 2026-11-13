# hyprdesk

[English version](README.md)

[![Licence : MIT](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)
![Hyprland 0.57+](https://img.shields.io/badge/Hyprland-0.57%2B-58e1ff)
![Rust 1.99+](https://img.shields.io/badge/Rust-1.99%2B-dea584)

**Un bureau que l'on fait apparaître dès qu'on en a besoin, dans
[Hyprland](https://hyprland.org).**

En tuiles, les fenêtres couvrent l'écran : `SUPER + D` (deux fois : le dock,
puis le bureau complet) ramène le fond d'écran, les icônes et les widgets. Les
fenêtres passent en flottant par-dessus le bureau le temps de l'utiliser, et
retrouvent leur mosaïque quand vous le quittez. Chaque workspace se souvient
de l'état où vous l'avez laissé.

Le tiroir de réglages dispose vos écrans, change la disposition du clavier ou
ajuste les raccourcis, avec le résultat sous les yeux au fil des changements.
Pour un conflit de raccourcis ou un réglage peu familier, ouvrez LYRA,
l'assistante facultative du bureau, depuis le dock.

[Installer](#installation) · [Prise en main](#prise-en-main) · [Configuration](#configuration) · [LYRA](#lyra) · [myMFA](#le-trousseau-mymfa) · [Contribuer](#contribuer)

![bureau hyprdesk : barre en haut, raccourcis d'applications, horloge, calendrier, widgets système, et un dock](assets/screenshots/desktop.png)

## Installation

Lancez l'installateur depuis un terminal, dans votre session Hyprland. Il
vérifie le système et montre les options retenues avant de modifier quoi que
ce soit.

| Critère | Détails |
| --- | --- |
| Système | Linux x86_64 ou aarch64, systemd, une session utilisateur ordinaire, et un accès `sudo` |
| Bureau | Hyprland 0.57+ et un pilote Vulkan ou GL |
| Gestionnaire de paquets | pacman (Arch), apt (Debian/Ubuntu), dnf (Fedora) ou zypper (openSUSE) |
| Construction | environ 4 Gio libres, 8 Gio si LYRA est à compiler, et un accès à GitHub et crates.io |

```bash
git clone https://github.com/jacquesh82/hyprdesk
cd hyprdesk
./scripts/install.sh
```

L'assistant propose la langue et le clavier, passe en revue les dépendances et
demande quelles intégrations activer. Si Hyprland est plus ancien que 0.57,
l'assistant propose de le mettre à jour ; reconnectez-vous ensuite à Hyprland.

L'installateur compile hyprdesk, l'installe dans `~/.local/bin` et met en place
son service systemd utilisateur. Il crée `~/.config/hypr/hyprdesk.lua` s'il
manque et sauvegarde `hyprland.lua` avant d'y toucher
(`hyprland.lua.avant-hyprdesk-*.bak`). Le bilan final montre les composants en
marche et ce qu'il reste à faire. Au premier démarrage, les Réglages
s'ouvrent d'eux-mêmes.

<details>
<summary>Options d'installation et intégrations</summary>

```bash
./scripts/install.sh --help
```

Avec des options, ou sans terminal, l'assistant est sauté. L'installateur
peut installer les dépendances de construction qui manquent et utiliser rustup
dans `~/.cargo` si la Rust du système est trop ancienne.

| Option | Ce qu'elle contrôle |
| --- | --- |
| `--defaults` | choix par défaut, sans assistant |
| `--lang fr\|en\|auto` | langue de l'interface et de l'installateur |
| `--keyboard fr:azerty` | clavier de la session, `disposition[:variante[:options]]` |
| `--system-keyboard` | applique la même disposition à tout le système |
| `--greeter` / `--no-greeter` | active ou saute l'écran de connexion greetd, qui utilise cage |
| `--optional-deps ask\|all\|none` | comment traiter les dépendances facultatives |
| `--no-lyra` | passe l'assistante LYRA |
| `--no-mymfa` | passe le trousseau myMFA, l'intégration Secret Service et le module PAM |
| `--no-hyprbars` | passe le plugin de barres de titre Hyprbars |
| `--no-deps` | gérez les prérequis vous-même ; `scripts/deps.sh --dry-run` les vérifie |
| `--no-autostart` | passe l'entrée de démarrage de Hyprland |
| `--no-lock-key` | passe le raccourci de verrouillage `SUPER + L` |
| `--remove-hyprpaper` | retire hyprpaper du démarrage automatique |
| `--verbose` | sortie complète des commandes au terminal |

- **LYRA** est compilée depuis [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools). `HYPRDESK_LYRA_SRC` pointe vers une copie locale, `HYPRDESK_LYRA_REF` choisit la branche.
- **myMFA** est un trousseau pour mots de passe et codes 2FA. `HYPRDESK_MYMFA_SRC` pointe vers une copie locale de [mindlog.mymfa.clients](https://github.com/lelabnet-creator/mindlog.mymfa.clients). Si l'installateur n'atteint pas le dépôt, il en note la raison au journal et continue.
- **Hyprbars** ajoute les boutons réduire, agrandir et fermer en bureau complet. L'installateur le compile avec `hyprpm` contre le Hyprland qui tourne ; la mise à jour le recompile quand cette version change.

`HYPRDESK_INSTALLER_TUI=0` remplace whiptail par des questions au terminal.
`NO_COLOR=1` ou `HYPRDESK_COLOR=0` coupe les couleurs, et `HYPRDESK_ASCII=1`
choisit les icônes ASCII.

</details>

## Prise en main

| Raccourci | Action |
| --- | --- |
| `SUPER + D` | Mode suivant : masqué → dock (fenêtres en place) → bureau complet (les fenêtres flottent par-dessus le fond, les icônes et les widgets) → masqué |
| `SUPER + ALT + D` | Fait apparaître ou disparaître le dock, fenêtres en place |
| `SUPER + CTRL + D` | Masque le bureau et le dock |
| `SUPER + SHIFT + D` | Ouvre les Réglages |
| `SUPER + R` | Recherche des applications, dans tous les modes. `:clip` fouille l'historique du presse-papiers |
| `ALT + Tab` | Change de fenêtre |
| `SUPER + Tab` | Aperçu des fenêtres — hyprexpo, Hyprspace, ou le sélecteur de hyprdesk |
| `SUPER + Échap` | Sortie de secours |

Le mode du bureau est mémorisé par workspace, d'une session à l'autre. Avec
plusieurs écrans, chacun suit le workspace qu'il affiche. **Réglages → Bureau →
Modes imposés** fixe le mode d'un écran ou d'un workspace précis. Une fenêtre
déplacée ou redimensionnée en bureau complet garde sa place, par application
et par workspace ; « Oublier ces places » les efface (**Réglages → Bureau**).

**Réglages → Bureau → Raccourcis** et `hyprdesk keys` montrent les liaisons
existantes et leur ligne d'origine dans `hyprland.lua`. hyprdesk respecte les
liaisons en place, sauf celles que vous lui confiez et son `SUPER + R` par
défaut, que l'on peut désactiver.

<details>
<summary>Navigation au clavier dans le dock</summary>

Ouvrez le dock en mode lanceur seul, ou par Tab depuis le bureau complet, pour
lui donner le clavier. Flèches, Tab et Maj + Tab changent de case, Début et
Fin vont aux bouts, Entrée ou Espace activent une case. Échap referme le dock
ou rend le clavier au bureau complet.

</details>

## Ce que vous pouvez faire

- **Faites le bureau vôtre.** Fonds d'écran et diaporamas par écran ou par
  workspace, raccourcis d'applications et de fichiers, thème tiré d'une
  image. Les modules de la barre se réorganisent au glisser, le dock
  regroupe les fenêtres par application.
- **Réglez Hyprland depuis les Réglages.** Disposez les écrans, réglez clavier
  et pavé tactile, choisissez un layout par workspace (Dwindle, Master,
  Scrolling ou Monocle), ajustez animations, raccourcis et délais de veille.
  Un changement de disposition des écrans revient seul au bout de 12 secondes
  si vous ne le gardez pas.
- **Préparez la session.** Démarrage automatique, capot du portable, écran de
  verrouillage avec lecteur d'empreintes, notifications, captures et
  enregistrement d'écran. L'installateur propose aussi un écran de connexion
  greetd et un [trousseau myMFA](#le-trousseau-mymfa).
- **Ajoutez des widgets en Lua.** Commencez par l'horloge, le calendrier, le
  système ou le lecteur, puis ajoutez vos propres widgets et modules de barre
  comme de simples fichiers Lua.

| Réglages des écrans | Widgets |
| --- | --- |
| ![Disposition et réglages des écrans](assets/screenshots/screens.png) | ![Réglages des widgets, avec horloge, calendrier et système sur le bureau](assets/screenshots/widgets.png) |

<details>
<summary>Plus de captures</summary>

| Bureau et dock | Réglages et apparence |
| --- | --- |
| ![Mode lanceur seul](assets/screenshots/mode-launcher.png) | ![Réglages → Bureau](assets/screenshots/settings-desktop.png) |
| ![Mode bureau masqué](assets/screenshots/mode-hidden.png) | ![Réglages → Clavier et souris](assets/screenshots/settings-input.png) |
| ![Réglages → Barre](assets/screenshots/bar.png) | ![Layouts par workspace](assets/screenshots/layouts.png) |
| ![Lanceur d'applications](assets/screenshots/launcher.png) | ![Réglages → Animations](assets/screenshots/settings-animations.png) |
| ![Icônes du bureau](assets/screenshots/icons.png) | ![Réglages → Lanceur (dock)](assets/screenshots/settings-launcher.png) |
| ![Réglages → Fond d'écran](assets/screenshots/wallpaper.png) | ![Réglages → Thème](assets/screenshots/theme.png) |
| ![Transitions du bureau et fondu entre les fonds](assets/screenshots/effects.png) | ![À propos et informations système](assets/screenshots/settings-about.png) |

</details>

## Ce que hyprdesk remplace

hyprdesk prend en charge ce qui demandait, dans une session, un outil à lui
tout seul. La page Configuration de Hyprland signale ce qui reste en marche,
et LYRA peut le ranger avec vous.

**Remplacés** — hyprdesk fait le travail :

- **Fond d'écran** (`hyprpaper`) : dessiné nativement, par workspace et par
  écran, avec diaporamas et fondus ; l'installateur peut retirer hyprpaper de
  l'autostart.
- **Écran de connexion** (`GDM`, `SDDM`, `LightDM`) : un greeter sous greetd
  et cage prend la place au prochain démarrage ; `sudo hyprdesk greeter
  setup restore` revient à l'écran d'avant.
- **Écran de verrouillage** (`hyprlock`) : verrouillage natif
  (ext-session-lock-v1) avec PAM et empreinte au besoin ; la liaison
  SUPER + L est réécrite vers `hyprdesk lock`.
- **Secret Service** (`gnome-keyring`) : avec myMFA, son démon tient le
  Secret Service — hyprdesk importe les secrets de gnome-keyring, masque ses
  unités, et un clic les lui rend.
- **Couleurs des écrans** (`wlsunset`, `gammastep`, `redshift`) : luminosité,
  contraste et gamma par écran, par tables gamma ; `hyprdesk colors free`
  arrête — et peut désactiver — un programme qui les tient encore.
- **Disposition des écrans** (écrite dans `monitors.lua`) : position,
  rotation, échelle, écran principal et branchement à chaud, appliqués en
  direct ; un `monitors.lua` existant est repris, pas écrasé.

**Fournis, en cohabitation** — hyprdesk ne prend le service que si personne
ne le tient : le démon de notifications (`mako`, `dunst`… restent maîtres
s'ils tournent), l'agent polkit (un agent déjà enregistré garde la session),
et le StatusNotifierWatcher de la zone de notification.

**Orchestrés** — l'outil externe reste, hyprdesk le pilote : `hypridle`
(configuration écrite et supervisée), `cliphist` + `wl-clipboard`
(l'historique derrière `:clip`), `hyprsunset` (éclairage nocturne),
`hyprshot` et `wf-recorder` (captures, enregistrement). La barre, le dock et
le lanceur sont natifs aussi : la page Configuration de Hyprland signale les
barres en double (waybar, ags, eww), et une liaison de lanceur existante est
reprise.

## Configuration

Ouvrez les Réglages depuis le dock, ou lancez `hyprdesk settings screens` pour
aller droit à la page des écrans. Les changements du tiroir s'appliquent en
direct et s'enregistrent d'eux-mêmes dans `settings.lua`. La bascule
**Configuration avancée** de l'en-tête révèle les pages expertes.

Pour un setup versionnable dans vos dotfiles, éditez
`~/.config/hypr/hyprdesk.lua`.

```lua
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.bar({ left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })
hd.icon({ app = "firefox" })
hd.widget("clock", { anchor = "top-right", format = "%H:%M" })
```

Votre configuration Lua prime sur les réglages enregistrés par le tiroir. Les
deux fichiers sont relus à l'enregistrement. Une configuration fautive est
signalée avec son fichier et sa ligne, et la dernière configuration valide
reste active. Voir la [référence de configuration](docs/ADVANCED.md#configuration-reference)
(en anglais).

<details>
<summary>Contrôles de session et récupération</summary>

- **Démarrage automatique** liste les programmes supervisés par hyprdesk, les
  entrées éligibles du bloc `hl.on("hyprland.start", …)` de `hyprland.lua`,
  et les entrées XDG. Les programmes de hyprdesk vont dans `autostart.lua`,
  chargé par `require("autostart")`. « Lancer » essaie une commande.
- **Couleurs des écrans** règle luminosité, contraste, gamma et chaque canal.
  Il accepte les courbes `.cal` (ArgyllCMS) et confie les profils `.icc` ou
  `.icm` à Hyprland via `monitors.lua`. `hyprdesk color-test` affiche la mire
  plein écran, `hyprdesk colors diag` diagnostique les conflits de tables
  gamma. La page peut arrêter un programme qui les tient, avec une option
  séparée pour désactiver aussi son service utilisateur ou son entrée de
  démarrage automatique.
- **Capot du portable** choisit ce qui se passe sur batterie, sur secteur, ou
  avec un écran externe branché : comme le système, mise en veille,
  hibernation, verrouillage, extinction de l'écran du portable, ou rien.
  hyprdesk verrouille avant de mettre en veille.
- **Configuration de Hyprland** signale les erreurs, les liaisons ou démarrages
  en double, et les programmes qui font doublon avec les services intégrés.
  « Tout ranger » sauvegarde `hyprland.lua` et applique les corrections sûres.
- **Restaurer la configuration par défaut**, dans **Réglages → À propos** ou
  par `hyprdesk config reset --yes`, remet le `hyprdesk.lua` d'exemple et
  efface les réglages du tiroir. Les deux fichiers sont sauvegardés sous
  `~/.config/hypr/hyprdesk/sauvegardes/<date>/`. Les fichiers de Hyprland, les
  thèmes et les widgets restent hors de la remise à zéro.

</details>

## LYRA

Ouvrez LYRA depuis le dock, ou lancez `hyprdesk assistant`. L'assistante
explique un réglage ou aide à résoudre un problème avec le contexte de la
session : écrans, workspaces, raccourcis, applications. `hyprdesk context`
montre ce que hyprdesk lui fournit.

Quand LYRA propose une commande ou un changement, vous le relisez et
l'approuvez dans le tiroir. Certains changements viennent avec un aperçu en
direct, à accepter ou à rejeter. Des boutons « Demander à LYRA » ouvrent une
conversation déjà remplie des détails utiles ; les scripts font de même avec
`hyprdesk ask <titre> <texte>`.

Choisissez Claude Code, Codex ou Zcode dans **Réglages → IA → Assistant** :
LYRA passe par l'abonnement CLI que vous avez déjà. **Réglages → IA → Autres
services IA** choisit les fournisseurs des autres fonctions IA — Anthropic,
OpenAI, Z.ai, et tout endpoint compatible OpenAI comme Ollama. Les clés API
de ces fournisseurs restent dans le trousseau de la session.

Le **SmartUI design** redessine les icônes des applications dans un style
cohérent. Les icônes validées forment le thème `hyprdesk-smartui` sous
`~/.local/share/icons`, utilisé partout : bureau, dock, lanceur, barre et
notifications. La génération automatique est active par défaut quand LYRA est
installée.

![Tiroir de LYRA : aperçu en direct et demande d'approbation d'une commande](assets/screenshots/assistant-chat.png)

<details>
<summary>Captures des réglages IA</summary>

| Assistant | Autres services IA | SmartUI design |
| --- | --- |
| ![Assistant et abonnement CLI](assets/screenshots/assistant-settings.png) | ![Fournisseurs](assets/screenshots/settings-ai-services.png) | ![Génération des icônes SmartUI](assets/screenshots/settings-smartui.png) |

</details>

## Le trousseau myMFA

myMFA est un trousseau facultatif de mots de passe et de codes 2FA. Installé
avec hyprdesk depuis son [dépôt public](https://github.com/lelabnet-creator/mindlog.mymfa.clients)
(`--no-mymfa` pour l'écarter), son démon tient le Secret Service — les
secrets que vos applications confient vont au coffre myMFA — et le module
`pam_mymfa` peut ouvrir ce coffre à la connexion. hyprdesk en est
l'interface :

- **La fenêtre myMFA** (`hyprdesk mymfa window`) montre les codes 2FA, les
  mots de passe, le tableau de bord et l'activité du coffre, cet appareil,
  ses réglages, et un scanner qui ajoute un compte depuis un QR (image ou
  capture d'écran). Le module `mymfa` de la barre montre l'état du coffre,
  et une carte de déverrouillage au style de hyprdesk répond aux
  applications qui le réclament.
- **Le lanceur** trouve codes et mots de passe avec `:2fa` et `:mdp`. La
  copie est faite par le démon lui-même, et un mot de passe copié s'efface
  du presse-papiers au bout de 30 secondes.
- **Réglages → Mots de passe** crée le coffre, ou installe la sauvegarde de
  [l'application web myMFA](https://app.mymfa.mindlog.today). Il peut aussi
  reprendre le Secret Service de gnome-keyring — import de ses secrets, puis
  arrêt — avec un bouton pour revenir en arrière.
- L'onglet **Appareil** lie le coffre de cette machine à celui du web,
  fusionne les deux, et montre le journal de synchronisation.

## Widgets et plugins

Activez les widgets dans **Réglages → Widgets** ou ajoutez-les à votre
configuration Lua. Les widgets intégrés sont `clock`, `calendar`, `system`,
`media` (MPRIS), `command` (sortie périodique d'une commande) et `lua`
(sortie d'une fonction en ligne). Un widget du bureau va sur l'écran
principal par défaut ; on peut viser un écran précis, ou tous, et choisir
s'il déménage ou se masque quand l'écran est débranché.

```lua
hd.widget("clock", { screen = "DP-2", if_missing = "primary" })
hd.widget("calendar", { screen = "DP-2", if_missing = "hide" })
```

La barre propose `workspaces`, `window`, `volume`, `brightness`, `tray` et
`desktop`. Pour ajouter un widget ou un plugin en Lua :
[Écrire un plugin](docs/WRITING_PLUGINS.md) (en anglais).

<details>
<summary>Plugins livrés et leurs dépendances</summary>

| Plugin | Rôle | S'appuie sur |
| --- | --- | --- |
| `reseau` | Réseaux filaire et Wi-Fi, VPN, WireGuard, mode avion | NetworkManager |
| `bluetooth` | Recherche, appariement et gestion des appareils | bluetoothctl |
| `batterie` | État des batteries, alertes et profils d'énergie | sysfs, power-profiles-daemon |
| `capture` | Captures d'écran et enregistrement | hyprshot, wf-recorder |
| `clavier` | Voir et changer de disposition clavier | Hyprland |
| `lecteur` | Morceau en cours et commandes de lecture | MPRIS |
| `majs` | Mises à jour en attente et mise à jour en un clic | pacman/AUR, apt, dnf, zypper, flatpak |
| `supports` | Ouvrir et éjecter les disques amovibles | udisks2 |
| `nuit` | Éclairage nocturne, à la demande ou selon un horaire | hyprsunset |
| `vkbd` | Clavier virtuel | wvkbd |
| `notifications` | Dernières notifications et Ne pas déranger | hyprdesk ou wayle |
| `session` | Verrouiller, mettre en veille, déconnecter, redémarrer, éteindre | systemd, verrouillage hyprdesk |

L'installateur place deux exemples modifiables, `meteo` et `notes`, dans
`~/.config/hypr/hyprdesk/widgets/`.

</details>

## Ligne de commande

| Commande | À quoi elle sert |
| --- | --- |
| `hyprdesk show` / `hyprdesk launcher-only` / `hyprdesk hide` | Règle le mode du bureau du workspace |
| `hyprdesk settings [paquet]` | Ouvre les Réglages, ou saute à une page |
| `hyprdesk theme new <image>` | Crée un thème depuis une image |
| `hyprdesk wallpaper split <image>` | Découpe une scène selon la disposition des écrans |
| `hyprdesk export [fichier]` / `hyprdesk import <archive>` | Partager ou restaurer un setup. `--theme` pour le thème seul, `--screens` pour les écrans |
| `hyprdesk check [fichier]` | Vérifier une configuration Lua sans toucher au démon |
| `hyprdesk status` / `hyprdesk doctor` | État de la session, ou diagnostic des dépendances |
| `hyprdesk --help` | La liste complète des commandes et options |

## Mettre à jour et dépanner

Depuis le dépôt cloné, `./scripts/update.sh` relève puis applique les mises à
jour. Le script réutilise les options de votre installation et s'arrête si le
dossier contient des modifications locales. `--check` ne donne que le relevé
(code de sortie `10` : mise à jour disponible), `--yes` applique sans
question.

- **L'installation échoue.** Le journal `~/.local/state/hyprdesk/install-<date>.log`
  montre l'étape fautive et sa sortie complète ; corrigez, puis relancez.
- **hyprdesk ne démarre pas.** `journalctl --user -u hyprdesk -n 50`, puis
  `hyprdesk doctor`.
- **myMFA est absent.** Vérifiez **Réglages → Mots de passe** et le journal
  d'installation. Si l'installateur n'atteint pas le dépôt myMFA, relancez-le
  avec `HYPRDESK_MYMFA_SRC` pointant vers une copie locale.
- **Hyprbars est absent.** Dans un terminal sous Hyprland :
  `hyprpm update`, puis
  `hyprpm add https://github.com/hyprwm/hyprland-plugins`.
- **Revenir à l'écran de connexion d'avant.** `sudo hyprdesk greeter setup restore`.

<details>
<summary>Désinstaller hyprdesk</summary>

Si l'écran de connexion de hyprdesk est en place, revenez d'abord à celui
d'avant avec `sudo hyprdesk greeter setup restore`, puis retirez le binaire.

```bash
systemctl --user disable --now hyprdesk.service
rm ~/.local/bin/hyprdesk
```

Retirez ensuite de `hyprland.lua` les entrées posées par hyprdesk, ou
reprenez une copie `hyprland.lua.avant-hyprdesk-*.bak`. Vous pouvez garder
votre `hyprdesk.lua` et votre `settings.lua` pour une installation ultérieure.

</details>

## Contribuer

Pour un rapport de bogue, indiquez ce que vous attendiez, ce qui s'est passé,
et le plus court chemin pour le reproduire. **Réglages → À propos** copie les
versions de hyprdesk, du système, de Hyprland et de LYRA pour votre rapport.
[Ouvrir un ticket](https://github.com/jacquesh82/hyprdesk/issues).

Utilisez **Rust 1.99 ou plus** et des builds verrouillés sur `Cargo.lock`. Le
lockfile épingle `winit-core` en `0.31.0-beta.2`, requis par la pile
d'interface.

```bash
cargo test --locked
bash scripts/verify.sh
cargo run --locked -- preview
```

Le script de vérification enchaîne format, Clippy strict, tests, build et
contrôles D-Bus isolés. `--visual` ajoute les captures des vues de
démonstration à 1440, 900 et 640 px, sous `/tmp/hyprdesk-visual` par défaut.
L'aperçu interactif emploie des données de démonstration et ne touche pas aux
réglages de la session. Le flou du compositeur et le multi-écran se
vérifient dans une session Hyprland.

Le [guide de développement](docs/ADVANCED.md#development) (en anglais)
détaille build et vérifications, et le [guide de contribution](CONTRIBUTING.md)
(en anglais) le parcours des pull requests.

## Documentation

- [Guide avancé](docs/ADVANCED.md) (anglais)
- [Écrire un plugin](docs/WRITING_PLUGINS.md) (anglais)
- [Plugins, widgets et lanceur](docs/PLUGINS.md) — conception interne (français)
- [Journal des versions](CHANGELOG.md) (anglais)
- [Guide de contribution](CONTRIBUTING.md) (anglais)

## Crédits et licence

hyprdesk est sous [licence MIT](LICENSE). La police
[Mindlog](https://github.com/jacquesh82/mindlog.fonts), embarquée, est sous
[SIL Open Font License 1.1](assets/fonts/mindlog/OFL.txt).
[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)
fournit LYRA et la bibliothèque d'IPC Hyprland `hyprpanel-core`.
