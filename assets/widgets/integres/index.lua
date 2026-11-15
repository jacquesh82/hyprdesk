-- Progression du premier scan de l'index de fichiers (recherche :f).
--
-- Au quotidien l'état vit sur la loupe du dock (pastille d'accent : prêt ;
-- ambre : scan en cours ; rien : arrêté) : ce widget ne prend de la place
-- que pendant une indexation, pour suivre son avancée.
--
-- hd.index_status() interroge directement le socket du démon d'index (le
-- même que `hyprdesk index status`), sans lancer de processus ; nil : démon
-- arrêté.
return {
    name = hd.t("Index de fichiers"),
    description = hd.t(
        "Progression du premier scan de la recherche :f ; le reste est sur la loupe du dock."),
    interval = 5,
    anchor = "bottom-left",
    render = function(o)
        local st = hd.index_status()
        if not st or st.phase ~= "scan" then
            return { state = "hidden" }
        end
        return {
            text = string.format(hd.t("indexation : %d fichiers"), st.progression),
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
