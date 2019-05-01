package switche

import org.scalajs.dom
import scalatags.JsDom.all._
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}
import scala.scalajs.js.annotation.JSImport


object SwitchenatorSjse extends js.JSApp {
   def main(): Unit = {
      println("Hello from sjseApp..")
      dom.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
      
      SwitcheState.handleRefreshRequest() // fire up first call
      
      // hmm how about keeping this updated say once a sec..
      //js.timers.setInterval(1000) {SwitcheState.handleRefreshRequest()}
      // ugh, ^ not worthwhile.. messes up scroll logic etc too, should just keep to doing when window is recalled back or gets focus etc
      // .. another angle, could do similar in a bkg task, but just query foreground window, and update only when that changes
      // .. though if really want to listen for changes, looks like can do that w SetWinEventHook, listening for EVENT_SYSTEM_FOREGROUND
      
      // k, but for now, prob still worthwhile doing like once a minute or so just to keep things somewhat fresh?
      js.timers.setInterval(30*1000) {SwitcheState.backgroundOnlyRefreshReq()}
      // ughh.. still makes it a pita to work w ui for debug/dev etc as the bkg is activated only when 'dismissed'
      
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



