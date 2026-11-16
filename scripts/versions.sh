# shellcheck shell=bash disable=SC2034
# Versions des composants mindlog installés avec hyprdesk : les releases
# publiées en même temps que cette version de hyprdesk (Cargo.toml). Lu par
# install.sh (sourcé) et par update.sh (comme données : seules les lignes
# NOM="${HYPRDESK_NOM:-valeur}" ci-dessous sont reconnues) ; à relever à
# chaque release de hyprdesk.
#
#   LYRA   https://github.com/jacquesh82/mindlog.hyprland_tools/releases
#   myMFA  https://github.com/lelabnet-creator/mindlog.mymfa.clients/releases
#          (tags linux-v* : le client Linux, seul installé par hyprdesk)
#
# Chaque composant est épinglé deux fois (MLG-154) : par son tag, et par le
# SHA complet du commit que ce tag désignait à la release. Après le clone,
# install.sh et update.sh comparent le commit obtenu à celui-ci et refusent
# de compiler s'il diffère (tag déplacé ou dépôt substitué). Relever un
# commit : git ls-remote <dépôt> 'refs/tags/<tag>^{}'.
#
# HYPRDESK_LYRA_REF et HYPRDESK_MYMFA_REF passent outre (ex. main, pour
# suivre le développement) : sans HYPRDESK_LYRA_COMMIT / HYPRDESK_MYMFA_COMMIT
# à côté, le commit n'est alors plus vérifié, et l'installation le signale.
# hyprpanel-core, bibliothèque de mindlog.hyprland_tools, est épinglé à part,
# par commit, dans Cargo.toml.
LYRA_REF="${HYPRDESK_LYRA_REF:-v0.1.0}"
LYRA_COMMIT="${HYPRDESK_LYRA_COMMIT:-3309cbfea75c73a81c753ed48290732c89ae5b1a}"
MYMFA_REF="${HYPRDESK_MYMFA_REF:-linux-v0.1.0}"
MYMFA_COMMIT="${HYPRDESK_MYMFA_COMMIT:-ad63ac8de409432a21053f2eae506c9ea700bc45}"
# Référence passée à la main sans son commit : rien à vérifier.
if [[ -n "${HYPRDESK_LYRA_REF:-}" && -z "${HYPRDESK_LYRA_COMMIT:-}" ]]; then LYRA_COMMIT=""; fi
if [[ -n "${HYPRDESK_MYMFA_REF:-}" && -z "${HYPRDESK_MYMFA_COMMIT:-}" ]]; then MYMFA_COMMIT=""; fi
# Plugins de Hyprland (Hyprbars, hyprwm/hyprland-plugins) : vide, hyprpm
# choisit le commit compatible avec le Hyprland qui tourne d'après les
# commit_pins (SHA complets) du dépôt de hyprwm ; un SHA ici l'impose
# (hyprpm add <url> <rev>).
HYPRLAND_PLUGINS_REV="${HYPRDESK_HYPRLAND_PLUGINS_REV:-}"
