-- Plugin de widget hyprdesk : la météo, via wttr.in.
-- Activer : Réglages ⚙ → Widgets, ou hd.widget("meteo", { city = "Lyon" })
-- (option aussi en français : ville).
return {
    name        = hd.t("Météo"),
    description = hd.t("Conditions actuelles (wttr.in) ; option : city."),
    interval    = 900,
    anchor      = "top-left",
    options     = { city = "Paris", ville = false },
    render = function(o)
        local v = (o.ville or o.city or ""):gsub("([^%w%-%_%.~])", function(c) return string.format("%%%02X", string.byte(c)) end)
        local out = hd.exec("curl -fsS --max-time 4 'https://wttr.in/" .. v .. "?format=%l:+%c+%t+%w&lang=" .. hd.lang() .. "'")
        if out == "" then return hd.t("météo indisponible") end
        return out
    end,
}
