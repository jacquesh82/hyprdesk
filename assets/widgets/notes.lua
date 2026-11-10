-- Plugin de widget hyprdesk : les premières lignes d'un fichier de notes.
-- Activer : Réglages ⚙ → Widgets, ou hd.widget("notes", { file = "~/todo.txt", lines = 8 })
-- (options aussi en français : fichier, lignes).
return {
    name        = hd.t("Notes"),
    description = hd.t("Premières lignes d'un fichier texte ; options : file, lines."),
    interval    = 10,
    anchor      = "left",
    options     = { file = "~/notes.txt", lines = 10, fichier = false, lignes = false },
    render = function(o)
        local file = o.fichier or o.file or ""
        local path = file:gsub("^~", os.getenv("HOME") or "~")
        local f = io.open(path, "r")
        if not f then return "(" .. file .. hd.t(" introuvable)") end
        local out, n = {}, 0
        for line in f:lines() do
            n = n + 1
            if n > (tonumber(o.lignes or o.lines) or 10) then break end
            out[#out + 1] = line
        end
        f:close()
        return table.concat(out, "\n")
    end,
}
