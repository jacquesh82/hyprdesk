# LAY-05 — Les textes utiles des réglages se sélectionnent et se copient

Écrit contre : `5b9781d` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tiroir des Réglages — résultat d'une remise par défaut (avec le chemin de la copie) ; note « surchargé dans hyprdesk.lua — sans effet ici » ; « Aussi dans hyprland.lua (lignes …) : valeurs calculées, gardées là-bas. » ; « Calculées dans hyprland.lua : … » ; note du plugin Hyprbars (chemin trouvé, commande, URL d'installation).
- Problème : chemins, avertissements et erreurs sont rendus par `text`. Rien ne s'en copie : ni glissé, ni double clic, ni Ctrl+C, ni le menu d'édition. Seul « À propos » offre une commande de copie.
- Preuve de design : HIG labels › Best practices — « Make useful label text selectable. » ; labels › Discussion — « A label is a static piece of text that people can read and often copy, but not edit. »
- Propriétaires : `src/ui/panel.rs` (`defaults_done`, `field`), `src/ui/hyprlists.rs` (`left_in_hyprland`), `src/ui/hyprset.rs` (notes « Calculées dans… »), `src/ui/plugins/desktop.rs` (Hyprbars), `src/ui/plugins/workspaces.rs` (même note que `field`, au féminin — non citée par l'audit, même phrase). Existant le plus proche : `src/ui/chat.rs` (`selecting`, `ChatMsg::EditorAction`).
- Périmètre et surfaces touchées : nouveau `src/ui/selectable.rs` ; les cinq sites ci-dessus. Aucun modèle, aucun message.
- Incertitude : aucune sur la règle. Une contrainte technique, relevée à l'itération B6 : sous Iced 0.14, la sélection d'un `text_editor` vit dans son `Content`, et tout — clic, glissé, touches — passe par `on_action`.

## Décision de design

Un composant unique, `selectable`, rend un texte statique **en lecture seule, sélectionnable et copiable**. Il enveloppe `text_editor`, seul widget d'Iced qui sache sélectionner, et garde le `Content` **dans l'arbre des widgets**, à l'endroit même du texte.

- Pourquoi pas l'état au modèle, comme `chat.rs` : la vue emprunte `&self`. Cinq textes demanderaient cinq états portés par quatre modèles, trois étant calculés à la volée (par fonction, par option, par règle). Chaque futur texte utile en redemanderait un.
- Pourquoi pas `lazy::Component` : son état naît de `Default`, sans texte ; un texte qui change à la même place de l'arbre (nouveau résultat de remise par défaut) resterait figé.
- Pourquoi pas `text_input` en lecture seule : une ligne, sans retour à la ligne.
- Pourquoi un composant plutôt qu'un dérivé : aucun propriétaire existant n'exprime « texte statique sélectionnable » ; `chat.rs` le fait à la main, pour un seul site. Le composant réutilise `text_editor` tel quel et `src/ui/tokens.rs` pour la couleur de sélection.

Fonctionnement :

- `tag` / `state` donnent à chaque texte sa place dans l'arbre ; `diff` remplace le contenu quand le texte change (la sélection s'efface alors, à juste titre).
- L'éditeur interne publie ses actions vers un message local, `Act`, que le composant traite sur place : sélection et déplacement sont joués, toute édition (`Action::Edit`) est reçue puis ignorée. Rien ne remonte à l'application.
- Copier est fait par l'éditeur lui-même (Ctrl+C, Ctrl+X, menu d'édition de `src/ui/edit.rs`, qui voit déjà les `text_editor`). Coller est ignoré.
- Apparence du `text` remplacé : même police, même taille, même couleur (sans couleur donnée : celle d'un `text`), largeur `Shrink`, retour à la ligne identique, marge nulle, fond transparent, sans bord. La sélection prend la couleur de sélection du thème (`tokens::selection`) : l'accent reste réservé à la sélection et aux actions.
- Molette : l'éditeur capture la molette survolée et demande un déplacement du curseur. Le composant ne retransmet ni la capture ni l'action : le tiroir continue de défiler sous la note.
- Clavier : l'éditeur ne répond aux touches qu'après un clic sur le texte (`Status::Focused`), jusqu'au clic ailleurs ou à Échap. Comme dans tout champ, la frappe y est absorbée ; ici sans effet. Hors de ce cas, il ne capture aucune touche.
- Curseur d'insertion : après un clic, l'éditeur montre son caret clignotant (sous Iced 0.14, pas de sélection sans focus). Même apparence que la citation de l'assistante en mode « Sélectionner ».

## Réutilisation

- `iced::widget::text_editor` (sélection, copie, double et triple clic, raccourcis) ; `tree::Tag`, `tree::State`, comme `lazy::Component` d'Iced.
- `crate::ui::tokens::selection` — couleur de sélection ; `crate::ui::tokens::ts` aux sites.
- Exemple : `src/ui/chat.rs:1319` (citation de l'assistante), `src/ui/edit.rs` (enveloppe d'une surface).

Primitive nouvelle, unavoidable : `src/ui/selectable.rs`. Elle appartient aux utilitaires d'interface, à côté de `edit.rs`. Doivent la partager : les cinq sites de l'écart ; plus tard, tout texte utile (chemin, adresse, erreur) des réglages. `chat.rs` garde sa version : là, le mode « Sélectionner » s'affiche en alternance avec un rendu Markdown, ce que le composant ne fait pas.

## Changements

1. `src/ui/selectable.rs` (nouveau) — `pub(in crate::ui) fn selectable(text) -> Selectable`, avec `.font`, `.size`, `.color` ; `impl From<Selectable> for Element<'a, M>` pour tout message `M`.
   - Change : texte statique sélectionnable, lecture seule, état dans l'arbre.
   - Preserve : la typo et la couleur demandées ; largeur et hauteur d'un `text`.
   - Verify : tests unitaires — une édition ne change ni le texte ni la sélection ; `SelectAll` sélectionne tout ; un texte changé efface la sélection ; Ctrl+C copie (par l'éditeur).
2. `src/ui/mod.rs` — déclarer le module.
3. `src/ui/panel.rs` — résultat de `defaults_done` ; note de `field`.
4. `src/ui/plugins/workspaces.rs` — la même note, au féminin.
5. `src/ui/hyprlists.rs` — `left_in_hyprland`.
6. `src/ui/hyprset.rs` — « Calculées dans hyprland.lua : … ».
7. `src/ui/plugins/desktop.rs` — note Hyprbars.
   - Change : `text(…)` → `selectable(…)`, mêmes réglages.
   - Preserve : phrases, tailles, couleurs, disposition.
   - Verify : compilation, porte complète, captures des pages concernées.

## Périmètre

- Héritent : rien d'autre (les `text` ordinaires ne changent pas).
- À vérifier : tiroir des Réglages (Général, Espaces, Environnement, Bureau), menu d'édition par clic droit, taille des lignes de 12 px.
- Exclus : `chat.rs` ; les textes courants (libellés de champs, aides) ; les autres surfaces que la boucle traitera par leurs propres écarts.

## Validation

- Produit : cliquer-glisser sur le chemin d'une copie de remise par défaut sélectionne ; Ctrl+C le copie ; Ctrl+V, la frappe et Ctrl+X ne changent pas le texte ; un double clic sélectionne un mot ; le menu d'édition propose Copier et Tout sélectionner.
- Interface : pages Général, Espaces, Environnement et Bureau du tiroir, à l'étroit et au large, texte sur une ligne et sur deux.
- Système : un seul composant pour tous les sites ; aucun état ni message ajouté aux modèles ; `tokens::selection` pour la couleur.
- Dépôt : `CARGO_BUILD_JOBS=6 cargo fmt --all && CARGO_BUILD_JOBS=6 cargo clippy --locked --workspace --all-targets -- -D warnings`, puis `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-B` → captures des pages concernées identiques à celles d'avant, hors sélection.

## Conditions d'arrêt

- Arrêter si l'arbre des widgets ne conserve pas l'état d'un cadre à l'autre (sélection perdue aussitôt) : repasser par l'état au modèle, écarts M à découper.
- Arrêter si l'éditeur interne capture des touches hors de son survol et de son clic : le tiroir a d'autres raccourcis.
- Arrêter si le rendu diffère visiblement d'un `text` (hauteur de ligne, marge) et que la correction dépasse le composant.

## Documentation de design

- Après acceptation : une ligne dans `DESIGN.md` — tout texte utile des réglages (chemin, adresse, erreur) est sélectionnable et copiable, en lecture seule ; et une mention dans `docs/GUIDE.fr.md` / `docs/GUIDE.md` si ces textes y sont décrits.

## Relecture à froid (sans outil d'agent)

Relu après vérification, dans les sources d'Iced 0.14.0 (`text_editor.rs`, `lazy/component.rs`, `widget/tree.rs`), de ce que le plan suppose.

- **Qu'est-ce qui casse ?** Trois points, dont un corrigé dans le plan. (1) L'éditeur capture la molette survolée (`Update::Scroll` : capture et `Action::Scroll`) : retransmise telle quelle, elle empêcherait le tiroir de défiler sous une note. Corrigé : la capture n'est pas retransmise pour la molette, l'action est ignorée. (2) Les touches ne sont traitées que si l'éditeur a le focus (`from_key_press` rend `None` hors `Status::Focused`), donc après un clic sur le texte : pas de capture globale, la deuxième condition d'arrêt ne se réalise pas. (3) L'état suit la place dans l'arbre (`Tree::diff` : même étiquette, même état) : deux notes qui permutent sont resynchronisées par `diff` sur comparaison du texte.
- **Que manque-t-il ?** La couleur des notes sans couleur explicite (`desktop.rs`) : précisé. `panel.rs` est un propriétaire partagé : la relecture du diff est obligatoire (consigne 6.8). Les tests ne peuvent pas couvrir le chemin d'événements (il faut un rendu) : ils portent sur l'état et la règle de lecture seule.
- **Le plan résout-il l'écart ?** Oui : chacun des cinq textes cités devient sélectionnable et copiable, en lecture seule, par un seul composant, sans état ni message dans les modèles.

### Relecture du diff (écart M ; `panel.rs` est un propriétaire partagé)

Relu à froid, sans outil d'agent, après le contrôle rapide (`fmt`, `clippy` à zéro avertissement).

- **Le diff fait-il exactement ce que l'écart demande ?** Oui. Six `text` deviennent `selectable`, mêmes phrases, tailles et couleurs : les cinq sites cités, plus la même note au féminin dans `workspaces.rs`. S'y ajoutent le composant (`src/ui/selectable.rs`, 4 tests), une ligne de module et une règle dans `DESIGN.md`. Aucun modèle, aucun message, aucune chaîne à traduire.
- **Respecte-t-il `DESIGN.md` et les conventions du dépôt ?** Oui. Une seule famille de police (celle donnée au site) ; la sélection prend la couleur de sélection du thème, l'accent reste réservé à la sélection et aux actions ; espacement et rayons inchangés (marge nulle, fond transparent, sans bord) ; français source ; aucune dépendance. Le composant enveloppe `text_editor` d'Iced au lieu de le refaire.
- **Régression, test manquant ?** Deux défauts trouvés en relisant, corrigés avant la porte. (1) `text_editor` occupe toute la largeur offerte, comme un champ : la note aurait pris la ligne entière (curseur en I et clic à vide sur toute sa largeur, et un rang centré aurait bougé). Le composant rend au texte la place qu'il prendrait en `text`, mesurée par un `text` posé dans les mêmes limites. (2) La molette survolée est capturée par l'éditeur : rendue au conteneur, pour que le tiroir défile. Vérifié dans les sources d'Iced : l'éditeur se déclare focusable aux opérations (`operation.focusable`), donc le menu d'édition de `edit.rs` le voit ; il ne traite les touches qu'après un clic sur lui. Reste non couvert par un test automatisé, faute de rendu : le chemin des événements — d'où la passe visuelle.
