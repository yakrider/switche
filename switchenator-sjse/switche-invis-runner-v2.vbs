Set ws = CreateObject("WScript.Shell" )
'// ws.Run "npm run-script start", 0, False
ws.Run """C:\Program Files\nodejs\node.exe"" ""D:\Sandboxes\git-repos\switchenator\switchenator-sjse\node_modules\.bin\..\electron\cli.js"" ""D:\Sandboxes\git-repos\switchenator\switchenator-sjse\main.js""", 0, False