
var exports = module.exports = {};

exports.hello = function hello() {
   return "hello from local js incl";
}


var ref = require('ref');
var ffi = require('ffi');

var voidPtr = ref.refType(ref.types.void);
var stringPtr = ref.refType(ref.types.CString);
var lpdwordPtr = ref.refType(ref.types.ulong);

function TEXT(text) {
   return new Buffer(text, 'ucs2').toString('binary');
 }

 
 // e.g https://github.com/MrTimcakes/node-hide/blob/master/main.js
 // note that references are in https://docs.microsoft.com/en-us/windows/desktop/api/winuser/
 // the A and W versions of things deal with / return strings are for Ansi and Unicode/Wide variant strings
 var user32 = new ffi.Library('user32.dll', {
   // BOOL EnumWindows( WNDENUMPROC lpEnumFunc, LPARAM lParam );
   // BOOL CALLBACK EnumWindowsProc( _In_ HWND   hwnd, _In_ LPARAM lParam );
   EnumWindows : ['int', [voidPtr, 'int32']],
   FindWindowW : ['int', ['string', 'string']],
   ShowWindow : ['int', ['int', 'int']],
   ShowWindowAsync : ['int', ['int', 'int']],
   SetForegroundWindow: ['int', ['int']],
   BringWindowToTop: ['long', ['long']],
   CloseWindow  : ['long', ['long']],
   GetWindowTextA  : ['int', ['int',stringPtr,'int']],
   GetWindowTextW  : ['int', ['int',stringPtr,'int']],
   GetWindowTextLengthA  : ['long', ['long']],
   GetWindowTextLengthW  : ['long', ['long']],
   IsWindowVisible  : ['int', ['int']],
   GetWindowModuleFileNameA : ['int', ['int',stringPtr,'int']],
   GetWindowModuleFileNameW : ['int', ['int',stringPtr,'int']],
   GetWindowThreadProcessId : ['int', ['int', lpdwordPtr]]
 });
 // note on EnumWindows usage.. check the ms docs, but from usage below, looks like it repeatedly calls the callback w new found
 // windows, until either the callback returns false, or it has nothing more to send.. also looks like it only gives 'top-level' windows
 // whatever that means, and apparently not any child-windows.. we'll have to see in practice if need to supplement w EnumChildWindows

 var kernel32 = new ffi.Library('kernel32.dll', {
    //HANDLE OpenProcess( DWORD dwDesiredAccess, BOOL  bInheritHandle, DWORD dwProcessId );
    OpenProcess : ['int', ['int','int','int']],
    //BOOL CloseHandle( HANDLE hObject );
    CloseHandle : ['int', ['int']]
 });

 function findWindow (name) {
   for(i=0;i<50;i++){ //ensure accurate reading, sometimes returns 0 when window does exist .. horrible horrible crap this is!
     handle = user32.FindWindowW(null, TEXT(name));
     if(handle!==0){break;}
   }
   return handle;
 }


// hmm, fails to load
/*
var oleacc = new ffi.Library('oleacc.dll' {
  // HANDLE WINAPI GetProcessHandleFromHwnd( _In_ HWND hwnd );
  GetProcessHandleFromHwnd : ['int', ['int']]
});*/
exports.getWindowProcessId = function getWindowProcessId (hwnd) {
   console.log('this oleacc call has been disabled')
   //return oleacc.GetProcessHandleFromHwnd(hwnd);
}
exports.getWindowThreadProcessId = function getWindowThreadProcessId (hwnd) {
   //DWORD GetWindowThreadProcessId( HWND hWnd, LPDWORD lpdwProcessId );
   var pidRef = ref.alloc(lpdwordPtr);
   user32.GetWindowThreadProcessId(hwnd,pidRef);
   var pid = pidRef.readInt32LE(0);
   return pid
}
exports.openProcess = function openProcess (pid) {
   //HANDLE OpenProcess( DWORD dwDesiredAccess, BOOL  bInheritHandle, DWORD dwProcessId );
   // re access-bitmap arg, sending 1040.. we want 0x0400 and 0x0010 (QUERY, and VM_READ), but we're passing as decimal int
   return kernel32.OpenProcess (1040,0,pid)
}
exports.getProcessExeFromPid = function getProcessExeFromPid (pid) {
   //HANDLE OpenProcess( DWORD dwDesiredAccess, BOOL  bInheritHandle, DWORD dwProcessId );
   // re access-bitmap arg, sending 1040.. we want 0x0400 and 0x0010 (QUERY, and VM_READ), but we're passing as decimal int
   var handle = kernel32.OpenProcess (1040,0,pid)
   var buf = new Buffer(512);
   // DWORD GetProcessImageFileNameA( HANDLE hProcess, LPSTR  lpImageFileName, DWORD  nSize );
   var copiedLen = psapi.GetProcessImageFileNameA(handle,buf,512);
   var name = ref.readCString(buf,0);
   //console.log('pid:',pid,', handle:',handle,', copied len:',copiedLen,', name: ', name)
   //BOOL CloseHandle( HANDLE hObject );
   kernel32.CloseHandle(handle);
   return name
}


var psapi = new ffi.Library('psapi.dll', {
  // DWORD GetProcessImageFileNameA( HANDLE hProcess, LPSTR  lpImageFileName, DWORD  nSize );
  // note that a process handle is not the same as the pid
  GetProcessImageFileNameA : ['int', ['int',stringPtr,'int']]
});
exports.getProcessExe = function getProcessExe (handle) {
   var buf = new Buffer(200);
   var copiedLen = psapi.GetProcessImageFileNameA(handle,buf,200);
   var name = ref.readCString(buf,0);
   return name
}
exports.getProcessExeFromHwnd = function getProcessExeFromHwnd(hwnd) {
   var handle = 0; //oleacc.GetProcessHandleFromHwnd(hwnd);
   var buf = new Buffer(200);
   var copiedLen = psapi.GetProcessImageFileNameA(handle,buf,200);
   var name = ref.readCString(buf,0);
   return name
}

 
 var enumWindowsArray = []; //{};
 var enumWindowsTimeout;
 var enumWindowsCallback;
 exports.getVisibleWindows = function getVisibleWindows (callback) {
   enumWindowsArray = []; //{};
   console.log(callback);
   enumWindowsCallback = callback;
   // ms docs api reference : BOOL EnumWindows( WNDENUMPROC lpEnumFunc, LPARAM lParam );
   // ms docs ref for callback : BOOL CALLBACK EnumWindowsProc( _In_ HWND   hwnd, _In_ LPARAM lParam );
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
     enumWindowsArray.push (name);
 
     return true;
   }), 0);
 }

exports.streamWindowsQuery = function streamWindowsQuery (callback, callId) {
  // no return, will repeatedly call callback w new windows found, incl non-visible ones
  // in theory returns 0/1 code, but chrome console says returns 'undefined'
  user32.EnumWindows (ffi.Callback ('int', ['long', 'int32'], callback), callId);
}
exports.checkWindowVisible = function checkWindowVisible (hwnd) {
  // in theory returns bool, but think its still 0/1
  return user32.IsWindowVisible(hwnd);
}
exports.getWindowText = function getWindowText (hwnd) {
  //var length = user32.GetWindowTextLengthA(hwnd);
  //if (length == 0) return "";
  //var limlen = (length < 200) ? length+1 : 200; // limit to 200 chars max
  //var buf = new Buffer(limlen);
  var buf = new Buffer(512);
  var copiedLen = user32.GetWindowTextW(hwnd, buf, 512);
  var name = ref.readCString(buf, 0); // reads till first null so shorter strings are fine
  return name;
}
exports.activateWindow = function activateWindow (hwnd) {
  // returns bool as 0/1
  user32.ShowWindowAsync(hwnd, 9); // 9 is the SW_RESTORE cmd, works whether minimized or not
  return user32.SetForegroundWindow(hwnd);
}
exports.getWindowModuleFile = function getWindowModuleFile (hwnd) {
  // uhh, this is pointless, only works for windows or children matching the same as the requesting process!
  // UINT GetWindowModuleFileNameW ( HWND   hwnd, LPWSTR pszFileName, UINT   cchFileNameMax );
  //GetWindowModuleFileNameA : ['int', ['long',stringPtr,'int']],
  var buf = new Buffer(512);
  var copiedLen = user32.GetWindowModuleFileNameA(hwnd,buf,512);
  var name = ref.readCString(buf,0);
  return name;
}
exports.getProcessIcon = function getProcessIcon (pid) {
}


exports.printVisibleWindows = function printVisibleWindows() {
   getVisibleWindows ( function (arr) {
    //console.log(enumWindowsArray);
    console.log(arr);
    //app.quit()
   });
 }

exports.activateTestWindow = function activateTestWindow() {
  var h = findWindow ('Untitled - Notepad');
  console.log('found handle : ' + h);
  user32.SetForegroundWindow(h);
}

 
 function test() { // huh, both options below seems really unreliable, often returning 0 for handle while finding it at other times, and not reliably activating window either
   var handle = user32v0.FindWindowW (null, TEXT('Untitled - Notepad'));
   //var h2 = user32.FindWindowW (null, TEXT('Untitled - Notepad'));
   var h2 = findWindow ('Untitled - Notepad');
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
