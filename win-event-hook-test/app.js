const cluster = require("cluster")

if (cluster.isMaster) {

    console.log("Main code here...")
    var fgndWorker = cluster.fork({task:'fgnd'})
    var killWorker = cluster.fork({task:'kill'})

    fgndWorker.on('message', function(msg) {console.log('fgnd worker: ', msg)});
    //killWorker.on('message', function(msg) {console.log('kill worker: ', msg)})
    killWorker.on('message', function(msg) {console.log('kill worker: msg-type:',msg.type,' hwnd:',msg.hwnd)});


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
            //process.send(ref.address(hwnd))
            console.log('fgnd hook :: hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
        }
    )
    // set the actual event hook
    //user32.SetWinEventHook(0x0003, 0x0003, null, pfnWinEventProc, 0, 0, 0 )
    
    const pfnWinEventProc2 = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
        function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
            //process.send(ref.address(hwnd))
            if (event===0x8001 & idObject===0) {
               //console.log('destroy hook :: hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
               process.send({type:'kill', hwnd:ref.address(hwnd)});
            } else if (event===0x800C && idObject===0) {
                //console.log('title hook :: hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
                process.send({type:'title', hwnd:ref.address(hwnd)});
            }
        }
    )
    // set the actual event hook
    //user32.SetWinEventHook(32769, 32769, null, pfnWinEventProc2, 0, 0, 0 )

    if (process.env.task=='fgnd') { console.log('fgnd worker reporting!..')
        user32.SetWinEventHook(3, 3, null, pfnWinEventProc, 0, 0, 0 )
    } else if (process.env.task=='kill') { console.log('kill worker reporting!..')
        user32.SetWinEventHook(0x8001, 0x800C, null, pfnWinEventProc2, 0, 0, 0 )
    }

    
    
    // winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
    function getMessage() { return user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
    while (0 != getMessage()) {}

}
