const electron = require('electron');
const path = require('path');



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
    mainWindow = new BrowserWindow({
        icon: `web/favicon.png`,
        width: 1000, height: 1400,
        x: 1200, y:20,
        frame: true,
        useContentSize: true,
        resizable: true,
        fullscreen: false,
        autoHideMenuBar: true,
        webgl: true,
        skipTaskbar: true,
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
        //
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
})
//app.on('ready', quickTest);
//app.on('ready', inclTest);
//app.on('ready', tests);

function hotkeyHandler() {
    //console.log('electron global hotkey pressed!')
    //mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyCall()', function(result){console.log(result)})
    mainWindow.webContents.executeJavaScript('window.handleElectronHotkeyCall()')
    mainWindow.show()
}

function tests() {
    quickTest();
    inclTest();
    // note that while the elec main in sjs runs, that outputs to the window... so gotta create that to test from sjs
    //createWindow();    
    //app.quit();
}


function quickTest() {
    console.log("direct js hello");
    //app.quit();
}

function inclTest() {
    var wapiTest = require('./win-helper');
    console.log(wapiTest.hello());  

    //wapiTest.printVisibleWindows();

    //app.quit();  
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


