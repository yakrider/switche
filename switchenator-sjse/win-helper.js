
var exports = module.exports = {};

exports.hello = function hello() {
   return "hello from local js incl";
}


var ref = require('ref');
var ffi = require('ffi');

var voidPtr = ref.refType(ref.types.void);
var stringPtr = ref.refType(ref.types.CString);


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
 // note on EnumWindows usage.. check the ms docs, but from usage below, looks like it repeatedly calls the callback w new found
 // windows, until either the callback returns false, or it has nothing more to send.. also looks like it only gives 'top-level' windows
 // whatever that means, and apparently not any child-windows.. we'll have to see in practice if need to supplement w EnumChildWindows
 
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

exports.printVisibleWindows = function printVisibleWindows() {
   getVisibleWindows ( function () {
     console.log(enumWindowsArray)
     //app.quit()
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
   
 
   //printVisibleWindows()
   app.quit();
 
   }  