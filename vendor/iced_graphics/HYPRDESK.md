# iced_graphics 0.14.0 — copie complétée par hyprdesk

Ajout : chiffres tabulaires (`src/text.rs`, MLG-274). `Font` d'iced ne
porte pas de variantes OpenType ; une famille dont le nom se termine par
`TABULAR_SUFFIX` (`"\u{1f}tnum"`) est cherchée sous son nom sans le
suffixe, avec la variante `tnum` activée dans cosmic-text. Les horloges et
le monitoring de hyprdesk s'en servent quand le thème veut des chiffres
tabulaires ; une police sans `tnum` garde ses chiffres d'origine.

## Origine

- Archive de départ : `iced_graphics-0.14.0.crate`, https://static.crates.io/crates/iced_graphics/iced_graphics-0.14.0.crate
- Empreinte (sha256) de l'archive : `234ca1c2cec4155055f68fa5fad1b5242c496ac8238d80a259bca382fb44a102` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/iced-rs/iced, commit `3997291f318a8bc06fa522f5579836fb3feb94df` (`.cargo_vcs_info.json` de l'archive).
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : à chaque nouvelle version d'iced, vérifier si `Font` porte enfin les variantes OpenType (alors retirer cette copie), sinon repartir de l'archive et réappliquer le correctif.
