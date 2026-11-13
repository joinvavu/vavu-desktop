//! VAVU desktop app: one window that opens joinvavu.com like a real app.
//! - Same account, data and features as the website (it *is* the website).
//! - Google sign-in opens in the computer's normal browser (Google blocks embedded windows),
//!   then comes back through the vavu:// link and finishes inside the app.
//! - Links to other sites open in the normal browser; payments stay inside.
//! - Downloads (CV, certificates) go to the Downloads folder. Lesson videos and audio are never downloaded as files.
//! - Checks for app updates on start and installs them.

use std::path::PathBuf;
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

const HOME: &str = "https://joinvavu.com";
const START: &str = "https://joinvavu.com/dashboard?source=desktop";

#[cfg(target_os = "windows")]
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";
#[cfg(target_os = "macos")]
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Pages that stay inside the VAVU window. Everything else opens in the normal browser.
fn stays_in_app(url: &Url) -> bool {
    match url.scheme() {
        "https" => {}
        "about" | "blob" | "data" | "tauri" | "asset" => return true,
        "http" => return url.host_str() == Some("localhost") || url.host_str() == Some("tauri.localhost"),
        _ => return false,
    }
    let host = url.host_str().unwrap_or("");
    host == "joinvavu.com"
        || host == "www.joinvavu.com"
        // Payments (QR Ph and cards) finish inside the app.
        || host == "checkout.xendit.co"
        || host.ends_with(".xendit.co")
        || host == "checkout.paymongo.com"
        || host.ends_with(".payrexhq.com")
        || host.ends_with(".payrex.com")
        // Lesson video players.
        || host.ends_with(".mediadelivery.net")
        || host == "www.youtube-nocookie.com"
        || host == "player.vimeo.com"
}

fn main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
}

fn focus(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// vavu://auth?code=…&next=/dashboard → finish Google sign-in inside the app window.
fn handle_deep_link(app: &AppHandle, url: &Url) {
    if url.scheme() != "vavu" {
        return;
    }
    let route = url.host_str().unwrap_or("");
    let mut target = Url::parse(START).unwrap();
    if route == "auth" {
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        if let Some(code) = q.get("code") {
            let next = q
                .get("next")
                .filter(|n| n.starts_with('/') && !n.starts_with("//"))
                .cloned()
                .unwrap_or_else(|| "/dashboard".into());
            target = Url::parse(&format!("{HOME}/auth/callback")).unwrap();
            target.query_pairs_mut().append_pair("code", code).append_pair("next", &next);
        }
    } else if route == "open" {
        // vavu://open/learn/… → that page inside the app.
        if let Ok(u) = Url::parse(&format!("{HOME}{}", url.path())) {
            target = u;
        }
    }
    if let Some(w) = main_window(app) {
        let _ = w.navigate(target);
    }
    focus(app);
}

/// Lesson media never leaves the app as a file: saved offline lessons, lesson videos and audio play only inside VAVU.
/// Blocks "Save video as…" and any download of lesson files (CVs, certificates and other documents still download).
fn is_lesson_media(url: &Url) -> bool {
    if matches!(url.scheme(), "blob" | "data") {
        return true;
    }
    let host = url.host_str().unwrap_or("");
    let path = url.path().to_ascii_lowercase();
    path.starts_with("/offline-media/")
        || path.starts_with("/api/offline/")
        || host.ends_with(".b-cdn.net")
        || host.ends_with(".mediadelivery.net")
        || [".mp4", ".m4a", ".m4v", ".m3u8", ".ts", ".webm", ".mov", ".aac", ".mp3"].iter().any(|ext| path.ends_with(ext))
}

fn file_name(path: &PathBuf) -> String {
    path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".into())
}

async fn check_for_update(app: AppHandle) {
    let Ok(updater) = app.updater() else { return };
    let Ok(Some(update)) = updater.check().await else { return };
    let version = update.version.clone();
    if update.download_and_install(|_, _| {}, || {}).await.is_err() {
        return;
    }
    // Windows: the installer closes and reopens the app by itself. Mac: ask to restart now.
    let ask = app
        .dialog()
        .message(format!("VAVU {version} is ready. Restart now to use the latest version?"))
        .title("VAVU update")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom("Restart now".into(), "Later".into()));
    let handle = app.clone();
    ask.show(move |restart| {
        if restart {
            handle.restart();
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch (or a vavu:// link) focuses the open window instead.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| focus(app)))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // vavu:// links (Google sign-in handoff).
            #[cfg(any(windows, target_os = "linux"))]
            {
                let _ = app.deep_link().register_all();
            }
            let h = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    handle_deep_link(&h, &url);
                }
            });

            let nav = handle.clone();
            let dl = handle.clone();
            let version = app.package_info().version.to_string();
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(Url::parse(START).unwrap()))
                .title("VAVU")
                .inner_size(1280.0, 820.0)
                .min_inner_size(380.0, 600.0)
                .center()
                .background_color(tauri::window::Color(18, 20, 43, 255))
                .user_agent(&format!("{UA} VAVUDesktop/{version}"))
                .on_navigation(move |url| {
                    if stays_in_app(url) {
                        true
                    } else {
                        let _ = nav.opener().open_url(url.as_str(), None::<&str>);
                        false
                    }
                })
                .on_download(move |_webview, event| {
                    match event {
                        DownloadEvent::Requested { url, destination } => {
                            if is_lesson_media(&url) {
                                return false;
                            }
                            // Save into the Downloads folder with the site's file name.
                            if destination.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true) {
                                if let Ok(dir) = dl.path().download_dir() {
                                    *destination = dir.join(file_name(destination));
                                }
                            }
                        }
                        DownloadEvent::Finished { path, success, .. } => {
                            if success {
                                let name = path.as_ref().map(file_name).unwrap_or_default();
                                let _ = dl.notification().builder().title("Download complete").body(format!("{name} is in your Downloads folder.")).show();
                            }
                        }
                        _ => {}
                    }
                    true
                })
                .build()?;
            let _ = window.set_focus();

            // Updates: check once at start (quietly; nothing happens if there is no new version).
            tauri::async_runtime::spawn(check_for_update(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VAVU");
}
