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
      function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) { process.send(ref.address(hwnd)) }
   )
   const pfnWinEventProc_objKill = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
      function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
        // obj-id 0x0000 is for window.. others can be for parts of it like caret etc, 0x8001 is obj-destroyed event, 0x800C is obj-name-changed
        if (event===0x8001 & idObject===0) {
            process.send({type:'ObjDestroyed', hwnd:ref.address(hwnd)});
        } else if (event===0x800C && idObject===0) {
            process.send({type:'TitleChanged', hwnd:ref.address(hwnd)});
        }
      }
   )
   if (process.env.task=='foreground-window') { //console.log('foreground-window worker reporting!..')
      user32.SetWinEventHook(3, 3, null, pfnWinEventProc_fgnd, 0, 0, 0 )
   } else if (process.env.task=='object-destroyed') { //console.log('obj-destroyed worker reporting!..')
      user32.SetWinEventHook(0x8001, 0x800C, null, pfnWinEventProc_objKill, 0, 0, 0 )
   }
   // winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
   function getMessage() { return user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
   while (0 != getMessage()) {}
}
// since worker above will be on a forever loop, the code below doesnt need to be on an 'else' block

var foregroundWindowWorker = cluster.fork ({task:'foreground-window'})
var objDestroyedWorker = cluster.fork ({task:'object-destroyed'})

//worker.on('message', function(msg) {console.log('got msg from worker: ', msg)})


// get the index of the first command line argument
const argv = process.argv.slice(
   process.argv.findIndex((arg) => !path.relative(arg, __filename)) + 1
);

// Module to control application life.
const app = electron.app;
const remote = electron.remote;
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
      width: 1050, height: 1400,
      x: 1000, y:20,
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
      mainWindow.loadURL(`file://${__dirname}/web/index-dev.html`);
   } else if (argv[0] === 'dev-server') {
      mainWindow.setSize(1850,1400);
      mainWindow.frame = true;
      mainWindow.webContents.openDevTools();
      //mainWindow.loadURL(`file://${__dirname}/web/index-dev.html`);
      //mainWindow.loadURL('http://localhost:8080/target/scala-2.12/scalajs-bundler/main/index-dev.html');
      mainWindow.loadURL('http://localhost:8080/web/index-dev-server.html')
   } else {
      mainWindow.loadURL(`file://${__dirname}/web/index.html`);
   }

   // Emitted when the window is closed.
   mainWindow.on('closed', () => {
      // Dereference the window object, usually you would store windows
      // in an array if your app supports multi windows, this is the time
      // when you should delete the corresponding element.
      mainWindow = null;
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

    mainWindow.on('focus',() => { mainWindow.webContents.executeJavaScript('window.handleElectronFocusEvent()') });
    mainWindow.on('blur', () => { mainWindow.webContents.executeJavaScript('window.handleElectronBlurEvent()') });
    mainWindow.on('show', () => { mainWindow.webContents.executeJavaScript('window.handleElectronShowEvent()') });
    mainWindow.on('hide', () => { mainWindow.webContents.executeJavaScript('window.handleElectronHideEvent()') });

}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
//
app.on('ready', () => {
   createWindow()
   globalShortcut.register ('F1', () => {hotkeyHandler()})
   globalShortcut.register ('Alt+1', () => {hotkeyHandler()})
   //globalShortcut.register ('Esc', () => {mainWindow.hide()})
   // lol ^ cant do that.. lots of ppl need Esc.. gonna have to handle it from inside window, not globally
   //globalShortcut.register ('F2', () => {hotkeyReverseHandler()}) // ofc cant do this either, hence the following
   globalShortcut.register ('Ctrl+Alt+F1', () => {hotkeyGlobalScrollDownHandler()})
   globalShortcut.register ('Ctrl+Alt+F2', () => {hotkeyGlobalScrollUpHandler()})
   globalShortcut.register ('Ctrl+Alt+F3', () => {hotkeyGlobalScrollEndHandler()})
})


function hotkeyHandler() {
   //mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyCall()', function(result){console.log(result)})
   mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyCall()')
   mainWindow.show()
}
function hotkeyGlobalScrollDownHandler() {
   mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyGlobalScrollDownCall()')
   mainWindow.show()
}
function hotkeyGlobalScrollUpHandler() {
   mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyGlobalScrollUpCall()')
   mainWindow.show()
}
function hotkeyGlobalScrollEndHandler() {
   mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyGlobalScrollEndCall()')
}

  
// Quit when all windows are closed.
app.on('window-all-closed', () => {
   // On OS X it is common for applications and their menu bar
   // to stay active until the user quits explicitly with Cmd + Q.
   if (process.platform !== 'darwin') {
      app.quit();
   }
});

// On OS X it's common to re-create a window in the app when the
// dock icon is clicked and there are no other windows open.
//
app.on('activate', function () {
   if (mainWindow === null) {
      createWindow();
   }
});


// setup to send window activation calls from worker thread setup above to webapp
//worker.on('message', function(msg) {console.log('got msg from worker: ', msg)})
foregroundWindowWorker.on('message', function(hwnd) { mainWindow.webContents.executeJavaScript(`window.handleWindowsFgndHwndReport(${hwnd})`) })
//objDestroyedWorker.on('message', function(hwnd) { mainWindow.webContents.executeJavaScript(`window.handleWindowsObjDestroyedReport(${hwnd})`) })
objDestroyedWorker.on('message', function(msg) {
   if (msg.type=='ObjDestroyed') {
      mainWindow.webContents.executeJavaScript(`window.handleWindowsObjDestroyedReport(${msg.hwnd})`) 
   } else if (msg.type=='TitleChanged') {
      mainWindow.webContents.executeJavaScript(`window.handleWindowsTitleChangedReport(${msg.hwnd})`) 
   }
})

