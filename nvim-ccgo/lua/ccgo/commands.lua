-- CCGO commands module
local M = {}

local config = {}

-- Execute a ccgo command
local function execute(cmd, args, opts)
  opts = opts or {}
  local ccgo = require("ccgo")
  local root = ccgo.get_project_root()

  if not root then
    vim.notify("Not in a CCGO project (CCGO.toml not found)", vim.log.levels.ERROR, { title = "CCGO" })
    return
  end

  local full_cmd = { config.executable, cmd }
  if args then
    for _, arg in ipairs(args) do
      table.insert(full_cmd, arg)
    end
  end

  local cmd_str = table.concat(full_cmd, " ")

  if config.use_terminal then
    -- Run in terminal
    vim.cmd("split | terminal cd " .. vim.fn.shellescape(root) .. " && " .. cmd_str)
    vim.cmd("startinsert")
  else
    -- Run in background with job control
    local output = {}
    local job_id = vim.fn.jobstart(full_cmd, {
      cwd = root,
      on_stdout = function(_, data)
        for _, line in ipairs(data) do
          if line ~= "" then
            table.insert(output, line)
          end
        end
      end,
      on_stderr = function(_, data)
        for _, line in ipairs(data) do
          if line ~= "" then
            table.insert(output, line)
          end
        end
      end,
      on_exit = function(_, code)
        if code == 0 then
          if config.notifications then
            vim.notify("Command completed: " .. cmd, vim.log.levels.INFO, { title = "CCGO" })
          end
          if opts.on_success then
            opts.on_success(output)
          end
        else
          vim.notify("Command failed: " .. cmd .. "\n" .. table.concat(output, "\n"), vim.log.levels.ERROR, { title = "CCGO" })
          if opts.on_error then
            opts.on_error(output)
          end
        end
      end,
    })

    if job_id <= 0 then
      vim.notify("Failed to start command: " .. cmd_str, vim.log.levels.ERROR, { title = "CCGO" })
    elseif config.notifications then
      vim.notify("Running: " .. cmd_str, vim.log.levels.INFO, { title = "CCGO" })
    end
  end
end

-- Build command
function M.build(platform, opts)
  opts = opts or {}
  local args = { platform or config.default_platform }

  if opts.arch then
    table.insert(args, "--arch")
    table.insert(args, opts.arch)
  end

  if opts.release then
    table.insert(args, "--release")
  end

  if opts.ide_project then
    table.insert(args, "--ide-project")
  end

  execute("build", args)
end

-- Test command
function M.test(opts)
  opts = opts or {}
  local args = {}

  if opts.filter then
    table.insert(args, "--filter")
    table.insert(args, opts.filter)
  end

  execute("test", args)
end

-- Bench command
function M.bench(opts)
  opts = opts or {}
  execute("bench", {})
end

-- Install command
function M.install(opts)
  opts = opts or {}
  local args = {}

  if opts.frozen then
    table.insert(args, "--frozen")
  end

  execute("install", args)
end

-- Clean command
function M.clean()
  execute("clean", {})
end

-- Doc command
function M.doc(opts)
  opts = opts or {}
  local args = {}

  if opts.open then
    table.insert(args, "--open")
  end

  execute("doc", args)
end

-- Tag command
function M.tag(version)
  local args = {}
  if version then
    table.insert(args, version)
  end
  execute("tag", args)
end

-- Package command
function M.package()
  execute("package", {})
end

-- Check command
function M.check()
  execute("check", {})
end

-- Publish command
function M.publish(target, opts)
  opts = opts or {}
  local args = { target }
  execute("publish", args)
end

-- Tree command (get dependency tree)
function M.tree(callback)
  local ccgo = require("ccgo")
  local root = ccgo.get_project_root()

  if not root then
    if callback then
      callback(nil, "Not in a CCGO project")
    end
    return
  end

  local output = {}
  vim.fn.jobstart({ config.executable, "tree", "--format", "json" }, {
    cwd = root,
    on_stdout = function(_, data)
      for _, line in ipairs(data) do
        if line ~= "" then
          table.insert(output, line)
        end
      end
    end,
    on_exit = function(_, code)
      if code == 0 and callback then
        local json_str = table.concat(output, "\n")
        local ok, result = pcall(vim.fn.json_decode, json_str)
        if ok then
          callback(result, nil)
        else
          callback(nil, "Failed to parse JSON")
        end
      elseif callback then
        callback(nil, "Command failed")
      end
    end,
  })
end

-- Interactive build with platform selection
function M.build_interactive()
  local telescope = require("ccgo.telescope")
  telescope.select_platform(function(platform)
    if platform then
      telescope.select_architecture(platform, function(arch)
        M.build(platform, { arch = arch })
      end)
    end
  end)
end

-- Setup user commands
function M.setup(cfg)
  config = cfg

  -- :CcgoBuild [platform] [--arch=<arch>] [--release] [--ide-project]
  vim.api.nvim_create_user_command("CcgoBuild", function(opts)
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
      M.build(platform, build_opts)
    else
      M.build_interactive()
    end
  end, {
    nargs = "*",
    complete = function(_, _, _)
      return require("ccgo").platforms
    end,
    desc = "Build CCGO project for a platform",
  })

  -- :CcgoBuildInteractive
  vim.api.nvim_create_user_command("CcgoBuildInteractive", function()
    M.build_interactive()
  end, {
    desc = "Build CCGO project with interactive platform selection",
  })

  -- :CcgoTest [--filter=<pattern>]
  vim.api.nvim_create_user_command("CcgoTest", function(opts)
    local test_opts = {}
    for _, arg in ipairs(opts.fargs) do
      if arg:match("^--filter=") then
        test_opts.filter = arg:gsub("^--filter=", "")
      end
    end
    M.test(test_opts)
  end, {
    nargs = "*",
    desc = "Run CCGO tests",
  })

  -- :CcgoBench
  vim.api.nvim_create_user_command("CcgoBench", function()
    M.bench()
  end, {
    desc = "Run CCGO benchmarks",
  })

  -- :CcgoInstall [--frozen]
  vim.api.nvim_create_user_command("CcgoInstall", function(opts)
    local install_opts = {}
    for _, arg in ipairs(opts.fargs) do
      if arg == "--frozen" then
        install_opts.frozen = true
      end
    end
    M.install(install_opts)
  end, {
    nargs = "*",
    desc = "Install CCGO dependencies",
  })

  -- :CcgoClean
  vim.api.nvim_create_user_command("CcgoClean", function()
    M.clean()
  end, {
    desc = "Clean CCGO build artifacts",
  })

  -- :CcgoDoc [--open]
  vim.api.nvim_create_user_command("CcgoDoc", function(opts)
    local doc_opts = {}
    for _, arg in ipairs(opts.fargs) do
      if arg == "--open" then
        doc_opts.open = true
      end
    end
    M.doc(doc_opts)
  end, {
    nargs = "*",
    desc = "Generate CCGO documentation",
  })

  -- :CcgoCheck
  vim.api.nvim_create_user_command("CcgoCheck", function()
    M.check()
  end, {
    desc = "Check CCGO environment",
  })

  -- :CcgoTree
  vim.api.nvim_create_user_command("CcgoTree", function()
    require("ccgo.tree").show()
  end, {
    desc = "Show CCGO dependency tree",
  })

  -- :CcgoPublish <target>
  vim.api.nvim_create_user_command("CcgoPublish", function(opts)
    local target = opts.fargs[1]
    if not target then
      vim.notify("Usage: :CcgoPublish <target>", vim.log.levels.WARN, { title = "CCGO" })
      return
    end
    M.publish(target)
  end, {
    nargs = 1,
    complete = function()
      return { "android", "ios", "macos", "ohos", "maven", "cocoapods", "spm", "index" }
    end,
    desc = "Publish CCGO package",
  })

  -- :CcgoTag [version]
  vim.api.nvim_create_user_command("CcgoTag", function(opts)
    M.tag(opts.fargs[1])
  end, {
    nargs = "?",
    desc = "Create git tag for CCGO project",
  })

  -- :CcgoPackage
  vim.api.nvim_create_user_command("CcgoPackage", function()
    M.package()
  end, {
    desc = "Package CCGO project",
  })
end

return M
