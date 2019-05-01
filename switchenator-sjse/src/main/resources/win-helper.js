
var exports = module.exports = {};

exports.hello = function hello() {
   return "hello from local js incl";
}


var ref = require('ref')
var ffi = require('ffi')
var refStruct = require('ref-struct')

var voidPtr = ref.refType(ref.types.void);
var stringPtr = ref.refType(ref.types.CString);
var lpdwordPtr = ref.refType(ref.types.ulong);

var point_t_x = ref.types.long
var point_t_y = ref.types.long
var point_t = refStruct ({ x:point_t_x, y:point_t_y })
// typedef struct tagPOINT { LONG x; LONG y; } POINT, *PPOINT;

var rect_t_left = ref.types.long
var rect_t_top = ref.types.long
var rect_t_right = ref.types.long
var rect_t_bottom = ref.types.long
var rect_t = refStruct ({left:rect_t_left, top:rect_t_top, right:rect_t_right, bottom:rect_t_bottom})
// typedef struct _RECT { LONG left; LONG top; LONG right; LONG bottom; } RECT, *PRECT;

var lpwndpl_t_length = ref.types.uint
var lpwndpl_t_flags = ref.types.uint
var lpwndpl_t_showCmd = ref.types.uint
var lpwndpl_t_ptMinPosition = ref.refType(point_t)
var lpwndpl_t_ptMaxPosition = ref.refType(point_t)
var lpwndpl_t_rcNormalPosition = ref.refType(rect_t)
var lpwndpl_t_rcDevice = ref.refType(rect_t)

var lpwndpl_t = refStruct ({
  length: lpwndpl_t_length,
  flags: lpwndpl_t_flags,
  showCmd: lpwndpl_t_showCmd,
  ptMinPosition: lpwndpl_t_ptMinPosition,
  ptMaxPosition: lpwndpl_t_ptMaxPosition,
  rcNormalPosition: lpwndpl_t_rcNormalPosition,
  rcDevice: lpwndpl_t_rcDevice
})
var lpwndpl_t_ptr = ref.refType(lpwndpl_t)
//var lpwndpl = new lpwndpl_t
// typedef struct tagWINDOWPLACEMENT {UINT length; UINT flags; UINT showCmd; POINT ptMinPosition; POINT ptMaxPosition; RECT rcNormalPosition; RECT rcDevice;} WINDOWPLACEMENT;

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
   SendMessageA : ['int', ['int','int','int','int']],
   SendMessageW : ['long', ['long','int','long','long']],
   GetWindowTextA  : ['int', ['int',stringPtr,'int']],
   GetWindowTextW  : ['int', ['int',stringPtr,'int']],
   GetWindowTextLengthA  : ['long', ['long']],
   GetWindowTextLengthW  : ['long', ['long']],
   IsWindowVisible  : ['int', ['int']],
   GetWindowModuleFileNameA : ['int', ['int',stringPtr,'int']],
   GetWindowModuleFileNameW : ['int', ['int',stringPtr,'int']],
   GetWindowThreadProcessId : ['int', ['int', lpdwordPtr]],
   //BOOL GetWindowPlacement( HWND hWnd, WINDOWPLACEMENT *lpwndpl );
   GetWindowPlacement : ['int', ['int',lpwndpl_t_ptr]]
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
});
exports.getWindowProcessId = function getWindowProcessId (hwnd) {
   console.log('this oleacc call has been disabled')
   return oleacc.GetProcessHandleFromHwnd(hwnd);
}*/

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
  var copiedLen = user32.GetWindowTextA(hwnd, buf, 512);
  var name = ref.readCString(buf, 0); // reads till first null so shorter strings are fine
  return name;
}

exports.getWindowPlacement = function getWindowPlacement (hwnd) {
  //BOOL GetWindowPlacement( HWND hWnd, WINDOWPLACEMENT *lpwndpl );
  var lpwndpl = new lpwndpl_t
  var bResult = user32.GetWindowPlacement (hwnd, lpwndpl.ref())
  console.log ('result is ', bResult)
  console.log ('showCmd is ', lpwndpl.showCmd)
}

exports.activateWindow = function activateWindow (hwnd) {
  var lpwndpl = new lpwndpl_t
  user32.GetWindowPlacement (hwnd, lpwndpl.ref())
  //console.log ('showCmd is ', lpwndpl.showCmd)
  if (lpwndpl.showCmd == 2) { // means minimized, gotta do the restore first
    user32.ShowWindow(hwnd, 9) // 9 is the SW_RESTORE cmd, required for minimized windows, but will 'restore' maximized ones
  } else {
    user32.ShowWindow(hwnd, 5) // 5 is SW_SHOW, it shows windows in cur size and position
  }
  return user32.SetForegroundWindow(hwnd);
}
exports.hideWindow = function hideWindow (hwnd) {
  return user32.ShowWindow(hwnd,0) // 0 is for hide cmd, can be shown by show cmd
}
exports.minimizeWindow = function minimizeWindow (hwnd) {
   // note that the u32 'CloseWindow' cmd actually minimizes it, to close, send it a WM_CLOSE msg
  return user32.CloseWindow(hwnd)
}
exports.showWindow = function showWindow (hwnd) {
   user32.ShowWindow(hwnd,5)
   return user32.SetForegroundWindow(hwnd)
}
exports.closeWindow = function closeWindow (hwnd) {
   // LRESULT SendMessage( HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam );
   // 16 is 0x0010 or WM_CLOSE, the WPARAM, LPARAM are ignored for this message
   // this closes most, but not Windows internal windows, hence trying the W version, but was no better
   //user32.SendMessageW(hwnd,16,0,0)
   return user32.SendMessageA(hwnd,16,0,0)
   //return 1 // meh
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

exports.printVisibleWindows = function printVisibleWindows() {
  getVisibleWindows ( function (arr) {
    //console.log(enumWindowsArray);
    console.log(arr);
    //app.quit()
  });
}


var icoExt = require('icon-extractor');
var defaultIconsCallback = function(ctxStr,path,data) {console.error('No callback registered for icon-extractor!!')};
var iconsCallback = defaultIconsCallback
icoExt.emitter.on ('icon', function(data){ iconsCallback (data.Context, data.Path, data.Base64ImageData) });
icoExt.emitter.on ('error', function(e){console.error(e)} );
exports.registerIconsCallback = function registerIconsCallback (callback) {iconsCallback = callback}
exports.unregisterIconsCallback = function unregisterIconsCallback() {iconsCallback = defaultIconsCallback}
exports.queueIconsQuery = function queueIconsQuery (ctxStr,path) { icoExt.getIcon(ctxStr,path); }

