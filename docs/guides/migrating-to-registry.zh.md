# 迁移到注册表式依赖方案

本指南面向"维护多个共享同一套内部包的库项目"的组织，讲解如何把
`git`+`branch`、`zip`+`local-path` 的依赖声明，替换为由共享索引仓库
支撑的 `version` + `registry` 解析方式。

文中以一个虚构的"comm-group"组织作为运行示例：一组内部 C++ 库
（`stdcomm`、`foundrycomm`、`logcomm`，以及若干叶子消费方）目前通过
固定的 Git 分支互相依赖。任何"十几个并列项目需要协调发布、又不想搭
托管包服务"的团队都适用同样的形态。

## 你能得到什么

**之前**：每个消费方的 `CCGO.toml` 都写着每个依赖的 Git URL 和"分发分支"。
所以 `stdcomm` 一发新版，下游每个 `CCGO.toml` 的分支都要联动改。
更糟的是 `dist-v1.0.0` 这样的分支会在你脚下被强推更新。

```toml
[[dependencies]]
name = "stdcomm"
git = "git@git.example.com:org/stdcomm.git"
branch = "dist-v1.0.0"
version = "1.0.0"

[[dependencies]]
name = "logcomm"
git = "git@git.example.com:org/logcomm.git"
branch = "dist-v1.0.0"
version = "1.0.0"
```

**之后**：每个消费方都指向同一个共享索引仓库，再按精确版本号写每个
依赖。索引为每个版本记录归档 URL 和 SHA-256 校验和。

```toml
[registries]
org = "git@git.example.com:org/ccgo-index.git"

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
registry = "org"

[[dependencies]]
name = "logcomm"
version = "25.2.9519653"
registry = "org"
```

* 按精确版本号锁死。再没有会动的 `dist-v...` 分支。
* 一个共享索引列出所有已发布版本及其校验和。
* 消费方从 CDN 直接下载一个归档 zip。不传 git 历史，消费侧 clone 后
  也不必再构建。
* 加一个新的兄弟库时，对现有消费方零改动 —— 只要新项目跑一次
  `ccgo publish index` 即可。

## 第一步 —— 引导索引仓库

新建一个 Git 仓库专门承载包索引。它一开始是空的，由各项目的
`ccgo publish index` 逐步填充。

```bash
# 在你的 Git 服务上：先建一个空仓库，例如 org/ccgo-index.git
# 本地给它一个初始提交，确保 `git clone` 可用：
git clone git@git.example.com:org/ccgo-index.git
cd ccgo-index

cat > index.json <<'EOF'
{
  "name": "org-index",
  "description": "Internal package index for org libraries",
  "version": "1",
  "package_count": 0
}
EOF

git add index.json
git commit -m "init: empty index"
git push origin master
```

引导到这一步就够了。具体目录结构（按包名分片，每个包一个 JSON）由
后续每次 `ccgo publish index` 调用按需创建。

## 第二步 —— 每个项目的 CI 工作流

对集合中的每个库，把现有的发布步骤（大概率是"打 tag、推送、更新分支"）
换成 build → package → upload → publish-index 这套链路：

```bash
# 在该库的仓库里：

# 1. 构建所有平台。
ccgo build all --release

# 2. 把构建产物打成 zip，目录布局与 `ccgo fetch` 解压到
#    .ccgo/deps/<name>/ 时所期望的一致。
ccgo package --release
# 产出：target/release/<plat>/<NAME>_CCGO_PACKAGE-<version>.zip

# 3. 把 zip 上传到你的 CDN / artifactory。
#    （沿用你已有的发布上传脚本即可。目标 URL 必须与下面
#    `ccgo publish index` 里传的模板一致。）
your-upload-script target/release/macos/STDCOMM_CCGO_PACKAGE-25.2.9519653.zip \
  https://artifacts.example.com/stdcomm/

# 4. 把新版本追加到共享索引里。每次 `ccgo publish index` 仅发布
#    一个版本(append-only,重复版本会被拒绝)。用 `--index-version`
#    指定版本号(tag 约定不是 `v<version>` 时再加 `--index-tag`)。
ccgo publish index \
  --index-repo git@git.example.com:org/ccgo-index.git \
  --index-name org-index \
  --index-version 25.2.9519653 \
  --archive-url-template "https://artifacts.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip" \
  --checksum \
  --index-push
```

`{name}`、`{version}`、`{tag}` 占位符按模板逐个替换。集合里每个库每次
发布新版本时都跑这条命令;索引仓库会自然累积条目 —— ccgo 只追加和排序,
旧版本跨多次 publish 都保留。

## 第三步 —— 迁移消费方 CCGO.toml

在每个消费方项目里，把现有的 `git`/`branch`（或 `zip`/`local-path`）
依赖声明替换成 `version` + `registry`：

```diff
+[registries]
+org = "git@git.example.com:org/ccgo-index.git"
+
 [[dependencies]]
 name = "stdcomm"
-git = "git@git.example.com:org/stdcomm.git"
-branch = "dist-v1.0.0"
-version = "1.0.0"
+version = "25.2.9519653"
+registry = "org"
```

接着运行 `ccgo fetch` 验证依赖可以走索引解析。生成的 `CCGO.lock` 会
记录 `source = "registry+git@..."` 与 checksum，下次
`ccgo fetch --locked` 就能据此精确重现同一份字节。

如果同一个消费方需要从多个集合拉依赖，就在 `[registries]` 里并列声明
几条，再在每个 `[[dependencies]]` 上自己写 `registry = "..."` 钉死即可。
单 dep 的 selector 会覆盖 TOML 的遍历顺序。

## 第四步 —— 由深到浅地推开（最深的依赖先）

按依赖图的拓扑顺序迁移，先从叶子开始。这样轮到迁移依赖 `stdcomm` 的
那个库时，`stdcomm` 已经能通过索引发布了 —— 两边的改动永远不必塞进
同一个 PR。

典型的推开顺序：

1. **`stdcomm`**（无内部依赖）。先接上 publish-to-index 的 CI 步骤。
   现有消费方暂时继续用 `git`/`branch` —— `stdcomm` 自身的 CCGO.toml
   对消费方零改动。
2. **`foundrycomm`、`logcomm`**（依赖 `stdcomm`）。先迁它们的消费侧：
   它们的 CCGO.toml 里把 `stdcomm` 改成 `version` + `registry` 写法。
   用 `ccgo fetch` + `ccgo build` 验证。然后再接上 publish-to-index
   的 CI 步骤，让它们自己也能被索引解析到。
3. **叶子消费方**（依赖 `foundrycomm` / `logcomm`）。把它们的
   CCGO.toml 全部指向索引。到这一步，它们要的每个依赖在索引里都已就位。

不需要一次性把整张图改完。在同一个 CCGO.toml 里同时混写老式
`git`/`branch` 和新式 `version`/`registry` 是合法的 —— 它们是
[解析优先级列表](../dependency-resolution.zh.md#解析优先级)中两条独立的
分支。

## 回滚

老式 `git`/`branch` 与 `zip` 声明照旧能用。注册表层是 opt-in 的，
仅在以下两个条件同时满足时才触发：(a) `[registries]` 非空；
(b) 某个依赖未显式写 `path`/`git`/`zip` 来源。如果某个项目踩到坑
—— 索引仓库不通、归档 URL 挂掉、checksum 不匹配 —— 把那个项目的
CCGO.toml 回退到旧的 `git = "..." / branch = "..."` 是一行 revert
的事。完全不必推平基础设施。

你也可以在同一个 CCGO.toml 里逐个依赖地迁。把绝大多数依赖留在
`git` / `branch`，只把风险最高的那一个换成 `version` + `registry`，
在 CI 上跑通之后再换下一个。

## 另请参阅

* [包注册表特性参考](../features/registry.zh.md) —— 索引 JSON schema、
  CLI、`--archive-url-template` 替换规则。
* [依赖解析](../dependency-resolution.zh.md) —— 跨所有源类型的完整
  解析优先级列表。
* [配置指南](../getting-started/configuration.zh.md#注册表依赖) ——
  `[registries]` 与 `[[dependencies]].registry` 的字段参考。
