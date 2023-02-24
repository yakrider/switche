package switche

import org.scalajs.dom
import scalatags.JsDom.all._
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}
import scala.scalajs.js.annotation.JSImport


object SwitchenatorSjse extends js.JSApp {
   def main(): Unit = {
      dom.document .getElementById("scala-js-root-div") .appendChild (SwitcheFacePage.getShellPage())

      SwitcheState.handleReq_Refresh() // fire up first call

      // the fgnd/close/title change listeners should in theory cover everything, but might be useful to periodically clean up random things that might fall through
      js.timers.setInterval(30*1000) {SwitcheState.handleReq_RefreshIdle()}

   }
}


@js.native @JSImport ("../../../../src/main/resources/win-helper.js", JSImport.Default)
object WinapiLocal extends js.Object {
   // underneath, most use user32.dll, some use kernel32.dll and psapi.dll
   def activateWindow (hwnd:Int):Int = js.native
   def minimizeWindow (hwnd:Int):Int = js.native
   //def getVisibleWindows (cb:js.Function1[js.Array[String], Unit]):Unit = js.native
   //def printVisibleWindows():Unit = js.native
   def streamWindowsQuery (cb:js.Function2[Int,Int,Boolean], callId:Int):Unit = js.native
   def checkWindowVisible (hwnd:Int):Int = js.native
   def getWindowText (hwnd:Int):String = js.native
   def showWindow (hwnd:Int):Int = js.native
   def hideWindow (hwnd:Int):Int = js.native
   def closeWindow (hwnd:Int):Int = js.native
   def getWindowThreadProcessId (hwnd:Int):Int = js.native
   def getProcessExeFromPid (pid:Int):String = js.native
   def getWindowProcessId (hwnd:Int):Int = js.native
   def getProcessExe (hand:Int):String = js.native
   def getProcessExeFromHwnd (hwnd:Int):String = js.native
   def checkWindowCloaked (hwnd:Int):Int = js.native
   //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
   //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )
   def hookFgndWindowChangeListener (cb:js.Function7[Int,Int,Int,Long,Long,Int,Int,Unit]):Unit = js.native
}

