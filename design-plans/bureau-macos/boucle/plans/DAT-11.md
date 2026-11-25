# Visite des Réglages : un essai par bloc

Written against: `84be3fd` (hig/plan-B)

## Evidence chain

- Surface: fenêtre de la visite guidée (`src/ui/tour.rs`, `STEPS`, `view_tour`), au-dessus du tiroir des Réglages.
- Problem: la visite est un texte à lire — dix blocs de paragraphes — ; rien n'y demande d'agir.
- Design evidence: HIG onboarding (« Teach through interactivity », « doesn't require people to memorize a lot of information »).
- Owner: `src/ui/tour.rs` ; détection au point d'entrée des commandes (`App::update`, `Msg::Control`, `src/ui/mod.rs`).
- Scope and affected surfaces: visite guidée seulement.
- Uncertainty: l'écran des espaces et la grille des applications, couches Overlay plein écran au clavier exclusif créées après la visite, la recouvrent : l'essai est fait à l'arrivée de la commande, et Échap les referme sur la visite. « Essayer » le changement de mode change vraiment le mode (launcher-only peut prendre le clavier) : SUPER + D, deux fois encore, ramène le mode d'origine.

## Design decision

Chaque bloc porte un essai d'une phrase, sous ses tuiles, au-dessus des boutons : « Essayez : … », un bouton « Essayer » (secondaire : « Suivant » reste l'action principale) et, une fois l'essai fait, « Fait » à la place du bouton. Deux sortes d'essais :

1. **Commande du shell** : le bloc nomme un geste réel — changer de mode (`Command::Cycle`, bloc « Bienvenue »), ouvrir l'écran des espaces (`Command::Overview`, bloc « Barre, dock et Panorama »), ouvrir la grille des applications (`Command::Apps`, bloc « Les applications »). « Essayer » envoie la commande ; le raccourci de l'utilisateur la fait aussi. L'essai est fait quand cette commande arrive à `App::update` pendant le bloc (une seule détection, que le geste vienne du bouton, du raccourci ou de la CLI).
2. **Réglage du tiroir** (les autres blocs) : « changez l'un de ces réglages dans les Réglages, derrière ». L'essai est fait quand un réglage a changé depuis l'entrée dans le bloc : `tour_go` garde une copie des réglages (`tour_from`), comparée à `self.settings` — un simple enregistrement sans changement ne compte pas. « Essayer » n'existe pas pour cette sorte : le tiroir est déjà ouvert sur le thème.

Rien n'est imposé : « Suivant » reste toujours disponible.

## Reuse

- Tuile du bloc (fond `fg` à 0,06, rayon 12) pour la ligne d'essai ; `button::secondary` comme « Précédent ».
- `control::Command` (déjà `PartialEq`) ; `Settings` (déjà `PartialEq`).

## Changes

1. `src/ui/tour.rs`
   - Change: `pub enum Trial { Command { text, run: control::Command }, Setting { text } }` ; champ `trial` sur chaque `Step` ; `TourMsg::Try` envoie `run` ; `tour_go` remet l'état d'essai et copie les réglages ; `App::tour_note(&Command)` marque l'essai fait ; `tour_trial_done()` ; ligne d'essai dans `view_tour`.
   - Preserve: blocs, tuiles, navigation, fin de visite, `tour_seen`.
   - Verify: tests : la commande du bloc marque l'essai, une autre non, le changement de bloc le remet ; un réglage changé marque un essai de réglage, un enregistrement sans changement non.
2. `src/ui/panel.rs` : `tour_tried: bool`, `tour_from: Option<Box<Settings>>` à côté de `tour`.
3. `src/ui/mod.rs` : `tour_note` appelé où `Msg::Control` est journalisé.
4. `locales/en.toml`, `DESIGN.md`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`.

## Relecture

Relu avant exécution par un agent indépendant. Retenu : recouvrement par les écrans ouverts (incertitude corrigée) ; comparaison des valeurs des réglages au lieu de la révision (faux « Fait » à chaque enregistrement) ; `Msg::DashboardCommand` appelle `update_inner` et échappe à la détection, sans effet pour ces trois commandes. Noté, hors de cet écart : la fenêtre de visite n'a pas de clavier propre (Échap y ferme le tiroir sans retenir `tour_seen`) — antérieur.

Diff relu par un agent indépendant. Corrigé : phrase de l'essai de réglage raccourcie (une ligne dans 780 px), côté droit de largeur fixe (« Essayer » puis « ✓ Fait » ne reflowent pas la phrase), test du retour au bloc et de la détection hors visite. Assumé : revenir à un bloc remet son essai à faire ; un réglage changé hors du tiroir pendant le bloc (CLI, dock) compte aussi.

## Scope

- Inherit: visite lancée depuis le tiroir, `hyprdesk tour`, la fin de l'assistant.
- Verify: navigation au clavier de la visite, fermeture.
- Exclude: glisser un module en essai (aucune commande ne le signale) ; réécriture des paragraphes.
