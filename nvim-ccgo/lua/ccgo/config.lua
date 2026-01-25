-- CCGO configuration module
local M = {}

-- Schema for configuration validation
M.schema = {
  executable = { type = "string", default = "ccgo" },
  default_platform = { type = "string", default = nil },
  auto_refresh = { type = "boolean", default = true },
  notifications = { type = "boolean", default = true },
  use_terminal = { type = "boolean", default = true },
  telescope_theme = { type = "string", default = "dropdown" },
}

-- Validate configuration
function M.validate(config)
  vim.validate({
    executable = { config.executable, "string" },
    default_platform = { config.default_platform, { "string", "nil" } },
    auto_refresh = { config.auto_refresh, "boolean" },
    notifications = { config.notifications, "boolean" },
    use_terminal = { config.use_terminal, "boolean" },
    telescope_theme = { config.telescope_theme, "string" },
  })
end

return M
