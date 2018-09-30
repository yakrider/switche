package com.yakrider.switchenator.switchface

import electron._

import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global, literal => JsObject}
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}
import org.scalajs.dom
import scalatags.JsDom.all._


@JSExport("SwitchFace.SwitchFaceApp")
class SwitchFaceApp (dirName: String, require: js.Function1[String, js.Any]) extends ElectronApp(require) with js.JSApp {

  // Keep a global reference of the window object, if you don't, the window will
  // be closed automatically when the JavaScript object is garbage collected.
  var mainWindow: Option[BrowserWindow] = None

  def createWindow() = {
    mainWindow = Some(BrowserWindow(JsObject(width = 1400, height = 800)))
    mainWindow foreach { window =>
      window.loadURL("file://" + dirName + "/index.html")
      window.webContents.openDevTools()
      // Emitted when the window is closed to dereference the window object
      // usually you would store windows in an array if your app supports multi windows
      // so this is the time when you should delete the corresponding element.
      window.on("closed"){ () => mainWindow = None }
    }
  }
  override def main() = {
    global.console.log("Starting switchenator-scalajs-electron-app...");
    electronApp.onceReady(createWindow _)
    // on OS X stuff convention is to remain active until user quit action, elsewhere quit on window close
    process.platform.asInstanceOf[String] match {
      case "darwin" =>  { electronApp onActivate {() => if (mainWindow.isEmpty) { createWindow() } } }
      case _ => { electronApp onWindowAllClosed {() => electronApp.quit()} }
    }
  }

}

@JSExport("SwitchFace.SwitchFaceWindow")
object SwitchFaceWindow {
   //global.document.getElementsByTagName("BODY").asInstanceOf[js.Array[js.Dynamic]].apply(0).style = "background: #eee;"
   global.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
}


object SwitchFacePage {
   def getHelloOutput() = {
      s"Hello! from node ${global.process.versions.node}, Chromium ${global.process.versions.chrome}, and Electron ${global.process.versions.electron}."
   }
   def getShellPage () = {
      val page = div ( div(getHelloOutput), s" **tada** " ).render
      page
   }
}
