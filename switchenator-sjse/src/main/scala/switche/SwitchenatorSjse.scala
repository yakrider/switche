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
   
   // second version w js only code
   @js.native @JSImport ("../../../../src/main/resources/win-icon-extractor.js", JSImport.Default)
   object NodeIconExtractor extends js.Object {
      def getIconStringFromHwndLater (hwnd:Int, ctx:String, cb:js.Function3[Int,String,String,Unit]): Unit = js.native
      def getIconStringFromExePathLater (path:String, ctx:String, cb:js.Function3[String,String,String,Unit]): Unit = js.native
   }
   
   def iconsCallback (ctx:String, exePath:String, encIm:js.Object):Unit = {
      //println(s".. from icon-extractor for ctx:${ctx} path:($path) and base-64 image data:"); //println(encIm.toString);
      val iconString = s"data:image/png;base64,${encIm.toString}"
      val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
      if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
      iconsExePathMap.put(exePath,iconCacheIdx)
      RenderSpacer.queueSpacedRender()
   }
   def init() = IconExtractor.registerIconsCallback (iconsCallback _)
   init()
   
   def iconStringFromHwndCallback (hwnd:Int, ctx:String, iconString:String):Unit = {
      if (!iconString.isEmpty) {
         val hwndExePathPair = HwndExePathPair(hwnd,ctx)
         val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
         if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
         iconsHwndMap.put(hwndExePathPair,iconCacheIdx)
         RenderSpacer.queueSpacedRender()
      } else {
         println (s"got empty icon-string callback for hwnd=${hwnd}, ctx=${ctx} !!")
         if (!iconsExePathMap.contains(ctx)) { queueIconExePathQuery(ctx) }
      }
   }
   def iconStringFromExePathCallback (exePath:String, ctx:String, iconString:String):Unit = {
      val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
      if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
      iconsExePathMap.put(exePath,iconCacheIdx)
      RenderSpacer.queueSpacedRender()
   }
   
   //def queueIconExePathQuery (path:String) = { IconExtractor.queueIconsQuery ("",path) }
   def queueIconExePathQuery (path:String) = {
      NodeIconExtractor.getIconStringFromExePathLater(path,"",iconStringFromExePathCallback _)
   }
   def queueIconHwndQuery (hwnd:Int,path:String) = { // println(hwnd +", "+path);
      NodeIconExtractor.getIconStringFromHwndLater(hwnd,path,iconStringFromHwndCallback _)
   }
   //def testIconExt() = js.timers.setTimeout(500) {queueIconExePathQuery("""C:\Windows\SysWOW64\explorer.exe""")}
   
   val iconsCache = mutable.ArrayBuffer[String]() // iconStrs stored in vector so its index can be stored by say hwnd table for efficiency
   val iconsCacheCheckMap = mutable.HashMap[String,Int]() // actual iconStr to its location in vector to give others
   val iconsHwndMap = mutable.HashMap[HwndExePathPair,Int]()
   val iconsExePathMap = mutable.HashMap[String,Int]()
   
   def getCachedIcon (hwnd:Int, path:String) = {
      val hwndExePathPair = HwndExePathPair(hwnd,path)
      iconsHwndMap .get(hwndExePathPair) .orElse(iconsExePathMap.get(path)) .map(iconsCache)
   }
   
   case class HwndExePathPair (hwnd:Int, exePath:String)
   val queriedExePathCache = mutable.HashSet[String]()
   val queriedHwndCache = mutable.HashSet[HwndExePathPair]()
   
   // this is yet another hack, as some windows, e.g. chrome apps, put up placeholder empty icons before they get done loading
   // since those end up same value, we're just gonna init w those, and if things happen to have such empty icons, we can later choose to
   // say retry on sporadic triggers like fgnd change etc (title change seems to be early enough to still not give icon)
   // note that the data is non-exclusive.. online generators produced different data for 16x16 black transp pngs, incl this one.. ¯\_(ツ)_/¯
   val empty16x16Image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAklEQVR4AewaftIAAAAPSURBVGMYBaNgFIwCKAAABBAAAY7F3VUAAAAASUVORK5CYII="
   iconsCache.+=(empty16x16Image); iconsCacheCheckMap.put(empty16x16Image,0)
   
   val whitelist = "chrome.exe,rundll32.exe,ApplicationFrameHost.exe,explorer.exe".split(",").toSet
   def processFoundHwndExePath (hwnd:Int, path:String) = {
      val hwndExePathPair = HwndExePathPair(hwnd,path)
      // the following line should ideally have been used, but it seems to cause unrecoverable lockup when used on everything !!!??
      //if (!queriedHwndCache.contains(hwndExePathPair)) { queriedHwndCache.add(hwndExePathPair); queueIconHwndQuery(hwnd,path) }
      val pathExePart = path.split("""\\""").last
      //if (true) {
      if (whitelist.contains(pathExePart)) {
         if ( !queriedHwndCache.contains(hwndExePathPair) || iconsHwndMap.get(hwndExePathPair).contains(0) ) {
            queriedHwndCache.add(hwndExePathPair); queueIconHwndQuery(hwnd,path)
         }
      } else {
         if (!queriedExePathCache.contains(path)) { queriedExePathCache.add(path); queueIconExePathQuery(path) }
      }
   }

   
   def printIconCaches() = {
      println (s"Printing icon caches.. (${iconsCache.size}):");
      iconsCache.foreach(println)
   }

}



