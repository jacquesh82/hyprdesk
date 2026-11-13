# shellcheck shell=bash disable=SC2034
# Versions des composants mindlog installés avec hyprdesk : les releases
# publiées en même temps que cette version de hyprdesk (Cargo.toml). Lu par
# install.sh et update.sh ; à relever à chaque release de hyprdesk.
#
#   LYRA   https://github.com/jacquesh82/mindlog.hyprland_tools/releases
#   myMFA  https://github.com/lelabnet-creator/mindlog.mymfa.clients/releases
#          (tags linux-v* : le client Linux, seul installé par hyprdesk)
#
# Un tag ou une branche ; HYPRDESK_LYRA_REF et HYPRDESK_MYMFA_REF passent
# outre (ex. main, pour suivre le développement).
# hyprpanel-core, bibliothèque de mindlog.hyprland_tools, est épinglé à part,
# par commit, dans Cargo.toml.
LYRA_REF="${HYPRDESK_LYRA_REF:-v0.1.0}"
MYMFA_REF="${HYPRDESK_MYMFA_REF:-linux-v0.1.0}"
