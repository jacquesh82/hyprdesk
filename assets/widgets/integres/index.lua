-- Progression du premier scan de l'index de fichiers (recherche :f).
--
-- Au quotidien l'état vit sur la loupe du dock (pastille d'accent : prêt ;
-- ambre : scan en cours ; rien : arrêté) : ce widget ne prend de la place
-- que pendant une indexation, pour suivre son avancée.
return {
    name = hd.t("Index de fichiers"),
    description = hd.t(
        "Progression du premier scan de la recherche :f ; le reste est sur la loupe du dock."),
    interval = 5,
    anchor = "bottom-left",
    render = function(o)
        local out = hd.exec("hyprdesk index status 2>/dev/null")
        local phase = out:match('"phase"%s*:%s*"([^"]*)"')
        if phase ~= "scan" then
            return { state = "hidden" }
        end
        local progression = tonumber(out:match('"progression"%s*:%s*(%d+)') or "0")
        return {
            text = string.format(hd.t("indexation : %d fichiers"), progression),
            tooltip = hd.t("Premier scan en cours ; l'index reste consultable via « :f »."),
            state = "active",
        }
    end,
    menu = function(o)
        return {
            { label = hd.t("Réindexer tout"), action = function() hd.spawn(hd.exe .. " index rebuild") end },
            { label = hd.t("Ouvrir la recherche (:f)"), action = function() hd.spawn(hd.exe .. " files") end },
        }
    end,
}
