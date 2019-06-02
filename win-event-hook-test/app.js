const cluster = require("cluster")

/* copied from winuser.h

#define EVENT_SYSTEM_FOREGROUND         0x0003
#define EVENT_SYSTEM_MINIMIZESTART      0x0016
#define EVENT_SYSTEM_MINIMIZEEND        0x0017

#define EVENT_OBJECT_CREATE                 0x8000  // hwnd + ID + idChild is created item
#define EVENT_OBJECT_DESTROY                0x8001  // hwnd + ID + idChild is destroyed item
#define EVENT_OBJECT_SHOW                   0x8002  // hwnd + ID + idChild is shown item
#define EVENT_OBJECT_HIDE                   0x8003  // hwnd + ID + idChild is hidden item

#define EVENT_OBJECT_NAMECHANGE             0x800C  // hwnd + ID + idChild is item w/ name change

*/

if (cluster.isMaster) {

    console.log("Main code here...")
    var fgndWorker = cluster.fork({task:'fgnd'})
    var killWorker = cluster.fork({task:'kill'})

    fgndWorker.on('message', function(msg) {console.log('fgnd worker: msg-type:',msg.type,' hwnd:',msg.hwnd)});
    killWorker.on('message', function(msg) {console.log('kill worker: msg-type:',msg.type,' hwnd:',msg.hwnd)});

    //worker.send({msgFromMaster: 'This is from master ' + process.pid + ' to worker ' + worker.pid + '.'});
    setInterval ( function() { fgndWorker.send('msg from master!') }, 1000 ); // just to see if it can receive stuff while waiting on messages


} else {
    const ffi = require("ffi")
    const ref = require("ref")
    
    const user32 = ffi.Library("user32", {
        //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
        //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )  
        SetWinEventHook: ["int", ["int", "int", "pointer", "pointer", "int", "int", "int"]],
        GetMessageA: ["bool", ['pointer', "int", "uint", "uint"]],
        PeekMessageA: ["bool", ['pointer', "int", "uint", "uint", "uint"]]
    })

    var stop = false;

    //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )  
    const pfnWinEventProc = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
        function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
            //console.log('destroy hook :: event:',event.toString(16),' hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
            if (idObject===0) { // only track at window level
                //console.log('fgnd hook :: event:',event.toString(16),' hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
                if (event===0x0003) {
                    console.log('asap printout'); stop = true; console.log(stop)
                    setTimeout(function(){console.log('delayed printout')},0)
                   process.send({type:'fgnd', hwnd:ref.address(hwnd)});
                } else if (event===0x0016) {
                   process.send({type:'minimize', hwnd:ref.address(hwnd)});
                } else if (event===0x0017) {
                   process.send({type:'minimize-end', hwnd:ref.address(hwnd)});
                }
            }

        }
    )
    // set the actual event hook
    //user32.SetWinEventHook(0x0003, 0x0003, null, pfnWinEventProc, 0, 0, 0 )
    
    const pfnWinEventProc2 = ffi.Callback("void", ["pointer", "int", 'pointer', "long", "long", "int", "int"],
        function (hWinEventHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime) {
            //console.log('destroy hook :: event:',event,' hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
            if (idObject===0) { // only track at window level
                //console.log('x0800? hook :: event:',event.toString(16),' hwnd:',ref.address(hwnd),' id:',idObject,' idChild:',idChild);
                if (event===0x8001) {
                process.send({type:'kill', hwnd:ref.address(hwnd)});
                } else if (event===0x8002) {
                    process.send({type:'show', hwnd:ref.address(hwnd)});
                } else if (event===0x8003) {
                    process.send({type:'hide', hwnd:ref.address(hwnd)});
                } else if (event===0x800C) {
                    process.send({type:'title', hwnd:ref.address(hwnd)});
                }
            }
        }
    )
    // set the actual event hook
    //user32.SetWinEventHook(32769, 32769, null, pfnWinEventProc2, 0, 0, 0 )

    if (process.env.task=='fgnd') { console.log('fgnd worker reporting!..')
        user32.SetWinEventHook(0x0003, 0x0017, null, pfnWinEventProc, 0, 0, 0 )
    } else if (process.env.task=='kill') { console.log('kill worker reporting!..')
        user32.SetWinEventHook(0x8001, 0x800C, null, pfnWinEventProc2, 0, 0, 0 )
    }

    process.on('message', function(msg) {
        console.log('Worker ' + process.pid + ' received message from master.', msg);
    });
    
    
    // winapi requires the thread to be waiting on getMessage to get win-event-hook callbacks!
    function getMessage() { user32.GetMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0) }
    function peekMessage() { user32.PeekMessageA (ref.alloc(ref.refType(ref.types.void)), null, 0, 0, 1) } // huh, doesnt work
    //while (0 != getMessage()) {}
    //function checkMessageAndRequeue() { getMessage(); setTimeout ( function() {checkMessageAndRequeue,10} ) }
    //function checkMessageAndRequeue() { peekMessage(); setTimeout ( function() {checkMessageAndRequeue,10} ) }
    //checkMessageAndRequeue()
    function callFnAndRequeue (fn,requeueDelay) { fn(); setTimeout ( function(){fn()}, requeueDelay ) }
    //function callFnAndRequeue (fn,requeueDelay) { fn(); console.log(stop); if (false==stop) { console.log('requeueing'); setTimeout ( function() {fn, requeueDelay} ) } }
    callFnAndRequeue (getMessage, 10000)
    //callFnAndRequeue (peekMessage, 10)
    //setInterval ( getMessage, 10 )


}
