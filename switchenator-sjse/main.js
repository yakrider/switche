const electron = require('electron');
const path = require('path');

const cluster = require("cluster")

if (!cluster.isMaster) {
   const ffi = require("ffi")
   const ref = require("ref")

   //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
   const user32 = ffi.Library("user32", {
      SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]],
      GetMessageA: ["bool", ['pointer', "int", "uint", "uint"]]
   })
   //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )
   const pfnWinEventProc_fgnd = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
      function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) { 
        if (idObject===0) {
           if        (event===0x0003) {
              process.send({type:'Fgnd', hwnd:ref.address(hwnd)});
           } else if (event===0x0017) {
              process.send({type:'MinimizeEnd', hwnd:ref.address(hwnd)});
           }
        }        
      }
   )
   const pfnWinEventProc_objKill = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
      function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
        // obj-id 0x0000 is for window.. others can be for parts of it like caret etc, 0x8001 is obj-destroyed event, 0x800C is obj-name-changed
        if (idObject===0) {
           if        (event===0x8001) {
              process.send({type:'ObjDestroyed', hwnd:ref.address(hwnd)});
           } else if (event===0x8002) {
              process.send({type:'ObjShown', hwnd:ref.address(hwnd)});
           } else if (event===0x8003) {
              process.send({type:'ObjHidden', hwnd:ref.address(hwnd)});
           } else if (event===0x800C) {
              process.send({type:'TitleChanged', hwnd:ref.address(hwnd)});
           } else if (event===0x8017) {
              process.send({type:'ObjCloaked', hwnd:ref.address(hwnd)});
           } else if (event===0x8018) {
              process.send({type:'ObjUnCloaked', hwnd:ref.address(hwnd)});
           }
        }
      }
   )

   // we'll put these in two separate hooks because the spread of events we want to listen to are in two different ranges
   // having separate hooks avoids pointless calls for events that'd end up within the wider range a single hook would have to cover
   // otoh, we can process both hook callbacks in this thread (i.e no need to have separate cluster forks each setting/listening to a hook each)
   user32.SetWinEventHook (0x0003, 0x0017, null, pfnWinEventProc_fgnd, 0, 0, 0 );
   user32.SetWinEventHook (0x8001, 0x8018, null, pfnWinEventProc_objKill, 0, 0, 0 );

   // winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
   function getMessage() { return user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
   while (0 != getMessage()) {}
}
// since worker above will be on a forever loop, the code below doesnt need to be on an 'else' block


var sysEventsWorker = cluster.fork ({task:'sys-events'})

//worker.on('message', function(msg) {console.log('got msg from worker: ', msg)})


// get the index of the first command line argument
const argv = process.argv.slice(
   process.argv.findIndex((arg) => !path.relative(arg, __filename)) + 1
);

// Module to control application life.
const app = electron.app;
//const remote = electron.remote;
const shell = electron.shell;
const globalShortcut = electron.globalShortcut;
const BrowserWindow = electron.BrowserWindow;

// Keep a global reference of the window object, if you don't, the window will
// be closed automatically when the JavaScript object is garbage collected.
//
var mainWindow;

// Create the browser window.
function createWindow() {

   var frameVal = (argv[0]==='dev' || argv[0]==='dev-server') ? true : false

   mainWindow = new BrowserWindow({
      icon: `web/favicon.png`,
      width: 980, height: 1180,
      x: 650, y:15,
      frame: frameVal,
      thickFrame : true,
      useContentSize: true,
      resizable: true,
      fullscreen: false,
      autoHideMenuBar: true,
      webgl: true,
      skipTaskbar: true,
      backgroundThrottling: false
   });

   // Disable the default menu bar.
   mainWindow.setMenu(null);

   // Open the DevTools in debug mode.
   if (argv[0] === 'dev') {
      mainWindow.setSize(1850,1400);
      mainWindow.frame = true;
      //mainWindow.webContents.openDevTools({ mode: "detach" });
      mainWindow.webContents.openDevTools();
      global.inDevMode = true;
      mainWindow.loadURL(`file://${__dirname}/web/index-dev.html`);
   } else if (argv[0] === 'dev-server') {
      mainWindow.setSize(1850,1400);
      mainWindow.frame = true;
      mainWindow.webContents.openDevTools();
      global.inDevMode = true;
      //mainWindow.loadURL(`file://${__dirname}/web/index-dev.html`);
      //mainWindow.loadURL('http://localhost:8080/target/scala-2.12/scalajs-bundler/main/index-dev.html');
      mainWindow.loadURL('http://localhost:8080/web/index-dev-server.html')
   } else {
      global.inDevMode = false;
      mainWindow.loadURL(`file://${__dirname}/web/index.html`);
   }

   // Emitted when the window is closed.
   mainWindow.on('closed', () => {
      // Dereference the window object, usually you would store windows
      // in an array if your app supports multi windows, this is the time
      // when you should delete the corresponding element.
      mainWindow = null;
      app.quit();  // since we're win-only single-window app, can just exit here
   });

   // Handle navigation to another link.
   let handleRedirect = (e, url) => {
      if (url != mainWindow.webContents.getURL()) {
         e.preventDefault();
         // Launch a new browser window.
         shell.openExternal(url);
      }
   }

    // Open URLs in the desktop browser.
    mainWindow.webContents.on('will-navigate', handleRedirect);
    mainWindow.webContents.on('new-window', handleRedirect);

    mainWindow.on('focus',() => { mainWindow.webContents.executeJavaScript('window.procAppEvent_Focus()') });
    mainWindow.on('blur', () => { mainWindow.webContents.executeJavaScript('window.procAppEvent_Blur()') });
    mainWindow.on('show', () => { mainWindow.webContents.executeJavaScript('window.procAppEvent_Show()') });
    mainWindow.on('hide', () => { mainWindow.webContents.executeJavaScript('window.procAppEvent_Hide()') });

}

// This method will be called when Electron has finished initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.on('ready', () => {
   createWindow()
   setupGlobalShortcuts()
   if (true == global.inDevMode) { mainWindow.webContents.executeJavaScript('window.procAppEvent_DevMode()') }
})

function setupGlobalShortcuts() {
   //globalShortcut.register ('F1', () => {sysHotkeyHandler_Invoke()})
   //globalShortcut.register ('Alt+1', () => {sysHotkeyHandler_Invoke()})
   //globalShortcut.register ('Super+F1', () => {sysHotkeyHandler_Invoke()})
   // ^^ disabling these options on F1 since we're having krusty send F21 (and have ralt-f1 send f1)

   // but even though we'll set mostly Fn[13+] keys via krusty for F1, we'll also add win-f12 so we're not fully reliant on krusty remappings
   globalShortcut.register ('Super+F12',       () => {sysHotkeyHandler_ScrollDown()})
   globalShortcut.register ('Super+Shift+F12', () => {sysHotkeyHandler_ScrollUp()})

   //globalShortcut.register ('Esc', () => {mainWindow.hide()})
   // lol ^ ofc cant do that.. lots of ppl need Esc.. so we'll handle it from inside window, not globally
   //globalShortcut.register ('F2', () => {hotkeyReverseHandler()}) // ofc cant do this either, hence the following

   // NOTE that combo-fn keys like Ctrl-FX or Alt-FX arent ideal here, esp if those are actually being sent via krusty etc ..
   // .. as that seems to cause slowdown and choppy nav etc (due to the wrapped ctrl etc from krusty, let alone the masking keys if alt!)
   // .. and esp for for scrolls, we'll want best key-repeat perf, so we'll use direct Fn keys
   //globalShortcut.register ( 'F15',      () => { sysHotkeyHandler("Invoke", true) } )      // not essential, we'll just use scroll-down for invocation
   globalShortcut.register ( 'F16',        () => { sysHotkeyHandler("ScrollDown", true) } )  // global invoker (kbd and right-btn-mouse-fwd)
   globalShortcut.register ( 'F17',        () => { sysHotkeyHandler("ScrollUp", true) } )    // global invoker for right-btn-mouse-backward
   //globalShortcut.register ('Shift+F16', () => { sysHotkeyHandler("ScrollUp", true) } )    // for krusty kbd hotkey, this eliminates wrapping shift-ups
   // (^^ but even better, we'll just handle that locally once switche is up rather that registering it globally to invoke switche from here)
   // while for the others, key-repeat perf is irrelevant, so we'll use Ctrl-Fns to not hog up Fn keys for any other apps etc
   globalShortcut.register ( 'Ctrl+F18', () => { sysHotkeyHandler("ScrollEnd", false) } )

   globalShortcut.register ( 'Alt+Ctrl+F19', () => { sysHotkeyHandler("Switch_Last") } )
   globalShortcut.register ( 'Alt+Ctrl+F20', () => { sysHotkeyHandler("Switch_TabsOutliner") } )
   globalShortcut.register ( 'Alt+Ctrl+F21', () => { sysHotkeyHandler("Switch_NotepadPP") } )
   globalShortcut.register ( 'Alt+Ctrl+F22', () => { sysHotkeyHandler("Switch_IDEA") } )
   globalShortcut.register ( 'Alt+Ctrl+F23', () => { sysHotkeyHandler("Switch_Winamp") } )
   globalShortcut.register ( 'Alt+Ctrl+F24', () => { sysHotkeyHandler("Switch_Browser") } )
}

function sysHotkeyHandler (fnStr, doShow) {
   mainWindow.webContents.executeJavaScript(`window.procHotkey_${fnStr}()`)
   if (doShow) mainWindow.show()
}


// setup to send window activation calls from worker thread setup above to webapp
sysEventsWorker.on('message', function(msg) {
   if (msg.type=='Fgnd' || msg.type=='MinimizeEnd') {
      mainWindow.webContents.executeJavaScript(`window.procWinReport_FgndHwnd(${msg.hwnd})`)
   } else if (msg.type=='ObjDestroyed' || msg.type=='ObjHidden' || msg.type=='ObjCloaked') {
      mainWindow.webContents.executeJavaScript(`window.procWinReport_ObjDestroyed(${msg.hwnd})`)
   } else if (msg.type=='ObjShown' || msg.type=='ObjUnCloaked') {
      mainWindow.webContents.executeJavaScript(`window.procWinReport_ObjShown(${msg.hwnd})`)
   } else if (msg.type=='TitleChanged') {
      mainWindow.webContents.executeJavaScript(`window.procWinReport_TitleChanged(${msg.hwnd})`)
   }
})

