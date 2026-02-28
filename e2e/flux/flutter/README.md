# twitter_flux (e2e/flux/flutter)

Flutter 端的 Flux 示例应用（Cupertino UI），可通过 Dart FFI 对接 Rust `flux_ffi`。

## 环境要求

- Flutter SDK（建议与仓库当前版本一致）
- Android SDK + 模拟器（本项目当前主要走 Android）
- Bazel（用于编译 Rust FFI 动态库）

## 编译与运行

### 1) 安装 Flutter 依赖

在本目录执行：

```bash
flutter pub get
```

### 2) 编译 Rust FFI 动态库（推荐）

在仓库根目录执行：

```bash
bazel build //rust/lib/flux_ffi:flux_ffi_dylib
```

> 产物位于：`bazel-bin/rust/lib/flux_ffi/libflux_ffi.dylib`

### 3) 启动 Android（调试运行）

回到本目录执行：

```bash
flutter run -d emulator-5554
```

若你本机设备 ID 不同，可先查看：

```bash
flutter devices
```

然后替换 `-d` 参数。

## 构建 APK

```bash
flutter build apk
```

构建输出通常在：

`build/app/outputs/flutter-apk/app-release.apk`

## 测试

### 单元/组件测试

```bash
flutter test
```

### 集成测试（Android）

```bash
flutter test integration_test -d emulator-5554
```

## 常见问题

1. **找不到设备**
   - 先执行 `flutter devices`，确认模拟器已启动。

2. **FFI 动态库未生成**
   - 先在仓库根目录执行 Bazel build：
     `bazel build //rust/lib/flux_ffi:flux_ffi_dylib`

3. **依赖异常**
   - 重新执行：`flutter clean && flutter pub get`
