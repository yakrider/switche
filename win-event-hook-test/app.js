const cluster = require("cluster")

if (cluster.isMaster) {

    console.log("Main code here...")
    var worker = cluster.fork()

    worker.on('message', function(msg) {console.log('got msg from worker: ', msg)})

} else {
    const ffi = require("ffi")
    const ref = require("ref")
    
    const user32 = ffi.Library("user32", {
        //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
        //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )  
        SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]],
        GetMessageA: ["bool", ['pointer', "int", "uint", "uint"]]
    })
    
    //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )  
    const pfnWinEventProc = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
        function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
            process.send(ref.address(hwnd))
        }
    )
    // set the actual event hook
    user32.SetWinEventHook(3, 3, null, pfnWinEventProc, 0, 0, 0 )
    
    // winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
    function getMessage() { return user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
    while (0 != getMessage()) {}

}
