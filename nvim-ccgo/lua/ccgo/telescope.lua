-- CCGO Telescope integration module
-- Provides pickers for platforms, architectures, and dependencies

local M = {}

-- Check if telescope is available
local function has_telescope()
  local ok, _ = pcall(require, "telescope")
  return ok
end

-- Fallback picker using vim.ui.select
local function fallback_select(items, opts, callback)
  vim.ui.select(items, opts, callback)
end

-- Platform icons
local platform_icons = {
  android = " ",
  ios = " ",
  macos = " ",
  windows = " ",
  linux = " ",
  ohos = "󰤯 ",
  tvos = "󰒃 ",
  watchos = "󰖉 ",
  kmp = " ",
}

-- Select a platform
function M.select_platform(callback)
  local ccgo = require("ccgo")
  local platforms = ccgo.platforms

  if has_telescope() then
    local pickers = require("telescope.pickers")
    local finders = require("telescope.finders")
    local conf = require("telescope.config").values
    local actions = require("telescope.actions")
    local action_state = require("telescope.actions.state")
    local themes = require("telescope.themes")

    local opts = themes.get_dropdown({
      prompt_title = "Select Platform",
      layout_config = {
        width = 0.4,
        height = 0.5,
      },
    })

    pickers.new(opts, {
      finder = finders.new_table({
        results = platforms,
        entry_maker = function(platform)
          local icon = platform_icons[platform] or "󰏗 "
          return {
            value = platform,
            display = icon .. platform,
            ordinal = platform,
          }
        end,
      }),
      sorter = conf.generic_sorter(opts),
      attach_mappings = function(prompt_bufnr)
        actions.select_default:replace(function()
          actions.close(prompt_bufnr)
          local selection = action_state.get_selected_entry()
          if selection and callback then
            callback(selection.value)
          end
        end)
        return true
      end,
    }):find()
  else
    -- Fallback to vim.ui.select
    local items = {}
    for _, platform in ipairs(platforms) do
      local icon = platform_icons[platform] or ""
      table.insert(items, icon .. platform)
    end

    fallback_select(items, {
      prompt = "Select Platform:",
      format_item = function(item)
        return item
      end,
    }, function(choice)
      if choice and callback then
        -- Remove icon from choice
        local platform = choice:gsub("^[^ ]+ ", "")
        callback(platform)
      end
    end)
  end
end

-- Select architecture for a platform
function M.select_architecture(platform, callback)
  local ccgo = require("ccgo")
  local architectures = ccgo.architectures[platform] or {}

  if #architectures == 0 then
    -- No architecture selection needed
    if callback then
      callback(nil)
    end
    return
  end

  -- "all" means all architectures as comma-separated list
  local all_archs = table.concat(architectures, ",")

  -- Add "all" option
  local items = { "all" }
  for _, arch in ipairs(architectures) do
    table.insert(items, arch)
  end

  if has_telescope() then
    local pickers = require("telescope.pickers")
    local finders = require("telescope.finders")
    local conf = require("telescope.config").values
    local actions = require("telescope.actions")
    local action_state = require("telescope.actions.state")
    local themes = require("telescope.themes")

    local opts = themes.get_dropdown({
      prompt_title = "Select Architecture (" .. platform .. ")",
      layout_config = {
        width = 0.4,
        height = 0.4,
      },
    })

    pickers.new(opts, {
      finder = finders.new_table({
        results = items,
        entry_maker = function(arch)
          return {
            value = arch,
            display = arch == "all" and "󰒆 all (default)" or "  " .. arch,
            ordinal = arch,
          }
        end,
      }),
      sorter = conf.generic_sorter(opts),
      attach_mappings = function(prompt_bufnr)
        actions.select_default:replace(function()
          actions.close(prompt_bufnr)
          local selection = action_state.get_selected_entry()
          if selection and callback then
            local arch = selection.value
            -- "all" means all architectures as comma-separated list
            callback(arch == "all" and all_archs or arch)
          end
        end)
        return true
      end,
    }):find()
  else
    fallback_select(items, {
      prompt = "Select Architecture:",
    }, function(choice)
      if choice and callback then
        -- "all" means all architectures as comma-separated list
        callback(choice == "all" and all_archs or choice)
      end
    end)
  end
end

-- Select build type
function M.select_build_type(callback)
  local types = { "release", "debug" }

  if has_telescope() then
    local pickers = require("telescope.pickers")
    local finders = require("telescope.finders")
    local conf = require("telescope.config").values
    local actions = require("telescope.actions")
    local action_state = require("telescope.actions.state")
    local themes = require("telescope.themes")

    local opts = themes.get_dropdown({
      prompt_title = "Select Build Type",
      layout_config = {
        width = 0.3,
        height = 0.3,
      },
    })

    pickers.new(opts, {
      finder = finders.new_table({
        results = types,
        entry_maker = function(t)
          local icon = t == "release" and "󰄬 " or "󰃤 "
          return {
            value = t,
            display = icon .. t,
            ordinal = t,
          }
        end,
      }),
      sorter = conf.generic_sorter(opts),
      attach_mappings = function(prompt_bufnr)
        actions.select_default:replace(function()
          actions.close(prompt_bufnr)
          local selection = action_state.get_selected_entry()
          if selection and callback then
            callback(selection.value)
          end
        end)
        return true
      end,
    }):find()
  else
    fallback_select(types, {
      prompt = "Select Build Type:",
    }, function(choice)
      if choice and callback then
        callback(choice)
      end
    end)
  end
end

-- Show dependencies picker
function M.show_dependencies()
  local commands = require("ccgo.commands")

  commands.tree(function(tree, err)
    if err then
      vim.notify("Failed to get dependency tree: " .. err, vim.log.levels.ERROR, { title = "CCGO" })
      return
    end

    if not tree or #tree == 0 then
      vim.notify("No dependencies found", vim.log.levels.INFO, { title = "CCGO" })
      return
    end

    if has_telescope() then
      local pickers = require("telescope.pickers")
      local finders = require("telescope.finders")
      local conf = require("telescope.config").values
      local previewers = require("telescope.previewers")
      local themes = require("telescope.themes")

      local opts = themes.get_ivy({
        prompt_title = "CCGO Dependencies",
      })

      local entries = {}
      local function flatten_tree(deps, level)
        level = level or 0
        for _, dep in ipairs(deps) do
          table.insert(entries, {
            name = dep.name,
            version = dep.version or "unknown",
            source = dep.source or "unknown",
            level = level,
          })
          if dep.dependencies then
            flatten_tree(dep.dependencies, level + 1)
          end
        end
      end
      flatten_tree(tree)

      pickers.new(opts, {
        finder = finders.new_table({
          results = entries,
          entry_maker = function(entry)
            local indent = string.rep("  ", entry.level)
            local icon = entry.level == 0 and "📦 " or "└─ "
            return {
              value = entry,
              display = indent .. icon .. entry.name .. " (" .. entry.version .. ")",
              ordinal = entry.name,
            }
          end,
        }),
        sorter = conf.generic_sorter(opts),
        previewer = previewers.new_buffer_previewer({
          title = "Dependency Info",
          define_preview = function(self, entry)
            local lines = {
              "Name: " .. entry.value.name,
              "Version: " .. entry.value.version,
              "Source: " .. entry.value.source,
            }
            vim.api.nvim_buf_set_lines(self.state.bufnr, 0, -1, false, lines)
          end,
        }),
      }):find()
    else
      -- Fallback: print tree
      require("ccgo.tree").show()
    end
  end)
end

-- Register Telescope extension
function M.register_extension()
  if not has_telescope() then
    return
  end

  local ok, telescope = pcall(require, "telescope")
  if ok then
    telescope.register_extension({
      exports = {
        ccgo = M.show_dependencies,
        platforms = M.select_platform,
        architectures = M.select_architecture,
      },
    })
  end
end

return M
