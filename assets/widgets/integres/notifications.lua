-- Notifications (service de wayle) : nombre, Ne pas déranger, dernières notifications.
local function status()
    local s = hd.exec("LC_ALL=C wayle notify status 2>/dev/null")
    if s == "" then return nil end
    return { count = tonumber(s:match("Notifications: (%d+)")) or 0, dnd = s:find("Do Not Disturb: enabled") ~= nil }
end

return {
    name        = hd.t("Notifications"),
    description = hd.t("Ne pas déranger et dernières notifications (wayle)."),
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
        local s = status() or { count = 0, dnd = false }
        local m = { { header = hd.t("Notifications") },
                    { label = hd.t("Ne pas déranger"), checked = s.dnd, action = function() hd.exec("LC_ALL=C wayle notify dnd") end } }
        local n = 0
        for app, title, body in hd.exec("LC_ALL=C wayle notify list 2>/dev/null"):gmatch("%[%d+%] ([^:\n]+): ([^\n]*)\n%s*([^\n]*)") do
            n = n + 1
            if n == 1 then m[#m + 1] = { header = hd.t("Récentes") } end
            if n > 6 then break end
            m[#m + 1] = { label = title ~= "" and title or app, detail = body ~= "" and body or app }
        end
        if s.count > 0 then
            m[#m + 1] = "-"
            m[#m + 1] = { label = hd.t("Tout effacer"), action = function() hd.exec("LC_ALL=C wayle notify dismiss-all") end }
        end
        return m
    end,
}
