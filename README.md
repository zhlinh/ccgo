## ccgo

c++ cross-platform build system used to fasten your development.

## Usage

### 1. install

```
# pypi page is https://pypi.org/project/ccgo/
pip3 install ccgo
```

### 2. create lib

```
ccgo lib create <YOUR_PROJECT_DIR>

# if you want to specify the template, use `--template-url`
ccgo lib create <YOUR_PROJECT_DIR> --template-url=https://github.com/zhlinh/ccgo-template.git --template-url=https://github.com/zhlinh/ccgo-template.git
```

### 3. build

```
# cd to the inner project files dir, 
  then can build any platform with `ccgo build <platform_name>`

# 3.1 Android
ccgo build android [--arch armeabi-v7a,arm64-v8a,x86_64]

# 3.2 iOS
ccgo build ios

# 3.3 macOS
ccgo build macos

# 3.4 windows
ccgo build windows

# 3.5 linux
ccgo build linux

# 3.6 tests, which based on googletest
ccgo build tests
```

## License

ccgo is available under the [MIT license](https://opensource.org/license/MIT).
See the LICENSE file for the full license text.