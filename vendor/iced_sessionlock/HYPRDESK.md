# iced_sessionlock 0.19.1 — copie complétée par hyprdesk

Correction : surface de l'écran de verrouillage reconfigurée après un échec
de présentation (`src/multi_window.rs`, MLG-175). L'original se contentait de
journaliser `Lost`, `Outdated`, `Timeout` ou `Other` : la surface n'était
reconfigurée que si sa taille changeait. Au réveil d'une mise en veille
(pilote NVIDIA), chaque présentation échouait donc à nouveau et l'écran
restait noir, le champ du mot de passe actif mais invisible. Désormais la
taille mémorisée est oubliée et un nouveau dessin demandé : la surface est
reconfigurée au dessin suivant.

## Origine

- Archive de départ : `iced_sessionlock-0.19.1.crate`, https://static.crates.io/crates/iced_sessionlock/iced_sessionlock-0.19.1.crate
- Empreinte (sha256) de l'archive : `519d0ba8279b4a2d9ce19bbc1824078135cc72c0dadf0dbb385c1507e0036f2f` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/waycrate/exwlshelleventloop ; l'archive ne porte pas de `.cargo_vcs_info.json`, le commit amont exact n'est donc pas connu : l'archive vérifiée fait référence.
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `iced_sessionlock` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version, vérifier si l'amont reconfigure enfin la surface après `Lost`/`Outdated`, sinon repartir de l'archive et réappliquer le correctif.

## Licence

- Code d'origine : MIT © waycrate/exwlshelleventloop — texte dans `LICENSE` ;
- Modifications hyprdesk : décrites ci-dessus ;
- Redistribution sous MIT.
