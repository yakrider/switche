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
const BrowserWindow = electron.BrowserWindow;

// Keep a global reference of the window object, if you don't, the window will
// be closed automatically when the JavaScript object is garbage collected.
//
var mainWindow;

// Create the browser window.
function createWindow() {
    mainWindow = new BrowserWindow({
        icon: `web/favicon.png`,
        width: 1600,
        height: 1200,
        frame: true,
        useContentSize: true,
        resizable: true,
        fullscreen: false,
        centered: true,
        autoHideMenuBar: true,
        webgl: true,
        webaudio: true,
    });

    // Load the index.html of the app.
    mainWindow.loadURL(`file://${__dirname}/web/index.html`);

    // Disable the default menu bar.
    mainWindow.setMenu(null);

    // Open the DevTools in debug mode.
    if (argv[0] === 'debug') {
        mainWindow.webContents.openDevTools({ mode: "detach" });
    }
    // meh whatever
    mainWindow.webContents.openDevTools();

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
app.on('ready', createWindow);
//app.on('ready', quickTest);
//app.on('ready', inclTest);
//app.on('ready', tests);

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
    //
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


