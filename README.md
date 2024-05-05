

### Switche - <small>A Searchable Task Switcher</small>

Switche is designed to be a fast, ergonomic, and search-first task switcher.

Configs are stored in a **switche.conf.toml** which can be opened via the menu. The default config file has reasonable settings and explanations for each config.

Some other key features are explained below:

#### Search
- Starting to type will auto-activate search mode and filtering
- Partial words can be entered separated by spaces, and will be matched separately
- Search matching operates on the exe filename, window title, and the z-index shown
- Escape exits search mode, Ctrl-Space activates selected item during search mode
- Other general navigation hotkeys as listed below work in search mode as well

#### Invocation options
- Hotkeys to invoke Switche can be changed in configs, default and recommended is F1
- Alternately, can use Alt-Tab combo, or use mouse-wheel while holding mouse-right-btn
- If so, releasing the Alt key or the right-button will activate the selected window
- The Release-Armed indicator Ⓡ lights up when that is the case.
- To disarm the state, press S, L, or Space key, or click empty space in the window

#### General Hotkeys:
- In the menu dropdown: Ctrl+R: Refresh, F5: Reload, Ctrl+G: Group Mode, Alt+F4: Quit
- Note that Refresh re-queries the windows list, and updates icons.
- Reload will additionally reload the UI, reset kbd/mose hooks, and reload configs.
- Enter, Space, Mouse-Click : Activate the window currently selected in switche list
- Esc : Escape out of search-mode if active, or else from switche window
- Tab, Arrow keys, PgUp, PgDown : Navigate the list of windows
- Alt + [ I, J, K, M, U, Comma ] : Navigate the list of windows
- Ctrl+W, mouse middle-click : Close the selected window
- Ctrl+P : Peek at the selected window for a few seconds
- Other hotkeys for direct switching to specific-applications can be setup in configs.

#### Windows Listings
- *Recents* section shows a subset of windows ordered by most recently in foreground
- The number shown here is configurable, but the minimum is 2 for quick switching
- The intended focus is on the *Grouped* section where windows are organized by exe
- The ordering of the grouped section can be set to auto or specified in configs
- Auto-ordering is based on a running tally of the average z-index of each group
- Windows can be specified in configs by exe and/or title to be excluded from the lists.

#### Special Considerations
- When not elevated/as-admin, Switche can not close/switch-to elevated app windows.
- Running elevated is recommended. The Elevation indicator serves as visual reminder.
- Switche system tray menu can setup auto-start at login (as elevated or normal user).
- The option for Auto-Start-as-Admin is disabled when not running elevated/as-admin.
- Clearing prior configs in Windows Task Scheduler can help with auto-start issues.
- Other apps using lower level kbd hooks can prevent Switche from receiving Alt-Tab.
- Mutiple Switche instances running simultaneously can lead to unexpected behavior.

Repository: https://github.com/yakrider/switche

Copyright @ 2024 yakrider  
Distributed under MIT license

---
Screenshot of Switche v2.2.7 :

<img width="721" src="https://github.com/yakrider/switche/assets/15984611/947d55f8-064b-48f9-a772-d9214ee8f8bb">


