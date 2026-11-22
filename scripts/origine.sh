#!/usr/bin/env bash
# shellcheck disable=SC1111
# (SC1111 : “ ” typographiques des traductions anglaises.)
# État d'origine (MLG-210) : ce qu'était la machine avant hyprdesk, pour
# « remettre comme avant » (scripts/uninstall.sh --restore).
#
# Sourcé par install.sh (origine_capture, avant toute modification) et par
# uninstall.sh (origine_restore). Demande ui.sh et deps.sh (hd_*, hyprdesk_t,
# hyprdesk_root), et les variables DRY, YES et REMOVE_PKGS (0/1) côté uninstall.
#
# Contenu de ~/.local/state/hyprdesk/origine/ (écrit une seule fois, jamais
# élagué ; hors de ~/.config/hypr, dont hyprdesk range les vieilles copies) :
#   manifest        une ligne par chemin suivi : « present <chemin> » ou
#                   « absent <chemin> » (chemins relatifs à $HOME, ou absolus
#                   pour /etc…) ; plus « date », « partielle » (hyprdesk déjà là)
#   utilisateur.tar les chemins de $HOME présents (liens symboliques gardés :
#                   services activés ou masqués de systemd --user)
#   systeme.tar     les chemins système présents et lisibles sans root
#   illisibles      chemins système présents mais illisibles : laissés tels quels
#   services-actifs services --user actifs à l'origine
#   paquets         paquets installés à l'origine (un par ligne)
#   paquets-ajoutes paquets que install.sh a installés depuis (deps.sh)
#   commandes       commandes présentes à l'origine (hyprpanel-chat, mymfa…)
#   gsettings       réglages GNOME que le moteur de thème peut poser (schéma
#                   clair/sombre, accent, police, icônes, curseur, sons) :
#                   « schéma clé valeur » ; relevé après coup (« tardif ») sur
#                   une origine plus ancienne que ce relevé

ORIGINE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/hyprdesk/origine"

# Chemins de $HOME que hyprdesk (installation ou démon) crée ou modifie.
ORIGINE_HOME_PATHS=(
    .config/hypr
    .config/kitty/kitty.conf
    .config/kitty/hyprdesk-theme.conf
    .config/gtk-3.0/gtk.css
    .config/gtk-3.0/hyprdesk-thunar.css
    .config/gtk-4.0/gtk.css
    .config/qt6ct/colors
    .config/mimeapps.list
    .config/xdg-terminals.list
    .config/autostart
    .config/systemd/user
    .config/hyprpanel/context.d/hyprdesk.md
    .local/share/icons/hyprdesk-smartui
    .local/share/fonts/hyprdesk
    .local/share/dbus-1/services/today.mindlog.MyMfa1.service
    .local/bin/hyprdesk
    .local/bin/hyprpanel
    .local/bin/hyprpanel-chat
    .local/bin/mymfa
)
# Chemins système (écran de connexion, PAM, clavier, polkit, GPU).
ORIGINE_SYS_PATHS=(
    /etc/greetd
    /etc/pam.d/greetd
    /etc/pam.d/hyprdesk-lock
    /etc/polkit-1/rules.d/50-hyprdesk-updates.rules
    /etc/hyprdesk
    /etc/systemd/system/hyprdesk-gpu.service
    /etc/vconsole.conf
    /etc/X11/xorg.conf.d/00-keyboard.conf
    /usr/local/bin/hyprdesk
    /var/lib/hyprdesk-greeter
)
# Paquets que hyprdesk installe hors de deps.sh (écran de connexion).
ORIGINE_EXTRA_PKGS=(greetd cage)
ORIGINE_CMDS=(hyprpanel-chat mymfa hyprlock hyprpaper)

origine_t() { hyprdesk_t "$@"; }

# Réglages GNOME que le moteur de thème peut poser (cibles gtk, fonts,
# icons, cursors, sounds) : « schéma clé ».
ORIGINE_GSETTINGS=(
    "org.gnome.desktop.interface gtk-theme"
    "org.gnome.desktop.interface color-scheme"
    "org.gnome.desktop.interface accent-color"
    "org.gnome.desktop.interface font-name"
    "org.gnome.desktop.interface icon-theme"
    "org.gnome.desktop.interface cursor-theme"
    "org.gnome.desktop.interface cursor-size"
    "org.gnome.desktop.sound theme-name"
)

# origine_gsettings_capture <fichier> [tardif] : valeurs en place, une par
# ligne. Fichier déjà là : seules les clés qui y manquent sont ajoutées
# (relevé tardif), les valeurs gardées ne bougent jamais.
origine_gsettings_capture() {
    local out="$1" late="${2:-}" pair v have="" lines=""
    command -v gsettings >/dev/null 2>&1 || return 0
    [[ -f "$out" ]] && have="$(cat -- "$out")"
    for pair in "${ORIGINE_GSETTINGS[@]}"; do
        grep -q "^$pair " <<< "$have" && continue
        # shellcheck disable=SC2086 # « schéma clé » : deux mots voulus
        v="$(gsettings get $pair 2>/dev/null)" || continue
        # Thème GTK du moteur en place : ce n'est pas l'origine.
        [[ "$pair" == *" gtk-theme" && "$v" == "'hyprdesk-"* ]] && continue
        lines+="$pair $v"$'\n'
    done
    [[ -n "$lines" ]] || { [[ -f "$out" ]] || : > "$out"; return 0; }
    if [[ -n "$late" ]]; then lines="# tardif $(date -Iseconds)"$'\n'"$lines"; fi
    printf '%s' "$lines" >> "$out"
}

origine_exists() { [[ -f "$ORIGINE_DIR/manifest" ]]; }

origine_pkgs() {
    case "$(hyprdesk_pm 2>/dev/null)" in
        pacman) pacman -Qq 2>/dev/null ;;
        apt-get) dpkg-query -W -f '${Package}\n' 2>/dev/null ;;
        dnf|zypper) rpm -qa --qf '%{NAME}\n' 2>/dev/null ;;
    esac | sort -u
}

# Photo de l'état d'origine, une seule fois. Rien n'est modifié ailleurs.
origine_capture() {
    if origine_exists; then
        hd_ok "$(origine_t "état d'origine déjà gardé" "original state already kept") (${ORIGINE_DIR/#$HOME/\~})"
        # Origine d'avant le relevé des réglages GNOME : relevés maintenant,
        # marqués « tardif » (ce qui est en place aujourd'hui).
        origine_gsettings_capture "$ORIGINE_DIR/gsettings" tardif
        return 0
    fi
    local tmp p
    tmp="$(mktemp -d "${ORIGINE_DIR%/*}/.origine-XXXXXX" 2>/dev/null)" || {
        mkdir -p "${ORIGINE_DIR%/*}" && tmp="$(mktemp -d "${ORIGINE_DIR%/*}/.origine-XXXXXX")"
    } || return 1
    {
        echo "date $(date -Iseconds)"
        # hyprdesk déjà passé par là : « comme avant » ramènera à cet instant.
        if [[ -e "$HOME/.config/hypr/hyprdesk.lua" || -e "$HOME/.config/systemd/user/hyprdesk.service" ]]; then
            echo "partielle 1"
        fi
    } > "$tmp/manifest"
    local home_present=() sys_present=()
    for p in "${ORIGINE_HOME_PATHS[@]}"; do
        if [[ -e "$HOME/$p" || -L "$HOME/$p" ]]; then
            echo "present $p" >> "$tmp/manifest"; home_present+=("$p")
        else
            echo "absent $p" >> "$tmp/manifest"
        fi
    done
    # ~/.config/hypr/hyprdesk/ (config de hyprdesk) et ses copies : pas l'origine.
    if ((${#home_present[@]} > 0)); then
        tar -C "$HOME" -cf "$tmp/utilisateur.tar" \
            --exclude=.config/hypr/hyprdesk --exclude='.config/hypr/*.avant-hyprdesk*' \
            -- "${home_present[@]}" 2>/dev/null || {
            rm -rf "$tmp"; hd_warn "$(origine_t "état d'origine non gardé (copie impossible)" "original state not kept (copy failed)")"; return 1
        }
    fi
    : > "$tmp/illisibles"
    for p in "${ORIGINE_SYS_PATHS[@]}"; do
        if [[ -e "$p" || -L "$p" ]]; then
            echo "present $p" >> "$tmp/manifest"
            # Illisible sans root : noté, et jamais touché à la restauration.
            if [[ -r "$p" ]] && { [[ ! -d "$p" ]] || [[ -x "$p" ]]; } \
                && ! find "$p" ! -readable -print -quit 2>/dev/null | grep -q .; then
                sys_present+=("${p#/}")
            else
                echo "$p" >> "$tmp/illisibles"
            fi
        else
            echo "absent $p" >> "$tmp/manifest"
        fi
    done
    if ((${#sys_present[@]} > 0)); then
        tar -C / -cf "$tmp/systeme.tar" -- "${sys_present[@]}" 2>/dev/null || true
    fi
    readlink /etc/systemd/system/display-manager.service > "$tmp/display-manager" 2>/dev/null || true
    systemctl --user list-units --type=service,socket --state=active --no-legend --plain 2>/dev/null \
        | awk '{print $1}' > "$tmp/services-actifs" || true
    origine_pkgs > "$tmp/paquets" || true
    : > "$tmp/paquets-ajoutes"
    for p in "${ORIGINE_CMDS[@]}"; do command -v "$p" >/dev/null 2>&1 && echo "$p"; done > "$tmp/commandes"
    origine_gsettings_capture "$tmp/gsettings"
    mv "$tmp" "$ORIGINE_DIR"
    hd_ok "$(origine_t "état d'origine gardé, pour tout remettre comme avant" "original state kept, to put everything back as it was") (${ORIGINE_DIR/#$HOME/\~})"
    if grep -q '^partielle' "$ORIGINE_DIR/manifest"; then
        hd_note "$(origine_t "hyprdesk était déjà installé : « comme avant » ramènera à l'état d'aujourd'hui" "hyprdesk was already installed: “as before” will bring back today's state")"
    fi
}

# Paquet installé par install.sh (appelé par hyprdesk_install_packages).
origine_note_pkgs() {
    [[ -f "$ORIGINE_DIR/paquets-ajoutes" ]] || return 0
    printf '%s\n' "$@" >> "$ORIGINE_DIR/paquets-ajoutes"
}

origine_has() { grep -qx "present $1" "$ORIGINE_DIR/manifest"; }
origine_cmd_was() { grep -qx "$1" "$ORIGINE_DIR/commandes" 2>/dev/null; }

# Paquets à retirer : ajoutés par hyprdesk et absents à l'origine.
origine_pkgs_added() {
    { cat "$ORIGINE_DIR/paquets-ajoutes" 2>/dev/null
      if ! grep -q '^/etc/greetd' "$ORIGINE_DIR/illisibles" 2>/dev/null && ! origine_has /etc/greetd; then
          printf '%s\n' "${ORIGINE_EXTRA_PKGS[@]}"
      fi
    } | sort -u | comm -23 - "$ORIGINE_DIR/paquets" | while read -r p; do
        [[ -n "$p" ]] && origine_pkgs | grep -qx -- "$p" && echo "$p"
    done
    return 0
}

origine_remove_pkgs() {
    case "$(hyprdesk_pm)" in
        pacman) hyprdesk_root pacman -Rns --noconfirm "$@" ;;
        apt-get) hyprdesk_root env DEBIAN_FRONTEND=noninteractive apt-get remove -y -qq "$@" ;;
        dnf) hyprdesk_root dnf remove -y -q "$@" ;;
        zypper) hyprdesk_root zypper --non-interactive --quiet remove "$@" ;;
    esac
}

# Remet l'état d'origine. $1 : dossier où ranger l'état actuel (rien n'est
# effacé : ce qui est apparu depuis y est déplacé). Après la désinstallation
# ordinaire (service arrêté, écran de connexion rendu).
origine_restore() {
    local keep="$1" p t='~'
    local h_list=() s_list=()
    while read -r state p; do
        [[ "$state" == present || "$state" == absent ]] || continue
        if [[ "$p" == /* ]]; then s_list+=("$p"); else h_list+=("$p"); fi
    done < "$ORIGINE_DIR/manifest"

    # -- Fichiers de l'utilisateur -----------------------------------------------------
    for p in "${h_list[@]}"; do
        local cur="$HOME/$p"
        if [[ -e "$cur" || -L "$cur" ]]; then
            if [[ "$DRY" == 1 ]]; then
                if origine_has "$p"; then
                    hd_info "$(origine_t "ferait : remis comme avant :" "would: put back:") $t/$p"
                else
                    hd_info "$(origine_t "ferait : rangé (absent avant hyprdesk) :" "would: moved aside (absent before hyprdesk):") $t/$p"
                fi
                continue
            fi
            mkdir -p "$keep/actuel/${p%/*}"
            mv -- "$cur" "$keep/actuel/$p"
        fi
        if origine_has "$p" && [[ "$DRY" == 0 ]]; then
            mkdir -p "$HOME/${p%/*}"
            if tar -C "$HOME" -xpf "$ORIGINE_DIR/utilisateur.tar" -- "$p" 2>/dev/null; then
                hd_ok "$t/$p $(origine_t "remis comme avant" "put back as it was")"
            else
                hd_warn "$t/$p : $(origine_t "non remis (voir" "not put back (see") ${keep/#$HOME/\~}/actuel)"
            fi
        elif [[ "$DRY" == 0 && -e "$keep/actuel/$p" ]]; then
            hd_ok "$t/$p $(origine_t "rangé : absent avant hyprdesk" "moved aside: absent before hyprdesk")"
        fi
    done
    # La config de hyprdesk, rangée avec ~/.config/hypr, ne revient pas.

    # -- Réglages GNOME (moteur de thème) ---------------------------------------------
    if [[ -f "$ORIGINE_DIR/gsettings" ]] && command -v gsettings >/dev/null 2>&1; then
        local schema key value
        grep -q '^# tardif' "$ORIGINE_DIR/gsettings" \
            && hd_note "$(origine_t "réglages GNOME relevés après l'installation de hyprdesk : remis à cet état-là" "GNOME settings recorded after hyprdesk was installed: put back to that state")"
        while read -r schema key value; do
            [[ -n "$schema" && "$schema" != \#* && -n "$value" ]] || continue
            [[ "$(gsettings get "$schema" "$key" 2>/dev/null)" == "$value" ]] && continue
            if [[ "$DRY" == 1 ]]; then
                hd_info "$(origine_t "ferait : remis comme avant :" "would: put back:") $schema $key"
            elif gsettings set "$schema" "$key" "$value" 2>/dev/null; then
                hd_ok "$schema $key $(origine_t "remis comme avant" "put back as it was")"
            else
                hd_warn "$schema $key : $(origine_t "non remis" "not put back")"
            fi
        done < "$ORIGINE_DIR/gsettings"
    else
        hd_note "$(origine_t "réglages GNOME (schéma clair/sombre, accent) non gardés à l'origine : laissés tels quels" "GNOME settings (light/dark scheme, accent) not kept originally: left as they are")"
    fi

    # -- Services de l'utilisateur ---------------------------------------------------
    # Activations et masques : des liens de ~/.config/systemd/user, remis ci-dessus.
    if [[ "$DRY" == 0 ]]; then
        systemctl --user daemon-reload >/dev/null 2>&1 || true
        local u
        for u in hyprdesk.service hyprpanel.service mymfa.service; do
            grep -qx "$u" "$ORIGINE_DIR/services-actifs" 2>/dev/null && continue
            systemctl --user stop "$u" >/dev/null 2>&1 || true
        done
        while read -r u; do
            [[ -n "$u" ]] || continue
            systemctl --user is-active --quiet "$u" 2>/dev/null && continue
            systemctl --user start "$u" >/dev/null 2>&1 \
                && hd_ok "$u $(origine_t "relancé (actif avant hyprdesk)" "restarted (active before hyprdesk)")" || true
        done < "$ORIGINE_DIR/services-actifs"
    else
        hd_info "$(origine_t "ferait : services --user remis comme avant (activés, masqués, lancés)" "would: user services put back (enabled, masked, running)")"
    fi

    # -- Système (sudo) ----------------------------------------------------------------
    local sys_todo=()
    for p in "${s_list[@]}"; do
        grep -qx -- "$p" "$ORIGINE_DIR/illisibles" 2>/dev/null && continue
        if origine_has "$p"; then
            # Présent avant : remis s'il a changé.
            if [[ -e "$p" || -L "$p" ]] && tar -C / -df "$ORIGINE_DIR/systeme.tar" -- "${p#/}" >/dev/null 2>&1; then
                continue
            fi
            sys_todo+=("$p")
        elif [[ -e "$p" || -L "$p" ]]; then
            sys_todo+=("$p")
        fi
    done
    if ((${#sys_todo[@]} > 0)); then
        if [[ "$DRY" == 1 ]]; then
            for p in "${sys_todo[@]}"; do
                if origine_has "$p"; then
                    hd_info "$(origine_t "ferait (sudo) : remis comme avant :" "would (sudo): put back:") $p"
                else
                    hd_info "$(origine_t "ferait (sudo) : rangé (absent avant hyprdesk) :" "would (sudo): moved aside (absent before hyprdesk):") $p"
                fi
            done
        elif hd_sudo_ready "$(origine_t "remettre les fichiers système comme avant" "put the system files back")"; then
            mkdir -p "$keep/systeme"
            [[ -e /etc/systemd/system/hyprdesk-gpu.service ]] && ! origine_has /etc/systemd/system/hyprdesk-gpu.service \
                && hyprdesk_root systemctl disable hyprdesk-gpu.service >/dev/null 2>&1 || true
            for p in "${sys_todo[@]}"; do
                if [[ -e "$p" || -L "$p" ]]; then
                    hyprdesk_root tar -C / -cf "$keep/systeme/$(echo "${p#/}" | tr / _).tar" -- "${p#/}" 2>/dev/null || true
                    hyprdesk_root rm -rf -- "$p"
                fi
                if origine_has "$p"; then
                    if hyprdesk_root tar -C / -xpf "$ORIGINE_DIR/systeme.tar" -- "${p#/}"; then
                        hd_ok "$p $(origine_t "remis comme avant" "put back as it was")"
                    else
                        hd_warn "$p : $(origine_t "non remis" "not put back")"
                    fi
                else
                    hd_ok "$p $(origine_t "retiré : absent avant hyprdesk (copie :" "removed: absent before hyprdesk (copy:") ${keep/#$HOME/\~}/systeme)"
                fi
            done
            hyprdesk_root chown -R "$(id -u):$(id -g)" "$keep/systeme" 2>/dev/null || true
            hyprdesk_root systemctl daemon-reload >/dev/null 2>&1 || true
        else
            hd_warn "$(origine_t "fichiers système non remis (sudo refusé) :" "system files not put back (sudo refused):") ${sys_todo[*]}"
        fi
    else
        hd_ok "$(origine_t "fichiers système : comme avant" "system files: as before")"
    fi
    if [[ -s "$ORIGINE_DIR/illisibles" ]]; then
        hd_note "$(origine_t "non gardés à l'origine (illisibles sans root), laissés tels quels :" "not kept originally (unreadable without root), left as they are:") $(tr '\n' ' ' < "$ORIGINE_DIR/illisibles")"
    fi

    # -- Paquets ---------------------------------------------------------------------
    local pkgs=()
    mapfile -t pkgs < <(origine_pkgs_added)
    if ((${#pkgs[@]} > 0)); then
        hd_info "$(origine_t "paquets installés pour hyprdesk :" "packages installed for hyprdesk:") ${pkgs[*]}"
        local go=0
        if [[ "$DRY" == 1 ]]; then
            hd_info "$(origine_t "ferait (après accord) : retrait de ces paquets" "would (once approved): remove these packages")"
        elif [[ "$YES" == 1 || "${REMOVE_PKGS:-0}" == 1 ]]; then
            go="${REMOVE_PKGS:-0}"
        elif [[ -t 0 ]]; then
            local reply
            printf '  %s?%s ' "$C_ACCENT$C_BOLD" "$C_RESET"
            read -r -p "$(origine_t "Les désinstaller aussi ? [o/N] " "Uninstall them too? [y/N] ")" reply
            [[ "$reply" =~ ^[oOyY] ]] && go=1
        fi
        if [[ "$go" == 1 ]] && hd_sudo_ready "$(origine_t "désinstaller les paquets" "uninstall the packages")"; then
            hd_run "$(origine_t "retrait des paquets" "removing packages")" origine_remove_pkgs "${pkgs[@]}" \
                || hd_warn "$(origine_t "certains paquets restent (d'autres en dépendent ?)" "some packages remain (others depend on them?)")"
        elif [[ "$DRY" == 0 ]]; then
            hd_note "$(origine_t "paquets gardés (--remove-packages pour les retirer) ; à la main :" "packages kept (--remove-packages to remove them); by hand:") $(hyprdesk_install_hint "$(hyprdesk_pm)" | sed 's/-S --needed/-Rns/; s/ install / remove /') ${pkgs[*]}"
        fi
    fi
    hd_note "$(origine_t "une mise à jour de Hyprland faite à l'installation ne se défait pas" "a Hyprland update done at install time cannot be undone")"
}
