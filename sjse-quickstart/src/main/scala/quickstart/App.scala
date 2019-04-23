package quickstart

import electron._

import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
//import scala.scalajs.js.annotation.{JSImport, JSExport}
import scala.scalajs.js.annotation._

//@JSExportTopLevel("SjseQuickStart.App")
@JSExportTopLevel(name="quickstart.App") @JSExportAll
class App(dirName: String, require: js.Function1[String, js.Any]) extends ElectronApp(require) {
  // Keep a global reference of the window object, if you don't, the window will
  // be closed automatically when the JavaScript object is garbage collected.
  var mainWindow: Option[BrowserWindow] = None

  def createWindow() = {
    // Create the browser window.
    mainWindow = Some(BrowserWindow(JsObject(width = 800, height = 600)))
    mainWindow foreach { window =>
      // and load the index.html of the app.
      window.loadURL("file://" + dirName + "/index.html")

      // Open the DevTools.
      window.webContents.openDevTools()

      // Emitted when the window is closed.
      // Dereference the window object, usually you would store windows
      // in an array if your app supports multi windows, this is the time
      // when you should delete the corresponding element.
      window.on("closed"){ () => mainWindow = None }
    }
  }
  def setupPlatformCloseAction () = {
    // on OS X stuff convention is to remain active until user quit action, elsewhere quit on window close
    process.platform.asInstanceOf[String] match {
      case "darwin" =>  { electronApp onActivate {() => if (mainWindow.isEmpty) { createWindow() } } }
      case _ => { electronApp onWindowAllClosed {() => electronApp.quit()} }
    }
  }
  def quit() = { electronApp.quit() }

  //override def main ():Unit = main(Array[String]())

  @JSExport
  def main(args:Array[String]):Unit = {

    g.console.log("Starting scalajs-electron-app...");

    //electronApp.onceReady(createWindow _)
    //setupPlatformCloseAction()

    QuickTestObject.runQuickTestCode(this)
    
  }

}

//@JSExportTopLevel("SjseQuickStart.Window")
@JSExportTopLevel(name="quickstart.Window") @JSExportAll
class Window {
  scala.scalajs.js.Dynamic.global.document.getElementsByTagName("BODY").asInstanceOf[js.Array[js.Dynamic]].apply(0).style = "background: #eee;"
}


@js.native
@JSImport("./win-helper.js", JSImport.Default)
object LocalJs extends js.Object {
   def hello():String = js.native
}


object QuickTestObject {

  def runQuickTestCode(app:App) = {
    println ("uhh, from SwitchNativeTests for now..")
    g.console.log("uhh, from SwitchNativeTests for now..")
    g.console.log("node version:", g.process.versions.node)
    g.console.log("chrome version:",  g.process.versions.chrome)
    g.console.log("electron version:", g.process.versions.electron)
    println ("from local js : " + LocalJs.hello())
    //g.console.log("g is:", g) // will print out a whole pile btw!
    //g.console.log("g.WinHelper is:", g.WinHelper) // says undefined
    //g.console.log("WinHelper is:", WinHelper) //wont compile
    //g.console.log("hello is:", g.hello) // says undefined
    //println(s"WinHelper from println is: ${WinHelper}")
    //println(g)
    //println(WinHelper)
    //println(hello)
    //println (hello)
    //println(s"hello from println is: ${hello}")
    //println(s"hello from println is: ${hello.hello}")
    //println(s"WinHelper.hello() from println is : ${WinHelper.hello()}") // throws error if WinHelper is undefined
    //g.console.log(WinHelper.hello())
    //g.console.log(g.require("WinHelper"))
    //g.console.log(g.require("win-helper"))

    app.quit()
  }

}