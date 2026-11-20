# 构建第三方 CMake 库

通过 `[build.cmake]` 直接接入上游 CMake 项目，无需修改上游源码或在工程根目录编写 `CMakeLists.txt`：

```toml
[package]
name = "opencv2"
version = "4.10.0"

[build]
cmake_file = "cmake/opencv-common.cmake"

[build.cmake]
source_dir = "libs_build/opencv"
target = "opencv_world"
install_include_dir = "include/opencv4"
arguments = ["-DBUILD_opencv_world=ON", "-DBUILD_TESTS=OFF", "-DBUILD_PERF_TESTS=OFF"]

[platforms.macos.build]
cmake_file = "cmake/opencv-macos.cmake"

[platforms.android.build.cmake]
arguments = ["-DBUILD_ANDROID_PROJECTS=OFF", "-DBUILD_ANDROID_EXAMPLES=OFF"]
```

`source_dir` 相对工程根目录解析，也支持绝对路径；`target` 是上游实际生成的静态库或动态库 target（支持 alias）。`install_include_dir` 相对 CMake install prefix，默认 `include`；根据上游实际安装布局调整它。三个字段均支持平台覆盖，平台值替换全局值，参数列表仍按全局、平台顺序追加。

全局 `cmake_file`、平台 `cmake_file` 按顺序在上游 `add_subdirectory` 前执行，适合设置各平台不同的模块、后端和编译选项。未配置时自动加载根目录 `CCGO.cmake`；空字符串关闭加载。ccgo 将 `--build-as` 映射为 `BUILD_SHARED_LIBS`，但上游必须遵循此变量，或通过扩展脚本配置其自定义开关。

```sh
ccgo build macos --arch arm64 --release --build-as static
ccgo build android --arch arm64-v8a --release --build-as shared --native-only
```

ccgo 在构建目录生成适配层，只构建指定库及其依赖；动态库使用 package 的库名输出，静态库沿 CMake target 链接关系合并依赖静态库。执行上游 install 后，将公共头文件刷新到当前平台构建目录的 `upstream-include/` 并用于 SDK 打包；显式 `[include].src` 可覆盖打包来源。上游应提供库和头文件的 install 规则。不同架构的头文件应兼容；最终 SDK 使用最后安装架构的头文件。

源码获取、版本锁定、补丁和模块裁剪继续由项目维护。升级 OpenCV 时更新源码 tag 和 package version 即可复用构建路径；第三方版本号不会由 ccgo 自动校验。Apple 静态库需按上游要求关闭 LTO，以便生成 XCFramework。

适配器适用于可通过 `add_subdirectory` 集成的 CMake 项目。静态合并支持普通 target 依赖及 `LINK_ONLY`、`BUILD_INTERFACE` 包装；复杂生成器表达式和绝对路径 archive 依赖会报错，避免生成缺少符号的 SDK。系统库和独立动态库不会被合并，消费者仍需显式链接、分发它们。依赖特殊顶层目录假设、复杂链接策略或额外资源的项目，可继续使用自定义根 `CMakeLists.txt` 适配。
