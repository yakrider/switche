
const {Worker, isMainThread, parentPort, workerData} = require ("worker_threads");

//const ffi = require("ffi")  // cant have this.. 'module did not self-register' if both threads load node-gyp packages like ffi

if (isMainThread) {

    const worker = new Worker(__filename, {});
    worker.on('message', (msg) => { console.log('worker sent: ', msg) } );
    worker.on('error', (err) => {console.log('worker error: ', err)} );

} else {
    //const ffi = require("ffi-napi")
    const ffi = require("ffi")
    //const cluster = require("cluster")
    const ref = require("ref")
    const refStruct = require("ref-struct")
    const wchar = require("ref-wchar")
    const msgType = ref.types.void
    const msgPtr = ref.refType(msgType)

    const user32 = ffi.Library("user32", {
        SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]],
        GetWindowTextW: ["int", ["pointer", "pointer", "int"]],
        GetWindowTextLengthW: ["int", ["pointer"]],
        GetMessageA: ["bool", [msgPtr, "int", "uint", "uint"]]
    })

    var intPtr = ref.refType('int');
    var callback = function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
        const windowTitleLength = user32.GetWindowTextLengthW(hwnd)
        const bufferSize = windowTitleLength * 2 + 4
        const titleBuffer = Buffer.alloc(bufferSize)
        user32.GetWindowTextW(hwnd, titleBuffer, bufferSize)
        const titleText = ref.reinterpretUntilZeros(titleBuffer, wchar.size)
        const finallyWindowTitle = wchar.toString(titleText)
        //console.log(finallyWindowTitle)
        //parentPort.postMessage(hwnd.toString)
        //console.log(hwnd);
        //console.log(hwnd.deref());
        //console.log(hwnd.toString())
        //console.log(ref.readInt64LE(hwnd))
        parentPort.postMessage(finallyWindowTitle)
        //parentPort.postMessage(ref.derefType(readUInt64LE(hwnd))
        //console.log("Callback!")
        //console.log(arguments)
    }
    const pfnWinEventProc = ffi.Callback('void', ['pointer', 'int', intPtr, 'long', 'long', 'int', 'int'], callback)

    user32.SetWinEventHook(3, 3, null, pfnWinEventProc, 0, 0, 0 )

    //setInterval ( function(){console.log('.')}, 1000 ) // keep the script alive
    //

    function getMessage() {
        return user32.GetMessageA(ref.alloc(msgPtr), null, 0, 0)
    }
    function endless() {
        let res = getMessage()	
        while(res != 0) { res = getMessage() }
    }

    parentPort.postMessage('hello from the worker!')

    //endless()
    while (getMessage()!=0){}

}