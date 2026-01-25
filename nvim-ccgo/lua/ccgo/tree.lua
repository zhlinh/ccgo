-- CCGO dependency tree viewer module

local M = {}

-- Tree buffer and window state
local state = {
  buf = nil,
  win = nil,
  tree_data = nil,
}

-- Tree icons
local icons = {
  package = "📦 ",
  dependency = "├─ ",
  last_dependency = "└─ ",
  indent = "│  ",
  space = "   ",
  shared = "🔗 ",
}

-- Format tree as lines
local function format_tree(deps, lines, prefix, is_last_list)
  lines = lines or {}
  prefix = prefix or ""
  is_last_list = is_last_list or {}

  for i, dep in ipairs(deps) do
    local is_last = i == #deps
    local icon = is_last and icons.last_dependency or icons.dependency

    -- Build the prefix for this line
    local line_prefix = ""
    for j, is_last_parent in ipairs(is_last_list) do
      if j < #is_last_list then
        line_prefix = line_prefix .. (is_last_parent and icons.space or icons.indent)
      end
    end

    local version = dep.version or ""
    local shared = dep.shared and icons.shared or ""
    local line = line_prefix .. icon .. dep.name .. " " .. version .. shared

    table.insert(lines, {
      text = line,
      name = dep.name,
      version = version,
      source = dep.source,
      level = #is_last_list,
    })

    -- Recurse for dependencies
    if dep.dependencies and #dep.dependencies > 0 then
      local new_is_last_list = vim.deepcopy(is_last_list)
      table.insert(new_is_last_list, is_last)
      format_tree(dep.dependencies, lines, prefix, new_is_last_list)
    end
  end

  return lines
end

-- Create tree buffer
local function create_buffer()
  if state.buf and vim.api.nvim_buf_is_valid(state.buf) then
    return state.buf
  end

  state.buf = vim.api.nvim_create_buf(false, true)

  -- Set buffer options
  vim.api.nvim_buf_set_option(state.buf, "buftype", "nofile")
  vim.api.nvim_buf_set_option(state.buf, "bufhidden", "hide")
  vim.api.nvim_buf_set_option(state.buf, "swapfile", false)
  vim.api.nvim_buf_set_option(state.buf, "filetype", "ccgo-tree")
  vim.api.nvim_buf_set_name(state.buf, "CCGO Dependencies")

  -- Set keymaps
  local opts = { buffer = state.buf, silent = true }
  vim.keymap.set("n", "q", function() M.close() end, opts)
  vim.keymap.set("n", "<Esc>", function() M.close() end, opts)
  vim.keymap.set("n", "r", function() M.refresh() end, opts)
  vim.keymap.set("n", "?", function() M.show_help() end, opts)

  return state.buf
end

-- Open tree window
local function open_window()
  if state.win and vim.api.nvim_win_is_valid(state.win) then
    vim.api.nvim_set_current_win(state.win)
    return state.win
  end

  local buf = create_buffer()

  -- Calculate window size
  local width = math.floor(vim.o.columns * 0.3)
  local height = vim.o.lines - 4

  -- Open vertical split on the right
  vim.cmd("botright " .. width .. "vsplit")
  state.win = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(state.win, buf)

  -- Set window options
  vim.api.nvim_win_set_option(state.win, "wrap", false)
  vim.api.nvim_win_set_option(state.win, "number", false)
  vim.api.nvim_win_set_option(state.win, "relativenumber", false)
  vim.api.nvim_win_set_option(state.win, "signcolumn", "no")
  vim.api.nvim_win_set_option(state.win, "foldcolumn", "0")
  vim.api.nvim_win_set_option(state.win, "cursorline", true)

  return state.win
end

-- Update buffer content
local function update_content(lines)
  if not state.buf or not vim.api.nvim_buf_is_valid(state.buf) then
    return
  end

  vim.api.nvim_buf_set_option(state.buf, "modifiable", true)

  local text_lines = {}
  for _, line in ipairs(lines) do
    table.insert(text_lines, line.text)
  end

  vim.api.nvim_buf_set_lines(state.buf, 0, -1, false, text_lines)
  vim.api.nvim_buf_set_option(state.buf, "modifiable", false)

  -- Store line data for navigation
  state.tree_data = lines
end

-- Show dependency tree
function M.show()
  local ccgo = require("ccgo")

  if not ccgo.is_ccgo_project() then
    vim.notify("Not in a CCGO project", vim.log.levels.WARN, { title = "CCGO" })
    return
  end

  open_window()

  -- Show loading message
  vim.api.nvim_buf_set_option(state.buf, "modifiable", true)
  vim.api.nvim_buf_set_lines(state.buf, 0, -1, false, { "Loading dependencies..." })
  vim.api.nvim_buf_set_option(state.buf, "modifiable", false)

  -- Fetch tree data
  require("ccgo.commands").tree(function(tree, err)
    if err then
      vim.api.nvim_buf_set_option(state.buf, "modifiable", true)
      vim.api.nvim_buf_set_lines(state.buf, 0, -1, false, { "Error: " .. err })
      vim.api.nvim_buf_set_option(state.buf, "modifiable", false)
      return
    end

    if not tree or #tree == 0 then
      vim.api.nvim_buf_set_option(state.buf, "modifiable", true)
      vim.api.nvim_buf_set_lines(state.buf, 0, -1, false, {
        "No dependencies",
        "",
        "Add dependencies to CCGO.toml:",
        "",
        "[[dependencies]]",
        'name = "example"',
        'git = "https://github.com/user/repo.git"',
        'branch = "main"',
      })
      vim.api.nvim_buf_set_option(state.buf, "modifiable", false)
      return
    end

    -- Add header
    local header = {
      { text = icons.package .. "CCGO Dependencies", level = -1 },
      { text = string.rep("─", 30), level = -1 },
    }

    local lines = format_tree(tree)
    for i = #header, 1, -1 do
      table.insert(lines, 1, header[i])
    end

    -- Add footer
    table.insert(lines, { text = "", level = -1 })
    table.insert(lines, { text = "Press ? for help, r to refresh, q to close", level = -1 })

    update_content(lines)
  end)
end

-- Close tree window
function M.close()
  if state.win and vim.api.nvim_win_is_valid(state.win) then
    vim.api.nvim_win_close(state.win, true)
  end
  state.win = nil
end

-- Toggle tree window
function M.toggle()
  if state.win and vim.api.nvim_win_is_valid(state.win) then
    M.close()
  else
    M.show()
  end
end

-- Refresh tree
function M.refresh()
  if state.win and vim.api.nvim_win_is_valid(state.win) then
    M.show()
  end
end

-- Show help
function M.show_help()
  local help_lines = {
    "CCGO Dependency Tree - Help",
    "============================",
    "",
    "Keybindings:",
    "  q, <Esc>  Close window",
    "  r         Refresh tree",
    "  ?         Show this help",
    "",
    "Icons:",
    "  📦  Package",
    "  ├─  Dependency",
    "  └─  Last dependency",
    "  🔗  Shared dependency",
  }

  vim.notify(table.concat(help_lines, "\n"), vim.log.levels.INFO, { title = "CCGO Help" })
end

return M
