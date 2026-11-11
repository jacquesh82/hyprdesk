-- Lecteur multimédia (MPRIS) : morceau en cours dans la barre ; menu avec
-- titre, artiste, album, position, et les commandes (précédent,
-- lecture / pause, suivant, ±10 s). Masqué quand rien n'est chargé.
-- L'état vient de `hyprdesk media status` (lignes clé=valeur).

local function cmd(args)
    hd.spawn(string.format("%q media %s", hd.exe, args))
end

local function status()
    local s = {}
    for line in (hd.exec(string.format("%q media status", hd.exe)) .. "\n"):gmatch("(.-)\n") do
        local k, v = line:match("^(%w+)=(.*)$")
        if k then s[k] = v end
    end
    if not s.player then return nil end
    s.playing = s.status == "playing"
    s.position, s.length = tonumber(s.position), tonumber(s.length)
    if (s.title or "") == "" and not s.playing then return nil end
    return s
end

local function clock(t)
    if not t then return "" end
    if t >= 3600 then return string.format("%d:%02d:%02d", t // 3600, t % 3600 // 60, t % 60) end
    return string.format("%d:%02d", t // 60, t % 60)
end

local function short(s, n)
    if utf8.len(s) and utf8.len(s) > n then
        return s:sub(1, (utf8.offset(s, n) or #s + 1) - 1) .. "…"
    end
    return s
end

return {
    name        = hd.t("Lecteur"),
    description = hd.t("Morceau en cours (MPRIS) ; menu : position et commandes."),
    interval    = 2,
    anchor      = "top-right",
    options     = { texte = true, longueur = 32 },
    render = function(o)
        local s = status()
        if not s then return { state = "hidden" } end
        local line = s.title ~= "" and s.title or s.player
        if (s.artist or "") ~= "" then line = line .. " — " .. s.artist end
        return {
            icon = s.playing and "media-playback-start-symbolic" or "media-playback-pause-symbolic",
            text = o.texte and short(line, tonumber(o.longueur) or 32) or "",
            tooltip = line,
            state = s.playing and "active" or "normal",
        }
    end,
    on_scroll = function(_, sens)
        cmd(sens > 0 and "seek 10" or "seek -10")
    end,
    menu = function()
        local s = status()
        local m = { { header = hd.t("Lecteur") } }
        if not s then
            m[#m + 1] = { label = hd.t("Aucun morceau en cours") }
            return m
        end
        -- Pochette : seulement un fichier local (file://, rien n'est téléchargé).
        if (s.art or ""):match("^file:///") then m[#m + 1] = { image = s.art } end
        m[#m + 1] = { label = s.title ~= "" and s.title or s.player,
                      detail = table.concat({ s.artist or "", s.album or "" }, (s.artist ~= "" and s.album ~= "") and " · " or "") }
        if s.length and s.length > 0 then
            m[#m + 1] = { label = clock(s.position) .. " / " .. clock(s.length), detail = s.player }
        else
            m[#m + 1] = { label = s.player }
        end
        m[#m + 1] = "-"
        m[#m + 1] = { label = s.playing and hd.t("Pause") or hd.t("Lecture"),
                      icon = s.playing and "media-playback-pause-symbolic" or "media-playback-start-symbolic",
                      action = function() cmd("play-pause") end }
        m[#m + 1] = { label = hd.t("Précédent"), icon = "media-skip-backward-symbolic", action = function() cmd("previous") end }
        m[#m + 1] = { label = hd.t("Suivant"), icon = "media-skip-forward-symbolic", action = function() cmd("next") end }
        if s.length and s.length > 0 then
            m[#m + 1] = { label = hd.t("Reculer de 10 s"), action = function() cmd("seek -10") end }
            m[#m + 1] = { label = hd.t("Avancer de 10 s"), action = function() cmd("seek 10") end }
        end
        return m
    end,
}
