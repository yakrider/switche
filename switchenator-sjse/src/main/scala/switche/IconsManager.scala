package switche

object IconsManager {
   import scala.scalajs.js
   import scala.collection.mutable
   import scala.scalajs.js.annotation.JSImport

   // second version w js only code
   @js.native @JSImport ("../../../../src/main/resources/win-icon-extractor.js", JSImport.Default)
   private object NodeIconExtractor extends js.Object {
      def getIconStringFromHwndLater (hwnd:Int, ctx:String, cb:js.Function3[Int,String,String,Unit]): Unit = js.native
      def getIconStringFromExePathLater (path:String, ctx:String, cb:js.Function3[String,String,String,Unit]): Unit = js.native
      def registerHwndIconStringCallback (cb:js.Function3[Int,Int,String,Unit]): Unit = js.native
      def unregisterHwndIconStringCallback (): Unit = js.native
      def queueIconStringFromHwnd (hwnd:Int): Unit = js.native
   }

   private case class HwndExePathPair (hwnd:Int, path:String)
   private case class IconCacheMapping (cacheIdx:Int, markedStale:Boolean=false)
   private val iconsCache          = mutable.ArrayBuffer[String]() // iconStrs in buffer whose index can be stored in maps by hwnd or path
   private val iconsCacheCheckMap  = mutable.HashMap[String,Int]() // actual iconStr to its location in vector to give others
   private val iconsHwndMap        = mutable.HashMap[HwndExePathPair,IconCacheMapping]()
   private val iconsExePathMap     = mutable.HashMap[String,Int]()
   private val queriedExePathCache = mutable.HashSet[String]()
   private val queriedHwndCache    = mutable.HashMap[Int,HwndExePathPair]()

   // this is the first version of node hwnd icon ext mechanism, used SendMessageA so preserved path ctx (but that could hang!)
   private def iconStringFromHwndCallback (hwnd:Int, ctx:String, iconString:String):Unit = {
      if (iconString.nonEmpty) {
         val hwndExePathPair = HwndExePathPair(hwnd,ctx)
         val iconCacheIdx = iconsCacheCheckMap.getOrElseUpdate(iconString,iconsCache.size)
         if (iconCacheIdx == iconsCache.size) {iconsCache.+=(iconString)}
         iconsHwndMap .put ( hwndExePathPair, IconCacheMapping (iconCacheIdx, markedStale=false))
         RenderSpacer.queueSpacedRender()
      } else {
         println (s"got empty icon-string callback for hwnd=${hwnd}, ctx=${ctx} !!")
         checkAndQueueIconExePathQuery(ctx,ctx)
      }
   }
   private def iconStringFromExePathCallback (exePath:String, ctx:String, iconString:String):Unit = {
      if (iconString.nonEmpty) {
         val iconCacheIdx = iconsCacheCheckMap .getOrElseUpdate (iconString,iconsCache.size)
         if (iconCacheIdx == iconsCache.size) { iconsCache.+=(iconString) }
         if (iconsExePathMap .put(exePath,iconCacheIdx) .isDefined) {
            RenderSpacer.queueSpacedRender() // only queue render if new
         }
      } else {
         println (s"got empty icon-string callback for path=${exePath}, ctx=${ctx} !!")
      }
   }
   private def queueIconExePathQuery (path:String, ctx:String) = {
      NodeIconExtractor.getIconStringFromExePathLater (path, ctx, iconStringFromExePathCallback _)
   }
   private def checkAndQueueIconExePathQuery (path:String, ctx:String) = {
      if (!queriedExePathCache.contains(path)) { queriedExePathCache.add(path); queueIconExePathQuery(path,ctx) }
   }

   // second version of node hwnd icon ext mechanism, uses SendMessageCallbackA, didnt preserve path ctx, so look up from hwnd
   private def nodeIconExtractorHwndIconStringCallback (hwnd:Int, hicon:Int, iconString:String):Unit = {
      queriedHwndCache .get(hwnd) .foreach {hepp => iconStringFromHwndCallback (hwnd,hepp.path,iconString) }
   }
   private def initNodeIconExtractor() = NodeIconExtractor.registerHwndIconStringCallback (nodeIconExtractorHwndIconStringCallback _)
   initNodeIconExtractor()

   private def queueIconHwndQuery (hwnd:Int,path:String) = { // println(hwnd +", "+path);
      //NodeIconExtractor.getIconStringFromHwndLater(hwnd,path,iconStringFromHwndCallback _)
      NodeIconExtractor.queueIconStringFromHwnd (hwnd)
   }

   private def hwndExePathPair (e:WinDatEntry) = {
      e.iconOverrideLoc .orElse (e.exePathName.map(_.fullPath)) .map (p => HwndExePathPair(e.hwnd,p))
   }
   def getCachedIcon (e:WinDatEntry) = {
      hwndExePathPair(e) .map { hpp =>
         iconsHwndMap.get(hpp).map(_.cacheIdx) .orElse(iconsExePathMap.get(hpp.path))
      } .flatten .map(iconsCache)
   }
   def removeCachedIconMapping (e:WinDatEntry) : Unit = {
      hwndExePathPair(e) .map { hpp => iconsHwndMap.remove(hpp); queriedHwndCache.remove(hpp.hwnd) }
   }
   def markCachedIconMappingStale (e:WinDatEntry) : Unit = {
      hwndExePathPair(e) .map { hpp =>
         iconsHwndMap .get(hpp) .map (_.copy(markedStale=true)) .foreach (icm => iconsHwndMap.update(hpp,icm));
   } }
   def clearDeadHwnds (wdes:Iterable[WinDatEntry]) = { wdes .foreach (removeCachedIconMapping) }
   def markCachedIconMappingsStale () = { SwitcheState.hMapCur .valuesIterator .foreach (markCachedIconMappingStale) }
   def queueIconRefresh (e:WinDatEntry) = {
      //RibbonDisplay.debugDisplayMsg("icon-refresh")
      markCachedIconMappingStale(e); processFoundHwndExePath(e);
   }


   // this is yet another hack, as some windows, e.g. chrome apps, put up placeholder empty icons before they get done loading
   // since those end up same value, we're just gonna init w those, and if things happen to have such empty icons, we can later choose to
   // say retry on sporadic triggers like fgnd change etc (title change seems to be early enough to still not give icon)
   // note that the data is non-exclusive.. online generators produced different data for 16x16 black transp pngs, incl this one.. ¯\_(ツ)_/¯
   private val empty16x16Image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAklEQVR4AewaftIAAAAPSURBVGMYBaNgFIwCKAAABBAAAY7F3VUAAAAASUVORK5CYII="
   iconsCache.+=(empty16x16Image); iconsCacheCheckMap.put(empty16x16Image,0)

   def processFoundHwndExePath (e:WinDatEntry) : Unit = {
      if (e.shouldExclude.contains(true)) return;
      hwndExePathPair(e) .foreach { hpp =>
         if ( !queriedHwndCache.get(e.hwnd).contains(hpp) ||
              iconsHwndMap.get(hpp) .exists (icm => icm.markedStale || icm.cacheIdx == 0)
         ) {
            queriedHwndCache.put(e.hwnd,hpp);
            if (e.iconOverrideLoc.isEmpty) {
               queueIconHwndQuery (e.hwnd, hpp.path)
               queueUnheardIconsCallbackFallbackCheck (hpp)
            } else {
               checkAndQueueIconExePathQuery (hpp.path, hpp.path)
            }
      } }
   }
   private def queueUnheardIconsCallbackFallbackCheck (hepp: HwndExePathPair) = {
      js.timers.setTimeout (1000) {unheardIconsCallbackFallbackCheck(hepp)}
   }
   private def unheardIconsCallbackFallbackCheck (hpp: HwndExePathPair): Unit = {
      if (!iconsHwndMap.contains(hpp)) {
         println (s"did not hear callback for ${hpp} within check period, falling back to exe icon query")
         checkAndQueueIconExePathQuery (hpp.path, hpp.path)
      }
   }

   def printIconCaches() = {
      println (s"Printing icon caches.. (${iconsCache.size}):");
      iconsCache.foreach(println)
   }

}

