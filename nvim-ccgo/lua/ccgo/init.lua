-- nvim-ccgo: Neovim plugin for CCGO cross-platform C++ build system
-- https://github.com/zhlinh/ccgo

local M = {}

-- Default configuration
M.config = {
  -- Path to ccgo executable
  executable = "ccgo",
  -- Default platform (auto-detected if nil)
  default_platform = nil,
  -- Auto-refresh dependencies when CCGO.toml changes
  auto_refresh = true,
  -- Show notifications for build results
  notifications = true,
  -- Run commands in terminal (true) or background (false)
  use_terminal = true,
  -- Telescope theme for pickers
  telescope_theme = "dropdown",
}

-- Available platforms
M.platforms = {
  "android",
  "ios",
  "macos",
  "windows",
  "linux",
  "ohos",
  "tvos",
  "watchos",
  "kmp",
}

-- Available architectures per platform
M.architectures = {
  android = { "armeabi-v7a", "arm64-v8a", "x86", "x86_64" },
  ios = { "arm64", "arm64-simulator", "x86_64-simulator" },
  macos = { "arm64", "x86_64" },
  windows = { "x64", "x86", "arm64" },
  linux = { "x86_64", "aarch64" },
  ohos = { "armeabi-v7a", "arm64-v8a", "x86_64" },
  tvos = { "arm64", "arm64-simulator" },
  watchos = { "arm64", "arm64-simulator" },
  kmp = {},
}

-- Detect current platform
local function detect_platform()
  local os_name = vim.loop.os_uname().sysname
  if os_name == "Darwin" then
    return "macos"
  elseif os_name == "Linux" then
    return "linux"
  elseif os_name:match("Windows") then
    return "windows"
  end
  return "linux"
end

-- Find CCGO.toml in current directory or parents
function M.find_ccgo_toml()
  local path = vim.fn.getcwd()
  while path ~= "/" do
    local toml_path = path .. "/CCGO.toml"
    if vim.fn.filereadable(toml_path) == 1 then
      return toml_path, path
    end
    path = vim.fn.fnamemodify(path, ":h")
  end
  return nil, nil
end

-- Check if we're in a CCGO project
function M.is_ccgo_project()
  local toml_path = M.find_ccgo_toml()
  return toml_path ~= nil
end

-- Get project root
function M.get_project_root()
  local _, root = M.find_ccgo_toml()
  return root
end

-- Setup function
function M.setup(opts)
  -- Merge user config with defaults
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})

  -- Auto-detect platform if not set
  if M.config.default_platform == nil then
    M.config.default_platform = detect_platform()
  end

  -- Load submodules
  require("ccgo.commands").setup(M.config)
  require("ccgo.lsp").setup(M.config)

  -- Setup autocommands
  M.setup_autocmds()

  -- Notify successful setup
  if M.config.notifications then
    vim.notify("CCGO plugin loaded", vim.log.levels.INFO, { title = "CCGO" })
  end
end

-- Setup autocommands
function M.setup_autocmds()
  local group = vim.api.nvim_create_augroup("CCGO", { clear = true })

  -- Auto-refresh dependencies when CCGO.toml changes
  if M.config.auto_refresh then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = group,
      pattern = "CCGO.toml",
      callback = function()
        require("ccgo.tree").refresh()
      end,
      desc = "Refresh CCGO dependencies on save",
    })
  end

  -- Set filetype for CCGO.toml
  vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    group = group,
    pattern = "CCGO.toml",
    callback = function()
      vim.bo.filetype = "toml"
      -- Add CCGO-specific buffer settings
      vim.b.ccgo_project = true
    end,
    desc = "Set filetype for CCGO.toml",
  })
end

return M
