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
