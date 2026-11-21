# sessionlockev 0.19.1 — copie complétée par hyprdesk

Correction : l'événement `ext_session_lock_v1.finished` n'est plus ignoré
(`src/lib.rs`). L'original le traitait en `delegate_noop` : quand Hyprland
refusait le verrou (session déjà verrouillée par un écran de verrouillage
disparu, autre verrou actif) ou y mettait fin, `hyprdesk lock` attendait pour
toujours sans rien verrouiller, en gardant son verrou d'instance ; tout
verrouillage suivant se croyait déjà fait et la session restait ouverte.
Désormais le verrou est détruit, la boucle s'arrête et `running*` rend
`SessionLockEventError::LockFinished`.

## Origine

- Archive de départ : `sessionlockev-0.19.1.crate`, https://static.crates.io/crates/sessionlockev/sessionlockev-0.19.1.crate
- Empreinte (sha256) de l'archive : `a50dd03affe5176bca3e763ebfc3b68f423e0096b4c1c6662d598b103b64c3be` (la somme que crates.io publie pour cette version)
- Dépôt amont : https://github.com/waycrate/exwlshelleventloop ; l'archive ne porte pas de `.cargo_vcs_info.json`, le commit amont exact n'est donc pas connu : l'archive vérifiée fait référence.
- Modifications de hyprdesk : `patches/hyprdesk.patch` (diff de l'archive à cette copie). `scripts/verifier-vendor.sh` retélécharge l'archive, vérifie l'empreinte, applique le correctif et compare à la copie ; la CI le lance (MLG-151). Après une modification voulue : `scripts/verifier-vendor.sh --regenerer`, puis la décrire ci-dessus.
- Veille : suivre les versions publiées de `sessionlockev` sur crates.io (et les avis RustSec, qui ne couvrent plus cette copie) ; à chaque nouvelle version, vérifier si l'amont traite enfin `finished`, sinon repartir de l'archive et réappliquer le correctif.

## Licence

- Code d'origine : MIT © waycrate/exwlshelleventloop — texte dans `LICENSE` ;
- Modifications hyprdesk : décrites ci-dessus ;
- Redistribution sous MIT.
