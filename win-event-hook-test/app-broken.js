const ffi = require("ffi-napi")

const user32 = ffi.Library("user32", {
    SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]]
})

const pfnWinEventProc = ffi.Callback("void", ["pointer", "int", "pointer", "long", "long", "int", "int"],
    function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
        console.log("Callback!")
        console.log(arguments)
    })


user32.SetWinEventHook(3, 3, null, pfnWinEventProc, 0, 0, 0)

setInterval ( function(){console.log('.')}, 1000 ) // keep the script alive

