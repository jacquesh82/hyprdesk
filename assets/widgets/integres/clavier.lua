-- Disposition du clavier (Hyprland) : la disposition active du clavier
-- principal ; clic : menu pour choisir, molette : disposition suivante.
--
-- Les dispositions elles-mêmes (input.kb_layout) restent réglées ailleurs :
-- hyprdesk n'écrit jamais dans inputs.lua.

-- Clavier principal, lu dans `hyprctl devices -j` (objets plats, sans
-- imbrication : un motif suffit, pas besoin d'analyseur JSON).
local function keyboard()
    local json = hd.exec("hyprctl devices -j 2>/dev/null")
    local first = nil
    for obj in json:gmatch("{[^{}]*}") do
        if obj:find('"active_keymap"', 1, true) then
            local kb = {
                layout = obj:match('"layout"%s*:%s*"([^"]*)"') or "",
                variant = obj:match('"variant"%s*:%s*"([^"]*)"') or "",
                index = tonumber(obj:match('"active_layout_index"%s*:%s*(%d+)')) or 0,
                keymap = obj:match('"active_keymap"%s*:%s*"([^"]*)"') or "",
            }
            if obj:find('"main"%s*:%s*true') then return kb end
            first = first or kb
        end
    end
    return first
end

local function split(s)
    local t = {}
    for part in (s .. ","):gmatch("([^,]*),") do t[#t + 1] = part end
    return t
end

-- Dispositions configurées : { code = "fr", variant = "azerty" }.
local function layouts(kb)
    local codes, variants, out = split(kb.layout), split(kb.variant), {}
    for i, code in ipairs(codes) do
        if code ~= "" then out[#out + 1] = { code = code, variant = variants[i] or "" } end
    end
    return out
end

local function switch(index)
    hd.exec("hyprctl switchxkblayout all " .. index)
end

return {
    name        = hd.t("Clavier"),
    description = hd.t("Disposition du clavier ; molette : disposition suivante."),
    interval    = 2,
    anchor      = "top-right",
    options     = { texte = true },
    render = function(o)
        local kb = keyboard()
        if not kb then return { state = "hidden" } end
        local list = layouts(kb)
        local cur = list[kb.index + 1] or list[1] or { code = "?" }
        return {
            icon = "input-keyboard-symbolic",
            text = o.texte and cur.code:upper() or "",
            tooltip = hd.t("Disposition : ") .. kb.keymap,
        }
    end,
    on_scroll = function(_, sens)
        local kb = keyboard()
        if kb and #layouts(kb) > 1 then
            hd.exec("hyprctl switchxkblayout all " .. (sens > 0 and "prev" or "next"))
        end
    end,
    menu = function()
        local kb = keyboard()
        local m = { { header = hd.t("Disposition du clavier") } }
        if not kb then return m end
        local list = layouts(kb)
        for i, l in ipairs(list) do
            local idx = i - 1
            m[#m + 1] = {
                label = l.code:upper(),
                detail = (idx == kb.index) and kb.keymap or (l.variant ~= "" and l.variant or nil),
                checked = idx == kb.index,
                action = function() switch(idx) end,
            }
        end
        if #list <= 1 then
            m[#m + 1] = { label = hd.t("Une seule disposition configurée"), detail = "input.kb_layout" }
        end
        return m
    end,
}
