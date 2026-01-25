-- CCGO LuaSnip snippets module

local M = {}

-- Check if LuaSnip is available
local function has_luasnip()
  local ok, _ = pcall(require, "luasnip")
  return ok
end

-- Define snippets
function M.setup()
  if not has_luasnip() then
    return
  end

  local ls = require("luasnip")
  local s = ls.snippet
  local t = ls.text_node
  local i = ls.insert_node
  local c = ls.choice_node
  local f = ls.function_node

  -- Helper to get current year
  local function current_year()
    return os.date("%Y")
  end

  local snippets = {
    -- Package metadata
    s("package", {
      t({ "[package]", "" }),
      t('name = "'), i(1, "my-project"), t({ '"', "" }),
      t('version = "'), i(2, "0.1.0"), t({ '"', "" }),
      t('description = "'), i(3, "A cross-platform C++ library"), t({ '"', "" }),
      t('authors = ["'), i(4, "Your Name <you@example.com>"), t({ '"]', "" }),
      t('license = "'), i(5, "MIT"), t({ '"', "" }),
      t('repository = "'), i(6, "https://github.com/user/repo"), t({ '"', "" }),
      i(0),
    }),

    -- Build configuration
    s("build", {
      t({ "[build]", "" }),
      t('cmake_minimum_version = "'), i(1, "3.21"), t({ '"', "" }),
      t('cpp_standard = "'), i(2, "17"), t({ '"', "" }),
      t('c_standard = "'), i(3, "11"), t({ '"', "" }),
      t("symbol_visibility = "), c(4, {
        t('"hidden"'),
        t('"default"'),
      }),
      t({ "", "" }),
      i(0),
    }),

    -- Git dependency
    s("dep-git", {
      t({ "[[dependencies]]", "" }),
      t('name = "'), i(1, "library-name"), t({ '"', "" }),
      t('version = "'), i(2, "0.0.0"), t({ '"', "" }),
      t('git = "'), i(3, "https://github.com/user/repo.git"), t({ '"', "" }),
      t('branch = "'), i(4, "main"), t({ '"', "" }),
      i(0),
    }),

    -- Path dependency
    s("dep-path", {
      t({ "[[dependencies]]", "" }),
      t('name = "'), i(1, "local-lib"), t({ '"', "" }),
      t('version = "'), i(2, "0.0.0"), t({ '"', "" }),
      t('path = "'), i(3, "../path/to/lib"), t({ '"', "" }),
      i(0),
    }),

    -- Registry dependency (simplified)
    s("dep", {
      t({ "[dependencies]", "" }),
      i(1, "fmt"), t(' = "^'), i(2, "10.0"), t({ '"', "" }),
      i(0),
    }),

    -- Android platform
    s("android", {
      t({ "[platforms.android]", "" }),
      t("min_sdk = "), i(1, "21"), t({ "", "" }),
      t('ndk_version = "'), i(2, "25.2.9519653"), t({ '"', "" }),
      t('stl = "'), i(3, "c++_shared"), t({ '"', "" }),
      i(0),
    }),

    -- iOS platform
    s("ios", {
      t({ "[platforms.ios]", "" }),
      t('min_version = "'), i(1, "13.0"), t({ '"', "" }),
      t("enable_bitcode = "), c(2, { t("false"), t("true") }), t({ "", "" }),
      i(0),
    }),

    -- macOS platform
    s("macos", {
      t({ "[platforms.macos]", "" }),
      t('min_version = "'), i(1, "10.15"), t({ '"', "" }),
      i(0),
    }),

    -- Windows platform
    s("windows", {
      t({ "[platforms.windows]", "" }),
      t('toolset = "'), c(1, { t("v143"), t("v142"), t("v141") }), t({ '"', "" }),
      t('runtime = "'), c(2, { t("MD"), t("MT"), t("MDd"), t("MTd") }), t({ '"', "" }),
      i(0),
    }),

    -- Linux platform
    s("linux", {
      t({ "[platforms.linux]", "" }),
      t('compiler = "'), c(1, { t("gcc"), t("clang") }), t({ '"', "" }),
      i(0),
    }),

    -- OHOS platform
    s("ohos", {
      t({ "[platforms.ohos]", "" }),
      t("min_sdk = "), i(1, "9"), t({ "", "" }),
      i(0),
    }),

    -- Maven publish
    s("publish-maven", {
      t({ "[publish.maven]", "" }),
      t('group_id = "'), i(1, "com.example"), t({ '"', "" }),
      t('artifact_id = "'), i(2, "my-library"), t({ '"', "" }),
      t('repository_url = "'), i(3, "https://maven.pkg.github.com/user/repo"), t({ '"', "" }),
      i(0),
    }),

    -- CocoaPods publish
    s("publish-cocoapods", {
      t({ "[publish.cocoapods]", "" }),
      t('pod_name = "'), i(1, "MyLibrary"), t({ '"', "" }),
      t('summary = "'), i(2, "A cross-platform C++ library"), t({ '"', "" }),
      t('homepage = "'), i(3, "https://github.com/user/repo"), t({ '"', "" }),
      t('license = "'), i(4, "MIT"), t({ '"', "" }),
      i(0),
    }),

    -- Feature
    s("feature", {
      t({ "[[features]]", "" }),
      t('name = "'), i(1, "feature-name"), t({ '"', "" }),
      t("default = "), c(2, { t("false"), t("true") }), t({ "", "" }),
      t('dependencies = ["'), i(3), t({ '"]', "" }),
      i(0),
    }),

    -- Workspace
    s("workspace", {
      t({ "[workspace]", "" }),
      t("members = ["), t({ "", "" }),
      t('  "'), i(1, "packages/lib-a"), t({ '",', "" }),
      t('  "'), i(2, "packages/lib-b"), t({ '",', "" }),
      t({ "]", "" }),
      i(0),
    }),

    -- Registry
    s("registry", {
      t({ "[registries]", "" }),
      i(1, "custom"), t(' = "'), i(2, "https://github.com/org/package-index.git"), t({ '"', "" }),
      i(0),
    }),

    -- Full CCGO.toml template
    s("ccgo-full", {
      t({ "[package]", "" }),
      t('name = "'), i(1, "my-project"), t({ '"', "" }),
      t('version = "'), i(2, "0.1.0"), t({ '"', "" }),
      t('description = "'), i(3, "A cross-platform C++ library"), t({ '"', "" }),
      t('authors = ["'), i(4, "Your Name <you@example.com>"), t({ '"]', "" }),
      t('license = "'), i(5, "MIT"), t({ '"', "" }),
      t({ "", "" }),
      t({ "[build]", "" }),
      t('cmake_minimum_version = "3.21"'), t({ "", "" }),
      t('cpp_standard = "17"'), t({ "", "" }),
      t('symbol_visibility = "hidden"'), t({ "", "" }),
      t({ "", "" }),
      t({ "[platforms.android]", "" }),
      t("min_sdk = 21"), t({ "", "" }),
      t({ "", "" }),
      t({ "[platforms.ios]", "" }),
      t('min_version = "13.0"'), t({ "", "" }),
      t({ "", "" }),
      t({ "[platforms.macos]", "" }),
      t('min_version = "10.15"'), t({ "", "" }),
      t({ "", "" }),
      t({ "# [[dependencies]]", "" }),
      t({ '# name = "fmt"', "" }),
      t({ '# git = "https://github.com/fmtlib/fmt.git"', "" }),
      t({ '# branch = "10.2.1"', "" }),
      i(0),
    }),
  }

  -- Add snippets for TOML filetype
  ls.add_snippets("toml", snippets)

  -- Also add for ccgo-toml if we define that filetype
  ls.add_snippets("ccgo-toml", snippets)
end

return M
