package switche

import org.scalajs.dom
import scalatags.JsDom.all._
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}
import scala.scalajs.js.annotation.JSImport


object SwitchenatorSjse extends js.JSApp {
   def main(): Unit = {
      dom.document.getElementById("scala-js-root-div").appendChild (SwitcheFacePage.getShellPage())
      
      SwitcheState.handleRefreshRequest() // fire up first call
      
      // the fgnd/close/title change listeners should in theory cover everything, but might be useful to periodically clean up random things that might fall through
      js.timers.setInterval(30*1000) {SwitcheState.backgroundOnlyRefreshReq()}
      
   }
}


@js.native @JSImport ("../../../../src/main/resources/win-helper.js", JSImport.Default)
object WinapiLocal extends js.Object {
   // underneath, most use user32.dll, some use kernel32.dll and psapi.dll
   def activateWindow (hwnd:Int):Int = js.native
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
   //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
   //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )
   def hookFgndWindowChangeListener (cb:js.Function7[Int,Int,Int,Long,Long,Int,Int,Unit]):Unit = js.native
}


object IconsManager {
   import scala.collection.mutable
   import scala.scalajs.js.annotation.JSImport

   @js.native @JSImport ("../../../../src/main/resources/win-helper.js", JSImport.Default)
   object IconExtractor extends js.Object {
      def registerIconsCallback (cb:js.Function3[String,String,js.Object,Unit]):Unit = js.native
      def unregisterIconsCallback():Unit = js.native
      def queueIconsQuery (ctxStr:String,path:String):Unit = js.native
   }
   
   def iconsCallback (ctx:String, path:String, encIm:js.Object) = {
      //println(s".. from icon-extractor for ctx:${ctx} path:($path) and base-64 image data:"); //println(encIm.toString);
      iconsCache.put(path,encIm.toString)
      RenderSpacer.queueSpacedRender()
   }
   def init() = IconExtractor.registerIconsCallback (iconsCallback _)
   init()

   def queueIconQuery (path:String) = IconExtractor.queueIconsQuery("",path)
   //def testIconExt() = js.timers.setTimeout(500) {queueIconQuery("""C:\Windows\SysWOW64\explorer.exe""")}
   
   case class IconsCacheEntry (path:String, encIcon:String)
   val iconsCache = mutable.HashMap[String,String]()
   val queriedCache = mutable.HashSet[String]()
   
   def processFoundIconPath (path:String) = { if (!queriedCache.contains(path)) { queriedCache.add(path); queueIconQuery(path) } }
   def getCachedIcon (path:String) = iconsCache.get(path)
   
   def printIconCaches() = {
      println (s"Printing icon caches (${iconsCache.size}):");
      iconsCache.toList.sortBy(_._1).foreach(println)
   }

}



