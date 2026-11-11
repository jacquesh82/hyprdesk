-- Supports amovibles (udisks2) : clés USB, disques externes, cartes SD.
-- Masqué quand aucun n'est branché ; notification « Ouvrir » au branchement ;
-- menu pour ouvrir (montage par udisksctl si besoin) et éjecter (démontage
-- puis mise hors tension). L'autorisation éventuelle passe par l'agent
-- polkit de hyprdesk.

local function q(s) return "'" .. tostring(s):gsub("'", "'\\''") .. "'" end

-- `lsblk -P` : une ligne KEY="valeur" par périphérique.
local function rows()
    local out = {}
    local list = hd.exec("lsblk -P -p -o NAME,PKNAME,LABEL,SIZE,RM,HOTPLUG,TYPE,FSTYPE,MOUNTPOINT,TRAN,MODEL 2>/dev/null")
    for line in (list .. "\n"):gmatch("(.-)\n") do
        local r = {}
        for k, v in line:gmatch('(%u+)="([^"]*)"') do r[k] = v end
        if r.NAME then out[#out + 1] = r end
    end
    return out
end

-- Volumes montables des disques amovibles : { dev, disk, name, size, mount }.
local function volumes()
    local all, by = rows(), {}
    for _, r in ipairs(all) do by[r.NAME] = r end
    local out = {}
    for _, r in ipairs(all) do
        local disk = r.TYPE == "disk" and r or by[r.PKNAME or ""]
        local removable = disk and (disk.RM == "1" or disk.HOTPLUG == "1" or disk.TRAN == "usb")
        local fs = r.FSTYPE or ""
        if removable and fs ~= "" and fs ~= "swap" and fs ~= "crypto_LUKS" then
            local name = r.LABEL ~= "" and r.LABEL or (disk.MODEL ~= "" and disk.MODEL or r.NAME)
            out[#out + 1] = { dev = r.NAME, disk = disk.NAME, name = name, size = r.SIZE,
                              mount = r.MOUNTPOINT ~= "" and r.MOUNTPOINT or nil }
        end
    end
    return out
end

-- Ouvre un volume, en le montant d'abord s'il ne l'est pas.
local function open_cmd(v)
    return "p=$(lsblk -no MOUNTPOINT " .. q(v.dev) .. "); "
        .. "[ -z \"$p\" ] && udisksctl mount -b " .. q(v.dev) .. " >/dev/null && p=$(lsblk -no MOUNTPOINT " .. q(v.dev) .. "); "
        .. "[ -n \"$p\" ] && xdg-open \"$p\""
end

local function eject_cmd(disk, vols)
    local parts = {}
    for _, v in ipairs(vols) do
        if v.disk == disk and v.mount then parts[#parts + 1] = "udisksctl unmount -b " .. q(v.dev) end
    end
    parts[#parts + 1] = "udisksctl power-off -b " .. q(disk)
    return table.concat(parts, " && ")
end

-- Volumes déjà vus : seuls les nouveaux sont annoncés (pas ceux présents au démarrage).
local known = nil

local function announce(vols)
    local now = {}
    for _, v in ipairs(vols) do
        now[v.dev] = true
        if known and not known[v.dev] then
            hd.spawn("sh -c " .. q(
                "r=$(notify-send -a hyprdesk -i drive-removable-media -A open=" .. q(hd.t("Ouvrir")) .. " "
                .. q(hd.t("Support connecté")) .. " " .. q(v.name .. " · " .. v.size) .. "); "
                .. "[ \"$r\" = open ] && { " .. open_cmd(v) .. "; }"))
        end
    end
    known = now
end

return {
    name        = hd.t("Supports amovibles"),
    description = hd.t("Clés USB et disques externes : ouvrir, éjecter ; masqué s'il n'y en a pas."),
    interval    = 3,
    anchor      = "top-right",
    options     = { notifier = true, texte = false },
    render = function(o)
        local vols = volumes()
        if o.notifier then announce(vols) end
        if #vols == 0 then return { state = "hidden" } end
        local names = {}
        for _, v in ipairs(vols) do
            names[#names + 1] = v.name .. " — " .. v.size .. (v.mount and (" · " .. v.mount) or "")
        end
        return { icon = "drive-removable-media-symbolic", text = o.texte and tostring(#vols) or "",
                 tooltip = table.concat(names, "\n") }
    end,
    menu = function()
        local vols = volumes()
        local m = { { header = hd.t("Supports amovibles") } }
        if #vols == 0 then
            m[#m + 1] = { label = hd.t("Aucun support branché") }
            return m
        end
        local disks, order = {}, {}
        for _, v in ipairs(vols) do
            m[#m + 1] = { label = v.name, icon = "drive-removable-media-symbolic",
                          detail = v.size .. " · " .. (v.mount or hd.t("non monté")),
                          action = function() hd.spawn("sh -c " .. q(open_cmd(v))) end }
            if not disks[v.disk] then disks[v.disk] = v.name; order[#order + 1] = v.disk end
        end
        m[#m + 1] = "-"
        for _, d in ipairs(order) do
            m[#m + 1] = { label = string.format(hd.t("Éjecter « %s »"), disks[d]), icon = "media-eject-symbolic",
                          action = function()
                              hd.spawn("sh -c " .. q(eject_cmd(d, vols) .. " || notify-send -a hyprdesk -u critical "
                                  .. q(hd.t("Éjection impossible")) .. " " .. q(hd.t("Un fichier du support est peut-être encore ouvert."))))
                          end }
        end
        return m
    end,
}
