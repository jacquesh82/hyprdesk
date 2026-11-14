#!/usr/bin/env bash
# Installation « nightly » de hyprdesk : compilé depuis ce dépôt en l'état —
# le dernier commit, ou l'arbre de travail tel que vous l'avez modifié —
# hors release. C'est le même installateur que scripts/install.sh (assistant,
# LYRA, myMFA, Hyprbars…), qui compile hyprdesk au lieu d'installer le paquet
# de la dernière release publiée.
#
#   ./scripts/install_nightly_build.sh               au terminal : l'assistant
#   ./scripts/install_nightly_build.sh [options]     direct (options de install.sh)
#
# Mise à jour quotidienne depuis le dépôt : git pull puis relancer ce script ;
# ./scripts/update.sh suit les releases publiées, pas les commits.
set -euo pipefail
cd "$(dirname "$0")/.."
exec scripts/install.sh --from-source "$@"
