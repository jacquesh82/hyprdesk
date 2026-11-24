# PAT-05 — La première notification critique d'une application explique le passage du Silence et permet de le refuser

Écrit contre : `d957308` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : bulles de notifications (calque `hyprdesk-notifications`), bulle d'une notification critique ; Silence (`hyprdesk notifications dnd`, tuile « Silence » du centre Continuité).
- Problème : trois niveaux seulement, lus dans l'indice d'urgence de l'application (`hint_urgency`, `src/notifications.rs`). Une notification critique se déclare elle-même, traverse le Silence (`src/ui/toasts.rs`, `TMsg::Event(Event::Shown)`, test `self.toasts.dnd && n.urgency != Urgency::Critical`), ne s'efface jamais (`Toast::lifetime_or`) et n'est expliquée à personne : aucune décision par application n'existe.
- Preuve de design : HIG managing-notifications › Best practices — « The first time a Time Sensitive notification arrives from your app, the system describes how such a notification works and gives people a way to turn it off » ; colonne Correction du plan : « Accorder le passage du Silence application par application, à la première notification critique (explication et refus possibles) ».
- Propriétaires : `src/notifications.rs` (état gardé d'une session à l'autre : `Silence`, `dnd_path`), `src/ui/toasts.rs` (bulles : `Toasts`, `toasts_update`, `view_toast`, `card_height`), scène d'aperçu « notifications » de `src/ui/preview.rs`.
- Périmètre et surfaces touchées : `src/notifications.rs`, `src/ui/toasts.rs`, `src/ui/preview.rs`, `src/defaults.rs` (`Held::Notifications`), `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`.
- Incertitude : la HIG ne dit pas si la première notification est montrée avant la réponse. Retenu : oui — elle est livrée comme aujourd'hui, l'explication est portée par sa bulle ; une critique retenue jusqu'à une réponse trahirait son urgence.

## Décision de design

- Une décision par application, gardée dans `$XDG_STATE_HOME/hyprdesk/passage-silence` (une ligne par application : `clé=1`, passage accordé ; `clé=0`, refusé). La clé est l'identifiant du hint `desktop-entry` (validé comme pour l'ouverture d'une application : ni vide, ni chemin, suffixe `.desktop` toléré), sinon le nom de l'émetteur. Décision inconnue d'une application : passage accordé, et c'est la première fois.
- À la première notification critique d'une application (Silence allumé ou non : ce qui est expliqué vaut dans les deux cas), l'application est notée accordée et **sa bulle porte l'explication** : une ligne secondaire « Une notification critique traverse le Silence et reste jusqu'à sa fermeture. » et un bouton « Bloquer pendant le Silence ». La bulle critique ne s'efface pas d'elle-même : l'explication reste lisible tant que la bulle est ouverte.
- « Bloquer pendant le Silence » note l'application refusée et retire l'explication de la bulle (la notification déjà livrée reste). Dès lors, sous Silence, les critiques de cette application vont à l'historique seules, comme les autres ; hors Silence, elles gardent leur bulle.
- Fermer la bulle (✕, action, remplacement par l'application) sans refuser, c'est garder le passage : l'application n'est plus réexpliquée.
- « Réinitialiser » l'état des notifications (`Held::Notifications`) retire aussi ce fichier : plus aucune application n'est notée, la prochaine critique réexplique. C'est le seul retour en arrière jusqu'au paquet de réglages Notifications (PAT-07 : révocation).

Pourquoi cela résout la racine : le passage du Silence devient une décision par application, expliquée à la première notification critique, refusable sur place. Rien ne change pour une application jamais vue tant qu'elle n'envoie pas de critique.

## Réutilisation

- Carte de bulle existante (`view_toast` : colonne tête, titre, corps, actions ; bouton d'action sans bordure de 28 px, HIG ACC-04), hauteur bornée comme le corps (« deux lignes au plus »).
- État gardé : même idiome que `Silence` (`layout::state_dir()`, dossier temporaire pendant les tests).
- `app_id` (`src/ui/toasts.rs`) pour la clé d'application.
- Exemplar : `src/notifications.rs` (`Silence::read`, `Silence::save`, tests `silence_garde_d_une_session_a_l_autre`).

Aucune primitive nouvelle.

## Changements

1. `src/notifications.rs`
   - Change : `pub struct Passage { apps: Vec<(String, bool)> }`, `passage_path()`, `Passage::load()` (fichier absent ou illisible : vide), `Passage::read(text)` (lignes `clé=0|1`, les autres ignorées), `Passage::granted(clé) -> Option<bool>`, `Passage::set(clé, accordé) -> Self`, `Passage::save()`.
   - Preserve : `Silence`, `render_state`, `dnd_path`, le serveur.
   - Verify : test unitaire de lecture, d'écriture et de rélecture.
2. `src/ui/toasts.rs`
   - Change : `fn passage_key(n) -> String` ; `pub struct Explain { key, id }` et champ `Toasts::explain: Vec<Explain>` — une par bulle expliquée : deux applications expliquées à la fois gardent chacune la leur ; une entrée sans bulle (fermée) s'oublie à la notification suivante ; `TMsg::Refuse(id)`. Dans `TMsg::Event(Event::Shown(n))`, avant le filtre du Silence : première critique de la clé → `Passage::set(clé, vrai)`, `explain = Some(..)` ; le filtre devient « Silence et non (critique dont l'application passe) ». `TMsg::Refuse(id)` : `Passage::set(clé, faux)`, `explain` retiré, surface redimensionnée (`sync_toasts`).
   - Change : `view_toast` ajoute, sous le corps et les actions, le texte secondaire (12 px, hauteur fixe de deux lignes : au-delà, le texte est rogné, le bouton reste) et le bouton quand `explain` désigne cette bulle. `explain` n'est lu que pour la bulle de même `id` : partie, elle ne s'applique plus à rien. `card_height(n, expliquée)`, `height` et `height_of` prennent le surcroît `4 + deux lignes (12 px, interligne du thème) + 4 + 28 px` — d'où `sync_toasts` et `sync_convex_toasts` redimensionnent la surface.
   - Preserve : la bulle critique d'une application déjà notée, la bulle hors Silence, les autres urgences, les autres styles de barre, `command` ; les assertions de hauteur existantes (le nouvel argument vaut `false`).
   - Verify : tests, avec une clé d'application propre à chaque test (le fichier gardé est commun aux tests d'un même processus) — la première critique explique, la seconde de la même application non ; une application refusée ne passe plus le Silence (historique seul), une autre application passe et est expliquée, même quand une première bulle expliquée est encore ouverte ; bulle fermée : passage gardé, explication oubliée ; hauteur de la carte expliquée et de la surface.
3. `src/ui/preview.rs`
   - Change : la scène « notifications » pose l'explication sur sa bulle critique ; les autres scènes la retirent avec leurs bulles.
   - Verify : capture de la scène (porte visuelle).
4. `src/defaults.rs`, `locales/en.toml`
   - Change : `Held::Notifications` retire aussi `passage_path()` ; son libellé (« notifications gardées effacées, « Ne pas déranger » coupé ») dit les passages du Silence oubliés, en anglais aussi.
   - Verify : compilation, `clippy` ; retrait couvert par le code existant.
5. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`
   - Change : les deux chaînes en anglais ; le paragraphe du Silence dit l'explication à la première critique et son refus.

## Périmètre

- Héritent : carte de l'historique (barre, centre Continuité, Panorama) — inchangée ; état publié `hyprdesk-notifications.txt` — inchangé.
- À vérifier : scène d'aperçu « notifications » ; hauteur de la surface avec quatre bulles dont une expliquée.
- Exclut : autorisation générale par application et révocation dans les Réglages (PAT-02, PAT-07, SYS-03) ; niveau « sensible au temps » (hors colonne Correction) ; toute option Lua ou commande nouvelle.

## Validation

- Produit : envoyer une notification critique (`notify-send -u critical`) : la bulle montre l'explication ; cliquer « Bloquer pendant le Silence » ; allumer le Silence ; renvoyer une critique de la même application : rien à l'écran, une carte dans l'historique.
- Interface : bulle critique avec et sans corps ni actions ; anglais ; quatre bulles visibles.
- Système : aucune seconde primitive d'état ni de bouton ; `Silence` reste le seul état global.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-A` → capture de la scène « notifications ».

## Conditions d'arrêt

- S'arrêter si le texte de l'explication ne tient pas en deux lignes à la largeur par défaut (380 px) : le bouton de refus serait rogné.
- S'arrêter si `Held::Notifications` couvre autre chose que l'état des notifications.

## Relue à froid (sans agent indépendant)

- Qu'est-ce qui casse ? `card_height`, `height` et `height_of` changent de signature : leurs appelants (`view_toast`, `sync_toasts`, `sync_convex_toasts`, deux tests) sont mis à jour sans perdre d'assertion. Une scène d'aperçu autre que « notifications » pourrait hériter de `explain` : les scènes le retirent avec leurs bulles. Les tests d'un même processus partagent le fichier gardé, et sa réécriture part d'une lecture : clé propre à chaque test nouveau, et un verrou de tests, pris par tout test qui fait passer une critique par `Event::Shown` (le test existant de la bulle critique compris), un seul à la fois. En exécution, une seule explication à la fois retirait son bouton de refus à la première bulle encore ouverte quand une autre application était expliquée : une explication par bulle (`Vec`). L'interligne du thème est réglable (0,8 à 3) : la hauteur des deux lignes est calculée, pas constante.
- Que manque-t-il ? Un retour en arrière après un refus : « Réinitialiser » l'état des notifications le donne jusqu'au paquet de réglages (PAT-07). Hors de ce lot, volontairement : révocable dans les Réglages, commande en ligne.
- Le plan résout-il l'écart ? Oui : décision par application, prise à la première notification critique, expliquée sur sa bulle, refusable, appliquée au filtre du Silence.

## Documentation de design

- Après acceptation : une phrase dans `docs/GUIDE.fr.md` et `docs/GUIDE.md` (Silence : première critique expliquée, refus par application, réinitialisation) ; rien dans `DESIGN.md`.
