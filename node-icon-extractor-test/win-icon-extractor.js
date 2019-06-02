var path = require('path');
var ffi = require('ffi');
var ref = require('ref');
var struct = require('ref-struct');
var fs = require('fs');
var jimp = require('jimp');
var bmp_js = require('bmp-js');

var IntPtr = ref.refType(ref.types.int);
var HANDLE = ref.refType(ref.types.void);

var PDWORD_PTR = ref.refType(ref.refType(ref.types.int));

var lpctstr = {
   name: 'lpctstr',
   indirection: 1,
   size: ref.sizeof.pointer,
   get: function(buffer, offset) {
      var _buf = buffer.readPointer(offset);
      if(_buf.isNull()) {
         return null;
      }
      return _buf.readCString(0);
   },
   set: function(buffer, offset, value) {
      var _buf = new Buffer(Buffer.byteLength(value, 'ucs2') + 2)
      _buf.write(value, 'ucs2')
      _buf[_buf.length - 2] = 0
      _buf[_buf.length - 1] = 0
      return buffer.writePointer(_buf, offset)
   },
   ffi_type: ffi.types.CString.ffi_type
};

var iconInfo = struct({
   'fIcon': ref.types.bool,
   'xHotspot': ref.types.ulong,
   'yHotspot': ref.types.ulong,
   'hbmMask': HANDLE,
   'hbmColor': HANDLE
});

var bitmapInfoHeader = struct({
   biSize: ref.types.ulong,
   biWidth: ref.types.long,
   biHeight: ref.types.long,
   biPlanes: ref.types.ushort,
   biBitCount: ref.types.ushort,
   biCompression: ref.types.ulong,
   biSizeImage: ref.types.ulong,
   biXPelsPerMeter: ref.types.long,
   biYPelsPerMeter: ref.types.long,
   biClrUsed: ref.types.ulong,
   biClrImportant: ref.types.ulong
});

var palleteColor = struct({
   red: ref.types.uint8,
   green: ref.types.uint8,
   blue: ref.types.uint8,
   void: ref.types.uint8
});

var bitmapInfo = struct({
   bmiHeader: bitmapInfoHeader
});

// Allocate color table for 16 colors
// The table size is dynamic, but needs to be preallocated
// After we load the actual table size, we slice unused part off
for (var i = 0; i < 16; i++) {
   bitmapInfo.defineProperty('color' + i, palleteColor);
}

var shell32 = ffi.Library('shell32', {
   // HICON ExtractAssociatedIconW (HINSTANCE hInst, LPSTR pszIconPath, WORD *piIcon );
   //'ExtractAssociatedIconW': ['HANDLE', [IntPtr, lpctstr, IntPtr]]
   'ExtractAssociatedIconW': ['int', [IntPtr, lpctstr, IntPtr]]
});
var gdi32 = ffi.Library('gdi32', {
   'GetDIBits': [ref.types.int32, [IntPtr, IntPtr, 'uint32', 'uint32', IntPtr, ref.refType(bitmapInfo), 'uint32'] ]
});
var user32 = ffi.Library('user32', {
   // BOOL GetIconInfo( HICON hIcon, PICONINFO pIconInfo );
   //'GetIconInfo': ['bool', [IntPtr, ref.refType(iconInfo)]],
   'GetIconInfo': ['bool', ['int', ref.refType(iconInfo)]],
   'GetDC': [HANDLE, [IntPtr]],
   //'DestroyIcon': ['bool', ['int']],
   'DestroyIcon': ['bool', ['int']],
   // LRESULT SendMessage( HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam );
   // typedef UINT_PTR WPARAM; // typedef LONG_PTR LPARAM; // typedef LONG_PTR LRESULT;
   'SendMessageA': ['int', ['int','int','int','int' ]],
   'SendMessageW': ['int', ['int','uint32','long','long' ]],
   // LRESULT SendMessageTimeoutA ( HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam, UINT fuFlags, UINT uTimeout, PDWORD_PTR lpdwResult );
   'SendMessageTimeoutA': ['int',['int','int','int','int','int','int',PDWORD_PTR]],
   'SendMessageTimeoutW': ['int',['int','int','int','int','int','int',PDWORD_PTR]],
   // ULONG_PTR GetClassLongPtrW( HWND hWnd, int nIndex );
   //'GetClassLongPtr': ['long', ['int', 'int']]
   'GetClassLongPtrA': ['long', ['int', 'int']],
   //'GetClassLongPtrW': ['long', [HANDLE, 'int']]
   //BOOL SendMessageCallbackA ( HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam, SENDASYNCPROC lpResultCallBack, ULONG_PTR dwData );
   //SENDASYNCPROC : void Sendasyncproc (HWND Arg1, UINT Arg2, ULONG_PTR Arg3, LRESULT Arg4)
   'SendMessageCallbackA' : ['bool',['int','int','int','int','pointer','int']],
   GetMessageA: ["bool", ['pointer', "int", "uint", "uint"]],
   PeekMessageA: ["bool", ['pointer', "int", "uint", "uint", "uint"]]
});

function loadBitmap(hbitmap, ident) {
   var bitmap = new bitmapInfo();
   
   // Clear bitmap info
   bitmap['ref.buffer'].fill(0);

   // Save the bmiheader size
   bitmap.bmiHeader.biSize = 40;

   // Load bitmap details
   var dc = user32.GetDC(null);
   if (dc.deref() == 0) {
      throw new Error("Failed to get screen DC.");
   }
   
   if (gdi32.GetDIBits(dc, hbitmap, 0, 0, null, bitmap.ref(), 0) == 0) {
      throw new Error("Failed to load BITMAP (" + ident + ") info.");
   }

   // Slice off the unused color table
   var colors = bitmap.bmiHeader.biBitCount < 24 ? ((1 << bitmap.bmiHeader.biBitCount) * 4) : 0;
   bitmap['ref.buffer'] = bitmap['ref.buffer'].slice(0, bitmap.bmiHeader.biSize + colors);

   // Disable compression
   bitmap.bmiHeader.biCompression = 0;

   // Load bitmap data
   var data = new Buffer(bitmap.bmiHeader.biSizeImage);
   if (gdi32.GetDIBits(dc, hbitmap, 0, bitmap.bmiHeader.biHeight, data, bitmap.ref(), 0) == 0) {
      throw new Error("Failed to load BITMAP data.");
   }

   // Prepare BMP header
   var header = new Buffer(2 + 4 + 4 + 4);
   
   // BMP signature (BM)
   header.writeUInt8(66, 0);
   header.writeUInt8(77, 1);
   // Size fo the BMP file, HEADER + COLOR_TABLE + DATA
   header.writeUInt32LE(data.byteLength + 54 + colors, 2);
   // Reserved
   header.writeUInt32LE(0, 6);
   // Offset of actual image data HEADER + COLOR_TABLE
   header.writeUInt32LE(54 + colors, 10);

   // Return resulting BMP file
   return {
      data: Buffer.concat([header, bitmap.ref(), data]),
      depth: bitmap.bmiHeader.biBitCount
   };
}

function getIconStringFromExe (target) {
   return new Promise ((resolve, reject) => { setTimeout(function() {
      target = path.resolve(target); // make sure the path is absolute
      var iconIndex = ref.alloc(ref.types.int32, 0);
      var hicon = shell32.ExtractAssociatedIconW (null, target, iconIndex);
      //console.log(hicon)
      getIconStringFromHIcon(hicon) .then((iconString) => { //console.log(iconString)
         user32.DestroyIcon(hicon); // release icon from memory
         resolve(iconString)
      }).catch (function(err) {
         user32.DestroyIcon(hicon)
         reject(err)
      });
   }); });
}
exports.getIconStringFromExe = function (target) {return getIconStringFromExe(target)}

/*
public Icon GetAppIcon(IntPtr hwnd) {
  IntPtr iconHandle = IntPtr.Zero;
  if (iconHandle == IntPtr.Zero) { iconHandle = SendMessage (hwnd, WM_GETICON, ICON_SMALL2, 0); }
  if (iconHandle == IntPtr.Zero) { iconHandle = SendMessage (hwnd, WM_GETICON, ICON_SMALL, 0); }
  if (iconHandle == IntPtr.Zero) { iconHandle = SendMessage (hwnd, WM_GETICON, ICON_BIG, 0); }
  if (iconHandle == IntPtr.Zero) { iconHandle = GetClassLongPtr (hwnd, GCL_HICON); }
  if (iconHandle == IntPtr.Zero) { iconHandle = GetClassLongPtr (hwnd, GCL_HICONSM); }
  if (iconHandle == IntPtr.Zero) { return null; }
  return Icon.FromHandle(iconHandle); 
} */
const GCL_HICONSM = -34; const GCL_HICON = -14;  
const ICON_SMALL = 0; const ICON_BIG = 1; const ICON_SMALL2 = 2; 
const WM_GETICON = 0x7F;
const SMTO_NORMAL = 0x0000; const SMTO_ABORTIFHUNG = 0x0002;

function getIconStringFromHwnd (hwnd) {
   return new Promise ((resolve, reject) => { setTimeout(function() {
      //var lpdwResult = ref.alloc(PDWORD_PTR)
      var lpdwResult = ref.alloc(ref.refType(ref.types.int))
      var hicon = 0;
      //if (hicon == 0) { hicon = user32.SendMessageA (hwnd, WM_GETICON, ICON_SMALL2, 0) }
      console.log(user32.SendMessageA (hwnd, WM_GETICON, ICON_SMALL2, 0))
      if (hicon == 0) { user32.SendMessageTimeoutA (hwnd, WM_GETICON, ICON_SMALL2, 0, SMTO_ABORTIFHUNG, 100, lpdwResult) }
      console.log(lpdwResult)
      console.log(lpdwResult, ", ", lpdwResult.readInt32LE(0))
      hicon = lpdwResult.readInt32LE(0)
      //console.log(lpdwResult, ", ", lpdwResult.deref(), ", ")
      //console.log(lpdwResult, ", ", lpdwResult.deref(), ", ", lpdwResult.deref().deref())
      //if (hicon == 0) { hicon = user32.SendMessageA (hwnd, WM_GETICON, ICON_SMALL, 0) }
      //if (hicon == 0) { hicon = user32.SendMessageA (hwnd, WM_GETICON, ICON_BIG, 0) }
      //if (hicon == 0) { hicon = user32.GetClassLongPtrA (hwnd, GCL_HICONSM) }
      //if (hicon == 0) { hicon = user32.GetClassLongPtrA (hwnd, GCL_HICON) }
      console.log('for hWnd:',hwnd,', got hIcon:', hicon)
      if (hicon == 0) { return reject (new Error ('All methods of querying for hIcon failed for hwnd:' + hwnd)) }
      resolve (getIconStringFromHIcon(hicon))
      //resolve (getIconStringFromHIcon(ref.alloc(ref.types.int, hicon)))
      //resolve (getIconStringFromHIcon(ref.address(hicon)))
   }); });
}
exports.getIconStringFromHwnd = function (hwnd) {return getIconStringFromHwnd(hwnd)}


function defaultHwndIconStringCallback (hwnd,hicon,iconString) {
   console.error('No callback registered for icon-extractor!')
   console.log('icon string for ',hwnd,' is:\n',iconString)
}
var hwndIconStringCallback = defaultHwndIconStringCallback
exports.registerHwndIconStringCallback = function (callback) {hwndIconStringCallback = callback}
exports.unregisterHwndIconStringCallback = function () {hwndIconStringCallback = defaultHwndIconStringCallback}

function SteppedWinSendMessageCallback (hwnd, msg, stepCtx, hicon) {
   console.log ('got callback!')
   console.log('hwnd:',hwnd,' arg1:',msg,' arg2:',stepCtx,' hicon:',hicon)
   if (hicon == 0) {
      callFnAndReQueueCapped (peekMessage, true)
      if (stepCtx == 0) {
         user32.SendMessageCallbackA (hwnd, WM_GETICON, ICON_SMALL2, 0, fficbSteppedWinSendMessageCallback, 1)
      } else if (stepCtx == 1) {
         user32.SendMessageCallbackA (hwnd, WM_GETICON, ICON_SMALL, 0, fficbSteppedWinSendMessageCallback, 2)
      } else if (stepCtx == 2) {
         user32.SendMessageCallbackA (hwnd, WM_GETICON, ICON_BIG, 0, fficbSteppedWinSendMessageCallback, 3)
      } else {
         if (hicon == 0) { hicon = user32.GetClassLongPtrA (hwnd, GCL_HICONSM) }
         if (hicon == 0) { hicon = user32.GetClassLongPtrA (hwnd, GCL_HICON) }
         if (hicon == 0) {
            console.log ("All methods of querying for hIcon failed for hwnd:" + hwnd)
            hwndIconStringCallback(hwnd,0,"") // failure is signalled up as empty iconString
         }
      }
   }
   if (hicon != 0) { // redo check as we might have updated it in above conditionals
      getIconStringFromHIcon(hicon) .then((iconString) => { 
         hwndIconStringCallback(hwnd,hicon,iconString)
      }) .catch ( function (err) { 
         console.error('error extracting icon for hwnd:'+hwnd+', hicon:'+hicon, err); 
         hwndIconStringCallback(hwnd,0,"") // failure is signalled up as empty iconString 
      } );
   }

}
var fficbSteppedWinSendMessageCallback = ffi.Callback ('void', ['int','int','int','int'], SteppedWinSendMessageCallback)

function queueIconStringFromHwnd (hwnd) {
   //setTimeout (function() {console.log('queued timeout ended.')}, 5000); //set some time so things dont exit immediately
   //callFnAndReQueue (peekMessage, 500,10)
   //callFnAndReQueueCapped (peekMessage, true)
   SteppedWinSendMessageCallback (hwnd,0,0,0)
   //user32.SendMessageCallbackA (hwnd, WM_GETICON, ICON_SMALL, 0, fficbWinSendMessageCallback, 0)
}
exports.queueIconStringFromHwnd = function (hwnd) {return queueIconStringFromHwnd(hwnd)}

// winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
function getMessage() { user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
function peekMessage() { user32.PeekMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0, 0) }
//while (0 != getMessage()) {}
//function callFnAndReQueue (fn,reQueueDelay,maxTimes) { fn(); setTimeout ( function() {fn()}, reQueueDelay ) }
//function callFnAndReQueue (fn,reQueueDelay,nMax) { fn(); if (nMax>0) { console.log('re-queueing'); setTimeout ( function() {callFnAndReQueue(fn,reQueueDelay,nMax-1)} ) } }
//function callFnAndReQueue (fn,reQueueDelay,nMax) { fn(); if (nMax>0) { setTimeout ( function() {callFnAndReQueue(fn,reQueueDelay,nMax-1)} ) } }

var globalReQueueCountdown = 0; var globalReQueueDelayMs = 100; var globalReQueueCountMax = 20;
function callFnAndReQueueCapped (fn,doCapReset) { 
   fn()
   if (doCapReset) {globalReQueueCountdown = globalReQueueCountMax} 
   globalReQueueCountdown--;
   if (globalReQueueCountdown > 0) { setTimeout ( function() {callFnAndReQueueCapped(fn,false)}, globalReQueueDelayMs ) } 
   //if (globalReQueueCountdown > 0) { console.log('re-queueing'); setTimeout ( function() {callFnAndReQueueCapped(fn,false)}, globalReQueueDelayMs ) } 
}

//callFnAndReQueue (getMessage, 10000)
//callFnAndReQueue (peekMessage, 10,500)
//setInterval ( getMessage, 10 )




function getIconStringFromHIcon (hicon) { //console.log(hicon);
   return new Promise((resolve, reject) => {
      var info = new iconInfo(); // load icon data      
      info['ref.buffer'].fill(0); // clear info struct

      if (!user32.GetIconInfo(hicon, info.ref())) {
         return reject (new Error("Failed to load icon info from hIcon."))
      }
      
      // Load icon bitmaps
      var colored = loadBitmap(info.hbmColor, 'colored');
      var mask = loadBitmap(info.hbmMask, 'mask');

      // Load bitmaps into standardized formats
      var colored_bmp = bmp_js.decode(colored.data);
      var mask_bmp = bmp_js.decode(mask.data);

      // Load the colored bmp
      // Little hack has to be applied, jimp currently doesn't support 32 bit BMP
      // Encoder uses 24 bit, so it loads fine
      jimp.read(bmp_js.encode(colored_bmp).data, (err, colored_img) => {
         if (err) return reject(err);
         // Bitmap can have 32 bits per color, but ignore the aplha channel
         var has_alpha = false;

         // 32 bit BMP can have alpha encoded, so we may not need the mask
         if (colored.depth > 24) {			
            // Scan the original BMP image, if any pixel has > 0 alpha, the mask wont be needed
            for (var xx = 0; xx < colored_bmp.width; xx++) {
               for (var yy = 0; yy < colored_bmp.height; yy++) {
                  var index = colored_img.getPixelIndex(xx, yy);
                  if (colored_bmp.data[index + 3] != 0) {
                     has_alpha = true;
                     break;
            } } }
         }
         // Ignore mask, if the colored icon has alpha encoded already (most does)
         if (has_alpha) { // Little hack again, assign actual RGBA data to image
            colored_img.bitmap = colored_bmp;
            colored_img.getBase64(jimp.MIME_PNG, (error, base64) => {
               if (err) return reject(err);               
               return resolve(base64);
            });
         } else { // Load mask and apply it
            jimp.read(bmp_js.encode(mask_bmp).data, (err, mask_img) => {
               if (err) return reject(err);               
               var masked_img = colored_img.mask(mask_img.invert(), 0, 0);
               masked_img.getBase64(jimp.MIME_PNG, (error, base64) => {
                  if (err) return reject(err);                  
                  return resolve(base64);
               });
            });
         }
      });
   });
}
exports.getIconStringFromHIcon = function (hicon) {return getIconStringFromHIcon(hicon)}

