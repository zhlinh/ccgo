# CCGO 路线图

> 版本：v3.0.10 | 更新时间：2026-01-21

## 项目状态概览

| 模块 | 进度 | 状态 |
|------|------|------|
| Python CLI | 100% | 功能完整，维护模式 |
| Rust CLI | 100% | 功能完整，零 Python 依赖 ✅ |
| 跨平台构建 | 100% | 支持 8 个平台 |
| Docker 构建 | 100% | 通用交叉编译 |
| 依赖管理 | 95% | Git、路径、仓库源，带锁文件 |
| 发布系统 | 100% | Maven、CocoaPods、SPM、OHPM、Conan |
| 模板系统 | 100% | 基于 Copier 的项目生成 |
| CMake 集成 | 100% | 集中化构建脚本 |
| Gradle 插件 | 100% | Android/KMP 约定插件 |
| 文档 | 100% | 带 i18n 的 MkDocs（本文档！）|

**支持平台**：Android、iOS、macOS、Windows、Linux、OpenHarmony、watchOS、tvOS、Kotlin 多平台

---

## 优先级定义

- **P0（关键）**：阻塞核心功能或发布
- **P1（高）**：重要功能或重大改进
- **P2（中）**：增强功能，有价值但不紧急
- **P3（低）**：长期规划，锦上添花

---

## P0 - 关键（当前版本 v3.1）🔥

### 1. Rust CLI 功能对等
**状态**：100% 完成 ✅ | **目标**：v3.1.0（2026 年 Q1）

- [x] 核心构建命令（build、test、bench、doc）✅
- [x] 依赖管理（带锁文件的 install）✅
- [x] 项目创建（new、init）✅
- [x] 版本管理（tag、package）✅
- [x] vendor 命令实现 ✅
- [x] update 命令用于依赖更新 ✅
- [x] run 命令用于示例/二进制文件 ✅
- [x] CI 命令编排（通过组合命令实现）✅
- [x] 从 Python 完全迁移到 Rust ✅
- [x] 零 Python 依赖（直接调用 Copier）✅

**理由**：Rust 提供更好的性能、类型安全和更简单的分发（单一二进制）。

### 2. 文档完善
**状态**：100% 完成 | **目标**：v3.1.0（2026 年 Q1）✅

- [x] 带 i18n 的 MkDocs 设置 ✅
- [x] 首页和快速入门 ✅
- [x] 完整的平台指南（Android、iOS、macOS、Linux、Windows、OpenHarmony、KMP）✅
- [x] CLI 参考文档 ✅
- [x] CCGO.toml 配置参考 ✅
- [x] CMake 集成指南 ✅
- [x] Gradle 插件参考 ✅
- [x] 迁移指南（从 Conan、Python 到 Rust）✅

**理由**：良好的文档对用户采用和减少支持负担至关重要。

### 3. 错误处理增强
**状态**：100% 完成 | **目标**：v3.1.0（2026 年 Q1）✅

- [x] Rust CLI 中的统一错误类型 ✅
- [x] 带上下文提示的自定义错误类型 ✅
- [x] 带有可操作提示的用户友好错误消息 ✅
- [x] 工具缺失时的优雅降级 ✅
- [x] 全面的配置验证 ✅
- [x] 带需求级别的工具检测模块 ✅
- [x] 集成到构建/发布命令 ✅
- [x] 构建失败的诊断与常见解决方案 ✅

---

## P1 - 高（v3.2-v3.3）🚀

### 4. 包注册表支持
**状态**：0% 完成 | **目标**：v3.2.0（2026 年 Q2）

- [ ] ccgo-registry 服务器实现
- [ ] 发布包到 ccgo-registry
- [ ] 包发现和搜索
- [ ] 语义版本解析
- [ ] 私有注册表支持
- [ ] 与现有注册表集成（Conan Center、vcpkg）

**理由**：在组织和社区内更轻松地共享依赖。

### 5. IDE 集成
**状态**：10% 完成 | **目标**：v3.2.0（2026 年 Q2）

- [ ] VS Code 扩展
  - CCGO.toml 语法高亮
  - 构建任务集成
  - 依赖树可视化
- [ ] CLion/Android Studio 插件
- [ ] Xcode 项目生成改进
- [ ] Visual Studio 项目生成

**理由**：更好的 IDE 支持改善开发者体验。

### 6. 构建性能优化
**状态**：40% 完成 | **目标**：v3.3.0（2026 年 Q2）

- [x] 并行平台构建 ✅
- [x] Docker 层缓存 ✅
- [ ] 增量构建（仅重建更改的源）
- [ ] 构建缓存共享（ccache、sccache 集成）
- [ ] 远程构建执行（distcc、icecc）
- [ ] 构建分析和性能分析

**理由**：更快的构建 = 更快乐的开发者。

### 7. 高级依赖功能
**状态**：30% 完成 | **目标**：v3.3.0（2026 年 Q2）

- [x] 带修订固定的 Git 依赖 ✅
- [x] 路径依赖 ✅
- [x] 锁文件生成 ✅
- [ ] 依赖覆盖/补丁
- [ ] 依赖 vendoring 改进
- [ ] 传递依赖解析
- [ ] 版本冲突解决策略
- [ ] 工作区依赖（monorepo 支持）

---

## P2 - 中（v3.4-v4.0）📦

### 8. 测试框架增强
**状态**：100% 完成 ✅ | **目标**：v3.4.0（2026 年 Q3）

- [x] Google Test 集成 ✅
- [x] Catch2 集成 ✅
- [x] 测试发现改进 ✅
  - GoogleTest、Catch2 和 CTest 发现
  - 按名称模式过滤测试
  - 基于测试套件的组织
- [x] 代码覆盖率报告 ✅
  - 支持 gcov、llvm-cov、lcov
  - HTML、LCOV、JSON、Cobertura 输出格式
  - 通过 --fail-under-coverage 强制阈值
- [x] 测试结果聚合 ✅
  - XML 结果解析（GoogleTest 格式）
  - 跨套件聚合
  - JUnit XML 导出
- [x] 基准测试结果比较 ✅
  - Google Benchmark JSON 解析
  - 基线比较与回归检测
  - Markdown/JSON 导出报告
- [x] 与 CI 服务集成 ✅
  - GitHub Actions、GitLab CI、Azure DevOps、Jenkins、TeamCity
  - 自动检测 CI 环境
  - 原生 CI 注解格式

### 9. 代码生成工具
**状态**：0% 完成 | **目标**：v3.5.0（2026 年 Q3）

- [ ] Protocol Buffers 支持
- [ ] Flat Buffers 支持
- [ ] gRPC 支持
- [ ] GraphQL 代码生成
- [ ] OpenAPI 客户端生成
- [ ] 自定义代码生成插件系统

### 10. 平台特定功能
**状态**：各异 | **目标**：v3.6.0（2026 年 Q4）

- [ ] **Android**
  - [ ] Jetpack Compose 原生互操作
  - [ ] Android Studio 插件
  - [ ] R8/ProGuard 配置
- [ ] **iOS/macOS**
  - [ ] SwiftUI 互操作辅助工具
  - [ ] Xcode Cloud 集成
  - [ ] App Clip 支持
- [ ] **OpenHarmony**
  - [ ] DevEco Studio 集成
  - [ ] ArkTS 互操作
- [ ] **Windows**
  - [ ] UWP 支持
  - [ ] WinUI 3 集成

### 11. 安全功能
**状态**：20% 完成 | **目标**：v3.7.0（2026 年 Q4）

- [x] 基本校验和验证 ✅
- [ ] 依赖的 GPG 签名验证
- [ ] 安全审计报告
- [ ] 依赖的 CVE 扫描
- [ ] 供应链安全（SLSA 合规）
- [ ] 代码签名自动化

---

## P3 - 低（v4.0+）🔮

### 12. WebAssembly 支持
**状态**：0% 完成 | **目标**：v4.0.0（2027）

- [ ] WASM 目标编译
- [ ] Emscripten 集成
- [ ] WASI 支持
- [ ] WebAssembly 系统接口（WASI）

### 13. AI 驱动功能
**状态**：0% 完成 | **目标**：v4.1.0（2027）

- [ ] 基于项目分析的依赖建议
- [ ] 构建配置优化建议
- [ ] 从其他构建系统自动迁移
- [ ] 从自然语言生成代码

### 14. 云构建服务
**状态**：0% 完成 | **目标**：v4.2.0（2027）

- [ ] 托管构建服务（ccgo-cloud）
- [ ] 分布式缓存
- [ ] 构建分析仪表板
- [ ] 团队协作功能

### 15. 高级平台支持
**状态**：0% 完成 | **目标**：v4.x（2027+）

- [ ] FreeBSD 支持
- [ ] Haiku OS 支持
- [ ] RISC-V 架构支持
- [ ] LoongArch 架构支持
- [ ] PlayStation/Xbox 平台（如果许可允许）

---

## 最近完成（v3.0）✅

### 测试框架增强（v3.0.12）
- [x] 测试发现改进
  - GoogleTest（`--gtest_list_tests`）、Catch2（`--list-tests`）、CTest（`ctest -N`）
  - 支持正则表达式的测试名称过滤
  - 基于套件的组织和列表
- [x] 代码覆盖率报告
  - 支持 gcov、llvm-cov、lcov 工具
  - 输出格式：HTML、LCOV、JSON、Cobertura、Summary
  - 通过 `--fail-under-coverage` 标志强制阈值
- [x] 测试结果聚合
  - GoogleTest XML 结果解析
  - 跨套件聚合，包含通过/失败/跳过计数
  - JUnit XML 导出用于 CI 集成
- [x] 基准测试结果比较
  - Google Benchmark JSON 解析
  - 可配置阈值的基线比较
  - 通过 `--fail-on-regression` 进行回归检测
  - Markdown/JSON 导出报告
- [x] CI 服务集成
  - GitHub Actions（工作流注解）
  - GitLab CI（可折叠部分）
  - Azure DevOps（任务命令）
  - Jenkins（控制台格式化）
  - TeamCity（服务消息）
  - 通过环境变量自动检测

### Rust CLI 迁移（部分）
- [x] 项目架构重新设计
- [x] 核心命令实现
- [x] 依赖管理系统
- [x] 构建编排
- [x] 配置解析（CCGO.toml）

### Docker 构建系统
- [x] 基于 Docker 的通用交叉编译
- [x] 所有平台的预构建 Docker 镜像
- [x] 镜像缓存和优化
- [x] 多阶段构建支持

### 统一发布
- [x] Maven（本地、私有、中央）发布
- [x] CocoaPods 发布
- [x] Swift Package Manager 发布
- [x] OHPM 发布
- [x] Conan 发布

### Git 集成
- [x] 自动版本标记
- [x] 提交消息生成
- [x] Git hooks（pre-commit）支持
- [x] 基于 Git 的依赖

---

## 如何贡献

我们欢迎贡献！以下是您可以帮助的方式：

1. **选择功能**：从 P1 或 P2 优先级中选择一项
2. **讨论**：开启 GitHub Discussion 或 Issue 讨论您的方法
3. **实现**：遵循我们的[贡献指南](contributing.md)
4. **测试**：确保您的更改在各平台上工作
5. **文档**：更新新功能的文档
6. **提交**：创建 pull request

详细指南请参阅[贡献指南](contributing.md)。

---

## 反馈

对 CCGO 的未来有想法？我们很乐意听到您的声音！

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) - 功能请求和想法
- [GitHub Issues](https://github.com/zhlinh/ccgo/issues) - 错误报告和任务
- 邮箱：zhlinhng@gmail.com

---

*本路线图是一份动态文档，可能会根据社区反馈和项目优先级进行更改。*
