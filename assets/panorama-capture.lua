-- Une seule exécution côté compositeur : la transition flottante perd
-- fullscreenClient. Le rétablir après TOUTES les mutations de géométrie.
local address, x, y, width, height, client_fullscreen = ...
local window = "address:" .. address
local function dispatch(command)
    local result = hl.dispatch(command)
    if type(result) == "table" and result.success == false then
        error(result.error or "échec du transfert vidéo")
    end
end
-- Dégager le plein écran du moniteur avant de passer en flottant.
dispatch(hl.dsp.window.fullscreen_state({internal=0, client=client_fullscreen, action="set", window=window}))
-- Hyprland vient de restaurer sa géométrie normale. La conserver aussi
-- lorsque hyprdesk n'avait pas encore observé cette fenêtre avant le plein écran.
local source = hl.get_window(window)
local at, size = source.at, source.size
local saved = { at.x or at[1], at.y or at[2], size.x or size[1], size.y or size[2] }
dispatch(hl.dsp.window.float({action="enable", window=window}))
dispatch(hl.dsp.window.pin({action="enable", window=window}))
dispatch(hl.dsp.window.resize({x=width, y=height, relative=false, window=window}))
dispatch(hl.dsp.window.move({x=x, y=y, relative=false, window=window}))
-- Changement de layout/espace terminé : conserver le mode vidéo du navigateur.
dispatch(hl.dsp.window.fullscreen_state({internal=0, client=client_fullscreen, action="set", window=window}))

return table.concat(saved, ",")
