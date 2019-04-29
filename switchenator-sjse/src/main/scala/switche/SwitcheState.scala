package switche

import scala.collection.mutable.{HashMap,LinkedHashMap}
import scala.scalajs.js


case class ExePathName (fullPath:String, name:String)
case class WinDatEntry (
   hwnd:Int, isVis:Option[Boolean]=None, winText:Option[String]=None,
   exePathName:Option[ExePathName]=None, shouldExclude:Option[Boolean]=None
)


object SwitcheState {
   
   var latestTriggeredCallId = 0; var cbCountForCallId = 0;
   var hMapCur = LinkedHashMap[Int,WinDatEntry]();
   var hMapPrior = LinkedHashMap[Int,WinDatEntry]();
   var inGroupedMode = true;
   var isDismissed = false;
   
   def parseIntoExePathName (path:String) = ExePathName(path, path.split("""\\""").lastOption.getOrElse(""))
   def prepForNewEnumWindowsCall (callId:Int) = {
      latestTriggeredCallId = callId; cbCountForCallId = 0;
      hMapPrior = hMapCur; hMapCur = LinkedHashMap[Int,WinDatEntry]();
   }
   def okToRenderImages() = true //false //cbCountForCallId > 1000
   def kickPostCbCleanup() = {cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(250){delayedTaskListCleanup(ccSnap)}}
   
   def procStreamWinQueryCallback (hwnd:Int, callId:Int):Boolean = {
      if (callId > latestTriggeredCallId) {
         println (s"something went screwy.. got win api callback with callId higher than latest sent! .. treating as latest!")
         prepForNewEnumWindowsCall(callId)
      }
      cbCountForCallId += 1
      if (callId == latestTriggeredCallId && !hMapCur.contains(hwnd)) {
         val dat = hMapPrior .get(hwnd) .orElse (Some(WinDatEntry (hwnd))) .map { priorDat =>
            // for isVis, if prior has data, use it, else queue query
            val isVis = priorDat .isVis .orElse { setAsnycQVisCheck(0,hwnd); None }
            // for winText, if prior has data, use it but queue query too, else query if isVis already true, else we'll handle in isVis cb
            val winText = priorDat .winText .map {wt => setAsnycQWindowText(0,hwnd); wt }
               .orElse { if (isVis.isDefined && true==isVis.get) {setAsnycQWindowText(0,hwnd)}; None }
            // for exePath/ico, if not in cache, we'll query later only as needed, ditto for exclusion flag
            WinDatEntry (hwnd, isVis, winText, priorDat.exePathName, priorDat.shouldExclude)
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
      kickPostCbCleanup()
      // update the map data, also if we're here then it was previously unknown, so if now its true, then queue query for winText
      hMapCur .get(hwnd) .foreach { d =>
         hMapCur .put (hwnd, d.copy(isVis = Some(isVis>0), shouldExclude = Some(isVis<=0).filter(identity)))
         if (isVis>0) {setAsnycQWindowText(0,hwnd)}
         //ElemsDisplay.queueRender() // no point ordering a render here as anything vis here always triggers a title query
   } }
   
   def cbProcWindowText (hwnd:Int, winText:String) = {
      kickPostCbCleanup()
      // update the map data, also if its displayable title (non-empty), queue render, then if exePath is undefined, queue that query too
      hMapCur .get(hwnd) .foreach {d =>
         if (!winText.isEmpty) {
            if (d.winText!=Some(winText)) { RenderSpacer.queueSpacedRender() }
            if (hMapCur.get(hwnd).map(_.exePathName.isEmpty).getOrElse(true)) { setAsnycQModuleFile(0,hwnd) }
         }
         hMapCur .put (hwnd, d.copy(winText = Some(winText)))
   } }
   
   def cbProcProcId (hwnd:Int, pid:Int) = {
      kickPostCbCleanup()
      val exePath = WinapiLocal.getProcessExeFromPid(pid)
      hMapCur .get(hwnd) .foreach {d =>
         val exePathName = Some(parseIntoExePathName(exePath))
         val exeUpdatedEntry = d.copy(exePathName = exePathName)
         val updatedShouldExclFlag = ExclusionsManager.shouldExclude(exeUpdatedEntry,latestTriggeredCallId)
         hMapCur .put (hwnd, exeUpdatedEntry.copy(shouldExclude = Some(updatedShouldExclFlag)))
         if (updatedShouldExclFlag!=true) {
            exePathName.map(_.fullPath).foreach(IconsManager.processFoundIconPath)
            RenderSpacer.queueSpacedRender()
      } }
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
   
   def handleRefreshRequest():Unit = { //println ("refresh called!")
      latestTriggeredCallId += 1
      prepForNewEnumWindowsCall(latestTriggeredCallId)
      js.timers.setTimeout(100) {RenderSpacer.queueSpacedRender()} // mandatory repaint per refresh, simpler this way to catch only ordering changes
      WinapiLocal.streamWindowsQuery (procStreamWinQueryCallback _, latestTriggeredCallId)
      //js.timers.setTimeout(50) {delayedTaskListCleanup()} // no need anymore, now we queue it kick-the-can style on each callback
   }
   
   def handleWindowActivationRequest(hwnd:Int):Unit = {
      // note that win rules to allow switching require the os to register our switche app processing the most recent ui input (which would've triggered this)
      // hence calling this immediately here can actually be flaky, but putting it on a small timeout seems to make it a LOT more reliable!
      // note also, that the set foreground doesnt bring back minimized windows, which requires showWindow, currently handled by js
      //WinapiLocal.activateWindow(hwnd)
      js.timers.setTimeout(10) {WinapiLocal.activateWindow(hwnd)}
      //js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      // also might as well queue up a refresh as things change, even if the app window might be going away anyway
      js.timers.setTimeout(25) {handleSelfWindowHideRequest()}
      js.timers.setTimeout(50) {handleRefreshRequest()}
   }
   def handleSelfWindowHideRequest() = {
      isDismissed = true
      hMapPrior.values.filter(ExclusionsManager.selfSelector).headOption.map(_.hwnd).map(WinapiLocal.hideWindow)
   }
   
   def handleExclPrintRequest() = {
      //val nonVisEs = hMapCur.values.filter(!_.isVis.getOrElse(false))
      //println (s"Printing non-vis entries (${nonVisEs.size}) :")
      //nonVisEs.foreach(println)
      
      val emptyTextEs = hMapCur.values.filter(e => e.isVis==Some(true) && e.winText==Some(""))
      println (s"isVis true empty title entries: (${emptyTextEs.size}) :")
      //emptyTextEs.foreach (e => println(e.toString))
      
      println(); println (s"Printing full data incl excl flags for titled Vis entries:")
      hMapCur.values.filter(e => e.isVis.filter(identity).isDefined && e.winText.filterNot(_.isEmpty).isDefined).foreach(println)
      
      println(); IconsManager.printIconCaches()
      
   }
   
   def handleGroupModeRequest() = { inGroupedMode = !inGroupedMode; SwitchFacePage.render() }

   def handleElectronHotkeyCall() = {
      //println ("..electron main reports global hotkey press!")
      if (isDismissed) { handleRefreshRequest() }
      // TODO : this should scroll through tasklist as well
   }
   js.Dynamic.global.updateDynamic("handleElectronHotkeyCall")(SwitcheState.handleElectronHotkeyCall _)
   
   def handleEscPress () = {handleSelfWindowHideRequest()}

   
}

object RenderSpacer {
   // so to many requestAnimationFrame interspersed are taking a lot of time, as they each are upto 100ms, so gonna bunch them up too
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d // js.Date.now()
   def queueSpacedRender () = {
      val targStamp = js.Date.now + minRenderSpacing
      if ( js.Date.now >= (lastRenderTargStamp-slop)) {
         lastRenderTargStamp = js.Date.now + minRenderSpacing
         js.timers.setTimeout (minRenderSpacing) {js.Dynamic.global.window.requestAnimationFrame({t:js.Any => SwitchFacePage.render()})}

   } }
}
