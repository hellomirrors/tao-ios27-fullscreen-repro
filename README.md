# tao-repro: `fullscreen` window renders black on iOS 27

Minimal reproduction for a [tao](https://github.com/tauri-apps/tao) bug: on iOS 27 a
window created with `fullscreen: true` stays black.

Fix: [tauri-apps/tao#1366](https://github.com/tauri-apps/tao/pull/1366), branch
[`hellomirrors/tao@fix/ios-redundant-setscreen`](https://github.com/hellomirrors/tao/tree/fix/ios-redundant-setscreen).

## What this is

The unmodified `create-tauri-app` vanilla template (tauri 2.12.1, tao 0.37.1,
wry 0.57.0) with two changes:

1. `src-tauri/tauri.conf.json`: `"fullscreen": true` on the main window.
2. `src-tauri/src/lib.rs`: an iOS-only `ios_diag` module that, 3 s after setup, writes
   the UIKit state of the tao window (scene, frames, view hierarchy) to
   `tmp/tao-repro.txt` in the app's data container.

## Run

```sh
cargo tauri ios init
cargo tauri ios build --target aarch64-sim --debug
xcrun simctl boot "iPad Pro 11-inch (M5)"
xcrun simctl install booted src-tauri/gen/apple/build/arm64-sim/tao-repro.app
xcrun simctl launch booted com.example.taorepro
cat "$(xcrun simctl get_app_container booted com.example.taorepro data)/tmp/tao-repro.txt"
```

To check the fix, uncomment the `[patch.crates-io]` section at the end of
`src-tauri/Cargo.toml` and build again.

## Result

iOS 27.0 simulator (24A434), iPad Pro 11-inch (M5), Xcode 27.0 (27A266a):

| configuration | screen |
|---|---|
| `fullscreen: true` | black – `screenshot-fullscreen-true.png` |
| `fullscreen: false` | template page – `screenshot-fullscreen-false.png` |
| `fullscreen: true` + fix | template page – `screenshot-fullscreen-true-patched.png` |

UIKit state (`diag-fullscreen-true.txt` vs. `diag-fullscreen-false.txt`):

- **Same in both:** the window belongs to a foreground-active `UIWindowScene`
  (`scene.windows` contains it, `scene.screen == window.screen`), is not hidden, has a
  full-screen frame; `TaoUIView → WryWebView → WKScrollView → WKContentView` is present
  with full frames, not hidden, alpha 1.
- **Different:** with `fullscreen: true` the window is the key window, while the working
  configurations aren't. The `WKScrollView` also lacks the two `_UITouchPassthroughView`s,
  and `WKContentView` is 1185 instead of 1158 pt high, so the safe-area layout differs too.

The window is therefore not detached from its scene: it is attached and laid out but
never composited. The cause is the unconditional `-[UIWindow setScreen:]` in tao's
`create_window`. For `Fullscreen::Borderless(None)`, the target is the window's own
screen. Skipping the call when the screen doesn't change fixes it.
