# layershellev 0.19.1 — copie complétée par hyprdesk

Ajout : glisser-déposer de fichiers vers les surfaces (`src/dnd.rs`).

- `wl_data_device_manager` lié après le siège (facultatif) ;
- `DispatchMessage::{DndEnter, DndMotion, DndLeave, DndDrop}` (`src/events.rs`) ;
- dès l'entrée, la liste `text/uri-list` est lue auprès de la source dans un fil à part (3 s au plus par lecture), pour ne pas figer la boucle : attendue 30 ms à l'entrée et portée par `DndEnter { paths }`, sinon redite par un second `DndEnter` au premier mouvement qui la trouve lue ; resservie au dépôt, attendue alors 3 s au plus. Wayland crée une offre par surface survolée : une lecture par surface.
- `leave` reçu après `drop` (wlroots l'envoie) n'émet pas de `DndLeave` : la surface est déjà servie.

À reporter lors d'une mise à jour (ou à proposer en amont : waycrate/exwlshelleventloop).

Ajout : titre et `app_id` des fenêtres xdg (`NewXdgWindowSettings` dans `src/events.rs`,
appliqués dans `src/lib.rs`). L'original laissait l'`app_id` vide : les règles de
Hyprland ne pouvaient pas reconnaître la fenêtre d'aperçu du Panorama
(app_id `hyprdesk-panorama-preview`, flottante et centrée, MLG-164).

## Origine

- Archive de départ : `layershellev-0.19.1.crate`, https://static.crates.io/crates/layershellev/layershellev-0.19.1.crate
- Empreinte (sha256) de l'archive : `69fbd3ced50d91a5a118e3e00f49c72040ea6b48b78b159a3936104fc2632925` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/waycrate/exwlshelleventloop ; l'archive ne porte pas de `.cargo_vcs_info.json`, le commit amont exact n'est donc pas connu : l'archive vérifiée fait référence.
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `layershellev` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version, repartir de l'archive et réappliquer le correctif.

## Licence

- Code d'origine : MIT © waycrate/exwlshelleventloop — texte dans `LICENSE` ;
- Modifications hyprdesk : décrites ci-dessus ;
- Redistribution sous MIT.
