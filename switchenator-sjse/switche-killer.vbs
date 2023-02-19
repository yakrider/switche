Set ws = CreateObject("WScript.Shell" )
'// ws.Run "npm run-script start", 0, False
'// ws.Run """C:\Program Files\nodejs\node.exe"" ""C:\yakdat\code\git-repos\switchenator\switchenator-sjse\node_modules\.bin\..\electron\cli.js"" ""C:\yakdat\code\git-repos\switchenator\switchenator-sjse\main.js""", 0, False
ws.Run "wmic process where ExecutablePath=""C:\\yakdat\\code\\git-repos\\switchenator\\switchenator-sjse\\node_modules\\electron\\dist\\electron.exe"" delete", 0, True