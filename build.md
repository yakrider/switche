
Switche is a single, Windows-only Rust application built on [GPUI](https://gpui.rs).
The old scala-js + Tauri frontend has been fully retired.

#### Build

From the repository root:

```
cargo build --manifest-path switche-back/Cargo.toml
```

For a release build, note that GPUI compiles its HLSL shaders via `fxc.exe`; if it isn't
found automatically, point `GPUI_FXC_PATH` at the Windows SDK's `fxc.exe`.

#### Run

```
switche-back/target/debug/Switche.exe
```

The app starts in the background (tray icon, no main window); invoke it with the configured
hotkey (F1 by default). Configs live in `switche.conf.toml` next to the executable if that
location is writeable, else in `%LOCALAPPDATA%\Switche\switche.conf.toml`.

#### Installer

From the repository root (stop any running release build first, since Windows locks the exe):

```
powershell -NoProfile -ExecutionPolicy Bypass -File installer\build-installer.ps1
```

(or use the `installer` run configuration in RustRover). This builds the release binary and
packages it with NSIS into `dist\Switche-Setup-<version>.exe`. The installer is per-user (no
admin prompt) and installs to `%LOCALAPPDATA%\Switche` — the same directory the old Tauri
installer used — so upgrading keeps existing configs and logs. It creates a Start Menu
shortcut, an optional desktop shortcut, and the HKCU uninstall entry. Uninstalling removes the
app, its shortcuts and the registry entry, but deliberately leaves `switche.conf.toml` and log
files in place.

Silent install/uninstall is supported via `/S`, with an optional target dir as the last
parameter (e.g. `Switche-Setup-4.0.0.exe /S /D=C:\path\to\dir`).

Requires NSIS (`makensis` on `PATH`, or at `C:\Program Files (x86)\NSIS`).

#### Debug vs release behavior

There are no environment-variable switches; the build profile decides:

- debug builds show the window on startup, skip the single-instance check (so they can run
  alongside an installed release instance), and use a 60ms navigation interval
- release builds start hidden in the tray, enforce a single instance, and use 30ms navigation

`dev.bat` stops any running debug instance, rebuilds, and launches it.
