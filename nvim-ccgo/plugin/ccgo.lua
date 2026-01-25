-- CCGO plugin loader
-- This file is automatically loaded by Neovim

-- Prevent loading twice
if vim.g.loaded_ccgo then
  return
end
vim.g.loaded_ccgo = true

-- Check Neovim version
if vim.fn.has("nvim-0.8") ~= 1 then
  vim.notify("nvim-ccgo requires Neovim 0.8 or higher", vim.log.levels.ERROR)
  return
end

-- Helper: ensure setup is called before executing command
local function ensure_setup(callback)
  return function(opts)
    local ccgo = require("ccgo")
    -- Auto-setup if not already done
    if not ccgo._setup_done then
      ccgo.setup()
      ccgo._setup_done = true
    end
    callback(opts)
  end
end

-- Create commands that auto-trigger setup (for lazy loading support)
-- These stub commands will be replaced by the real ones after setup()

-- Helper: safely create command (delete if exists first)
local function create_command(name, fn, opts)
  pcall(vim.api.nvim_del_user_command, name)
  vim.api.nvim_create_user_command(name, fn, opts)
end

create_command("CcgoSetup", function()
  require("ccgo").setup()
end, {
  desc = "Setup CCGO plugin with default configuration",
})

create_command("CcgoInfo", function()
  local ccgo = require("ccgo")
  local lines = {
    "CCGO Plugin Info",
    "================",
    "",
    "Project root: " .. (ccgo.get_project_root() or "Not in a CCGO project"),
    "CCGO.toml: " .. (ccgo.find_ccgo_toml() or "Not found"),
    "Default platform: " .. (ccgo.config.default_platform or "auto"),
    "Executable: " .. ccgo.config.executable,
  }
  print(table.concat(lines, "\n"))
end, {
  desc = "Show CCGO plugin information",
})

-- Lazy-load command stubs (will trigger setup and then execute)
create_command("CcgoBuild", ensure_setup(function(opts)
  local args = opts.fargs
  local platform = args[1]
  local build_opts = {}

  for i = 2, #args do
    local arg = args[i]
    if arg:match("^--arch=") then
      build_opts.arch = arg:gsub("^--arch=", "")
    elseif arg == "--release" then
      build_opts.release = true
    elseif arg == "--ide-project" then
      build_opts.ide_project = true
    end
  end

  if platform then
    require("ccgo.commands").build(platform, build_opts)
  else
    require("ccgo.commands").build_interactive()
  end
end), {
  nargs = "*",
  complete = function() return require("ccgo").platforms end,
  desc = "Build CCGO project for a platform",
})

create_command("CcgoTest", ensure_setup(function(opts)
  local test_opts = {}
  for _, arg in ipairs(opts.fargs) do
    if arg:match("^--filter=") then
      test_opts.filter = arg:gsub("^--filter=", "")
    end
  end
  require("ccgo.commands").test(test_opts)
end), {
  nargs = "*",
  desc = "Run CCGO tests",
})

create_command("CcgoTree", ensure_setup(function()
  require("ccgo.tree").show()
end), {
  desc = "Show CCGO dependency tree",
})

create_command("CcgoInstall", ensure_setup(function(opts)
  local install_opts = {}
  for _, arg in ipairs(opts.fargs) do
    if arg == "--frozen" then
      install_opts.frozen = true
    end
  end
  require("ccgo.commands").install(install_opts)
end), {
  nargs = "*",
  desc = "Install CCGO dependencies",
})

create_command("CcgoClean", ensure_setup(function()
  require("ccgo.commands").clean()
end), {
  desc = "Clean CCGO build artifacts",
})

create_command("CcgoCheck", ensure_setup(function()
  require("ccgo.commands").check()
end), {
  desc = "Check CCGO environment",
})
