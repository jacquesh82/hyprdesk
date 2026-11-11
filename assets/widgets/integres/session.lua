-- Session : verrouiller (écran de hyprdesk), veille, déconnexion, redémarrage, extinction.
return {
    name        = hd.t("Session"),
    description = hd.t("Verrouiller, mettre en veille, se déconnecter, redémarrer, éteindre."),
    interval    = 3600,
    anchor      = "top-right",
    -- Commandes de la session (à changer pour un autre gestionnaire que
    -- systemd ou une sortie propre à son setup), confirmation des gestes lourds.
    options     = { confirmer = true, veille = "systemctl suspend", deconnexion = "hyprctl dispatch 'hl.dsp.exit()'",
                    redemarrer = "systemctl reboot", eteindre = "systemctl poweroff" },
    render = function()
        return { icon = "system-shutdown-symbolic", tooltip = hd.t("Session") }
    end,
    menu = function(o)
        local host = hd.exec("cat /etc/hostname 2>/dev/null")
        local function ask(t) return o.confirmer and t or nil end
        return {
            { header = (os.getenv("USER") or "") .. (host ~= "" and (" · " .. host) or "") },
            { label = hd.t("Verrouiller"), icon = "system-lock-screen-symbolic", action = function() hd.spawn(string.format("%q lock", hd.exe)) end },
            { label = hd.t("Mettre en veille"), icon = "weather-clear-night-symbolic", action = function() hd.spawn(o.veille) end },
            "-",
            { label = hd.t("Se déconnecter…"), icon = "system-log-out-symbolic", confirm = ask(hd.t("Fermer la session ?")),
              action = function() hd.spawn(o.deconnexion) end },
            { label = hd.t("Redémarrer…"), icon = "system-reboot-symbolic", confirm = ask(hd.t("Redémarrer maintenant ?")),
              action = function() hd.spawn(o.redemarrer) end },
            { label = hd.t("Éteindre…"), icon = "system-shutdown-symbolic", confirm = ask(hd.t("Éteindre maintenant ?")),
              action = function() hd.spawn(o.eteindre) end },
        }
    end,
}
