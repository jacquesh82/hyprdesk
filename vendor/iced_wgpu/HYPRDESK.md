# iced_wgpu 0.14.0 — copie complétée par hyprdesk

Correction : base de polices des SVG commune à tout le processus
(`src/image/vector.rs`, MLG-156). L'original en chargeait une par cache SVG, donc
par moteur de rendu, donc par surface : à la première icône de chaque
tiroir ouvert (assistant LYRA, Réglages, menus), `load_system_fonts()`
analysait toutes les polices du système sur le fil principal (3 179 polices,
2 Go sur le poste de développement ; 0,1 à 4 s de gel selon le cache disque).
Elle est désormais chargée une seule fois, et seulement pour un SVG qui
contient du texte (`<text`) ou qui est compressé ; les icônes reçoivent
une base vide, qui ne change rien à leur rendu.

## Origine

- Archive de départ : `iced_wgpu-0.14.0.crate`, https://static.crates.io/crates/iced_wgpu/iced_wgpu-0.14.0.crate
- Empreinte (sha256) de l'archive : `ff144a999b0ca0f8a10257934500060240825c42e950ec0ebee9c8ae30561c13` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/iced-rs/iced, commit `3997291f318a8bc06fa522f5579836fb3feb94df` (`.cargo_vcs_info.json` de l'archive).
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `iced_wgpu` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version d'iced, vérifier si l'amont partage enfin la base de polices (le `TODO: Reuse cosmic-text font database` d'origine), sinon repartir de l'archive et réappliquer le correctif.
