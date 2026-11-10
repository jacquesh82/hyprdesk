-- Session : verrouiller (écran de hyprdesk), veille, déconnexion, redémarrage, extinction.
return {
    name        = hd.t("Session"),
    description = hd.t("Verrouiller, mettre en veille, se déconnecter, redémarrer, éteindre."),
    interval    = 3600,
    anchor      = "top-right",
    render = function()
        return { icon = "system-shutdown-symbolic", tooltip = hd.t("Session") }
    end,
    menu = function()
        local host = hd.exec("cat /etc/hostname 2>/dev/null")
        return {
            { header = (os.getenv("USER") or "") .. (host ~= "" and (" · " .. host) or "") },
            { label = hd.t("Verrouiller"), icon = "system-lock-screen-symbolic", action = function() hd.spawn(string.format("%q lock", hd.exe)) end },
            { label = hd.t("Mettre en veille"), icon = "weather-clear-night-symbolic", action = function() hd.spawn("systemctl suspend") end },
            "-",
            { label = hd.t("Se déconnecter…"), icon = "system-log-out-symbolic", confirm = hd.t("Fermer la session ?"),
              action = function() hd.spawn("hyprctl dispatch 'hl.dsp.exit()'") end },
            { label = hd.t("Redémarrer…"), icon = "system-reboot-symbolic", confirm = hd.t("Redémarrer maintenant ?"),
              action = function() hd.spawn("systemctl reboot") end },
            { label = hd.t("Éteindre…"), icon = "system-shutdown-symbolic", confirm = hd.t("Éteindre maintenant ?"),
              action = function() hd.spawn("systemctl poweroff") end },
        }
    end,
}
