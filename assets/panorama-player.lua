-- Cadre des lecteurs Panorama, dans le compositeur (aucun IPC par image).
-- Le bail est renouvelé par hyprdesk ; un arrêt du processus libère le cadre.
__hyprdesk_player = __hyprdesk_player or { frames = {} }
local P = __hyprdesk_player
if P.timer then return end
P.timer = hl.timer(function()
    local now = os.time()
    for addr, r in pairs(P.frames) do
        local ok, live = pcall(function()
            local w = hl.get_window("address:" .. addr)
            if not w or now > r.until_time then return false end
            local tagged = false
            for _, tag in pairs(w.tags or {}) do
                if tag == "hyprdesk-pip" then tagged = true end
            end
            if not tagged then return false end
            local at, size = w.at, w.size
            local x, y = at.x or at[1], at.y or at[2]
            local width, height = size.x or size[1], size.y or size[2]
            if width ~= r.w or height ~= r.h then
                hl.dispatch(hl.dsp.window.resize({ x = r.w, y = r.h, relative = false, window = "address:" .. addr }))
            end
            if x ~= r.x or y ~= r.y then
                hl.dispatch(hl.dsp.window.move({ x = r.x, y = r.y, relative = false, window = "address:" .. addr }))
            end
            return true
        end)
        if not ok or not live then P.frames[addr] = nil end
    end
end, { timeout = 33, type = "repeat" })
