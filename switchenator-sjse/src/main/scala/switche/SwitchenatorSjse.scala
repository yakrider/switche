package switche

import org.scalajs.dom
import scalatags.JsDom.all._

import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}
import scala.scalajs.js.annotation.JSImport


object SwitchenatorSjse extends js.JSApp {
   def main(): Unit = {
      println("Hello from sjseApp..")
      //g.console.log(g.document)
      g.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
      
      SwitcheState.handleRefreshRequest() // fire up first call
      
      // hmm how about keeping this updated say once a sec..
      //js.timers.setInterval(1000) {SwitcheState.handleRefreshRequest()}
      // ugh, ^ not worthwhile.. messes up scroll logic etc too, should just keep to doing when window is recalled back or gets focus etc
      
      // k, but for now, prob still worthwhile doing like once a minute or so just to keep things somewhat fresh?
      js.timers.setInterval(60*1000) {SwitcheState.handleRefreshRequest()}
   }
}


@js.native @JSImport ("../../../../src/main/resources/win-helper.js", JSImport.Default)
object WinapiLocal extends js.Object {
   // from user32.dll
   def activateWindow (hwnd:Int):Int = js.native
   //def getVisibleWindows (cb:js.Function1[js.Array[String], Unit]):Unit = js.native
   //def printVisibleWindows():Unit = js.native
   def streamWindowsQuery (cb:js.Function2[Int,Int,Boolean], callId:Int):Unit = js.native
   def checkWindowVisible (hwnd:Int):Int = js.native
   def getWindowText (hwnd:Int):String = js.native
   def getWindowThreadProcessId (hwnd:Int):Int = js.native
   def getProcessExeFromPid (pid:Int):String = js.native
   // from psapi.dll and oleacc.dll
   def getWindowProcessId (hwnd:Int):Int = js.native
   def getProcessExe (hand:Int):String = js.native
   def getProcessExeFromHwnd (hwnd:Int):String = js.native
}


