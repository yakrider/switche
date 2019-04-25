
import scala.scalajs.js
import scala.scalajs.js.DynamicImplicits._
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel, JSGlobal, JSImport}
import org.scalajs.dom
import org.scalajs.dom.raw.MouseEvent
import scalatags.JsDom.all._

import js.JSConverters._
import scala.collection.mutable
import scala.collection.mutable.{ArrayBuffer, HashMap, HashSet, LinkedHashMap, LinkedHashSet, Seq}


object SwitchenatorSjse extends js.JSApp {
    def main(): Unit = {
        println("Hello from sjseApp..")
        g.console.log("uhh, from sjse main for now...")
        //g.console.log(g.document)
        //g.console.log(g.document.getElementById("scala-js-root-div"))
        //g.console.log(SwitchFacePage.getShellPage())
        g.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
        SwitcheState.handleRefreshRequest()
    }
}


@js.native
@JSImport("../../../../src/main/resources/win-helper.js", JSImport.Default)
object WinapiLocal extends js.Object {
   // from user32.dll
   //def printVisibleWindows():Unit = js.native
   def activateTestWindow():Unit = js.native
   def activateWindow (hwnd:Int):Int = js.native
   //def getVisibleWindows(cb:Array[String]=>Unit):Unit = js.native
   def getVisibleWindows (cb:js.Function1[js.Array[String], Unit]):Unit = js.native
   def streamWindowsQuery (cb:js.Function2[Int,Int,Boolean], callId:Int):Unit = js.native
   def checkWindowVisible (hwnd:Int):Int = js.native
   def getWindowText (hwnd:Int):String = js.native
   def getWindowModuleFile (hwnd:Int):String = js.native
   
   // from psapi.dll and oleacc.dll
   def getWindowProcessId (hwnd:Int):Int = js.native
   def getProcessExe (hand:Int):String = js.native
   def getProcessExeFromHwnd (hwnd:Int):String = js.native
}

/*
   -- some thoughts on rules..
      - note that it might be easier to explore these using jna code from the initial switchback project
   - not sure if all things with empty titles can be ignored.. but maybe ok to start out with
   ?- but most things w len/width 0 windows can be ignored, and most, but not all, have empty titles..
      ?- meaning checking title can be an initial shortcut, but if find empty, can go ahead and check win sizes
   - looks like there's 'Desktop' window named 'Program Manager' usually at bottom of stack, not sure if should exclude it
   - there'll be a bunch of other spurious windows explorer windows.. looks like filtering for empty title should clar most/all of those
   - some w hidden windows should always be ignored.. like our own ShakenMouseEnlarger.exe
   - most things from ApplicationFrameHost.exe can prob be ignored
      - especially if they have dup titles w some other windows thing (e.g. Store, Settings)
   - winamp seems to give two windows too, size wise looks like one is for the titlebar-only/windowshade mode.. but doesnt matter which one we keep
   
   -- so anyway, will need some mix of generic and specific rules
      - abstract rule by exe .. specific instantiation for ShakenMouseEnlarger
      - generic rule for empty titles
      - abstract rule by exe/title .. specific inst for explorer 'DesktopWindow'
      - abstract combined rule .. specific inst (dup ApplicationFrameHost), inst (dup winamp)
         - e.g. chained AND rule w/ a generic is dup title, and inst of check exe, maybe even check exe of other dup
         
   - hopefully can avoid having to read win sizes etc.. likely to slow things down etc
   - hopefully dont have to rely on list order either (e.g. that ShakenMouseEnlarger/Start are at top, and the desktop is at bottom
   
   -- as for actual processing strategy..
      -- we want to make approx presentation fast, then increase accuracy as we go along
      - display pre-existing render immediately
      - then maka a EnumWindows call w streaming callbacks w just hwnds
      - have callback handlers (atomically? no, js is single-threaded!) update an ordering holder, or update a new one per winlist call
      - on each callback, check if hwnd already in cache, if so update display w it in right position
         - if not in cache, fire off call for isVis and get Title checks, when those are back, update cache
         - given results, if meet inclusion criteria, update/reorder display, if need more info queue more calls (e.g exe loc)
         - ditto w handling any further calls make async like w results from exe loc lookup
      - for the ones in cache, and now displayed, still fire off lookups, as will want updated title at least
      - as things come back, stuff that doesnt come up shoudl move lower and lower in queue, should prob put a timeout to remove them after some time
         - could also explicitly queue a check for isWindow before removing the ones that dont come back.. not clear if should leave those in cache
         
      - also, since all this might (hopefully) be really fast, and its pointless redoing layout repeatedly w/o display, should call requestAnimationFrame(cb)
         - will let us naively continue making page updates on any winapi callback, while the re-render only happens right when chrome is about to repaint (~16.6ms)
         - also, its just a one time request that get collated till next paint, so everytime we get win callback that precipitates a change, queue it up, thats it
   
 */

object ExclusionManager {
   import scala.collection.mutable
   //type ExclFnType = (Int,String,Option[String]) => Boolean
   type ExclFnType = WinDatEntry => Boolean
   
   object Exclusions {
      val exclEmptyTitles: ExclFnType = {e => e.winText.map(!_.isEmpty).getOrElse(false)}
      
      val titleMatchExclusions = Set[String]("ShakenMouseEnlarger")
      val exclTitleMatches = {e:WinDatEntry => e.winText.filter{t => !titleMatchExclusions.contains(t)}.isDefined}
      
      val exclusions = mutable.ArrayBuffer[ExclFnType] (
         exclEmptyTitles, exclTitleMatches
      )
   }
   def procExclusions (ws:Seq[WinDatEntry]) = {
   
   }
   
}

// ugh, gonna use strings for cache entry instead of enums etc for now.. also later, will have to extend w at least icon data
// current cache types: 'exe', 'isVis', 'winText', 'excl'?(no, the rest in cache is enough)
//case class CacheEntry (hwnd:Int, entryType:String, valString:String, cacheStamp:Long)
case class WinDatEntry (hwnd:Int, isVis:Option[Boolean], winText:Option[String], exePath:Option[String])

object SwitcheState {
   var latestTriggeredCallId = 0; //var curCbCallIndex = 0;
   var hMapCur = LinkedHashMap[Int,WinDatEntry]();
   var hMapPrior = LinkedHashMap[Int,WinDatEntry]();
   //var wDatCache = HashMap[Int,Map[String,CacheEntry]]()
   
   def prepForNewEnumWindowsCall (callId:Int) = {
      println (s"starting new win enum-windows call w callId: ${callId}")
      latestTriggeredCallId = callId
      hMapPrior = hMapCur; hMapCur = LinkedHashMap[Int,WinDatEntry]();
      
      //println (s"debug: processName for 13520: ${WinapiLocal.getProcessExe(13520)}") // --> empty string :(
      //println (s"debug: processId for 13520: ${WinapiLocal.getProcessExeFromHwnd(hwnd)}")
   }
   
   def procStreamWinQueryCallback (hwnd:Int, callId:Int):Boolean = {
      //val shouldQueueRender = false;
      if (callId > latestTriggeredCallId) {
         println (s"something went screwy.. got win api callback with callId higher than latest sent! .. treating as latest!")
         prepForNewEnumWindowsCall(callId)
      }
      if (callId == latestTriggeredCallId && !hMapCur.contains(hwnd)) {
         val dat = hMapPrior .get(hwnd) .orElse (Some(WinDatEntry (hwnd,None,None,None))) .map { priorDat =>
            // for isVis, if prior has data, use it, else query asap
            val isVis = priorDat .isVis .orElse { setAsnycQVisCheck(0,hwnd); None }
            // for winText, if prior has data, use it but queue query too, else query if isVis already true, else we'll handle in isVis cb
            val winText = priorDat .winText .map {wt => setAsnycQWindowText(0,hwnd); wt }
               .orElse { if (isVis.isDefined && true==isVis.get) {setAsnycQWindowText(0,hwnd)}; None }
            // for exePath, if not in cache, we'll query later only as needed
            val exePath = priorDat.exePath
            WinDatEntry (hwnd, isVis, winText, exePath)
         } .get
         hMapCur .put (hwnd, dat)
      }
      //if (shouldQueueRender) ElemsDisplay.queueRender()
      return (callId <= latestTriggeredCallId) // cancel further callbacks on this callId if its no longer the latest call
   }
   
   def setAsnycQVisCheck (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcVisCheck(hwnd, WinapiLocal.checkWindowVisible(hwnd))}
   def setAsnycQWindowText (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcWindowText(hwnd, WinapiLocal.getWindowText(hwnd))}
   //def setAsnycQModuleFile (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcModuleFile(hwnd, WinapiLocal.getWindowModuleFile(hwnd))}
   //def setAsnycQModuleFile (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcModuleFile(hwnd, WinapiLocal.getProcessExeFromHwnd(hwnd))}
   def setAsnycQModuleFile (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcProcId(hwnd, WinapiLocal.getWindowProcessId(hwnd))}
   
   def cbProcVisCheck (hwnd:Int, isVis:Int) = {
      // update the map data, also if we're here then it was previously unknown, so if now its true, then queue query for winText
      hMapCur .get(hwnd) .foreach { d =>
         hMapCur .put (hwnd, d.copy(isVis = Some(isVis>0)))
         if (isVis>0) {setAsnycQWindowText(0,hwnd)}
         //ElemsDisplay.queueRender()
   } }
   def cbProcWindowText (hwnd:Int, winText:String) = {
      // update the map data, also if its displayable title (non-empty), and exePath is undefined, queue that query too
      hMapCur .get(hwnd) .foreach {d =>
         hMapCur .put (hwnd, d.copy(winText = Some(winText)))
         if (!winText.isEmpty) { ElemsDisplay.queueRender(); setAsnycQModuleFile(0,hwnd) }
   } }
   def cbProcModuleFile (hwnd:Int, exePath:String) = { println(exePath)
      hMapCur .get(hwnd) .foreach {d =>
         hMapCur .put (hwnd, d.copy(exePath = Some(exePath)))
         if (!exePath.isEmpty) { ElemsDisplay.queueRender() }
   } }
   def cbProcProcId (hwnd:Int, pid:Int) = { println (s"got pid ${pid}")}
   def delayedTaskListCleanup () = {
      // hmm, prob could just wipe out the whole prior some reasonable time after queries are done as a form of cleanup?
      hMapPrior = hMapCur;
   }
   
   def getRenderList() = {
      /* - first gotta reconcile cur and prior hlists, in case not all the results have streamed back (so supplement new data w old)
         - next run excl tests to filter stuff out (or can do that before reconciliation too, whatever)
         - then do the actual render, incl loading icons etc as need be
      */
      val hListComb = hMapCur.values.toSeq .++ ( hMapPrior.values.filterNot{d => hMapCur.contains(d.hwnd)}.toSeq )
      // tmp filtering logic
      val renderList = hListComb .filter {d => d.isVis.getOrElse(false) && !d.winText.getOrElse("").isEmpty }
      println (s"Rendered list count: ${renderList.size}")
      renderList
   }
   def handleRefreshRequest() = {
      latestTriggeredCallId += 1
      prepForNewEnumWindowsCall(latestTriggeredCallId)
      WinapiLocal.streamWindowsQuery (procStreamWinQueryCallback _, latestTriggeredCallId)
      js.timers.setTimeout(50) {delayedTaskListCleanup()}
   }
   def handleWindowActivationRequest(hwnd:Int) = {
      // note that win rules to allow switching require the os to register our switche app processing the most recent ui input (which would've triggered this)
      // hence calling this immediately here can actually be flaky, but putting it on a small timeout seems to make it a LOT more reliable!
      // note also, that the set foreground doesnt bring back minimized windows, which requires showWindow, currently handled by js
      //WinapiLocal.activateWindow(hwnd)
      js.timers.setTimeout(10) {WinapiLocal.activateWindow(hwnd)}
      //js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
   }
   def handleExclPrintRequest() = {
      //val nonVisEs = hMapCur.values.filter(!_.isVis.getOrElse(false))
      //println (s"Printing non-vis entries (${nonVisEs.size}) :")
      //nonVisEs.foreach(println)
      val emptyTextEs = hMapCur.values.filter(e => e.isVis==Some(true) && e.winText==Some(""))
      println (s"Printing isVis true empty title entries (${emptyTextEs.size}) :")
      emptyTextEs.foreach (e => println(e.toString))
      val exclEs = Seq[String]();
      println (s"Printing Excluded entries (${exclEs.size}) :")
   }
   
   
}

object SwitcheFaceConfig {
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.raw.Element) { e.innerHTML = ""}
   def clearedElem (e:dom.raw.Element) = { e.innerHTML = ""; e }
}
object ElemsDisplay {
   import SwitcheFaceConfig._
   
   def makeElemBox (e:WinDatEntry) = {
      val exeSpan = span (`class`:="exeSpan", e.exePath.map(_.split("""\\""").last.take(12)).getOrElse("exe..").toString)
      val icoSpan = span (`class`:="exeIcoSpan", "ico")
      val titleSpan = span (`class`:="titleSpan", e.winText.getOrElse("title").toString)
      div (`class`:="elemBox", exeSpan, nbsp(3), icoSpan, nbsp(), titleSpan, onclick:= {ev:MouseEvent => SwitcheState.handleWindowActivationRequest(e.hwnd)})
   }
   
   def render() = {
      val elems = SwitcheState.getRenderList.map(makeElemBox)
      clearedElem(SwitchFacePage.elemsDiv).appendChild(div(elems).render)
      clearedElem(SwitchFacePage.countSpan).appendChild(span(s"(${elems.size})").render)
   }
   def queueRender() = g.window.requestAnimationFrame({t:js.Any => render()})
   
}

object SwitchFacePage {
   import SwitcheFaceConfig._
   
   val elemsDiv = div (id:="elemsDiv").render
   //val topRibbon = div (id:="topRibbon").render
   val countSpan = span (id:="countSpan").render
   
   def getTopRibbonDiv() = {
      val reloadLink = a ( href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a ( href:="#", "Refresh", onclick:= {e:MouseEvent => SwitcheState.handleRefreshRequest()} )
      val printExclLink = a (href:="#", "ExclPrint", onclick:= {e:MouseEvent => SwitcheState.handleExclPrintRequest()} )
      div (id:="top-ribbon", reloadLink, nbsp(4), refreshLink, nbsp(4), printExclLink, nbsp(4), countSpan).render
   }
   def getShellPage () = {
      val topRibbon = getTopRibbonDiv()
      val page = div (topRibbon, elemsDiv)
      
      page.render
   }
   
}


