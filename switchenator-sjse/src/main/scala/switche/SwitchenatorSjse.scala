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
      
      // the fgnd window listener handles most change, but this is useful periodically to clean up on closed windows etc
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
      SwitcheState.kickPostCbCleanup() // sucks that it has squirreled in here, but tight accounting of this helps keep perf tight
      //println(s".. from icon-extractor for ctx:${ctx} path:($path) and base-64 image data:"); //println(encIm.toString);
      iconsCache.put(ctx,encIm.toString) // note here the hack of passing/putting ctx as path instead of dealing better w path convs
      RenderSpacer.queueSpacedRender() // if we just got a new icon, prob gotta queue redraw too
   }
   def init() = IconExtractor.registerIconsCallback (iconsCallback _)
   init()
   def queueIconQuery (path:String) = IconExtractor.queueIconsQuery(path,convWinapiPath(path))
   def testIconExt() = js.timers.setTimeout(500) {queueIconQuery("""C:\Windows\SysWOW64\explorer.exe""")}
   
   case class IconsCacheEntry (path:String, encIcon:String)
   val iconsCache = mutable.HashMap[String,String]()
   val queriedCache = mutable.HashSet[String]()
   
   // ughh, this is a royal pain, but stupid winapi gives paths like /Device/HarddiskVolume5\.. which the IconExtractor can't deal with.. goddamn
   // not sure what longer term soln is, esp if sharing for others, but for now, can hardcode the manipulations necessary
   def convWinapiPath (path:String) = path.replace("""\Device\HarddiskVolume5""","""C:""").replace("""\Device\HarddiskVolume4""","""D:\""")
   
   def processFoundIconPath (path:String) = { if (!queriedCache.contains(path)) { queriedCache.add(path); queueIconQuery(path) } }
   def getCachedIcon (path:String) = iconsCache.get(path)
   
   def printIconCaches() = {
      println (s"Printing icon caches (${iconsCache.size}):");
      iconsCache.toList.sortBy(_._1).foreach(println)
   }

}



