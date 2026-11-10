-- Plugin de widget hyprdesk : les premières lignes d'un fichier de notes.
-- Activer : Réglages ⚙ → Widgets, ou hd.widget("notes", { fichier = "~/todo.txt", lignes = 8 })
return {
    name        = "Notes",
    description = "Premières lignes d'un fichier texte ; options : fichier, lignes.",
    interval    = 10,
    anchor      = "left",
    options     = { fichier = "~/notes.txt", lignes = 10 },
    render = function(o)
        local path = (o.fichier or ""):gsub("^~", os.getenv("HOME") or "~")
        local f = io.open(path, "r")
        if not f then return "(" .. o.fichier .. " introuvable)" end
        local out, n = {}, 0
        for line in f:lines() do
            n = n + 1
            if n > (tonumber(o.lignes) or 10) then break end
            out[#out + 1] = line
        end
        f:close()
        return table.concat(out, "\n")
    end,
}
