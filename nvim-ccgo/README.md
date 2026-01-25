# nvim-ccgo

Neovim plugin for [CCGO](https://github.com/zhlinh/ccgo) - the cross-platform C++ build system.

## Features

- **CCGO.toml Support**
  - Syntax highlighting via Tree-sitter (TOML)
  - Schema validation via Taplo LSP
  - Code completion and hover documentation

- **Build Commands**
  - `:CcgoBuild [platform]` - Build for a platform
  - `:CcgoBuildInteractive` - Interactive platform/architecture selection
  - `:CcgoTest` - Run tests
  - `:CcgoBench` - Run benchmarks
  - `:CcgoInstall` - Install dependencies
  - `:CcgoClean` - Clean build artifacts

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

> **Note**: nvim-ccgo is a subdirectory of the [ccgo](https://github.com/zhlinh/ccgo) repository, not a standalone repo.

### Remote Installation (from GitHub)

Install from the ccgo repository on GitHub. Since nvim-ccgo is a subdirectory, you need to add it to the runtimepath.

#### lazy.nvim

Create a new file `~/.config/nvim/lua/plugins/ccgo.lua`:

```lua
-- ~/.config/nvim/lua/plugins/ccgo.lua
return {
  "zhlinh/ccgo",
  name = "nvim-ccgo",
  dependencies = {
    "nvim-telescope/telescope.nvim", -- optional
    "L3MON4D3/LuaSnip", -- optional
  },
  config = function(plugin)
    -- Add nvim-ccgo subdirectory to runtimepath
    vim.opt.rtp:append(plugin.dir .. "/nvim-ccgo")
    require("ccgo").setup({
      executable = "ccgo",
      use_terminal = true,
      notifications = false,
    })
  end,
}
```

#### packer.nvim

Add to `~/.config/nvim/lua/plugins.lua`:

```lua
-- ~/.config/nvim/lua/plugins.lua
use {
  "zhlinh/ccgo",
  rtp = "nvim-ccgo",  -- specify subdirectory
  requires = {
    "nvim-telescope/telescope.nvim", -- optional
    "L3MON4D3/LuaSnip", -- optional
  },
  config = function()
    require("ccgo").setup()
  end,
}
```

### Manual Installation

```bash
# Clone the ccgo repository
git clone https://github.com/zhlinh/ccgo ~/.local/share/nvim/site/pack/ccgo/start/ccgo

# Create symlink to nvim-ccgo (or copy the directory)
ln -s ~/.local/share/nvim/site/pack/ccgo/start/ccgo/nvim-ccgo \
      ~/.local/share/nvim/site/pack/ccgo/start/nvim-ccgo
```

Then add the following line to your Neovim config file `~/.config/nvim/init.lua`:

```lua
-- ~/.config/nvim/init.lua
-- Add this line at the end of the file
require("ccgo").setup()
```

### Local Development / Testing

For local development or testing, point directly to the nvim-ccgo directory on your machine.

#### lazy.nvim

Create a new file `~/.config/nvim/lua/plugins/ccgo.lua`:

```lua
-- ~/.config/nvim/lua/plugins/ccgo.lua
return {
  dir = "/path/to/ccgo/nvim-ccgo",  -- Replace with your actual path
  name = "nvim-ccgo",
  dependencies = {
    "nvim-telescope/telescope.nvim", -- optional, for platform picker
    "L3MON4D3/LuaSnip", -- optional, for snippets
  },
  config = function()
    require("ccgo").setup({
      executable = "ccgo",
      use_terminal = true,
      notifications = false,
    })
  end,
}
```

Example paths:
- macOS/Linux: `dir = "~/Projects/ccgo/nvim-ccgo"`
- Windows: `dir = "C:/Users/you/Projects/ccgo/nvim-ccgo"`

#### packer.nvim

Add to `~/.config/nvim/lua/plugins.lua`:

```lua
-- ~/.config/nvim/lua/plugins.lua
use {
  "/path/to/ccgo/nvim-ccgo",  -- Replace with your actual path
  config = function()
    require("ccgo").setup()
  end,
}
```

### Verify Installation

After installation, restart Neovim and run:

```vim
:CcgoInfo
```

You should see plugin information. If you get "Command not found", check your installation path.

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
  notifications = false,

  -- Run commands in terminal (true) or background (false)
  use_terminal = true,

  -- Telescope theme for pickers
  telescope_theme = "dropdown",
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:CcgoBuild [platform]` | Build for a platform (interactive if no platform) |
| `:CcgoBuildInteractive` | Build with interactive platform/arch selection |
| `:CcgoTest [--filter=pattern]` | Run tests |
| `:CcgoBench` | Run benchmarks |
| `:CcgoInstall [--frozen]` | Install dependencies |
| `:CcgoClean` | Clean build artifacts |
| `:CcgoDoc [--open]` | Generate documentation |
| `:CcgoCheck` | Check environment |
| `:CcgoTree` | Show dependency tree |
| `:CcgoPublish <target>` | Publish package |
| `:CcgoTag [version]` | Create git tag |
| `:CcgoPackage` | Package project |
| `:CcgoInfo` | Show plugin info |

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
    vim.keymap.set("n", "<leader>cb", "<cmd>CcgoBuildInteractive<cr>", opts)
    vim.keymap.set("n", "<leader>ct", "<cmd>CcgoTest<cr>", opts)
    vim.keymap.set("n", "<leader>ci", "<cmd>CcgoInstall<cr>", opts)
    vim.keymap.set("n", "<leader>cc", "<cmd>CcgoClean<cr>", opts)
    vim.keymap.set("n", "<leader>cd", "<cmd>CcgoTree<cr>", opts)
  end,
})
```

## Dependency Tree

The `:CcgoTree` command opens a sidebar with your project's dependencies:

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
