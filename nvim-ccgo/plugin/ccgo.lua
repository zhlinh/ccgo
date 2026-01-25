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

-- Lazy loading: only set up commands here
-- Full setup happens when user calls require("ccgo").setup()

-- Create basic commands that trigger plugin load
vim.api.nvim_create_user_command("CcgoSetup", function()
  require("ccgo").setup()
end, {
  desc = "Setup CCGO plugin with default configuration",
})

-- Provide info command
vim.api.nvim_create_user_command("CcgoInfo", function()
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
