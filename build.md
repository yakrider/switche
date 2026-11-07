
Switche is now a single, Windows-only Rust application built on [GPUI](https://gpui.rs).
The old scala-js + Tauri frontend has been retired; the retained `switche-front` folder is
only kept around temporarily for reference and is not part of the build.

#### Build

From the repository root:

```
cargo build --manifest-path switche-back/Cargo.toml
```

For a release build, note that GPUI compiles its HLSL shaders via `fxc.exe`; if it isn't
found automatically, point `GPUI_FXC_PATH` at the Windows SDK's `fxc.exe`.

#### Run

```
switche-back/target/debug/switche.exe
```

The app starts hidden in the tray-less background; invoke it with the configured hotkey
(F1 by default). Configs live in `switche.conf.toml` next to the executable if that location
is writeable, else in `%LOCALAPPDATA%\Switche\switche.conf.toml`.

#### Dev conveniences

- `SWITCHE_SHOW_ON_START=1` — show the window immediately on startup
- `SWITCHE_SKIP_SINGLE_INSTANCE=1` — allow a dev build to run while the installed app is up
