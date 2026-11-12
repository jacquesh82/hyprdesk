#!/usr/bin/env bash
# Ancien nom de l'assistant d'installation, gardé pour les habitudes : il est
# désormais dans install.sh (lancé sans option au terminal).
exec "$(dirname "$0")/install.sh" "$@"
