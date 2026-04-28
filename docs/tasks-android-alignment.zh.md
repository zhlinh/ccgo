# 任务：对齐 Rust ccgo-rs 与 Python pyccgo 的 Android 构建输出

## 功能概述

将 Rust ccgo-rs 的 Android 构建输出结构与 Python pyccgo 版本对齐，确保兼容性与一致性。

## 用户故事

- **US1**：作为开发者，我希望 Rust 版 Android 构建产生与 Python 版相同的归档结构，使消费者获得一致的体验。
- **US2**：作为开发者，我希望 release 库被 strip，使输出文件更小、可直接用于生产。
- **US3**：作为开发者，我希望获得 symbols 归档，以便排查生产环境崩溃。

## 依赖关系

```
Phase 1（Setup）→ Phase 2（基础）→ US1/US2/US3（可并行）→ Phase 5（打磨）
```

---

## Phase 1：Setup（已跳过 —— 暂时保留 Python ccgo 现状）

- [ ] T001 在 `../ccgo/setup.py` 中将 Python ccgo 包重命名为 pyccgo
- [ ] T002 在 `../ccgo/ccgo/main.py` 中将 Python ccgo CLI 入口更新为 pyccgo
- [ ] T003 如存在，则在 pyproject.toml 或 setup.cfg 中更新 Python 包名

---

## Phase 2：基础 —— Android NDK 工具链增强 ✅ 已完成

- [x] T004 在 `src/build/toolchains/android_ndk.rs` 中为 AndroidNdkToolchain 添加 llvm-strip 路径检测
- [x] T005 在 `src/build/toolchains/android_ndk.rs` 中为 AndroidNdkToolchain 添加 STL 库路径访问器（libc++_shared.so）
- [x] T006 在 `src/build/toolchains/android_ndk.rs` 中为 AndroidNdkToolchain 添加 strip_library() 方法

---

## Phase 3：用户故事 1 —— 归档结构对齐 ✅ 已完成

**目标**：产出与 Python 版本一致的归档结构

**独立测试**：用两个工具分别构建 Android，对比 ZIP 内容

- [x] T007 [US1] 在 `src/build/platforms/android.rs` 与 `src/commands/build.rs` 中将平台名从 "android" 改为 "Android"
- [x] T008 [US1] 在 `src/commands/build.rs` 中通过 BuildTarget::Display trait 让 cmake_build_dir 使用 "Android"
- [x] T009 [P] [US1] 在 `src/build/platforms/android.rs` 中为 symbols staging 添加 obj/ 目录结构
- [ ] T010 [P] [US1] 在 `src/build/archive.rs` 中为 ArchiveBuilder 添加 haars/ 目录支持（已延期 —— AAR 支持）
- [x] T011 [US1] 在 `src/build/platforms/android.rs` 中更新 AndroidBuilder，将未 strip 的库加入 symbols staging

---

## Phase 4：用户故事 2 —— 库 strip ✅ 已完成

**目标**：从 release 共享库中剥离调试符号

**独立测试**：对比 strip 前后的文件大小

- [x] T012 [US2] 在 `src/build/platforms/android.rs` 中实现 AndroidBuilder 的 strip_shared_libraries() 方法
- [x] T013 [US2] 在 `src/build/platforms/android.rs` 中，于 build_link_type() 后对 release 构建调用 strip_shared_libraries()
- [x] T014 [P] [US2] 在 `src/build/platforms/android.rs` 中为共享库输出添加 STL 库（libc++_shared.so）拷贝

---

## Phase 5：用户故事 3 —— Symbols 归档 ✅ 已完成

**目标**：生成包含未 strip 库的 -SYMBOLS.zip 用于崩溃调试

**独立测试**：构建 Android，验证 -SYMBOLS.zip 存在且包含未 strip 的 .so 文件

- [x] T015 [US3] 在 `src/build/platforms/android.rs` 中为 AndroidBuilder 创建 symbols staging 目录结构（obj/{arch}/）
- [x] T016 [US3] 在 `src/build/platforms/android.rs` 中，在 strip 之前将未 strip 的共享库拷贝到 symbols staging
- [x] T017 [US3] 在 `src/build/platforms/android.rs` 中，在主归档之后调用 archive.create_symbols_archive()
- [x] T018 [US3] 在 `src/build/platforms/android.rs` 中更新 BuildResult 以返回 symbols_archive 路径

---

## Phase 6：打磨与横切关注点 ✅ 已完成

- [x] T019 在 `src/build/platforms/android.rs` 中为 strip 操作添加 verbose 日志
- [x] T020 在 `src/build/platforms/android.rs` 中更新 clean()，处理 "Android" 目录名
- [ ] T021 [P] 在 `src/build/toolchains/android_ndk.rs` 中为 AndroidNdkToolchain 的新方法添加单元测试（已延期）
- [x] T022 [P] 在 `src/build/platforms/android.rs` 中更新文档/注释以反映新的输出结构
- [ ] T023 通过构建测试项目验证输出与 Python 版本一致（手动测试）

---

## 并行执行机会

### Phase 2（基础）内：
- T004、T005、T006 可并行（互相独立的方法添加）

### Phase 3（US1）内：
- T009 与 T010 可并行（不同的归档增项）

### Phase 4（US2）内：
- T014 可与 T012/T013 并行（STL 与 strip 互不相关）

### Phase 6（打磨）内：
- T021 与 T022 可并行（测试 vs 文档）

---

## 总结

| 指标 | 数量 |
|--------|-------|
| 总任务 | 23 |
| 已完成 | 17 |
| 已延期 | 3 |
| 已跳过 | 3 |
| 完成率 | 74% |

**状态**：✅ Android 核心对齐已完成。归档结构、库 strip 与 symbols 归档已与 Python pyccgo 输出对齐。

**已修改的文件**：
- `src/build/toolchains/android_ndk.rs` —— 新增 llvm_strip_path()、stl_library_path()、strip_library()、copy_stl_library()
- `src/build/platforms/android.rs` —— 新增 strip_shared_libraries()、symbols 归档支持，并将平台名更新为 "Android"
- `src/commands/build.rs` —— 将 BuildTarget::Android 的 display 更新为 "Android"

**剩余工作（已延期）**：
- T010：AAR/HAR 支持（haars/ 目录）—— 需要额外的 Gradle 集成
- T021：AndroidNdkToolchain 方法的单元测试
- T023：用测试项目进行人工验证
