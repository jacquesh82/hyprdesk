# layershellev 0.19.1 — copie complétée par hyprdesk

Ajout : glisser-déposer de fichiers vers les surfaces (`src/dnd.rs`).

- `wl_data_device_manager` lié après le siège (facultatif) ;
- `DispatchMessage::{DndEnter, DndMotion, DndLeave, DndDrop}` (`src/events.rs`) ;
- au dépôt, la liste `text/uri-list` est lue auprès de la source (3 s au plus).

À reporter lors d'une mise à jour (ou à proposer en amont : waycrate/exwlshelleventloop).

## Licence

- Code d'origine : MIT © waycrate/exwlshelleventloop — texte dans `LICENSE` ;
- Modifications hyprdesk : décrites ci-dessus ;
- Redistribution sous MIT.
