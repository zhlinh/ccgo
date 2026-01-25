# nvim-ccgo

Neovim plugin for [CCGO](https://github.com/zhlinh/ccgo) - the cross-platform C++ build system.

## Features

- **CCGO.toml Support**
  - Syntax highlighting via Tree-sitter (TOML)
  - Schema validation via Taplo LSP
  - Code completion and hover documentation

- **Build Commands**
  - `:ccgoBuild [platform]` - Build for a platform
  - `:ccgoBuildInteractive` - Interactive platform/architecture selection
  - `:ccgoTest` - Run tests
  - `:ccgoBench` - Run benchmarks
  - `:ccgoInstall` - Install dependencies
  - `:ccgoClean` - Clean build artifacts

- **Telescope Integration**
  - Platform picker with icons
  - Architecture selector
  - Dependency browser

- **Dependency Tree Viewer**
  - Visual tree view of project dependencies
  - Auto-refresh on CCGO.toml changes

- **LuaSnip Snippets**
  - 14+ snippets for common patterns
  - Package, build, dependencies, platforms, publishing

## Requirements

- Neovim >= 0.8
- [CCGO CLI](https://github.com/zhlinh/ccgo) installed
- Optional:
  - [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) + [taplo](https://taplo.tamasfe.dev/) for LSP
  - [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) for pickers
  - [LuaSnip](https://github.com/L3MON4D3/LuaSnip) for snippets

## Installation

### [lazy.nvim](https://github.com/folke/lazy.nvim)

#### Option 1: From GitHub (after plugin is published)

Create a new file `~/.config/nvim/lua/plugins/ccgo.lua`:

```lua
-- ~/.config/nvim/lua/plugins/ccgo.lua
return {
  "zhlinh/nvim-ccgo",
  dependencies = {
    "nvim-telescope/telescope.nvim", -- optional, for platform picker
    "L3MON4D3/LuaSnip", -- optional, for snippets
  },
  ft = "toml",  -- lazy load on TOML files
  cmd = { "ccgoBuild", "ccgoTest", "ccgoTree" },  -- lazy load on commands
  config = function()
    require("ccgo").setup({
      -- Path to ccgo executable (default: "ccgo")
      executable = "ccgo",
      -- Default platform (auto-detected if nil)
      default_platform = nil,
      -- Auto-refresh dependencies when CCGO.toml changes
      auto_refresh = true,
      -- Show notifications for build results
      notifications = true,
      -- Run commands in terminal (true) or background (false)
      use_terminal = true,
    })
  end,
}
```

#### Option 2: From local path

If you have the plugin locally, use `dir` instead of the GitHub path:

```lua
-- ~/.config/nvim/lua/plugins/ccgo.lua
return {
  dir = "/path/to/nvim-ccgo",  -- absolute path to the plugin directory
  dependencies = {
    "nvim-telescope/telescope.nvim", -- optional, for platform picker
    "L3MON4D3/LuaSnip", -- optional, for snippets
  },
  ft = "toml",
  cmd = { "ccgoBuild", "ccgoTest", "ccgoTree" },
  config = function()
    require("ccgo").setup()
  end,
}
```

Or add to your existing plugins file (e.g., `~/.config/nvim/lua/plugins/init.lua`):

```lua
-- ~/.config/nvim/lua/plugins/init.lua
return {
  -- ... other plugins ...

  {
    dir = "/path/to/nvim-ccgo",  -- or "zhlinh/nvim-ccgo" when published
    ft = "toml",
    cmd = { "ccgoBuild", "ccgoTest", "ccgoTree" },
    config = function()
      require("ccgo").setup()
    end,
  },
}
```

### [packer.nvim](https://github.com/wbthomason/packer.nvim)

Add to `~/.config/nvim/lua/plugins.lua` (or wherever your packer config is):

```lua
-- ~/.config/nvim/lua/plugins.lua
return require("packer").startup(function(use)
  -- ... other plugins ...

  -- From GitHub (after plugin is published)
  use {
    "zhlinh/nvim-ccgo",
    requires = {
      "nvim-telescope/telescope.nvim", -- optional
      "L3MON4D3/LuaSnip", -- optional
    },
    config = function()
      require("ccgo").setup()
    end,
  }

  -- Or from local path
  -- use {
  --   "/path/to/nvim-ccgo",
  --   requires = { ... },
  --   config = function() require("ccgo").setup() end,
  -- }
end)
```

### Manual Installation

Clone to your Neovim packages directory:

```bash
git clone https://github.com/zhlinh/nvim-ccgo ~/.local/share/nvim/site/pack/plugins/start/nvim-ccgo
```

## Configuration

```lua
require("ccgo").setup({
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
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:ccgoBuild [platform]` | Build for a platform (interactive if no platform) |
| `:ccgoBuildInteractive` | Build with interactive platform/arch selection |
| `:ccgoTest [--filter=pattern]` | Run tests |
| `:ccgoBench` | Run benchmarks |
| `:ccgoInstall [--frozen]` | Install dependencies |
| `:ccgoClean` | Clean build artifacts |
| `:ccgoDoc [--open]` | Generate documentation |
| `:ccgoCheck` | Check environment |
| `:ccgoTree` | Show dependency tree |
| `:ccgoPublish <target>` | Publish package |
| `:ccgoTag [version]` | Create git tag |
| `:ccgoPackage` | Package project |
| `:ccgoInfo` | Show plugin info |

## LSP Setup (Taplo)

For schema validation and completion in `CCGO.toml`, configure Taplo LSP:

```lua
-- Using nvim-lspconfig
require("lspconfig").taplo.setup({
  settings = vim.tbl_deep_extend(
    "force",
    {},
    require("ccgo.lsp").get_taplo_settings()
  ),
})
```

Or add to your existing Taplo configuration:

```lua
require("lspconfig").taplo.setup({
  settings = {
    evenBetterToml = {
      schema = {
        enabled = true,
        associations = {
          ["CCGO.toml"] = "file://" .. vim.fn.stdpath("data") .. "/lazy/nvim-ccgo/schemas/ccgo.schema.json",
        },
      },
    },
  },
})
```

## Snippets

If you have LuaSnip installed, snippets are automatically available in TOML files:

| Trigger | Description |
|---------|-------------|
| `package` | Package metadata section |
| `build` | Build configuration |
| `dep-git` | Git dependency |
| `dep-path` | Path dependency |
| `dep` | Registry dependency |
| `android` | Android platform config |
| `ios` | iOS platform config |
| `macos` | macOS platform config |
| `windows` | Windows platform config |
| `linux` | Linux platform config |
| `ohos` | OpenHarmony platform config |
| `publish-maven` | Maven publishing config |
| `publish-cocoapods` | CocoaPods publishing config |
| `feature` | Feature definition |
| `workspace` | Workspace config |
| `registry` | Custom registry |
| `ccgo-full` | Full CCGO.toml template |

To enable snippets:

```lua
require("ccgo.snippets").setup()
```

## Keybindings

The plugin doesn't set any global keybindings by default. Here's a suggested setup:

```lua
-- Only in CCGO projects
vim.api.nvim_create_autocmd("BufEnter", {
  pattern = "CCGO.toml",
  callback = function()
    local opts = { buffer = true, silent = true }
    vim.keymap.set("n", "<leader>cb", "<cmd>ccgoBuildInteractive<cr>", opts)
    vim.keymap.set("n", "<leader>ct", "<cmd>ccgoTest<cr>", opts)
    vim.keymap.set("n", "<leader>ci", "<cmd>ccgoInstall<cr>", opts)
    vim.keymap.set("n", "<leader>cc", "<cmd>ccgoClean<cr>", opts)
    vim.keymap.set("n", "<leader>cd", "<cmd>ccgoTree<cr>", opts)
  end,
})
```

## Dependency Tree

The `:ccgoTree` command opens a sidebar with your project's dependencies:

```
📦 CCGO Dependencies
──────────────────────────────
├─ fmt 10.2.1
├─ spdlog 1.12.0
│  └─ fmt 10.2.1 🔗
└─ nlohmann-json 3.11.3

Press ? for help, r to refresh, q to close
```

Keybindings in tree view:
- `q` / `<Esc>` - Close window
- `r` - Refresh tree
- `?` - Show help

## Telescope Extension

Use Telescope to browse dependencies:

```vim
:Telescope ccgo
```

Or via Lua:

```lua
require("ccgo.telescope").show_dependencies()
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Related Projects

- [CCGO](https://github.com/zhlinh/ccgo) - Cross-platform C++ build system
- [vscode-ccgo](../vscode-ccgo) - VS Code extension
- [jetbrains-ccgo](../jetbrains-ccgo) - JetBrains IDE plugin
