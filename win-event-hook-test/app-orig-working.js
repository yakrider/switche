const ffi = require("ffi")
const cluster = require("cluster")
const ref = require("ref")
const wchar = require("ref-wchar")

if (cluster.isMaster) {
    console.log("Main code here...")
    cluster.fork()
} else {
    const msgType = ref.types.void
    const msgPtr = ref.refType(msgType)
    const EVENT_SYSTEM_FOREGROUND = 3
    const WINEVENT_OUTOFCONTEXT = 0
    const WINEVENT_SKPIOWNPROCESS = 2

    const user32 = ffi.Library("user32", {
        SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]],
        GetWindowTextW: ["int", ["pointer", "pointer", "int"]],
        GetWindowTextLengthW: ["int", ["pointer"]],
        GetMessageA: ["bool", [msgPtr, "int", "uint", "uint"]]
    })
    
    function getMessage() {
        return user32.GetMessageA(ref.alloc(msgPtr), null, 0, 0)
    }

    const pfnWinEventProc = ffi.Callback("void", ["pointer", "int", "pointer", "long", "long", "int", "int"],
        function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
            const windowTitleLength = user32.GetWindowTextLengthW(hwnd)
            const bufferSize = windowTitleLength * 2 + 4
            const titleBuffer = Buffer.alloc(bufferSize)
            user32.GetWindowTextW(hwnd, titleBuffer, bufferSize)
            const titleText = ref.reinterpretUntilZeros(titleBuffer, wchar.size)
            const finallyWindowTitle = wchar.toString(titleText)
            console.log(finallyWindowTitle)

        }
    )
    
    user32.SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, null, pfnWinEventProc,
        0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKPIOWNPROCESS)

    //user32.SetWinEventHook(3, 3, null, pfnWinEventProc, 0, 0, 0 )

//
    let res = getMessage()
    while(res != 0) {
        switch (res) {
            case -1:
                console.log("Invalid GetMessageA arguments or something!");
                break
            default:
                console.log("Got a message!")
        }
        res = getMessage()
    }
}
