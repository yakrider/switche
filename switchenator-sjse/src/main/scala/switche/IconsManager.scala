package switche

object IconsManager {
   import scala.scalajs.js
   import scala.collection.mutable
   import scala.scalajs.js.annotation.JSImport

   // second version w js only code
   @js.native @JSImport ("../../../../src/main/resources/win-icon-extractor.js", JSImport.Default)
   object NodeIconExtractor extends js.Object {
      def getIconStringFromHwndLater (hwnd:Int, ctx:String, cb:js.Function3[Int,String,String,Unit]): Unit = js.native
      def getIconStringFromExePathLater (path:String, ctx:String, cb:js.Function3[String,String,String,Unit]): Unit = js.native
      def registerHwndIconStringCallback (cb:js.Function3[Int,Int,String,Unit]): Unit = js.native
      def unregisterHwndIconStringCallback (): Unit = js.native
      def queueIconStringFromHwnd (hwnd:Int): Unit = js.native
   }

   // this is the first version of node hwnd icon ext mechanism, used SendMessageA so preserved path ctx (but that could hang!)
   def iconStringFromHwndCallback (hwnd:Int, ctx:String, iconString:String):Unit = {
      if (!iconString.isEmpty) {
         val hwndExePathPair = HwndExePathPair(hwnd,ctx)
         val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
         if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
         iconsHwndMap.put(hwndExePathPair,iconCacheIdx)
         RenderSpacer.queueSpacedRender()
      } else {
         println (s"got empty icon-string callback for hwnd=${hwnd}, ctx=${ctx} !!")
         checkAndQueueIconExePathQuery(ctx)
      }
   }
   def iconStringFromExePathCallback (exePath:String, ctx:String, iconString:String):Unit = {
      val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
      if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
      iconsExePathMap.put(exePath,iconCacheIdx) .map(_ => Unit) .getOrElse(RenderSpacer.queueSpacedRender()) // only queue render if new
   }
   def queueIconExePathQuery (path:String) = {
      NodeIconExtractor.getIconStringFromExePathLater(path,"",iconStringFromExePathCallback _)
   }
   def checkAndQueueIconExePathQuery (path:String) = {
      if (!queriedExePathCache.contains(path)) { queriedExePathCache.add(path); queueIconExePathQuery(path) }
   }

   // second version of node hwnd icon ext mechanism, uses SendMessageCallbackA, didnt preserve path ctx, so look up from hwnd
   def nodeIconExtractorHwndIconStringCallback (hwnd:Int, hicon:Int, iconString:String):Unit = {
      queriedHwndCache.get(hwnd).foreach {case hepp => iconStringFromHwndCallback (hwnd,hepp.exePath,iconString) }
   }
   def initNodeIconExtractor() = NodeIconExtractor.registerHwndIconStringCallback (nodeIconExtractorHwndIconStringCallback _)
   initNodeIconExtractor()

   def queueIconHwndQuery (hwnd:Int,path:String) = { // println(hwnd +", "+path);
      //NodeIconExtractor.getIconStringFromHwndLater(hwnd,path,iconStringFromHwndCallback _)
      NodeIconExtractor.queueIconStringFromHwnd (hwnd)
   }

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
   val queriedHwndCache = mutable.HashMap[Int,HwndExePathPair]()

   // this is yet another hack, as some windows, e.g. chrome apps, put up placeholder empty icons before they get done loading
   // since those end up same value, we're just gonna init w those, and if things happen to have such empty icons, we can later choose to
   // say retry on sporadic triggers like fgnd change etc (title change seems to be early enough to still not give icon)
   // note that the data is non-exclusive.. online generators produced different data for 16x16 black transp pngs, incl this one.. ¯\_(ツ)_/¯
   val empty16x16Image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAklEQVR4AewaftIAAAAPSURBVGMYBaNgFIwCKAAABBAAAY7F3VUAAAAASUVORK5CYII="
   iconsCache.+=(empty16x16Image); iconsCacheCheckMap.put(empty16x16Image,0)

   def processFoundHwndExePath (hwnd:Int, path:String) = {
      val hwndExePathPair = HwndExePathPair(hwnd,path)
      if ( !queriedHwndCache.get(hwnd).contains(hwndExePathPair) || iconsHwndMap.get(hwndExePathPair).contains(0) ) {
         queriedHwndCache.put(hwnd,hwndExePathPair); queueIconHwndQuery(hwnd,path)
         queueUnheardIconsCallbackFallbackCheck (hwndExePathPair)
      }
   }
   def queueUnheardIconsCallbackFallbackCheck (hepp: HwndExePathPair) = {
      js.timers.setTimeout (1000) {unheardIconsCallbackFallbackCheck(hepp)}
   }
   def unheardIconsCallbackFallbackCheck (hepp: HwndExePathPair): Unit = {
      if (!iconsHwndMap.contains(hepp)) {
         println (s"did not hear callback for ${hepp} within check period, falling back to exe icon query")
         checkAndQueueIconExePathQuery (hepp.exePath)
      }
   }

   def printIconCaches() = {
      println (s"Printing icon caches.. (${iconsCache.size}):");
      iconsCache.foreach(println)
   }

}



