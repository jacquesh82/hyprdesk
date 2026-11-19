-- hyprdesk : zones d'écran (MLG-172).
-- Un écran découpé en zones (1×2, 1×3, 2×2, 3×3) : chaque fenêtre pavée garde
-- sa zone ; maximiser ou mettre en plein écran ne couvre que sa zone
-- (l'application se croit en plein écran, Hyprland la dessine dans la zone).
-- Les fenêtres étiquetées « hyprdesk-pip » (vidéo du Panorama) restent aussi
-- dans leur cadre en plein écran.
--
-- Rejouable : un layout n'est enregistré qu'une fois par état Lua (le
-- réenregistrer est une erreur, même sous pcall) ; les abonnements sont
-- remplacés.

__hyprdesk_zones = __hyprdesk_zones or { zone = {}, specs = {}, registered = {} }
local Z = __hyprdesk_zones
Z.zone = Z.zone or {}
-- Fenêtres ouvertes depuis le chargement : placées selon la souris.
Z.opened = Z.opened or {}
Z.specs = Z.specs or {}
Z.registered = Z.registered or {}

local PREFIX = "lua:hyprdesk-zones-"
local PIP = "hyprdesk-pip"

-- Cellules d'une spécification { cols = {fractions}, rows = {fractions} }
-- dans `area` ; un écran portrait échange colonnes et lignes.
local function cells(area, spec)
    local cols, rows = spec.cols, spec.rows
    if area.h > area.w then cols, rows = rows, cols end
    local out = {}
    local y = area.y
    for r, fr in ipairs(rows) do
        local h = (r == #rows) and (area.y + area.h - y) or math.floor(area.h * fr + 0.5)
        local x = area.x
        for c, fc in ipairs(cols) do
            local w = (c == #cols) and (area.x + area.w - x) or math.floor(area.w * fc + 0.5)
            out[#out + 1] = { x = x, y = y, w = w, h = h, row = r, col = c }
            x = x + w
        end
        y = y + h
    end
    return out, #cols, #rows
end

local function cell_at(list, px, py)
    for i, b in ipairs(list) do
        if px >= b.x and px < b.x + b.w and py >= b.y and py < b.y + b.h then return i end
    end
end

-- Pavage dans une zone : une fenêtre = toute la zone ; sinon une principale
-- (55 % du grand côté) et une pile.
local function tile(list, box)
    local n = #list
    if n == 0 then return end
    if n == 1 then list[1]:place(box) return end
    local wide = box.w >= box.h
    local main = math.floor((wide and box.w or box.h) * 0.55 + 0.5)
    if wide then
        list[1]:place({ x = box.x, y = box.y, w = main, h = box.h })
    else
        list[1]:place({ x = box.x, y = box.y, w = box.w, h = main })
    end
    local rest = n - 1
    for i = 2, n do
        local a = math.floor(((wide and box.h or (box.h - main))) * (i - 2) / rest + 0.5)
        local b = math.floor(((wide and box.h or (box.h - main))) * (i - 1) / rest + 0.5)
        if wide then
            list[i]:place({ x = box.x + main, y = box.y + a, w = box.w - main, h = b - a })
        else
            list[i]:place({ x = box.x, y = box.y + main + a, w = box.w, h = b - a })
        end
    end
end

-- Centre de la fenêtre (position et taille actuelles).
local function center(w)
    local function xy(v)
        if type(v) ~= "table" then return nil end
        return v.x or v[1], v.y or v[2]
    end
    local x, y = xy(w.at)
    local sw, sh = xy(w.size)
    if not (x and y and sw and sh) then return nil end
    return x + sw / 2, y + sh / 2
end

local function recalc(ctx, spec)
    local list = cells(ctx.area, spec)
    local per = {}
    for i = 1, #list do per[i] = { tiled = {}, full = nil } end
    local fresh = {}
    for _, t in ipairs(ctx.targets) do
        local w = t.window
        local addr = w and w.address or ("target" .. t.index)
        local z = Z.zone[addr]
        if not z or z > #list then
            fresh[#fresh + 1] = { t = t, addr = addr }
        else
            local full = w and (w.fullscreen_client or 0) ~= 0
            if full then per[z].full = t else table.insert(per[z].tiled, t) end
        end
    end
    -- Nouvelle fenêtre : la zone sous la souris ; fenêtre déjà ouverte (écran
    -- qu'on vient de découper) : la zone de son centre ; sinon la moins occupée.
    for _, f in ipairs(fresh) do
        local z
        local w = f.t.window
        if w and not Z.opened[f.addr] then
            local cx, cy = center(w)
            z = cx and cell_at(list, cx, cy)
        end
        if not z then
            local c = hl.get_cursor_pos()
            z = c and cell_at(list, c.x, c.y)
        end
        if not z then
            z = 1
            for i = 2, #list do
                if #per[i].tiled < #per[z].tiled then z = i end
            end
        end
        Z.zone[f.addr] = z
        table.insert(per[z].tiled, f.t)
    end
    for i, p in ipairs(per) do
        tile(p.tiled, list[i])
        if p.full then p.full:place(list[i]) end
    end
end

-- Relance le placement (layout_msg ne le fait pas) : réaffirmer l'état
-- plein écran de la fenêtre suffit.
local function refresh(w)
    local sel = "address:" .. w.address
    hl.dispatch(hl.dsp.window.fullscreen_state({
        internal = 0, client = w.fullscreen_client or 0, action = "set", window = sel,
    }))
end

local function layout_msg(ctx, msg, spec)
    local w = hl.get_active_window()
    if not w then return false end
    local arg = msg:match("^zone%s+(%S+)$")
    if not arg then return false end
    local list, ncols = cells(ctx.area, spec)
    local n = #list
    local cur = Z.zone[w.address] or 1
    local nz
    if arg == "next" then nz = cur % n + 1
    elseif arg == "prev" then nz = (cur - 2) % n + 1
    elseif arg == "left" or arg == "right" or arg == "up" or arg == "down" then
        local c = list[cur]
        local dc = (arg == "left" and -1) or (arg == "right" and 1) or 0
        local dr = (arg == "up" and -1) or (arg == "down" and 1) or 0
        for i, b in ipairs(list) do
            if b.col == c.col + dc and b.row == c.row + dr then nz = i end
        end
    else nz = tonumber(arg) end
    if not nz or nz < 1 or nz > n then return false end
    Z.zone[w.address] = nz
    refresh(w)
    return true
end

-- Enregistre (une fois) le layout `name` ; la spécification est remplaçable.
function Z.define(name, cols, rows)
    Z.specs[name] = { cols = cols, rows = rows }
    if Z.registered[name] then return end
    Z.registered[name] = true
    hl.layout.register("hyprdesk-zones-" .. name, {
        recalculate = function(ctx) return recalc(ctx, Z.specs[name]) end,
        layout_msg = function(ctx, msg) return layout_msg(ctx, msg, Z.specs[name]) end,
    })
end

local function tagged(w, tag)
    local t = w.tags
    if type(t) == "string" then return (" " .. t .. " "):find(tag, 1, true) ~= nil end
    if type(t) == "table" then
        for _, v in pairs(t) do if v == tag then return true end end
    end
    return false
end

local function confined(w)
    if tagged(w, PIP) then return true end
    local ws = w.workspace
    return ws ~= nil and not w.floating
        and (ws.tiled_layout or ""):sub(1, #PREFIX) == PREFIX
end

-- Maximiser / plein écran (raccourci ou application) : ramenés à un plein
-- écran côté application seulement ; le layout (ou le cadre de la vidéo)
-- garde la fenêtre dans sa zone. Second appui : sortie.
local function on_fullscreen(w)
    if not w or not confined(w) then return end
    local internal = w.fullscreen or 0
    if internal == 0 then return end
    local sel = "address:" .. w.address
    local client = w.fullscreen_client or 0
    local was = Z.full and Z.full[w.address]
    Z.full = Z.full or {}
    if was then
        Z.full[w.address] = nil
        client = 0
    else
        Z.full[w.address] = true
        if client == 0 then client = internal end
    end
    hl.dispatch(hl.dsp.window.fullscreen_state({ internal = 0, client = client, action = "set", window = sel }))
    if client ~= 0 then
        -- les autres fenêtres de la zone sont pavées derrière
        hl.dispatch(hl.dsp.focus({ window = sel }))
        pcall(function() hl.dispatch(hl.dsp.window.alter_zorder({ mode = "top", window = sel })) end)
    end
end

local function on_state(w)
    if not w or not Z.full or not Z.full[w.address] then return end
    if (w.fullscreen_client or 0) == 0 then Z.full[w.address] = nil end
end

-- Une règle de workspace ne s'applique qu'à la création du workspace ou
-- quand elle change : la réaffirmer quand un workspace change d'écran.
local function reassert()
    for _, r in ipairs(Z.rules or {}) do
        pcall(function() r:set_enabled(false); r:set_enabled(true) end)
    end
end

for _, k in ipairs({ "sub_fs", "sub_state", "sub_destroy", "sub_move", "sub_mon", "sub_open" }) do
    if Z[k] then pcall(function() Z[k]:remove() end) end
end
Z.sub_fs = hl.on("window.fullscreen", function(w) pcall(on_fullscreen, w) end)
Z.sub_state = hl.on("window.update_state", function(w) pcall(on_state, w) end)
Z.sub_open = hl.on("window.open", function(w)
    if w and w.address then Z.opened[w.address] = true end
end)
Z.sub_move = hl.on("workspace.move_to_monitor", function() pcall(reassert) end)
Z.sub_mon = hl.on("monitor.added", function() pcall(reassert) end)
Z.sub_destroy = hl.on("window.destroy", function(w)
    if w and w.address then
        Z.zone[w.address] = nil
        Z.opened[w.address] = nil
        if Z.full then Z.full[w.address] = nil end
    end
end)
