
import scala.scalajs.js
import scala.scalajs.js.DynamicImplicits._
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel, JSGlobal, JSImport}
import org.scalajs.dom
import org.scalajs.dom.raw.MouseEvent
import scalatags.JsDom.all._

import js.JSConverters._
import scala.collection.mutable
import scala.collection.mutable.{ArrayBuffer, HashMap, HashSet, LinkedHashMap, LinkedHashSet}


object SwitchenatorSjse extends js.JSApp {
    def main(): Unit = {
        println("Hello from sjseApp..")
        //g.console.log(g.document)
        g.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
        
        SwitcheState.handleRefreshRequest() // fire up first call
        
        // hmm how about keeping this updated say once a sec..
        //js.timers.setInterval(1000) {SwitcheState.handleRefreshRequest()}
        // ugh, ^ not worthwhile.. messes up scroll logic etc too, should just keep to doing when window is recalled back or gets focus etc
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
   def getWindowThreadProcessId (hwnd:Int):Int = js.native
   def getProcessExeFromPid (pid:Int):String = js.native
   
   // from psapi.dll and oleacc.dll
   def getWindowProcessId (hwnd:Int):Int = js.native
   def getProcessExe (hand:Int):String = js.native
   def getProcessExeFromHwnd (hwnd:Int):String = js.native
}


object ExclusionsManager {
   import scala.collection.mutable
   //type ExclFnType = (Int,String,Option[String]) => Boolean
   type ExclFnType = WinDatEntry => Boolean
   
   private object RulesExcluder {
      // NOTE: design is for these to return true IF the entries ARE to be excluded
      val exclInvis: ExclFnType = {!_.isVis.getOrElse(false)}
      val exclEmptyTitles: ExclFnType = {_.winText.getOrElse("").isEmpty}
      
      val titleMatchExclusions = Set[String]()
      val exclTitleMatches: ExclFnType = {_.winText.map(titleMatchExclusions.contains).getOrElse(true)}
      
      val exeMatchExclusions = Set[String](
         "SearchUI.exe", "shakenMouseEnlarger.exe", "ShellExperienceHost.exe",
         "MicrosoftEdgeCP.exe", "MicrosoftEdge.exe", // microshit seems to put all these almost-there windows while actual stuff comes under ApplicationFrameHost
         "WindowsInternal.ComposableShell.Experiences.TextInput.InputApp.exe"
      )
      val exclExeMatches: ExclFnType = {_.exeName.map(exeMatchExclusions.contains).getOrElse(true) }
      
      val exeAndTitleMatchExclusions = Set[(Option[String],Option[String])] (
         //(Some("ShellExperienceHost.exe"), Some("Start"))
         //(Some("ShellExperienceHost.exe"), Some("Windows Shell Experience Host"))
         //(Some("ShellExperienceHost.exe"), Some("New notification"))
         (Some("explorer.exe"), Some("Program Manager")),
         (Some("electron.exe"), Some("Sjs-Electron-Local-JS-Test")),
         (Some("SystemSettings.exe"), Some("Settings")),
         (Some("ApplicationFrameHost.exe"), Some("Microsoft Edge"))
      )
      val exclExeAndTitleMatches: ExclFnType = {e => exeAndTitleMatchExclusions.contains((e.exeName,e.winText)) }
      
      val exclusions = List[ExclFnType] (
         // NOTE: exclInvis and exclEmptyTitles have also been partially built into upstream processing to eliminate pointless winapi queries etc
         //exclInvis, exclEmptyTitles, exclTitleMatches, exclExeMatches, exclTitleAndExeMatches
         exclInvis, exclEmptyTitles, exclExeMatches, exclExeAndTitleMatches
      )
      //def shouldExclude (e:WinDatEntry) = exclInvis(e) || exclEmptyTitles(e) || exclExeMatches(e) || exclTitleMatches(e) || exclExeAndTitleMatches(e)
      def shouldExclude(e:WinDatEntry) = exclusions.exists(_(e))
   }
   object WinampDupExcluder {
      var curCallId = -1; var alreadySeen = false;
      def exclWinampDup (e:WinDatEntry, callId:Int): Boolean = { //println("winamp checked!")
         var shouldExclude = false
         if (curCallId != callId) { curCallId = callId; alreadySeen = false } // reset upon new callId
         if (e.exeName.map(_=="winamp.exe").getOrElse(false)) { shouldExclude = alreadySeen; alreadySeen = true } // update if see winamp
         shouldExclude
      }
   }
   def shouldExclude (e:WinDatEntry, callId:Int) = {
      // if value already calculated, use that, else check other excluders in a short-circuiting manner
      e.shouldExclude.getOrElse ( RulesExcluder.shouldExclude(e) || WinampDupExcluder.exclWinampDup(e,callId) )
   }
   
}

// ugh, gonna use strings for cache entry instead of enums etc for now.. also later, will have to extend w at least icon data
// current cache types: 'exe', 'isVis', 'winText', 'excl'?(no, the rest in cache is enough)
//case class CacheEntry (hwnd:Int, entryType:String, valString:String, cacheStamp:Long)
case class WinDatEntry (hwnd:Int, isVis:Option[Boolean], winText:Option[String], exeName:Option[String], shouldExclude:Option[Boolean])

object SwitcheState {
   var latestTriggeredCallId = 0; var cbCountForCallId = 0;
   var hMapCur = LinkedHashMap[Int,WinDatEntry]();
   var hMapPrior = LinkedHashMap[Int,WinDatEntry]();
   //var wDatCache = HashMap[Int,Map[String,CacheEntry]]()
   var inGroupedMode = true;
   
   def prepForNewEnumWindowsCall (callId:Int) = {
      println (s"starting new win enum-windows call w callId: ${callId}")
      latestTriggeredCallId = callId; cbCountForCallId = 0;
      hMapPrior = hMapCur; hMapCur = LinkedHashMap[Int,WinDatEntry]();
   }
   
   def procStreamWinQueryCallback (hwnd:Int, callId:Int):Boolean = {
      if (callId > latestTriggeredCallId) {
         println (s"something went screwy.. got win api callback with callId higher than latest sent! .. treating as latest!")
         prepForNewEnumWindowsCall(callId)
      }
      cbCountForCallId += 1
      if (callId == latestTriggeredCallId && !hMapCur.contains(hwnd)) {
         val dat = hMapPrior .get(hwnd) .orElse (Some(WinDatEntry (hwnd,None,None,None,None))) .map { priorDat =>
            // for isVis, if prior has data, use it, else queue query
            val isVis = priorDat .isVis .orElse { setAsnycQVisCheck(0,hwnd); None }
            // for winText, if prior has data, use it but queue query too, else query if isVis already true, else we'll handle in isVis cb
            val winText = priorDat .winText .map {wt => setAsnycQWindowText(0,hwnd); wt }
               .orElse { if (isVis.isDefined && true==isVis.get) {setAsnycQWindowText(0,hwnd)}; None }
            // for exePath, if not in cache, we'll query later only as needed, ditto for exclusion flag
            WinDatEntry (hwnd, isVis, winText, priorDat.exeName, priorDat.shouldExclude)
         } .get
         hMapCur .put (hwnd, dat)
      }
      //ElemsDisplay.queueRender() // no point queueing render here as any worthwhile update will trigger queries and callbacks w new info
      return (callId <= latestTriggeredCallId) // cancel further callbacks on this callId if its no longer the latest call
   }
   
   def setAsnycQVisCheck (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcVisCheck(hwnd, WinapiLocal.checkWindowVisible(hwnd))}
   def setAsnycQWindowText (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcWindowText(hwnd, WinapiLocal.getWindowText(hwnd))}
   def setAsnycQModuleFile (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcProcId(hwnd, WinapiLocal.getWindowThreadProcessId(hwnd))}
   
   def cbProcVisCheck (hwnd:Int, isVis:Int) = {
      cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(10){delayedTaskListCleanup(ccSnap)}
      // update the map data, also if we're here then it was previously unknown, so if now its true, then queue query for winText
      hMapCur .get(hwnd) .foreach { d =>
         hMapCur .put (hwnd, d.copy(isVis = Some(isVis>0), shouldExclude = Some(isVis<=0).filter(identity)))
         if (isVis>0) {setAsnycQWindowText(0,hwnd)}
         //ElemsDisplay.queueRender() // no point ordering a render here as anything vis here always triggers a title query
   } }
   def cbProcWindowText (hwnd:Int, winText:String) = {
      cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(10){delayedTaskListCleanup(ccSnap)}
      // update the map data, also if its displayable title (non-empty), queue render, then if exePath is undefined, queue that query too
      hMapCur .get(hwnd) .foreach {d =>
         hMapCur .put (hwnd, d.copy(winText = Some(winText)))
         if (!winText.isEmpty) {
            SwitchFacePage.queueRender();
            if (hMapCur.get(hwnd).map(_.exeName.isEmpty).getOrElse(true)) { setAsnycQModuleFile(0,hwnd) }
         }
   } }
   def cbProcModuleFile (hwnd:Int, exePath:String) = { println(exePath) // if this is obsolete, should clear it out
      cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(10){delayedTaskListCleanup(ccSnap)}
      hMapCur .get(hwnd) .foreach {d =>
         hMapCur .put (hwnd, d.copy(exeName = Some(exePath)))
         if (!exePath.isEmpty) { SwitchFacePage.queueRender() }
   } }
   def cbProcProcId (hwnd:Int, pid:Int) = {
      cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(10){delayedTaskListCleanup(ccSnap)}
      // note that we're putting a Some("") if we dont get a decent path, as we dont want to keep querying it subsequently regardless
      val exePath = WinapiLocal.getProcessExeFromPid(pid)
      //val exeName = exePath.split("""\\""").lastOption.map(_.split("""\.""").reverse.take(2).reverse.mkString(".")).orElse(Some(""))
      val exeName = exePath.split("""\\""").lastOption.orElse(Some(""))
      //println (s"**for hwnd ${hwnd}, got pid ${pid}, which gave file : ${exeNameOpt.getOrElse("")}")
      hMapCur .get(hwnd) .foreach {d =>
         val exeUpdatedEntry = d.copy(exeName = exeName)
         val updatedExclFlag = ExclusionsManager.shouldExclude(exeUpdatedEntry,latestTriggeredCallId)
         hMapCur .put (hwnd, exeUpdatedEntry.copy(shouldExclude = Some(updatedExclFlag)))
         //if (None != exeName && !exeName.get.isEmpty) { ElemsDisplay.queueRender() }
         if (!updatedExclFlag) { SwitchFacePage.queueRender() }
      }
   }
   def delayedTaskListCleanup (cbCountSnapshot:Int) = {
      if (cbCountSnapshot == cbCountForCallId) {
         println (s"triggering delayed cleanup of prior call cache.. cbCount for this call was $cbCountForCallId")
         hMapPrior = hMapCur;
      }
   }
   
   def getRenderList() = {
      val hListComb = hMapCur.values.toSeq .++ ( hMapPrior.values.filterNot{d => hMapCur.contains(d.hwnd)}.toSeq )
      //val renderList = hListComb .filter {d => d.isVis.getOrElse(false) && !d.winText.getOrElse("").isEmpty }
      val renderList = hListComb .filterNot(e => ExclusionsManager.shouldExclude(e,latestTriggeredCallId))
      //println (s"Rendered list count: ${renderList.size}")
      renderList
   }
   def handleRefreshRequest() = {
      latestTriggeredCallId += 1
      prepForNewEnumWindowsCall(latestTriggeredCallId)
      WinapiLocal.streamWindowsQuery (procStreamWinQueryCallback _, latestTriggeredCallId)
      //js.timers.setTimeout(50) {delayedTaskListCleanup()} // no need anymore, now we queue it kick-the-can style on each callback
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
      //val exclEs = Seq[String]();
      //println (s"Printing Excluded entries (${exclEs.size}) :")
      println (s"Printing full data incl excl flags for titled Vis entries:")
      hMapCur.values.filter(e => e.isVis.filter(identity).isDefined && e.winText.filterNot(_.isEmpty).isDefined).foreach(println)
   }
   def handleGroupModeRequest() = { inGroupedMode = !inGroupedMode; SwitchFacePage.render() }
   
   
}

object SwitcheFaceConfig {
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.raw.Element) { e.innerHTML = ""}
   def clearedElem (e:dom.raw.Element) = { e.innerHTML = ""; e }
}

object ElemsDisplay {
   import SwitcheFaceConfig._
   val elemsDiv = div (id:="elemsDiv").render
   var elemsCount = 0

   def getElemsDiv = elemsDiv
   def makeElemBox (e:WinDatEntry, dimExeSpan:Boolean=false) = {
      val exeSpanClass = s"exeSpan${if (dimExeSpan) " dim" else ""}"
      val exeSpan = span (`class`:=exeSpanClass, e.exeName.getOrElse("exe..").toString)
      val icoSpan = span (`class`:="exeIcoSpan", "ico")
      val titleSpan = span (`class`:="titleSpan", e.winText.getOrElse("title").toString)
      div (`class`:="elemBox", exeSpan, nbsp(3), icoSpan, nbsp(), titleSpan, onclick:= {ev:MouseEvent => SwitcheState.handleWindowActivationRequest(e.hwnd)})
   }
   def updateElemsDiv (renderList:Seq[WinDatEntry]) = {
      val elemsList = if (SwitcheState.inGroupedMode) {
         val groupedList = renderList.zipWithIndex.groupBy(_._1.exeName).mapValues(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2._2).map(_._2._1)
         groupedList.map(l => Seq(l.take(1).map(makeElemBox(_,false)),l.tail.map(makeElemBox(_,true))).flatten).flatten
      } else { renderList.map(makeElemBox(_)) }
      clearedElem(elemsDiv) .appendChild (div(elemsList).render)
   }
}

object RibbonDisplay {
   import SwitcheFaceConfig._
   import SwitcheState._
   val countSpan = span (id:="countSpan").render
   def updateCountsSpan (n:Int) = clearedElem(countSpan).appendChild(span(s"($n)").render)
   def getTopRibbonDiv() = {
      val reloadLink = a ( href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a ( href:="#", "Refresh", onclick:={e:MouseEvent => handleRefreshRequest()} )
      val printExclLink = a (href:="#", "ExclPrint", onclick:={e:MouseEvent => handleExclPrintRequest()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleGroupModeRequest()} )
      div (id:="top-ribbon", reloadLink, nbsp(4), refreshLink, nbsp(4), printExclLink, nbsp(4), countSpan, nbsp(4), groupModeLink).render
   }
}


object SwitchFacePage {
   import SwitcheFaceConfig._
   
   def getShellPage () = {
      val topRibbon = RibbonDisplay.getTopRibbonDiv()
      val elemsDiv = ElemsDisplay.getElemsDiv
      val page = div (topRibbon, elemsDiv)
      page.render
   }
   def queueRender() = g.window.requestAnimationFrame({t:js.Any => render()})
   def render() = {
      val renderList = SwitcheState.getRenderList()
      ElemsDisplay.updateElemsDiv(renderList)
      RibbonDisplay.updateCountsSpan(renderList.size)
   }
   
}


