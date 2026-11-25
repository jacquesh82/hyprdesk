# PAT-04 — Le Silence a une fin que l'on choisit en l'activant

Écrit contre : `cae8c8c` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : centre de contrôle de Continuité, tuile « Silence » ; ligne de commande `hyprdesk notifications dnd`.
- Problème : le Silence est un booléen gardé d'une session à l'autre (`Toasts::command`, `src/ui/toasts.rs` ; fichier `dnd` de `notifications::dnd_path`). Une seule fin existe : la main. Ni durée, ni échéance.
- Preuve de design : HIG managing-notifications › Integrating with Focus — « A Focus helps people filter notifications during a time period they reserve for an activity » ; colonne Correction du plan : « Faire choisir une fin à l'activation (une heure, jusqu'à ce soir, jusqu'à désactivation) ».
- Propriétaires : `src/notifications.rs` (état gardé), `src/ui/toasts.rs` (commande, bulles), `src/ui/plugins/bar/popover/continuity.rs` (tuile), carte de menu `Kind::Menu` de `src/ui/plugins/bar/popover.rs` (choix).
- Périmètre et surfaces touchées : `src/notifications.rs`, `src/ui/toasts.rs`, `src/ui/mod.rs` (requête, abonnement), `src/ui/plugins/bar/popover.rs`, `…/popover/continuity.rs`, `src/main.rs`, `src/sources/mod.rs`, `src/control.rs`, `src/lib.rs`, `docs/CONTROL.md`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `locales/en.toml`.
- Incertitude : la HIG ne chiffre ni l'heure de « ce soir » ni la durée. Retenu : 18 h, une heure.

## Décision de design

Le Silence porte une échéance facultative, à l'horloge murale, gardée dans le même fichier `dnd` (rétrocompatible). Trois fins, présentées à l'activation :

| Fin | Mot (commande) | Échéance |
| --- | --- | --- |
| 1 heure | `1h` | maintenant + 1 h |
| Jusqu'à ce soir | `soir` | aujourd'hui à 18 h ; proposée seulement avant 18 h |
| Jusqu'à désactivation | (aucun) | aucune |

- À l'échéance, le Silence s'éteint : la bulle suivante revient, l'état publié l'indique, le fichier est retiré. Une échéance passée n'est jamais relue comme un Silence en cours (redémarrage, reprise de veille).
- La tuile « Silence » éteinte ouvre la carte de menu des trois fins (le choix est l'activation) ; allumée, elle nomme sa fin (« Silence · jusqu'à 18:00 ») et un clic l'éteint.
- `hyprdesk notifications dnd on` sans mot reste « jusqu'à désactivation » ; `dnd` seul bascule comme avant ; `dnd on 1h|soir` ajoute une fin.

Pourquoi cela résout la racine : l'absence est celle d'une fin autre que la main. Tout le reste (une seule bulle critique, l'historique alimenté sous Silence, l'autre démon) est inchangé.

## Réutilisation

- Carte de menu existante : `popover::Kind::Menu { entries, source: MenuSource::Silence }`, ouverte par un nouveau `popover::Request::Silence`, comme `Request::Window` ouvre le menu de la fenêtre. Clavier, fermeture après choix, entrées désactivées : acquis.
- Tuile `dashboard_controls` / `dashboard_pill` de `continuity.rs` : seuls le libellé et l'action changent.
- Propriétaire de l'état : `notifications::dnd_path`, déjà retiré par le réglage par défaut `Held::Notifications`.
- Exemple : `dashboard_connection` (Wi-Fi) ouvre un menu depuis la tuile voisine par `Msg::OpenPopover`.

Aucune primitive nouvelle.

## Changements

1. `src/notifications.rs`
   - `SilenceEnd { Hour, Evening }` (`parse`, `deadline(now) -> Option<i64>`) ; `Silence { until: Option<i64> }` (`load`, `save`, échéance passée = coupé) ; `render_state` écrit `dnd_until=<échéance>`.
   - Préserver : `1\n` seul = sans fin ; `dnd=0|1`, `count`, `n=` inchangés.
   - Vérifier : tests unitaires des trois fins, du soir passé, de la relecture (ancien contenu, échéance passée).
2. `src/ui/toasts.rs`
   - `Toasts::dnd_until` ; `command` lit `dnd on [1h|soir]` ; `TMsg::Expire` et `silence_ends(until)` (abonnement, un message à l'échéance, réveil borné à 5 min) ; vérification à l'arrivée d'une notification.
   - Préserver : `dnd` seul, `dnd on`, `dnd off`, `clear`.
   - Vérifier : tests `command` et expiration.
3. `src/ui/mod.rs`
   - `Request::Silence` → `Kind::Menu` ; abonnement quand le Silence a une fin.
   - Vérifier : compilation, porte complète.
4. `src/ui/plugins/bar/popover.rs` : `Request::Silence`, `MenuSource::Silence` ; l'action n de l'entrée mène à `dnd on` + sa fin.
5. `src/ui/plugins/bar/popover/continuity.rs` : tuile et libellé du volet Notifications.
6. `src/main.rs`, `src/control.rs`, `src/lib.rs`, `src/sources/mod.rs` : commande à trois mots, usage, `dnd_until` lu par le module de barre.
7. Documentation et `locales/en.toml`.

## Périmètre

- Héritent : modules de barre Lua (`hd.notifications().dnd_until`), `hyprdesk notifications list`.
- À vérifier : aperçu (`preview.rs` : `_ => return`), styles de barre autres que Continuité (n'ont pas la tuile).
- Exclu : liste d'applications autorisées (`PAT-02`), niveaux d'interruption (`PAT-05`), réglages des notifications (`PAT-07`, `SYS-03`), panneau de son.

## Validation

- Produit : allumer le Silence « 1 heure » depuis la tuile → tuile « Silence · jusqu'à <heure> » ; à l'échéance la tuile s'éteint et une notification normale montre une bulle.
- Interface : menu à deux entrées après 18 h ; ligne de commande `dnd on soir` refusée après 18 h.
- Système : aucun booléen parallèle, aucune seconde primitive de menu.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si la carte `Kind::Menu` ne peut pas être ouverte depuis le centre de contrôle, si l'abonnement `toasts::stream` ne peut pas être doublé d'un minuteur, ou si le champ `dnd` des modules Lua devait changer de sens.

## Documentation de design

- `docs/CONTROL.md`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `locales/en.toml` ; aucune décision nouvelle pour `DESIGN.md` (composition de widgets existants).

## Relecture à froid (sans outil d'agent)

- Qu'est-ce qui casse ? `dnd on` et `dnd` seul gardent leur sens : les scripts existants sont préservés. Le fichier `dnd` ancien (`1\n`) se relit sans fin. `parse_notifications` ignore les clés inconnues : un ancien lecteur lit encore l'état.
- Que manque-t-il ? Une fin programmée pendant une mise en veille : le minuteur ne doit pas se fier au seul temps monotone — réveil borné et relecture à l'arrivée d'une notification, inscrits au changement 2. Après 18 h, « jusqu'à ce soir » n'a plus de sens : entrée retirée du menu, commande refusée.
- Le plan résout-il l'écart ? Oui : les trois fins de la colonne Correction, choisies à l'activation dans le centre de contrôle et sur la ligne de commande, gardées d'une session à l'autre, expirant seules.
