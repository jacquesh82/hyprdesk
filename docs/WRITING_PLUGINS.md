# Writing a hyprdesk plugin

*[README](../README.md) · [Advanced guide](ADVANCED.md)*

A plugin is one Lua file that returns a table. hyprdesk picks it up as soon
as you save it, with no recompiling and no restart. The same plugin works as
a desktop widget and as a bar module.

This guide builds one step by step. For the full API, see the
[configuration reference](ADVANCED.md#configuration-reference) and the LuaLS
definitions written by `hyprdesk stubs`.

- [1. Hello world](#1-hello-world)
- [2. Options](#2-options)
- [3. Rich rendering: icon, tooltip, state](#3-rich-rendering-icon-tooltip-state)
- [4. Clicks, scroll and menus](#4-clicks-scroll-and-menus)
- [5. Running commands](#5-running-commands)
- [6. Translations](#6-translations)
- [7. A complete example](#7-a-complete-example)
- [8. Limits and good practice](#8-limits-and-good-practice)
- [9. Debugging](#9-debugging)
- [10. Sharing a plugin](#10-sharing-a-plugin)

## 1. Hello world

Create `~/.config/hypr/hyprdesk/widgets/hello.lua`:

```lua
return {
    name        = "Hello",
    description = "My first plugin",
    interval    = 60,                -- seconds between two render() calls
    anchor      = "top-left",        -- default place on the desktop
    render = function(o)
        return "Hello, " .. (os.getenv("USER") or "world")
    end,
}
```

Save the file. **hello** now shows up in Settings → Widgets, and you can drag
it onto the desktop. You can also declare it in `hyprdesk.lua`:

```lua
hd.widget("hello")                              -- on the desktop
hd.bar({ right = { "hello", "volume" } })       -- in the bar
```

The plugin's **id** is its file name (`hello`). It must contain only
lowercase letters, digits, `-` and `_`. A larger plugin can be a folder
instead: `widgets/hello/init.lua`, next to its own files.

| Field | Required | Meaning |
|---|---|---|
| `render` | yes | `function(o)` that returns text, or a table (§ 3) |
| `name`, `description` | no | shown in Settings |
| `interval` | no | refresh period, in seconds |
| `anchor` | no | default position: `top-left`, `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right` |
| `options` | no | options and their default values (§ 2) |
| `on_click`, `on_scroll`, `menu` | no | interaction (§ 4) |
| `on_toggle` | no | Optional confirmed toggle action for a control composition; return `toggled = true` or `false` from `render` to expose its actual state. |

## 2. Options

Declare options with their default values. hyprdesk hands them to every
function as `o`:

```lua
return {
    name = "Disk", interval = 120,
    options = { mount = "/", warn_at = 90, show_text = true },
    render = function(o)
        local pct = tonumber(hd.exec("df --output=pcent " .. o.mount .. " | tail -1 | tr -dc 0-9")) or 0
        return { text = o.show_text and (pct .. " %") or nil,
                 icon = "drive-harddisk-symbolic",
                 state = pct >= o.warn_at and "warning" or "normal" }
    end,
}
```

A value can come from three places. The first one that is set wins:

1. the instance: `hd.widget("disk", { mount = "/home" })`;
2. Settings → Plugins → **Configure**, saved as
   `hd.config({ plugin_options = { disk = { warn_at = 80 } } })`. It applies
   to every instance;
3. the defaults in `options`.

In **Configure**, a boolean option becomes a toggle, and a number or a string
becomes a text field. The label is the key with `_` replaced by spaces, so
choose readable keys (`warn_at`, not `wa`).

## 3. Rich rendering: icon, tooltip, state

`render` can return plain text, or a table:

```lua
return {
    text    = "42 %",                       -- optional
    icon    = "battery-good-symbolic",      -- theme icon name, or an image path
    tooltip = "Battery: 42 %, 1 h 20 left",
    state   = "normal",                     -- normal | active | warning | critical | hidden
}
```

- Icons whose name ends in **`-symbolic`** are tinted with the theme colors.
  Prefer them in the bar.
- `state` colors the module: `active` (accent), `warning`, `critical`.
- `toggled` optionally reports a confirmed boolean state. `on_toggle(o)` changes it through the plugin's existing source; throw an error if the system did not confirm the change. The UI shows pending/error state and polls the same widget again. This does not replace `on_click` or `menu`.
- **`hidden`** removes the module entirely: it takes no space in the bar.
  Use it when there is nothing to show (no battery, no device plugged in).
  The bundled plugins all do this.

## 4. Clicks, scroll and menus

```lua
on_click  = function(o) hd.spawn("pavucontrol") end,
on_scroll = function(o, dir) hd.exec("wpctl set-volume @DEFAULT_SINK@ 5%" .. (dir > 0 and "+" or "-")) end,
menu = function(o)
    return {
        { header = "Audio" },
        { label = "Mute", icon = "audio-volume-muted-symbolic", checked = muted,
          action = function() hd.exec("wpctl set-mute @DEFAULT_SINK@ toggle") end },
        { label = "Output", detail = "Speakers", disabled = true },
        "-",                                                    -- separator
        { label = "Reset…", confirm = "Reset audio settings?",  -- asks first
          action = function() end },
    }
end,
```

In the bar, a plugin with only a `menu` opens it on left click. A plugin with
both runs `on_click` on left click and opens the menu on **right click**.
`dir` is `+1` or `-1`. `menu` is called each time the menu opens, so it
always shows fresh data.

| Entry field | Meaning |
|---|---|
| `header` | section title |
| `label`, `detail`, `icon` | text, secondary text, icon |
| `checked`, `disabled` | check mark, greyed out |
| `confirm` | question asked before running `action` |
| `input` | turns the entry into a text field (the value is the placeholder); Enter calls `action(text)` |
| `secret` | with `input`: masked input, wiped from memory once sent (passwords) |
| `image` | a **local** image (path or `file:///…`), such as an album cover; nothing is downloaded |
| `action` | `function(text)` run on click (or on Enter for `input`) |

This is how the bundled `reseau` plugin asks for a Wi-Fi password:

```lua
{ input = "Password", label = string.format("Wi-Fi “%s”", ssid), secret = true,
  action = function(pw) if pw and pw ~= "" then connect_with_password(ssid, pw) end end }
```

## 5. Running commands

| Helper | Behavior |
|---|---|
| `hd.exec(cmd)` | runs `sh -c cmd` and **waits**, returning its standard output with trailing newlines removed. **5 s maximum**; on timeout the process group is killed. A missing command (exit code 127) raises a clear error |
| `hd.spawn(cmd)` | starts a program through Hyprland and returns **immediately**. Use it for GUI apps and long tasks |
| `hd.exe` | path of the running hyprdesk binary: `hd.spawn(hd.exe .. " lock")` |

- To call hyprdesk, always use **`hd.exe`**, never `"hyprdesk"` from the
  `PATH`: the binary in the `PATH` may be another version, or not there at
  all.
- Quote anything that comes from outside (user input, file names) before
  putting it in a shell string. `string.format("%q", s)` is a quick way.
- Standard Lua (`io.open`, `os.getenv`, `os.date`, `string.*`) is available.
  To read a file under `/sys` or `/proc`, `io.open` is cheaper than
  `hd.exec("cat …")`.

## 6. Translations

hyprdesk's interface is in French or English (`hd.config({ language = … })`).
Two helpers let a plugin follow it:

- `hd.lang()` returns `"fr"` or `"en"`. Use it to ship your own texts:

  ```lua
  local T = { fr = { title = "Disque" }, en = { title = "Disk" } }
  local t = T[hd.lang()]
  return { name = t.title, render = function(o) … end }
  ```

- `hd.t(text)` looks a French text up in hyprdesk's own catalog
  (`locales/en.toml`) and returns it unchanged if it is missing. The bundled
  plugins use it. A third-party plugin should rely on `hd.lang()`.

## 7. A complete example

A WireGuard indicator: an icon in the bar, red when the tunnel is down, and a
menu to bring it up or down.

```lua
-- ~/.config/hypr/hyprdesk/widgets/wireguard.lua
local T = {
    fr = { name = "WireGuard", up = "Tunnel actif", down = "Tunnel coupé",
           connect = "Connecter", disconnect = "Déconnecter…", sure = "Couper le tunnel ?" },
    en = { name = "WireGuard", up = "Tunnel up", down = "Tunnel down",
           connect = "Connect", disconnect = "Disconnect…", sure = "Bring the tunnel down?" },
}
local t = T[hd.lang()]

local function is_up(o)
    return hd.exec("ip link show " .. o.interface .. " 2>/dev/null") ~= ""
end

return {
    name = t.name, description = "WireGuard tunnel status",
    interval = 5, anchor = "top-right",
    options = { interface = "wg0", hide_when_down = false },

    render = function(o)
        local up = is_up(o)
        if not up and o.hide_when_down then return { state = "hidden" } end
        return {
            icon    = up and "network-vpn-symbolic" or "network-vpn-disabled-symbolic",
            tooltip = (up and t.up or t.down) .. " (" .. o.interface .. ")",
            state   = up and "active" or "critical",
        }
    end,

    menu = function(o)
        local up = is_up(o)
        return {
            { header = t.name .. " · " .. o.interface },
            { label = t.connect, icon = "network-vpn-symbolic", disabled = up,
              action = function() hd.spawn("pkexec wg-quick up " .. o.interface) end },
            { label = t.disconnect, confirm = t.sure, disabled = not up,
              action = function() hd.spawn("pkexec wg-quick down " .. o.interface) end },
        }
    end,
}
```

```lua
-- hyprdesk.lua
hd.bar({ right = { "wireguard", "reseau", "volume", "session" } })
```

`pkexec` asks for the password through the session's polkit agent (hyprdesk's
own, unless another one is already running), so no terminal is needed.

## 8. Limits and good practice

- **Time budget.** Each call into Lua (`render`, `menu`, an `action`…) has
  **5 seconds**. Past that it is stopped and the plugin is flagged in
  Settings. Other plugins keep working.
- **Render only when needed.** `render` runs off the UI thread at the plugin's
  `interval`, and **only while the widget is visible**. Choose the largest
  interval that still feels right: 1–5 s for a live state, minutes for
  network data.
- **Don't block.** Anything that can take more than a second (network,
  package managers) belongs in `hd.spawn` or behind a cached file. You can
  also cap the command itself, for example `curl --max-time 4`.
- **Hide when empty.** Return `{ state = "hidden" }` rather than an empty
  string or a placeholder.
- **No global state you depend on.** A config reload builds a fresh Lua state
  and runs your file again.
- **Overriding a bundled plugin.** A file with the same id replaces it, for
  example `widgets/session.lua`. Copy the original from
  `assets/widgets/integres/` in the repository and adapt it.

## 9. Debugging

- **Errors.** A plugin that fails (Lua error, missing command, time budget
  exceeded) is flagged in Settings, and the other plugins keep running. A
  syntax error in `hyprdesk.lua` raises a notification with `file:line`, and
  the last working configuration stays active.
- **`hyprdesk check`** runs the configuration without touching the daemon.
- **Logs.** `print()` output and errors go to the daemon's log:
  `journalctl --user -u hyprdesk -f`. For more detail, set
  `HYPRDESK_LOG=info,hyprdesk=debug`.
- **Autocompletion.** Run `hyprdesk stubs` and point LuaLS at the generated
  `hyprdesk.meta.lua`.

## 10. Sharing a plugin

- **Folders**, searched in this order (the first match wins):
  `~/.config/hypr/hyprdesk/widgets/`, `$XDG_DATA_HOME/hyprdesk/widgets/`,
  then `$XDG_DATA_DIRS/*/hyprdesk/widgets/` (for example
  `/usr/share/hyprdesk/widgets/` for a distribution package). Your user folder
  always overrides a system-wide copy.
- **Settings → Plugins → Add** installs a third-party plugin.
- **`hyprdesk export`** puts your plugins in the setup archive, next to your
  configuration, and `hyprdesk import` installs them on another machine.

Starting points: `meteo.lua` and `notes.lua` (copied to your widgets folder on
first run) and the twelve bundled plugins in
[`assets/widgets/integres/`](../assets/widgets/integres/).
