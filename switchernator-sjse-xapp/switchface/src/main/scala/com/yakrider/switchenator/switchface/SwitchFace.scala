package com.yakrider.switchenator.switchface

import electron._

import scala.scalajs.js
import scala.scalajs.js.DynamicImplicits._
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel, JSGlobal}
import org.scalajs.dom
import scalatags.JsDom.all._
//import com.mscharley.nodejs.Require
//import nodejs.Require
//import org. .. how to get require the js.. although looks like thats not really the 'require' kwd commonly used


@JSExport("SwitchFace.SwitchFaceApp")
class SwitchFaceApp (dirName: String, require: js.Function1[String, js.Any]) extends ElectronApp(require) with js.JSApp {

  // Keep a global reference of the window object, if you don't, the window will
  // be closed automatically when the JavaScript object is garbage collected.
  var mainWindow: Option[BrowserWindow] = None

  def createWindow() = {
    //mainWindow = Some(BrowserWindow(JsObject(width = 600, height = 1200)))
    mainWindow = Some(BrowserWindow(JsObject(width = 1200, height = 1200)))
    mainWindow foreach { window =>
      window.loadURL("file://" + dirName + "/index.html")
      window.webContents.openDevTools()
      // Emitted when the window is closed to dereference the window object
      // usually you would store windows in an array if your app supports multi windows
      // so this is the time when you should delete the corresponding element.
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

  override def main() = {
    g.console.log("Starting switchenator-scalajs-electron-app...");
    //electronApp.onceReady(createWindow _)
    //setupPlatformCloseAction()
    //SwitchNativeTests.runQuickTestCode(this)
    //println(require("win-helper.js"))
    //println(require("switchface-jsdeps.js"))
    //println(require("WinHelper"))
    println(require)
    println (require("win32-api"))
    println (require("ffi"))
  }

}

@JSExport("SwitchFace.SwitchFaceWindow")
object SwitchFaceWindow {
   //g.document.getElementsByTagName("BODY").asInstanceOf[js.Array[js.Dynamic]].apply(0).style = "background: #eee;"
   g.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
}


object SwitchFacePage {
   def getHelloOutput() = {
      s"Hello! from node ${g.process.versions.node}, Chromium ${g.process.versions.chrome}, and Electron ${g.process.versions.electron}."
   }
   def getShellPage () = {
      val helloOutDiv = div (getHelloOutput)
      val testOutDiv = div ( getTestOutput )
      val page = div ( helloOutDiv, br, testOutDiv, br, s" **done!** " ).render
      page
   }
   def getTestOutput() = { div ( testOutListFiles, testGetActiveWindow ) }
   def testOutListFiles() = SwitchNativeTests.listFiles(".").map(div(_))
   def testGetActiveWindow() = div ( SwitchNativeTests.getActiveWindow().toString )
}


@js.native
@JSGlobal("WinHelper")
object WinHelper extends js.Object {
  def hellofn(): js.Any = js.native
}


object SwitchNativeTests {

@js.native
//@JSGlobal("WinHelper.hellofn") // will complain of hellofn undefined, as it cant find WinHelper presumably despite commonJSName declaration
@JSGlobal("WinHelper.hellofn")
object WinHelper extends js.Object {
  def hellofn(): js.Any = js.native
}

  def runQuickTestCode(app:SwitchFaceApp) = {
    g.console.log("uhh, from SwitchNativeTests for now..")
    g.console.log("node version:", g.process.versions.node)
    g.console.log("chrome version:",  g.process.versions.chrome)
    g.console.log("electron version:", g.process.versions.electron)
    //g.console.log("g is:", g) // will print out a whole pile btw!
    g.console.log("g.WinHelper is:", g.WinHelper) // says undefined
    g.console.log("WinHelper is:", WinHelper) //wont compile
    //g.console.log("hello is:", g.hello) // says undefined
    //println(s"WinHelper from println is: ${WinHelper}")
    println(g)
    println(WinHelper)
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
   //val fs = g.require("fs")
   //val wa = g.require ("win32-api")
   //g.require ("win32-api")


   //val u32 = win32api.U.load()
   //val u32 = g.User32.load()
   //val u32 = g.User32
   //g.console.log(u32)
   //g.console.log(g.require("fs"))
   //g.console.log(g.require("win32-api"))

   //val locJsTest = g.require("win-helper")
   //object WinApiHelper { val winHelper = js.Dynamic.global.require("win-helper") }
   //object WinApiHelper { val winHelper = js.Dynamic.global.require("./win-helper.js") }
   //g.console.log(g.hello)
   //import WinHelper._
   //g.console.log(g.WinHelper.hello)
   //g.console.log(WinHelper.hello)
   //g.console.log(g.require("win-helper").hello)
   //g.console.log(g.require("win-helper").hello)
   //g.console.log(g.require("./win-helper").hello)
   //g.console.log(g.require("win-helper.js").hello)
   //g.console.log(g.require("./win-helper.js").hello)


   //def getActiveWindow() = u32.GetActiveWindow()}
   def getActiveWindow() = {"disabled"}//u32.GetActiveWindow()}

   //def listFiles(path:String):Seq[String] = {fs.readdirSync(path).asInstanceOf[js.Array[String]]}
   def listFiles(path:String):Seq[String] = Seq("disabled") // {fs.readdirSync(path).asInstanceOf[js.Array[String]]}

}




















































