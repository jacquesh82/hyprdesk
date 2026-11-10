-- Plugin de widget hyprdesk : la météo, via wttr.in.
-- Activer : Réglages ⚙ → Widgets, ou hd.widget("meteo", { ville = "Lyon" })
return {
    name        = "Météo",
    description = "Conditions actuelles (wttr.in) ; option : ville.",
    interval    = 900,
    anchor      = "top-left",
    options     = { ville = "Paris" },
    render = function(o)
        local v = (o.ville or ""):gsub("([^%w%-%_%.~])", function(c) return string.format("%%%02X", string.byte(c)) end)
        local out = hd.exec("curl -fsS --max-time 4 'https://wttr.in/" .. v .. "?format=%l:+%c+%t+%w&lang=fr'")
        if out == "" then return "météo indisponible" end
        return out
    end,
}
