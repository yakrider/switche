package switche

import scala.collection.mutable.LinkedHashMap
import scala.scalajs.js


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
