# 构建 Profile

命名构建 profile 让你可以定义可复用的配置片段，并通过一个简单的标志来选择它，而不必重复冗长的命令行参数。

## 概述

Profile 是存储在 `CCGO.toml` 的 `[profile.<name>]` 表中的一组命名构建配置——包括 release 模式、链接类型、features、CMake 标志、依赖链接方式和输出包名。在 `ccgo build` 命令中传入 `--profile <name>`，所有 profile 配置将叠加在全局配置之上生效。

```bash
ccgo build android --profile sanitize
ccgo build ios --profile release-shared
ccgo build macos --profile fat-static
```

## 内置 Profile

以下两个 profile 无需声明即可直接使用：

| 名称 | 效果 |
|------|------|
| `debug` | `release = false`——保留调试符号，不启用优化 |
| `release` | `release = true`——等同于传入 `--release` |

```bash
ccgo build android --profile release   # 等同于：ccgo build android --release
ccgo build android --profile debug     # 显式 debug 构建
```

## 定义自定义 Profile

在 `CCGO.toml` 中添加 `[profile.<name>]` 表：

```toml
[profile.sanitize]
release = false
link_type = "both"

[profile.sanitize.cmake]
c_flags   = ["-fsanitize=address", "-fno-omit-frame-pointer"]
cpp_flags = ["-fsanitize=address", "-fno-omit-frame-pointer"]
```

然后通过以下命令使用：

```bash
ccgo build macos --profile sanitize
```

## Profile 字段

### 标量字段

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `inherits` | 字符串 | 要继承的父 profile 名称 | — |
| `name` | 字符串 | 覆盖输出包名 | 包名 |
| `release` | 布尔值 | `true` = release，`false` = debug | — |
| `link_type` | 字符串 | `"static"` \| `"shared"` \| `"both"` | — |
| `jobs` | 整数 | 并行构建任务数 | 自动检测 |

### [profile.\<name\>.cmake]

当此 profile 激活时，应用于所有平台的额外 CMake 标志。

| 字段 | 类型 | 描述 |
|------|------|------|
| `merge` | 字符串 | `"replace"`（默认）或 `"extend"`——参见[合并策略](#合并策略) |
| `arguments` | 数组 | 直接传递给 cmake configure 的原始参数 |
| `c_flags` | 数组 | 追加到 `CMAKE_C_FLAGS` 的标志 |
| `cpp_flags` | 数组 | 追加到 `CMAKE_CXX_FLAGS` 的标志 |

### [profile.\<name\>.features]

此 profile 激活时启用的 features。

| 字段 | 类型 | 描述 |
|------|------|------|
| `merge` | 字符串 | `"replace"`（默认）或 `"extend"` |
| `list` | 数组 | 要启用的 feature 名称列表 |

### [profile.\<name\>.dep_linkage]

此 profile 激活时依赖的默认链接方式。

| 字段 | 类型 | 描述 |
|------|------|------|
| `default` | 字符串 | 适用于所有构建类型的链接方式 |
| `on_shared` | 字符串 | 当消费者构建动态库时的覆盖值 |
| `on_static` | 字符串 | 当消费者构建静态库时的覆盖值 |

**链接方式取值：** `"shared-external"` \| `"static-embedded"` \| `"static-external"`

### 平台级覆盖

在 profile 内部，可以针对特定平台覆盖 CMake 标志或依赖链接方式：

```toml
[profile.sanitize.platforms.android.build.cmake]
merge     = "extend"
cpp_flags = ["-fsanitize=address"]

[profile.sanitize.platforms.android.build.dep_linkage]
default = "static-embedded"
```

支持的平台：`android`、`ios`、`macos`、`windows`、`linux`、`ohos`

## 合并策略

列表字段（`cmake.arguments`、`cmake.c_flags`、`cmake.cpp_flags`、`features.list`）都携带一个 `merge` 字段，控制其与父 profile 累积列表的合并方式：

| 取值 | 行为 |
|------|------|
| `"replace"` | 丢弃父 profile 的列表，仅使用当前 profile 的列表（默认） |
| `"extend"` | 将当前 profile 的列表追加到父 profile 累积列表之后 |

```toml
[profile.base]
[profile.base.cmake]
cpp_flags = ["-Wall", "-Wextra"]

[profile.strict]
inherits = "base"
[profile.strict.cmake]
merge     = "extend"          # 追加到 base 的 ["-Wall", "-Wextra"] 之后
cpp_flags = ["-Werror"]       # 最终结果：["-Wall", "-Wextra", "-Werror"]
```

若不设置 `merge = "extend"`，`strict` profile 会完全替换 `base` 的标志。

## 继承

Profile 支持通过 `inherits` 实现单继承：

```toml
[profile.base]
release = false
[profile.base.cmake]
cpp_flags = ["-Wall"]

[profile.sanitize]
inherits = "base"       # 继承 release=false 和 cpp_flags=["-Wall"]
[profile.sanitize.cmake]
merge     = "extend"
cpp_flags = ["-fsanitize=address"]   # 最终：["-Wall", "-fsanitize=address"]
```

**规则：**
- 仅支持单继承（`inherits` 只接受一个名称，不支持列表）
- 支持继承链：`sanitize` 继承 `base`，`base` 继承 `debug`
- 循环引用会被检测并报告为错误
- 内置 profile `debug` 和 `release` 始终可以作为 `inherits` 的目标

## 优先级顺序

配置按从低到高的优先级依次应用，后者覆盖前者：

1. 硬编码默认值（例如 release = false）
2. 全局 `CCGO.toml` 配置（`[build]`、`[build.cmake]`、`[platforms.X.build.cmake]`）
3. 继承链（从最古老的祖先到最近的父 profile 依次应用）
4. 当前激活 profile 的自身设置
5. CLI 标志（始终最高优先级——`--release`、`--link-type`、`--features`、`--jobs`）

例如，若 profile 设置了 `release = true`，同时命令行也传入了 `--release`，最终结果仍为 `release = true`（二者一致）。若 profile 设置了 `release = false` 而命令行传入了 `--release`，则 CLI 优先。

## CMake 标志合并顺序

CMake 标志通过四个层次累积（越靠前优先级越低）：

```
[build.cmake]                                  全局 CCGO.toml CMake 标志
  + [platforms.android.build.cmake]            全局平台级 CMake 标志
  + [profile.X.cmake]                          profile 全局 CMake 标志
  + [profile.X.platforms.android.build.cmake]  profile 平台级 CMake 标志
```

四个层次均为拼接（非替换），确保 `[build.cmake]` 和平台级覆盖中的标志与 profile 专属标志共同生效。

## 覆盖包名

Profile 中的 `name` 字段可以覆盖用于构建产物和 SDK 归档的包名。适用于以不同名称发布不同平台或配置的构建产物：

```toml
[profile.release-debug-symbols]
inherits = "release"
name = "mylib-with-symbols"
[profile.release-debug-symbols.cmake]
cpp_flags = ["-g"]
```

## 示例：常用 Profile

### Debug + ASan（地址/未定义行为检查）

```toml
[profile.asan]
inherits  = "debug"
link_type = "both"

[profile.asan.cmake]
c_flags   = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
cpp_flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
```

### Release + 符号隐藏

```toml
[profile.release-hidden]
inherits  = "release"
link_type = "shared"

[profile.release-hidden.cmake]
cpp_flags = ["-fvisibility=hidden", "-fvisibility-inlines-hidden"]
```

### 全静态（所有依赖静态编译）

```toml
[profile.fat-static]
inherits  = "release"
link_type = "static"

[profile.fat-static.dep_linkage]
default = "static-embedded"
```

### 平台专属覆盖

```toml
[profile.neon]
inherits = "release"

[profile.neon.platforms.android.build.cmake]
merge     = "extend"
arguments = ["-DANDROID_ARM_NEON=TRUE"]
```

## 另请参阅

- [CCGO.toml 参考 — \[profile.\<name\>\]](../reference/ccgo-toml.zh.md#profilename)
- [CCGO.toml.example](../../CCGO.toml.example) — 完整注释模板
- [构建系统](build-system.zh.md)
