# « Annuler » après un retrait du bureau ou du dock

Written against: `95d666b` (hig/plan-B)

## Evidence chain

- Surface: menus contextuels du bureau et du dock (`src/ui/context.rs`, `CMsg::Remove`, `CMsg::RemoveWidget`, `CMsg::Pin` quand l'application est épinglée).
- Problem: « Retirer du bureau » (icône, widget) et « Retirer du dock » agissent sans retour arrière ; seul l'historique de configuration (`src/history.rs`) permet de revenir, loin de l'action.
- Design evidence: HIG undo-and-redo (« Offer an easy way to undo and redo many actions ») ; la part « dépôt d'une fenêtre sur un autre espace » est faite par DAT-12 (`917242e`, ligne « Annuler le déplacement » de l'écran des espaces).
- Owner: `src/ui/context.rs` (les actions), `src/ui/toasts.rs` (les bulles de hyprdesk : le résumé du Silence en est déjà une, `summary`).
- Scope and affected surfaces: menus du bureau et du dock ; pile des bulles.
- Uncertainty: sans le service de notifications tenu par hyprdesk (wayle, mako…), pas de bulle : l'annulation n'est pas proposée, l'action reste comme aujourd'hui.

## Design decision

Après chacun de ces trois retraits, une bulle de hyprdesk, en tête de la pile, nomme ce qui a été retiré (« « Firefox » retirée du dock », « « notes.txt » retirée du bureau », « Widget « Horloge » retiré du bureau ») et porte une action « Annuler le retrait ». La bulle suit le délai commun (6 s), comme toute bannière. L'action rétablit l'élément à sa place d'origine (même rang dans sa liste) puis enregistre (`commit`). Une seule annulation en attente : un nouveau retrait remplace la précédente (et sa bulle).

## Reuse

- Bulles : `Toast` et `crate::notifications::build`, comme le résumé du Silence (`resume_notification`) ; identifiant réservé `UNDO_ID = u32::MAX` (le serveur numérote à partir de 1, comme `SUMMARY_ID = 0`), indice `transient` : jamais dans l'historique.
- Signalement aux applications : `Toasts::is_summary` devient `is_own` (résumé ou annulation) aux trois endroits qui taisent `Invoke`, `Dismiss` (fermeture, expiration).
- Réglages : `Settings::icons`, `Settings::widgets`, `launcher_pinned` ; `App::commit`.

## Changes

1. `src/ui/context.rs`
   - Change: `pub(super) enum ShellUndo { Icon { at, def: IconDef }, Widget { at, def: WidgetDef }, Pin { at, app } }` avec `fn apply(self, settings: &mut Settings, pinned: Vec<String>)` (réinsertion bornée au rang d'origine ; une application déjà réépinglée ne l'est pas deux fois) et le texte de la bulle. `CMsg::Remove`, `CMsg::RemoveWidget`, `CMsg::Pin` (retrait seulement) capturent l'élément et son rang avant de le retirer, puis appellent `show_undo`.
   - Preserve: les retraits eux-mêmes, la fermeture du menu, l'épinglage (sans annulation : il ne retire rien).
   - Verify: test unitaire : retrait puis `apply` rend la liste d'origine, pour les trois cas.
2. `src/ui/toasts.rs`
   - Change: `UNDO_ID` ; `App::show_undo(text, undo)` : range `self.shell_undo = Some(undo)`, retire une bulle `UNDO_ID` précédente, insère la nouvelle en tête si `toasts.active` (sinon rien), `sync_toasts`. `TMsg::Invoke(UNDO_ID, "undo")` : `shell_undo.take()` → `apply` → `commit` ; la bulle part. Fermeture ou expiration de la bulle : `shell_undo = None`.
   - Preserve: résumé du Silence, autorisations, explication, clavier de la pile (ENT-01).
   - Verify: test : `show_undo` pose la bulle sans l'historique ; `Invoke(UNDO_ID, "undo")` rétablit et retire la bulle ; `Close(UNDO_ID)` oublie l'annulation.
3. `src/ui/mod.rs` : champ `shell_undo: Option<context::ShellUndo>`.
4. `locales/en.toml`, `DESIGN.md` (ligne des bulles ou des menus), `docs/GUIDE.fr.md`, `docs/GUIDE.md`.

## Relecture

Relu avant exécution par un agent indépendant. Retenu : `is_own` compare l'identifiant (`UNDO_ID`), pas un drapeau ; aucun champ ajouté à `Toast` ; tout retrait de la bulle (action, clic sur la bulle, fermeture, expiration, remplacement) oublie l'annulation ; glyphe explicite `edit-undo-symbolic` ; délai = `notifications.timeout`, gelé quand la pile tient le clavier (ENT-01) ; la bulle se montre sous le Silence (réponse à un geste) ; un widget déclaré par Lua (absent de `settings.widgets`) n'est pas retiré d'ici, donc pas d'annulation. Tests : `un_retrait_s_annule_depuis_sa_bulle`, `une_icone_retiree_revient_a_son_rang`.

## Scope

- Inherit: bureau (icônes, widgets), dock.
- Verify: bulles de notifications ordinaires, résumé du Silence, centre de notifications (l'annulation n'y entre pas).
- Exclude: annulation au clavier (Ctrl+Z) — aucune surface du shell n'a le clavier après un menu ; « Mettre à la corbeille » (la corbeille est déjà le retour arrière) ; rétablissement (redo).
