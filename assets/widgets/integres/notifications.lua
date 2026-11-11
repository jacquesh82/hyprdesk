-- Notifications : nombre, Ne pas déranger, dernières notifications.
-- Le démon en place : wayle s'il tourne (sa CLI), sinon hyprdesk lui-même
-- (son état dans $XDG_RUNTIME_DIR/hyprdesk-notifications.txt, et
-- `hyprdesk notifications dnd|clear`).

local STATE = (os.getenv("XDG_RUNTIME_DIR") or "/tmp") .. "/hyprdesk-notifications.txt"

local function wayle()
    local s = hd.exec("LC_ALL=C wayle notify status 2>/dev/null")
    if s == "" then return nil end
    local st = { source = "wayle", list = {},
                 count = tonumber(s:match("Notifications: (%d+)")) or 0,
                 dnd = s:find("Do Not Disturb: enabled") ~= nil }
    for app, title, body in hd.exec("LC_ALL=C wayle notify list 2>/dev/null"):gmatch("%[%d+%] ([^:\n]+): ([^\n]*)\n%s*([^\n]*)") do
        st.list[#st.list + 1] = { app = app, title = title, body = body }
        if #st.list >= 6 then break end
    end
    return st
end

local function native()
    local f = io.open(STATE, "r")
    if not f then return nil end
    local st = { source = "hyprdesk", list = {}, count = 0, dnd = false }
    for line in f:lines() do
        local k, v = line:match("^([a-z]+)=(.*)$")
        if k == "dnd" then st.dnd = v == "1"
        elseif k == "count" then st.count = tonumber(v) or 0
        elseif k == "n" and #st.list < 6 then
            local app, title, body = v:match("^([^\t]*)\t([^\t]*)\t?(.*)$")
            if app then st.list[#st.list + 1] = { app = app, title = title, body = body } end
        end
    end
    f:close()
    return st
end

local function status()
    return wayle() or native()
end

local function act(st, what)
    if st.source == "wayle" then
        hd.exec(what == "dnd" and "LC_ALL=C wayle notify dnd" or "LC_ALL=C wayle notify dismiss-all")
    else
        hd.exec(string.format("%q notifications %s", hd.exe, what))
    end
end

return {
    name        = hd.t("Notifications"),
    description = hd.t("Ne pas déranger et dernières notifications (wayle, sinon hyprdesk)."),
    interval    = 5,
    anchor      = "top-right",
    render = function()
        local s = status()
        if not s then return { state = "hidden" } end
        return {
            icon = s.dnd and "notifications-disabled-symbolic" or "preferences-system-notifications-symbolic",
            tooltip = s.dnd and hd.t("Ne pas déranger") or (s.count .. hd.t(" notification(s)")),
            state = s.dnd and "active" or "normal",
        }
    end,
    menu = function()
        local s = status() or { count = 0, dnd = false, list = {}, source = "hyprdesk" }
        local m = { { header = hd.t("Notifications") },
                    { label = hd.t("Ne pas déranger"), checked = s.dnd, action = function() act(s, "dnd") end } }
        for i, n in ipairs(s.list) do
            if i == 1 then m[#m + 1] = { header = hd.t("Récentes") } end
            m[#m + 1] = { label = n.title ~= "" and n.title or n.app, detail = n.body ~= "" and n.body or n.app }
        end
        if s.count > 0 then
            m[#m + 1] = "-"
            m[#m + 1] = { label = hd.t("Tout effacer"), action = function() act(s, "clear") end }
        end
        return m
    end,
}
