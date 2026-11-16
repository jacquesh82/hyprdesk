# iced_layershell 0.19.1 — copie complétée par hyprdesk

Ajout : événements iced du glisser-déposer de fichiers, à partir de ceux
ajoutés à layershellev (voir vendor/layershellev/HYPRDESK.md).

- entrée : position, puis `window::Event::FileHovered` (chemin vide) ;
- mouvement : `CursorMoved` ; sortie : `FilesHoveredLeft` ;
- dépôt : position, puis un `window::Event::FileDropped(chemin)` par fichier.

Fichiers touchés : `src/event.rs`, `src/conversion.rs`, `src/multi_window.rs`.

Ajout : lecture du presse-papiers par `wl-paste` (`src/clipboard.rs`). Le
data device ajouté à layershellev pour le glisser-déposer reçoit la
sélection à la place de celui de smithay-clipboard (Hyprland n'en sert
qu'un par client) : sans ce détour, Ctrl+V ne collait rien dans les champs
texte. Repli sur smithay-clipboard si `wl-paste` manque ou échoue ; rien
quand le presse-papiers est désactivé (`disable_clipboard`).

Ajout : `renderer_information()` (`src/lib.rs`), le moteur obtenu à la
création du compositeur (`src/multi_window.rs`). `ICED_BACKEND` dit quoi
essayer, wgpu retombe sur tiny-skia sans le dire : Réglages → Performance
affiche ainsi le moteur qui tourne vraiment. `set_backend()` donne la liste
des moteurs à essayer sans passer par `ICED_BACKEND`, dont hériteraient les
sous-processus du démon.

Ajout : `damage` (`src/lib.rs`, `src/multi_window.rs`). L'original redessine
et présente toutes les surfaces après chaque lot de messages ; l'application
peut déclarer, message par message, les surfaces qu'il touche
(`damage::all/only/none`). Un lot dont un message n'a rien déclaré redessine
tout, comme l'original (MLG-131).

Ajout : écriture du presse-papiers par `wl-copy`, et smithay-clipboard relié
seulement si wl-clipboard manque (`src/clipboard.rs`). Son fil tournait sa
propre boucle sur la connexion Wayland partagée, réveillé par chaque événement
des surfaces (≈ 44 fois/s au repos, MLG-136).

## Origine

- Archive de départ : `iced_layershell-0.19.1.crate`, https://static.crates.io/crates/iced_layershell/iced_layershell-0.19.1.crate
- Empreinte (sha256) de l'archive : `51b59e8af02bf4c51db2ee5d130293e6e1e5ac4ae9f74a1e57990a19221aa0d5` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/waycrate/exwlshelleventloop ; l'archive ne porte pas de `.cargo_vcs_info.json`, le commit amont exact n'est donc pas connu : l'archive vérifiée fait référence.
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `iced_layershell` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version, repartir de l'archive et réappliquer le correctif.

## Licence

- Code d'origine : MIT © waycrate/exwlshelleventloop — texte dans `LICENSE` ;
- Modifications hyprdesk : décrites ci-dessus ;
- Redistribution sous MIT.
