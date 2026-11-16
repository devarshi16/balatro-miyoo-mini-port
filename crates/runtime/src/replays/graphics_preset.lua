-- Options > Settings > Graphics > Preset must sit above other options, fit on screen, and apply clocks live.
local phase, wait = 0, 0

local function tap(key)
    love.keypressed('miyoo_' .. key, 'miyoo_' .. key, false)
    love.keyreleased('miyoo_' .. key, 'miyoo_' .. key)
end

local function find_node(node, predicate)
    if type(node) ~= 'table' then return nil end
    if predicate(node) then return node end
    if node.children then
        for _, child in pairs(node.children) do
            local found = find_node(child, predicate)
            if found then return found end
        end
    end
    return nil
end

local function assert_in_room(node, label)
    local t = node.VT or node.T
    assert(t.x >= -0.1 and t.y >= -0.1 and
        t.x + t.w <= G.TILE_W + 0.1 and t.y + t.h <= G.TILE_H + 0.1,
        string.format('%s exceeds screen: %.2f %.2f %.2f %.2f', label, t.x, t.y, t.w, t.h))
end

return function(frame)
    assert(frame < 4000 or phase == 8, 'graphics-preset test timed out in phase ' .. phase)
    if phase == 0 then
        if G.STATE == G.STATES.SPLASH then
            tap('a')
            return
        end
        if G.STATE ~= G.STATES.MENU or G.CONTROLLER.locked then return end
        G.FUNCS.options()
        phase, wait = 1, frame + 20
    elseif phase == 1 and frame > wait then
        if G.CONTROLLER.locked then return end
        G.FUNCS.settings(nil, true)
        phase, wait = 2, frame + 20
    elseif phase == 2 and frame > wait then
        if G.CONTROLLER.locked or not G.OVERLAY_MENU then return end
        local graphics_tab = find_node(G.OVERLAY_MENU.UIRoot, function(node)
            return node.config and node.config.ref_table and
                node.config.ref_table.tab_definition_function_args == 'Graphics'
        end)
        assert(graphics_tab, 'Graphics tab button not found')
        graphics_tab:click()
        phase, wait = 3, frame + 20
    elseif phase == 3 and frame > wait then
        assert_in_room(G.OVERLAY_MENU.UIRoot.children[1], 'settings panel')
        local tab_box = G.OVERLAY_MENU:get_UIE_by_ID('tab_contents').config.object
        local rows = tab_box and tab_box.UIRoot and tab_box.UIRoot.children
        assert(rows and #rows == 5, 'expected 5 rows in Graphics tab, got ' .. tostring(rows and #rows))
        local preset_label = find_node(rows[1], function(node)
            return node.config and node.config.text == 'Preset'
        end)
        assert(preset_label, 'Preset is not the first row in Graphics tab')
        local cycle = find_node(rows[1], function(node)
            return node.config and node.config.focus_args and node.config.focus_args.type == 'cycle'
        end)
        assert(cycle, 'Preset option cycle not found')
        assert(G.SETTINGS.GRAPHICS.miyoo_preset == 'eco', 'expected default eco preset')
        G.CONTROLLER:snap_to{node = cycle}
        phase, wait = 4, frame + 3
    elseif phase == 4 and frame > wait then
        tap('right')
        phase, wait = 5, frame + 15
    elseif phase == 5 and frame > wait then
        assert(G.SETTINGS.GRAPHICS.miyoo_preset == 'performance',
            'D-pad right did not switch preset to performance')
        phase, wait = 6, frame + 5
        return 'graphics-preset-perf'
    elseif phase == 6 and frame > wait then
        tap('left')
        phase, wait = 7, frame + 15
    elseif phase == 7 and frame > wait then
        assert(G.SETTINGS.GRAPHICS.miyoo_preset == 'eco',
            'D-pad left did not switch preset back to eco')
        io.stderr:write('[controls-test] PASS: graphics preset menu, D-pad toggle, and screen bounds\n')
        phase = 8
        return 'graphics-preset-eco'
    end
end
