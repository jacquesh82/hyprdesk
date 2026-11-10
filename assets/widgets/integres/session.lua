-- Session : verrouiller, veille, déconnexion, redémarrage, extinction.
return {
    name        = "Session",
    description = "Verrouiller, mettre en veille, se déconnecter, redémarrer, éteindre.",
    interval    = 3600,
    anchor      = "top-right",
    render = function()
        return { icon = "system-shutdown-symbolic", tooltip = "Session" }
    end,
    menu = function()
        local host = hd.exec("cat /etc/hostname 2>/dev/null")
        return {
            { header = (os.getenv("USER") or "") .. (host ~= "" and (" · " .. host) or "") },
            { label = "Verrouiller", icon = "system-lock-screen-symbolic", action = function() hd.spawn("hyprlock") end },
            { label = "Mettre en veille", icon = "weather-clear-night-symbolic", action = function() hd.spawn("systemctl suspend") end },
            "-",
            { label = "Se déconnecter…", icon = "system-log-out-symbolic", confirm = "Fermer la session ?",
              action = function() hd.spawn("hyprctl dispatch 'hl.dsp.exit()'") end },
            { label = "Redémarrer…", icon = "system-reboot-symbolic", confirm = "Redémarrer maintenant ?",
              action = function() hd.spawn("systemctl reboot") end },
            { label = "Éteindre…", icon = "system-shutdown-symbolic", confirm = "Éteindre maintenant ?",
              action = function() hd.spawn("systemctl poweroff") end },
        }
    end,
}
