# Building third-party CMake libraries

Use an upstream source tree directly without changing its files or writing a root `CMakeLists.txt`:

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
```

`source_dir` resolves relative to the project root (absolute paths also work). `target` names the upstream static or shared library, including aliases. `install_include_dir` resolves inside the install prefix and defaults to `include`; choose the directory matching upstream's actual layout. Platform configuration replaces these scalar fields, while argument lists append as usual.

Global and platform `cmake_file` scripts run before upstream configuration, in that order. Without explicit configuration, the root `CCGO.cmake` is discovered; an empty string disables loading. Use these scripts for platform-specific backends and module selection. ccgo maps `--build-as` to `BUILD_SHARED_LIBS`; upstream must honor that variable or expose equivalent switches configured by your scripts.

The generated adapter builds only the selected target and its dependencies. Shared libraries use the package library name. Static libraries merge archives from the target link graph. Upstream install rules must install the libraries and public headers. Headers are refreshed into the platform build directory's `upstream-include/` for SDK packaging; explicit `[include].src` overrides the packaging source. Headers must be compatible across architectures; the last installed architecture supplies the final headers.

Source retrieval, version locking, patches, and module selection remain project responsibilities. Upgrading OpenCV can reuse this configuration after updating its source tag and the package version; ccgo does not verify that these versions match. Disable upstream LTO where required for Apple XCFramework packaging.

The adapter requires an upstream project compatible with `add_subdirectory`. Static dependency traversal supports ordinary CMake targets and `LINK_ONLY`/`BUILD_INTERFACE` wrappers. Unsupported generator expressions and absolute archive dependencies fail explicitly. System libraries and independent shared dependencies are not embedded and must still be linked or distributed by consumers. Projects requiring custom top-level directory behavior, link policies, or resources can continue using a custom root `CMakeLists.txt`.
