# iced_layershell 0.19.1 — copie complétée par hyprdesk

Ajout : événements iced du glisser-déposer de fichiers, à partir de ceux
ajoutés à layershellev (voir vendor/layershellev/HYPRDESK.md).

- entrée : position, puis `window::Event::FileHovered` (chemin vide) ;
- mouvement : `CursorMoved` ; sortie : `FilesHoveredLeft` ;
- dépôt : position, puis un `window::Event::FileDropped(chemin)` par fichier.

Fichiers touchés : `src/event.rs`, `src/conversion.rs`, `src/multi_window.rs`.
