// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let win = app.get_webview_window("main").unwrap();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let w = win.clone();
                    win.run_on_main_thread(move || ios_diag::log(&w)).unwrap();
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "ios")]
mod ios_diag {
    use objc2::msg_send;
    use objc2::runtime::{AnyObject, Bool};
    use objc2::encode::{Encode, Encoding};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGRect(f64, f64, f64, f64);
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding = Encoding::Struct(
            "CGRect",
            &[
                Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]),
                Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]),
            ],
        );
    }

    /// Logs whether the tao `UIWindow` is attached to a `UIWindowScene`.
    pub fn log(win: &tauri::WebviewWindow) {
        let RawWindowHandle::UiKit(h) = win.window_handle().unwrap().as_raw() else {
            return;
        };
        unsafe {
            let view = h.ui_view.as_ptr() as *mut AnyObject;
            let window: *mut AnyObject = msg_send![view, window];
            let scene: *mut AnyObject = msg_send![window, windowScene];
            let screen: *mut AnyObject = msg_send![window, screen];
            let hidden: Bool = msg_send![window, isHidden];
            let key: Bool = msg_send![window, isKeyWindow];
            let frame: CGRect = msg_send![window, frame];
            let vframe: CGRect = msg_send![view, frame];
            let (mut sstate, mut sscreen, mut swins, mut contains) = (-9isize, std::ptr::null_mut::<AnyObject>(), 0usize, false);
            if !scene.is_null() {
                sstate = msg_send![scene, activationState];
                sscreen = msg_send![scene, screen];
                let wins: *mut AnyObject = msg_send![scene, windows];
                swins = msg_send![wins, count];
                let c: Bool = msg_send![wins, containsObject: window];
                contains = c.as_bool();
            }
            let sbounds: CGRect = msg_send![screen, bounds];
            let root: *mut AnyObject = msg_send![window, rootViewController];
            let rview: *mut AnyObject = if root.is_null() { root } else { msg_send![root, view] };
            let line = format!(
                "TAO-REPRO fullscreen={:?}\n window={window:?} hidden={} key={} frame={:?}\n window.screen={screen:?} bounds={:?}\n scene={scene:?} activationState={sstate} scene.screen={sscreen:?} scene.windows={swins} containsWindow={contains}\n rootVC={root:?} rootVC.view={rview:?} tauriView={view:?} viewFrame={:?}",
                win.is_fullscreen().ok(), hidden.as_bool(), key.as_bool(),
                (frame.0, frame.1, frame.2, frame.3), (sbounds.0, sbounds.1, sbounds.2, sbounds.3),
                (vframe.0, vframe.1, vframe.2, vframe.3)
            );
            let mut line = line;
            fn dump(v: *mut AnyObject, depth: usize, out: &mut String) {
                unsafe {
                    let cls: *mut AnyObject = msg_send![v, class];
                    let name: *mut AnyObject = msg_send![cls, description];
                    let cstr: *const std::ffi::c_char = msg_send![name, UTF8String];
                    let f: CGRect = msg_send![v, frame];
                    let h: Bool = msg_send![v, isHidden];
                    let a: f64 = msg_send![v, alpha];
                    let layer: *mut AnyObject = msg_send![v, layer];
                    let lh: Bool = msg_send![layer, isHidden];
                    out.push_str(&format!(
                        "\n {}{} {v:?} frame=({},{},{},{}) hidden={} alpha={a} layerHidden={}",
                        "  ".repeat(depth),
                        std::ffi::CStr::from_ptr(cstr).to_string_lossy(),
                        f.0, f.1, f.2, f.3, h.as_bool(), lh.as_bool()
                    ));
                    if depth < 6 {
                        let subs: *mut AnyObject = msg_send![v, subviews];
                        let n: usize = msg_send![subs, count];
                        for i in 0..n {
                            let sv: *mut AnyObject = msg_send![subs, objectAtIndex: i];
                            dump(sv, depth + 1, out);
                        }
                    }
                }
            }
            dump(window, 0, &mut line);
            println!("{line}");
            std::fs::write(std::env::temp_dir().join("tao-repro.txt"), line).unwrap();
        }
    }
}
