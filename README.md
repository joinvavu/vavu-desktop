# VAVU desktop app (Windows + Mac)

A small, fast desktop app for **VAVU — Virtual Assistant, Value Unlocked**. It opens
[joinvavu.com](https://joinvavu.com) in its own window, so students get the same account,
lessons, Coach VAVU, practice interviews, CV and portfolio as the website, with the VAVU icon
in the Start menu, desktop, Dock and Launchpad.

Download: **https://joinvavu.com/download**

## How it works
- Built with [Tauri 2](https://tauri.app) (uses the computer's own web engine: WebView2 on Windows, WebKit on Mac), so installers are only a few MB.
- Every website update appears in the app right away. App updates (rare) install automatically.
- Google sign-in opens in the computer's normal browser, then returns to the app through a `vavu://` link.
- Links to other websites open in the normal browser. Payments (QR Ph) stay inside the app.
- Downloads (CV, certificates) go to the Downloads folder. Lesson videos and audio are never downloaded as files:
  saved offline lessons stay encrypted inside the app (Offline Learning Mode, see joinvavu/vavu docs/OFFLINE.md).
- Camera and microphone are asked for only on Practice Your Interview.

## Release a new version
1. Bump `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Create a tag `vX.Y.Z` (or run **Actions → Release VAVU desktop → Run workflow**).
3. GitHub builds the Windows `.exe`/`.msi` and the Mac universal `.dmg` and publishes the release.
   joinvavu.com/download picks it up within 10 minutes.

Signing and auto-update keys are optional secrets; see the top of `.github/workflows/release.yml`.
