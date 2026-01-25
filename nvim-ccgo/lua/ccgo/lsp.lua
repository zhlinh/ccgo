-- CCGO LSP configuration module
-- Configures Taplo LSP with CCGO schema for CCGO.toml files

local M = {}

local config = {}

-- Get the plugin's schema path
local function get_schema_path()
  -- Find the plugin directory
  local source = debug.getinfo(1, "S").source:sub(2)
  local plugin_dir = vim.fn.fnamemodify(source, ":h:h:h:h")
  local schema_path = plugin_dir .. "/schemas/ccgo.schema.json"

  -- Check if schema exists
  if vim.fn.filereadable(schema_path) == 1 then
    return schema_path
  end

  -- Fallback: try to find in runtime path
  local rtp_schemas = vim.api.nvim_get_runtime_file("schemas/ccgo.schema.json", false)
  if #rtp_schemas > 0 then
    return rtp_schemas[1]
  end

  return nil
end

-- Configure Taplo LSP for CCGO.toml
function M.setup_taplo()
  local schema_path = get_schema_path()

  if not schema_path then
    vim.notify(
      "CCGO schema not found. Schema validation will not be available.",
      vim.log.levels.WARN,
      { title = "CCGO" }
    )
    return
  end

  -- Check if lspconfig is available
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    -- User doesn't have lspconfig, provide manual instructions
    vim.notify(
      "nvim-lspconfig not found. Please install it for LSP support.",
      vim.log.levels.INFO,
      { title = "CCGO" }
    )
    return
  end

  -- Check if taplo is configured
  local taplo_config = lspconfig.taplo

  if taplo_config then
    -- Get existing taplo settings or create new
    local existing_settings = taplo_config.manager and taplo_config.manager.config and taplo_config.manager.config.settings or {}

    -- Merge CCGO schema into taplo settings
    local ccgo_schema = {
      evenBetterToml = {
        schema = {
          enabled = true,
          repositoryEnabled = true,
          associations = {
            ["CCGO.toml"] = "file://" .. schema_path,
            ["**/CCGO.toml"] = "file://" .. schema_path,
          },
        },
      },
    }

    -- Deep merge settings
    local merged_settings = vim.tbl_deep_extend("force", existing_settings, ccgo_schema)

    -- Apply updated configuration
    taplo_config.setup({
      settings = merged_settings,
    })
  end
end

-- Provide schema configuration for users to add to their taplo setup
function M.get_taplo_settings()
  local schema_path = get_schema_path()

  if not schema_path then
    return {}
  end

  return {
    evenBetterToml = {
      schema = {
        enabled = true,
        repositoryEnabled = true,
        associations = {
          ["CCGO.toml"] = "file://" .. schema_path,
          ["**/CCGO.toml"] = "file://" .. schema_path,
        },
      },
    },
  }
end

-- Setup function
function M.setup(cfg)
  config = cfg

  -- Create autocmd to apply schema when taplo attaches
  vim.api.nvim_create_autocmd("LspAttach", {
    group = vim.api.nvim_create_augroup("CCGOLsp", { clear = true }),
    callback = function(args)
      local client = vim.lsp.get_client_by_id(args.data.client_id)
      if client and client.name == "taplo" then
        local bufname = vim.api.nvim_buf_get_name(args.buf)
        if bufname:match("CCGO%.toml$") then
          -- Taplo is attached to a CCGO.toml file
          if config.notifications then
            vim.notify("CCGO schema active", vim.log.levels.INFO, { title = "CCGO" })
          end
        end
      end
    end,
    desc = "CCGO LSP attach handler",
  })
end

-- Print schema path for debugging
function M.info()
  local schema_path = get_schema_path()
  if schema_path then
    print("CCGO Schema: " .. schema_path)
  else
    print("CCGO Schema: Not found")
  end
end

return M
