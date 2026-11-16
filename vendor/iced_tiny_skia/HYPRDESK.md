# iced_tiny_skia 0.14.1 — copie complétée par hyprdesk

Correction : base de polices des SVG commune à tout le processus
(`src/vector.rs`, MLG-156). L'original en chargeait une par cache SVG, donc
par moteur de rendu, donc par surface : à la première icône de chaque
tiroir ouvert (assistant LYRA, Réglages, menus), `load_system_fonts()`
analysait toutes les polices du système sur le fil principal (3 179 polices,
2 Go sur le poste de développement ; 0,1 à 4 s de gel selon le cache disque).
Elle est désormais chargée une seule fois, et seulement pour un SVG qui
contient du texte (`<text`) ou qui est compressé ; les icônes reçoivent
une base vide, qui ne change rien à leur rendu.

## Origine

- Archive de départ : `iced_tiny_skia-0.14.1.crate`, https://static.crates.io/crates/iced_tiny_skia/iced_tiny_skia-0.14.1.crate
- Empreinte (sha256) de l'archive : `c267596d742714b1853cc10c3983a367762816fc4836bd3b79f76ce76787d6f8` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/iced-rs/iced, commit `0ecf60664df7b8ac7d7aef5f7279d5323027f693` (`.cargo_vcs_info.json` de l'archive).
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `iced_tiny_skia` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version d'iced, vérifier si l'amont partage enfin la base de polices (le `TODO: Reuse cosmic-text font database` d'origine), sinon repartir de l'archive et réappliquer le correctif.
