-- Trousseau myMFA (MLG-44) : état du coffre, déverrouillage, verrouillage,
-- et accès aux codes 2FA et mots de passe du lanceur (:2fa, :mdp).
--
-- L'état est écrit par le démon hyprdesk dans $XDG_RUNTIME_DIR/hyprdesk/mymfa
-- (absent, sans-coffre, verrouille, ouvert) ; client myMFA absent : le module
-- se cache.

local function state()
    local dir = os.getenv("XDG_RUNTIME_DIR")
    if not dir then return "absent" end
    local f = io.open(dir .. "/hyprdesk/mymfa", "r")
    if not f then return "absent" end
    local s = f:read("l") or "absent"
    f:close()
    return s
end

local function hyprdesk(args)
    hd.spawn(string.format("%q %s", hd.exe, args))
end

return {
    name        = hd.t("myMFA"),
    description = hd.t("Trousseau myMFA : coffre verrouillé ou ouvert, codes 2FA et mots de passe."),
    interval    = 2,
    anchor      = "top-right",
    render = function()
        local s = state()
        if s == "absent" then return { state = "hidden" } end
        if s == "sans-coffre" then
            return { icon = "system-lock-screen-symbolic", state = "warning",
                     tooltip = hd.t("myMFA : pas encore de coffre (Réglages → Mots de passe)") }
        end
        if s == "verrouille" then
            return { icon = "system-lock-screen-symbolic", state = "normal",
                     tooltip = hd.t("myMFA : coffre verrouillé — clic pour le déverrouiller") }
        end
        return { icon = "system-lock-screen-symbolic", state = "active",
                 tooltip = hd.t("myMFA : coffre ouvert") }
    end,
    menu = function()
        local s = state()
        local m = { { header = hd.t("myMFA") } }
        if s == "verrouille" then
            m[#m + 1] = { label = hd.t("Déverrouiller le coffre…"), icon = "system-lock-screen-symbolic",
                          action = function() hyprdesk("mymfa unlock") end }
        elseif s == "ouvert" then
            m[#m + 1] = { label = hd.t("Codes 2FA"), icon = "dialog-password-symbolic", detail = ":2fa",
                          action = function() hyprdesk("mymfa codes") end }
            m[#m + 1] = { label = hd.t("Mots de passe"), icon = "dialog-password-symbolic", detail = ":mdp",
                          action = function() hyprdesk("mymfa logins") end }
            m[#m + 1] = { label = hd.t("Verrouiller le coffre"), icon = "system-lock-screen-symbolic",
                          action = function() hyprdesk("mymfa lock") end }
        end
        m[#m + 1] = { label = hd.t("Réglages des mots de passe…"), icon = "preferences-system-symbolic",
                      action = function() hyprdesk("settings passwords") end }
        return m
    end,
}
