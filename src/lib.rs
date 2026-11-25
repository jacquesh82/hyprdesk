//! hyprdesk — bibliothèque commune aux binaires : `hyprdesk` (démon et
//! client), `hyprdesk-lock` (écran de verrouillage), `hyprdesk-greeter`
//! (écran de connexion) et `hyprdesk-polkit` (agent d'authentification).
//!
//! Chaque binaire ne lie que le code qu'il atteint : l'écran de verrouillage
//! n'embarque ni le démon, ni Lua, ni l'index de fichiers.

pub mod a11y;
pub mod adopt;
pub mod ai_services;
pub mod apps;
pub mod apptheme;
pub mod assistant;
pub mod autostart;
pub mod bar_offer;
pub mod binds_file;
pub mod brightness;
pub mod capture;
pub mod chat;
pub mod cli_config;
pub mod cli_tools;
pub mod clipboard;
pub mod colors;
pub mod config;
pub mod control;
pub mod defaults;
pub mod design_options;
pub mod desktop_files;
pub mod doctor;
pub mod essentials;
pub mod fileindex;
pub mod fonts;
pub mod fx;
pub mod genfiles;
pub mod gpu;
pub mod greeter;
pub mod greeter_setup;
pub mod greeter_ui;
pub mod history;
pub mod hypr;
pub mod hyprbars;
pub mod hyprcheck;
pub mod hyprpm;
pub mod hyprset;
pub mod hyprsrc;
pub mod hyprwrite;
pub mod i18n;
pub mod idle;
pub mod inputs;
pub mod keymap;
pub mod launch;
pub mod layout;
pub mod lid;
pub mod lock;
pub mod mymfa;
pub mod notifications;
pub mod osd;
pub mod panorama;
pub mod perf;
pub mod policy;
pub mod polkit;
pub mod portal;
pub mod processes;
pub mod render;
pub mod render_gpu;
pub mod screens;
pub mod search;
pub mod session;
pub mod session_start;
pub mod settings;
pub mod share;
pub mod single;
pub mod smartui;
pub mod sources;
pub mod span;
pub mod storage;
pub mod switcher;
pub mod takeover;
pub mod theme;
pub mod theme_engine;
pub mod tiling;
pub mod tips;
pub mod tray;
pub mod ui;
pub mod update_progress;
pub mod updates;
pub mod video;
pub mod voice;
pub mod wallpaper_fx;
pub mod widgets;
pub mod windows;
pub mod zones;

// Traductions : `crate::tr!` et `crate::i18n` partout dans le code.
pub use hyprdesk_i18n::tr;

/// Aide de la ligne de commande (`hyprdesk --help`).
pub const USAGE: &str = "\
hyprdesk — bureau à la demande pour Hyprland

Usage : hyprdesk [--config <fichier>] [commande]

Commandes :
  daemon            lance le démon (par défaut) ; une seule instance par session
  daemon --replace  remplace proprement l'instance en place
  toggle            full-desktop, ou no-desktop s'il y est déjà
  toggle-launcher   launcher-only, ou no-desktop s'il y est déjà
  show | hide       full-desktop (fenêtres mises de côté) / no-desktop
  launcher-only     fenêtres en place, lanceur (dock) accessible en bas
  cycle             mode suivant : no-desktop → launcher-only → full-desktop
  settings [paquet[/onglet] | ?mots]
                    ouvre l'écran Réglages (ex. bureau/raccourcis, ou « ?gamma » : sur la recherche), sans changer le mode
  toggle-settings   ouvre les Réglages, ou les referme s'ils sont ouverts (raccourci keys.settings)
  focus <surface>  le clavier va à une surface du shell : bar (barre), dock, notifications,
                    settings (volet des Réglages) ; rien ne se referme, le mode ne change pas
  force-quit        ouvre la boîte « Forcer à quitter » (raccourci keys.force_quit), ou la
                    referme : une ligne par application, confirmation en deux temps
  keyboard-access   bascule l'accès clavier complet : Tab atteint chaque commande des Réglages
                    (raccourci keys.keyboard_access)
  a11y dump         imprime l'arbre d'accessibilité des surfaces, tel qu'un lecteur d'écran le lit
  assistant         ouvre ou referme l'assistant (tiroir LYRA), sans changer le mode
  search            ouvre ou referme le lanceur avec recherche (toutes les applications)
  files             ouvre ou referme la recherche de fichiers (loupe du dock, :f)
  apps              ouvre ou referme la grille des applications (plein écran, catégories)
  overview          vue d'ensemble des fenêtres (overview_command, sinon le sélecteur)
  notifications [list | dnd [on [1h|soir] | off] | clear]
                    notifications de hyprdesk : historique, Ne pas déranger (une heure, jusqu'à ce soir)
  ask <titre> <texte>
                    demande à LYRA (nouvelle conversation, affichée)
  switch [prev]     sélecteur de fenêtres (Alt+Tab, keys.switch) : ouvre, ou passe
                    à la fenêtre suivante (prev : précédente)
  tour              ouvre les réglages sur la visite guidée
  wizard            assistant de configuration : les premiers réglages, étape par étape
                    (paquet : desktop, icons, launcher, wallpaper, widgets, bar,
                    screens, workspaces, effects, theme…)
  export [fichier] [--theme] [--screens]
                    archive du setup à partager (configs, plugins, fonds d'écran ;
                    --theme : le thème seul — couleurs, fonds, effets ;
                    --screens : monitors.lua aussi)
  import <archive> [--theme] [--dry-run] [--screens]
                    applique un setup partagé (sauvegarde l'actuel d'abord) ;
                    --theme : n'en prendre que le thème (une archive de thème
                    ne change jamais que le thème)
  theme [list]      « Thèmes personnels »
  theme new <image|pack.zip|dossier> [--name NOM]
                    crée un thème depuis une scène : couleurs tirées de l'image,
                    scène étalée sur tous les écrans (bureau et verrouillage)
  theme apply <nom|archive>
                    applique un thème (le reste de la configuration ne change pas)
  fx compare <effet> [a,b,c|all] [--seconds N] [--rounds N]
                    compare un effet à l'œil : chaque valeur posée en direct et
                    nommée à l'écran (ex. : fx compare blur.variant 4,5,6),
                    puis la valeur d'origine remise
  wallpaper split <image> [dossier] [--scale N]
                    découpe une scène selon la disposition des écrans : une
                    image par écran (<écran>.png) ; --scale 2 pour du HiDPI
  context           décrit l'environnement hyprdesk (Markdown), pour un assistant IA
  lock              verrouille la session (écran de verrouillage de hyprdesk)
  open <rôle>       lance l'application par défaut : browser, terminal, files, editor,
                    image, video, audio, pdf, mail, ai
  config            réglages en ligne de commande, par paquet :
                    config [list [paquet]] | get <clé> | set <clé> <valeur> | unset <clé>
                    config history | diff <id> | rollback <id|date> [--yes] : historique sur 30 jours
  menu <module>     ouvre un menu de la barre (volume, clock, tray, reseau, session…)
  osd <volume|mic|brightness|kbd> <+N|-N|toggle>
                    règle le son ou la luminosité (touches XF86, keys.media)
  media <play-pause|next|previous|status|seek ±N>
                    commande le lecteur en cours (MPRIS) ; status : son état
                    (clé=valeur) ; seek : avance ou recule de N secondes
  voice [setup|say [--en] <texte>|status]
                    LYRA vocale : installe Piper et Vosk (hors ligne), fait
                    parler la voix, ou dit si tout est prêt
  key <touche> [mods]
                    envoie une touche à la fenêtre active (télécommande du
                    Panorama) : key Up, key Return, key f, key Tab SHIFT
  capture <zone|fenetre|ecran>
                    capture d'écran (Images/Captures, copiée ; touches Impr)
  capture enregistrer [zone] | capture arreter
                    enregistre l'écran actif ou une zone (Vidéos/Enregistrements),
                    ou arrête l'enregistrement en cours
  greeter [--tty | --demo] [--window]
                    écran de connexion pour greetd (connexion, choix de l'environnement) ;
                    --tty : au terminal ; --demo : sans greetd, pour l'essayer ;
                    --window : fenêtre ordinaire plein écran (cage, sans layer-shell)
  greeter setup <status|profile|packages|install|enable|restore>
                    met l'écran de hyprdesk à la place de GDM (en root ; Réglages →
                    Écran de connexion) ; restore : revient au gestionnaire d'avant
  mymfa <open|unlock|setup|lock|codes|logins>
                    trousseau myMFA : fenêtre complète, déverrouillage, création du coffre, verrouillage, lanceur sur
                    les codes 2FA (:2fa) ou les mots de passe (:mdp)
  panorama [toggle|show|hide|page <n|id>|detect|preview [LxH]|icons [sources|music|shortcuts|all] [--force]|fullscreen]
                    Panorama des écrans bandeau tactiles : afficher, escamoter, aller à une
                    page ; detect : relit les écrans et refait les propositions ;
                    preview : aperçu en fenêtre flottante (2560x720 par défaut) ;
                    icons : icônes des sources vidéo et des raccourcis dessinées par SmartUI (LYRA)
  panorama layout <single|1x2|1x3|2x2|3x3|dashboard> | video <n|nom|close>
                    une page, l'écran découpé en zones ou la composition ; ouvre une source du
                    widget Vidéo (Free TV, Netflix…) dans sa zone, ou la ferme
  polkit-ui         agent d'authentification polkit de la session (lancé par le démon)
  polkit-agent      agent d'authentification polkit au terminal (essai, ou session sans démon)
  updates [--upgrade | --refresh | --report]
                    paquets à mettre à jour (pacman, AUR, apt, dnf, zypper, flatpak) ;
                    --upgrade : mise à jour non interactive (pkexec), échec confié au démon
  gpu [status | nvidia | display | vfio | toggle] [--boot | --next-boot] [--device <adresse>]
                    GPU NVIDIA réservé aux VM : bascule à chaud entre vfio-pci et nvidia
                    (pkexec) ; --boot : garde aussi ce pilote aux prochains démarrages
                    display : nvidia avec les écrans branchés sur la carte (nvidia_drm)
                    --next-boot : pilote des prochains démarrages, sans basculer maintenant
  reload            relit la configuration
  status            état du démon
  quit              arrête le démon
  chat-mock         faux moteur d'assistant (protocole serve, pour essais)
  preview [dossier] aperçu isolé ; capture les vues si un dossier est fourni
  check [fichier]   exécute et vérifie la configuration sans modifier le démon
  doctor            dépendances externes : présentes, absentes, et ce qui en dépend
  doctor --missing  les absentes, une par ligne (nom, paquet Arch, fonction ; tabulations)
  autostart [list | add | remove | on | off <commande>]
                    démarrage automatique (autostart.lua, hyprland.lua, XDG)
  hyprland-check [--json]
                    ménage de hyprland.lua : erreurs, doublons, conflits avec hyprdesk
  hyprland-fix (--safe | <ligne>… | backups)
                    applique les corrections du ménage (copie de hyprland.lua d'abord)
  window-places [forget]
                    places des fenêtres choisies en plein bureau ; les oublier
  icons [list | add app|path|exec <valeur> | remove <id>]
                    icônes du bureau
  app-theme [status | apply]
                    thème de hyprdesk appliqué aux applications (kitty, GTK, Qt)
  color-test        mire plein écran des couleurs sur tous les écrans (ouvre ou ferme)
  screens-off       éteint tous les écrans sans mettre la machine en veille ;
                    un changement de volume les rallume (idle_wake_volume)
  colors [list | set <écran> <réglage> <valeur> | reset <écran> | calibration <écran> <fichier>|none | diag | free [--disable]]
                    couleurs de chaque écran (luminosité, contraste, gamma, canaux, calibration)
  lid [status | close | open]
                    capot du portable : état et effet de sa fermeture (Réglages → Verrouillage → Veille)
  window minimize | maximize | close
                    fenêtre active : réduire (le dock la rappelle), agrandir, fermer
  keys              raccourcis de hyprdesk : où ils agissent, conflits avec hyprland.lua
  binds [split]     raccourcis de hyprland.lua à ranger dans binds.lua (fait au démarrage)
  init              écrit la configuration d'exemple si aucune n'existe
  keyboard [disposition [variante [options]]]
                    clavier de la session : sans argument, celui en vigueur ;
                    sinon appliqué et écrit dans inputs.lua (ex. keyboard fr azerty)
  stubs [dossier]   écrit les définitions LuaLS (hyprdesk.meta.lua) pour l'autocomplétion
  snippet           lignes à ajouter à hyprland.lua (raccourci, flou, démarrage)
  index daemon      démon d'indexation des fichiers (recherche :f du lanceur) ;
                    options : --paths /a,/b --exclude nom --interval secondes
  index query <mot> cherche dans l'index (--limit N) : nom, type, chemin, taille
  index status      état de l'indexation (JSON) ; rebuild : rescan complet ;
                    stop : arrêt propre (sauvegarde du snapshot)

Configuration : $HYPRDESK_CONFIG, sinon ~/.config/hypr/hyprdesk.lua";

/// Extrait de `hyprland.lua` (`hyprdesk snippet`).
pub const SNIPPET: &str = r#"-- hyprdesk : à ajouter dans ~/.config/hypr/hyprland.lua

-- Démarrage : SEULEMENT si le service systemd utilisateur n'est pas activé
-- (une seule instance tourne de toute façon : la seconde s'arrête aussitôt),
-- dans le bloc hl.on("hyprland.start", …) ; hyprdesk remplace hyprpaper.
hl.exec_cmd("hyprdesk")

-- Les raccourcis de hyprdesk (SUPER + D par défaut) sont installés par
-- hyprdesk lui-même : voir hd.config({ keys = … }) ou Réglages → Bureau.
--
-- Bureau affiché, les raccourcis de Hyprland sont suspendus (submap
-- « hyprdesk ») : ils n'agissent plus sur les fenêtres cachées. Pour en
-- garder un actif sur le bureau, marquez-le submap_universal, par exemple
-- le changement de workspace (le bureau suit alors le workspace) :
hl.bind(mainMod .. " + ampersand", hl.dsp.focus({ workspace = 1 }), { submap_universal = true })
hl.bind(mainMod .. " + Right",     hl.dsp.focus({ workspace = "+1" }), { submap_universal = true })

-- Captures : ajouter submap_universal AUX raccourcis existants (ne pas les dupliquer).
-- Sans cette option, Print et les autres raccourcis de capture sont suspendus sur le bureau.
-- Exemples :
-- hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region --clipboard-only"), { submap_universal = true })
-- hl.bind("SUPER + Print", hl.dsp.exec_cmd("hyprshot -m output -m active"), { submap_universal = true })

-- Barre de hyprdesk (hd.bar) : flou derrière elle, comme une barre classique.
hl.layer_rule({ name = "hyprdesk-bar-blur", match = { namespace = "^hyprdesk-bar$" }, blur = true, ignore_alpha = 0.2 })
"#;

pub mod companion;
