// Modules to control application life and create native browser window
const {app, BrowserWindow} = require('electron')

var ref = require('ref');
var ffi = require('ffi');

var voidPtr = ref.refType(ref.types.void);
var stringPtr = ref.refType(ref.types.CString);


// Keep a global reference of the window object, if you don't, the window will
// be closed automatically when the JavaScript object is garbage collected.
let mainWindow

function createWindow () {
  // Create the browser window.
  mainWindow = new BrowserWindow({width: 800, height: 600})

  // and load the index.html of the app.
  mainWindow.loadFile('index.html')

  // Open the DevTools.
  //mainWindow.webContents.openDevTools()
  console.log('test log output')
  //test()

  // Emitted when the window is closed.
  mainWindow.on('closed', function () {
    // Dereference the window object, usually you would store windows
    // in an array if your app supports multi windows, this is the time
    // when you should delete the corresponding element.
    mainWindow = null
  })
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
//app.on('ready', createWindow)
//app.on('ready', test)
app.on('ready', inclTest)

// Quit when all windows are closed.
app.on('window-all-closed', function () {
  // On OS X it is common for applications and their menu bar
  // to stay active until the user quits explicitly with Cmd + Q
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', function () {
  // On OS X it's common to re-create a window in the app when the
  // dock icon is clicked and there are no other windows open.
  if (mainWindow === null) {
    createWindow()
  }
})

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.

function TEXT(text) {
  return new Buffer(text, 'ucs2').toString('binary');
}


var user32v0 = new ffi.Library('user32', {
  'FindWindowW': ['int', ['string', 'string']],
  'SetForegroundWindow': ['int', ['int']],
  'BringWindowToTop': ['int', ['int']],
  'ShowWindow': ['int', ['int', 'int']],
  'ShowWindowAsync': ['int', ['int', 'int']],
});


// e.g https://github.com/MrTimcakes/node-hide/blob/master/main.js
var user32 = new ffi.Library('user32.dll', {
  EnumWindows : ['bool', [voidPtr, 'int32']],
  FindWindowW : ['int', ['string', 'string']],
  ShowWindow : ['int', ['int', 'int']],
  ShowWindowAsync : ['int', ['int', 'int']],
  SetForegroundWindow: ['long', ['long']],
  BringWindowToTop: ['long', ['long']],
  CloseWindow  : ['long', ['long']],
  GetWindowTextA  : ['long', ['long', stringPtr, 'long']],
  GetWindowTextLengthA  : ['long', ['long']],
  IsWindowVisible  : ['long', ['long']]
});

function findWindow (name) {
  for(i=0;i<50;i++){ //ensure accurate reading, sometimes returns 0 when window does exist .. horrible horrible crap this is!
    handle = user32.FindWindowW(null, TEXT(name))
    if(handle!==0){break;}
  }
  return handle;
}


var enumWindowsArray = []; //{};
var enumWindowsTimeout;
var enumWindowsCallback;
function getVisibleWindows (callback) {
  enumWindowsArray = []; //{};
  enumWindowsCallback = callback;
  user32.EnumWindows (ffi.Callback ('bool', ['long', 'int32'], function (hwnd, lParam) {
    clearTimeout(enumWindowsTimeout);
    enumWindowsTimeout = setTimeout(enumWindowsCallback,50,enumWindowsArray); // 50ms after last run, assume ended
    if (!user32.IsWindowVisible(hwnd)) return true;
    var length = user32.GetWindowTextLengthA(hwnd);
    if (length == 0) return true;

    var buf = new Buffer(length+1);
    user32.GetWindowTextA(hwnd, buf, length+1);
    var name = ref.readCString(buf, 0);

    //enumWindowsArray[hwnd] = name;
    //enumWindowsArray.push (const {h:hwnd,n:name})
    //enumWindowsArray.push (hwnd,name)
    enumWindowsArray.push (name)

    return true;
  }), 0);
}
function printVisibleWindows() {
  getVisibleWindows ( function () {
    console.log(enumWindowsArray)
    app.quit()
  });
}

function test() { // huh, both options below seems really unreliable, often returning 0 for handle while finding it at other times, and not reliably activating window either
  var handle = user32v0.FindWindowW (null, TEXT('Untitled - Notepad'));
  //var h2 = user32.FindWindowW (null, TEXT('Untitled - Notepad'));
  var h2 = findWindow ('Untitled - Notepad')
  var h3 = user32.FindWindowW (null, TEXT('Rapid Environment Editor'));
  console.log('handle is : ' + handle); // hmm, at least this works!
  console.log('h2 is : ' + h2); // hmm, at least this works!
  console.log('h3 is : ' + h3); // hmm, at least this works!
  // look up show-window details at https://docs.microsoft.com/en-us/windows/desktop/api/winuser/nf-winuser-showwindow
  //user32v0.ShowWindow(handle,0); // --> this will literally hide the window (so wont be visible in taskbar!)
  //user32v0.ShowWindow(handle,1); // - works sometimes, though seems very unreliable.. hmm, dont think ShowWindow is designed for this purpose
  //user32v0.ShowWindowAsync(handle,1);
  //user32v0.SetForegroundWindow(handle); // <- works
  //user32.BringWindowToTop(handle); // <- 
  //user32.ShowWindow(h2,1);
  //user32.ShowWindowAsync(h2,1);
  //user32.BringWindowToTop(h3); // <- does not work
  //user32.SetForegroundWindow(h3); // <- that works reliably too if handle is available
  //user32.SetForegroundWindow(2232222); // <- that works reliably
  console.log('done!');
  //app.quit();

  var wapiTest = require('./win-helper');
  console.log(wapiTest.hello());  app.quit();

  //printVisibleWindows()

}

function inclTest() {
  var wapiTest = require('./win-helper');
  console.log(wapiTest.hello());  
  app.quit();

}


