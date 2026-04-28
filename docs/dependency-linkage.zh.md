# 依赖链接

两个正交维度共同决定一个依赖如何进入你的构建产物：

| 维度 | CCGO.toml 字段 | 取值 |
|---|---|---|
| **你**自己产出什么 | `[build].link_type` | `static`、`shared`、`both` |
| **依赖**与你的关系 | `[[dependencies]].linkage`（项目级默认值见 `[build].default_dep_linkage`）| `shared-external`、`static-embedded`、`static-external` |

## 链接取值

* **`shared-external`** —— 依赖保持为独立的 `.so`/`.dylib`/`.dll`，你的二进制记录运行时依赖（ELF 上为 `DT_NEEDED`）。当依赖能产出共享产物时，这是 shared 消费者的默认选项。最适合在多个消费者间共享同一应用内依赖 —— 避免胖归档为每个消费者都复制一份依赖。
* **`static-embedded`** —— 依赖的 `.a` 在链接时被归档进你的二进制。你的二进制变得自包含；多个消费者各自携带一份依赖代码。当依赖只发布 `.a` 时是默认回退方案。
* **`static-external`** —— 依赖保留为独立的 `.a`，你的 `.a` 仅记录该依赖关系而不合并。仅对 static 消费者有效；最终可执行文件的链接器在 exe 链接时解析符号。即"细链"模型。
* `shared-embedded` —— **不存在**。尝试设置该值会产生解析错误，并指向两个有效的备选项。`.so` 不能被归档进另一个 `.so`。

## 决策矩阵

| 消费者 | 依赖提供 | 提示 | 结果 |
|---|---|---|---|
| `static` | 任意 |（任意）| `static-external`（仅有 `.so` 时为 `shared-external`）|
| `shared` | 仅 `.a` |（任意）| `static-embedded`（强制；`shared-external` 提示报错）|
| `shared` | 仅 `.so` |（任意）| `shared-external`（强制；`static-embedded` 提示报错）|
| `shared` | 两者皆有 / 源码 | 缺省 | `shared-external` |
| `shared` | 两者皆有 / 源码 | `shared-external` | `shared-external` |
| `shared` | 两者皆有 / 源码 | `static-embedded` | `static-embedded` |
| `shared` | 任意 | `static-external` | **报错** —— 会在 `.so` 中留下未解析的外部静态符号引用 |

## 何时需要覆盖

大多数项目根本不需要设置 `linkage`。默认值
（当依赖发布 `.so` 时，shared 消费者使用 shared-external）能在多个同级消费者
之间避免膨胀，同时不带意外。

需要在某个依赖上设 `linkage = "static-embedded"` 的场景：
* 该依赖较小，且你希望保持自包含（少发布一个 `.so`）。
* 你正在通过 Maven/CocoaPods 向外部开发者发布 SDK，他们没有
  独立安装传递依赖的途径。
* 该依赖只发布 `.a`，你想消除构建日志中的自动回退提示。

## 构建期日志

ccgo 在 CMake configure 时会针对每个依赖输出一行 `STATUS`：

```
[ccgo] stdcomm:    linkage=shared-external (DT_NEEDED to dep.so)
[ccgo] tinyhelper: linkage=static-embedded (.a archived into target)
[ccgo] zstd:       linkage=static-embedded (auto, no .so available)
```

`(auto, ...)` 表示该结果来自回退，而非显式的 `linkage` 字段；
`(auto, no .so available)` 特指依赖未发布共享形式，因此只能内嵌。

## 示例

```toml
[package]
name = "logcomm"
version = "1.0.0"

[build]
link_type = "shared"                 # 我产出 .so
default_dep_linkage = "shared-external"  # 我的依赖默认值

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
# 应用默认 linkage → shared-external（libstdcomm.so 保持独立）

[[dependencies]]
name = "tinyhelper"
version = "0.3.0"
linkage = "static-embedded"          # 显式：归档进 liblogcomm.so
```

可读为："我是一个共享库。默认情况下我的依赖都是外部的。tinyhelper
是例外 —— 把它焊进我里面。" 三种语义、三个设置、毫无歧义。

## 仅源码依赖

当一个依赖只发布源码（`.ccgo/deps/<name>/` 目录下有 `src/` 和
`CCGO.toml`，但没有当前目标平台的 `lib/<platform>/` 产物）时，
`ccgo build` 会自动递归：在解析 linkage 之前，它会在该依赖目录内
派生 `ccgo build <platform> --build-as <derived>`，然后将
`dep/lib/<platform>/` symlink 到新的构建输出，让消费者的
`FindCCGODependencies.cmake` 巡查能找到产物。

`--build-as` 的取值由消费者对该依赖解析后的提示派生：

| 消费者对依赖的提示 | 递归 `--build-as` |
|---|---|
| `shared-external`（默认）| `shared` |
| `static-embedded` / `static-external` | `static` |
|（无提示）| `both` |

依赖自身的 `[build].link_type` 声明**不会**决定递归实例化时
产出什么 —— 由消费者的需求决定。如果你在某次构建中设置了
`--linkage stdcomm=static-embedded` 且 `stdcomm` 是仅源码依赖，
就会得到 `.a`；如果切换到 `--linkage stdcomm=shared-external`，
ccgo 会在下次构建时把 `stdcomm` 重建为 `.so`。

### 缓存

实例化步骤会在
`.ccgo/deps/<name>/.ccgo_materialize_<platform>_<build_as>.fingerprint`
持久化一份按平台、按 `--build-as` 隔离的指纹。指纹是
（按字典序的源码树 mtime + size + 路径）+ `CCGO.toml` 内容
+ 请求的 `--build-as` 的 SHA-256。当指纹匹配且 `lib/<platform>/`
仍存在产物时，后续构建会跳过递归 spawn。按 `build_as` 拆分 sidecar
是为了避免同一路径源码依赖的两次并行构建（一个想要
`--build-as shared`、另一个想要 `--build-as static`）在共享 sidecar 上发生竞态。

行为矩阵：

| 状态 | 动作 |
|---|---|
| 无 `lib/<platform>/`、无 fingerprint | 派生构建，写入 fingerprint |
| 无 `lib/<platform>/`、fingerprint 存在 | 派生构建（lib 已被删除）|
| `lib/<platform>/` 存在、fingerprint 匹配 | 跳过（缓存命中）|
| `lib/<platform>/` 存在、fingerprint 不匹配 | 派生构建（源码已变）|
| `lib/<platform>/` 存在、无 fingerprint | 信任已构建产物，写入 fingerprint |

"信任已构建产物"路径对那些自带手工策划的 `lib/<platform>/`
布局的 fixture 和项目（例如 xcframework symlink）很重要。它们在
首次调用时被打上 fingerprint，自此参与正常的源码变更失效流程。

### 哪些参数会传播到递归构建

递归 `ccgo build` 调用会继承：

* `--release`（来自父构建的 release 标志）
* `--arch <csv>`（小写化；与父构建的 `--arch` 对齐）
* `--build-as <variant>`（按上文的提示派生）

它**不会**继承：

* `--linkage` —— 依赖自己的 `[[dependencies]]` 由它自己的
  CCGO.toml 决定。消费者的逐依赖 linkage 提示只作用于该消费者
  与该依赖的关系，不作用于该依赖与其自身依赖的关系。

### 失败模式

如果递归构建失败，父 ccgo 会以依赖名加复现命令 bail：

```
recursive `ccgo build` for source-only dep 'stdcomm' (--build-as shared) failed
with exit code Some(1). The dep at .ccgo/deps/stdcomm could not be compiled —
check its CCGO.toml and try `ccgo build macos --build-as shared` inside that
directory to reproduce.
```

### 消费者 CMake 模板中的源码与二进制优先级

当一个 path-source 依赖同时发布 `src/` 和预构建的 `lib/<platform>/`
产物（后者是 bridge 在材化 spawn 成功后填入的）时，如果解析后的
linkage 为 `shared-external` 且 bridge 已在预期深度放置可用的共享
库，消费者的 CMake 模板现在会跳过内联源码编译。具体说：

* `consumer/.ccgo/deps/<name>/lib/<platform>/shared/<name>.xcframework/`
  存在（Apple）或 `lib/<platform>/shared/<arch>/lib<name>.so` 存在
  （Android/OHOS/Linux）→ 该依赖的 `src/` 在 `<consumer>-deps`
  聚合中被跳过。消费者的主共享目标改为通过
  DT_NEEDED / LC_LOAD_DYLIB 连接到 `libleaf.dylib` / `libleaf.so`。
* `static-embedded` linkage 仍按预期把依赖源码编入消费者归档
  （或链接到预构建的 `.a`）。

只有在没有共享产物且 linkage 提示要求或最终落到 `static-embedded`
时，才会回退到内联源码编译。这是 linkage 矩阵所期望的契约。

### 跨平台 bridge

bridge 步骤在 Windows 上以 NTFS 目录 junction（`mklink /J`）替代
Unix `symlink`。Junction 不需要管理员权限或开发者模式，且只能在
依赖所在的同一卷上工作 —— `cmake_build/` 目录树正好满足。它们
在 `FindCCGODependencies.cmake` 所做的 `EXISTS` / `file(GLOB ...)`
巡查中表现与 symlink 完全一致。

## 另请参阅

- [`dependency-resolution.zh.md`](dependency-resolution.zh.md) —— ccgo 在
  做出 linkage 决策之前如何查找并拉取依赖。
- Rust 源码：`src/build/linkage.rs` —— 纯决策矩阵与文件系统扫描器。
  上文的决策表与单元测试一一对应。
