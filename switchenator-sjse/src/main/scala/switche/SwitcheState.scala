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
   var isDismissed = false; var isAppForeground = true;
   var curElemId = ""
   
   def parseIntoExePathName (path:String) = ExePathName(path, path.split("""\\""").lastOption.getOrElse(""))
   def prepForNewEnumWindowsCall (callId:Int) = {
      latestTriggeredCallId = callId; cbCountForCallId = 0;
      hMapPrior = hMapCur; hMapCur = LinkedHashMap[Int,WinDatEntry]();
   }
   //def okToRenderImages() = true //false //cbCountForCallId > 1000
   def kickPostCbCleanup() = {cbCountForCallId += 1; val ccSnap = cbCountForCallId; js.timers.setTimeout(80){delayedTaskListCleanup(ccSnap)}}
   
   def cbStreamWinQueryCallback (hwnd:Int, callId:Int):Boolean = {
      if (callId > latestTriggeredCallId) {
         println (s"something went screwy.. got win api callback with callId higher than latest sent! .. treating as latest!")
         prepForNewEnumWindowsCall(callId)
      }
      kickPostCbCleanup()
      if (callId == latestTriggeredCallId && !hMapCur.contains(hwnd)) {
         val dat = hMapPrior .get(hwnd) .orElse (Some(WinDatEntry (hwnd))) .map { priorDat =>
            // for isVis, if prior has data, use it, else queue query
            val isVis = priorDat .isVis .orElse { setAsnycQVisCheck(20,hwnd); None }
            // for winText, if prior has data, use it but queue query too, else query if isVis already true, else we'll handle in isVis cb
            val winText = priorDat .winText .map {wt => setAsnycQWindowText(20,hwnd); wt }
               .orElse { if (isVis.isDefined && true==isVis.get) {setAsnycQWindowText(20,hwnd)}; None }
            // for exePath/ico, if not in cache, we'll query later only as needed, ditto for exclusion flag
            WinDatEntry (hwnd, isVis, winText, priorDat.exePathName, priorDat.shouldExclude)
         } .get
         hMapCur .put (hwnd, dat)
      }
      //ElemsDisplay.queueRender() // no point queueing render here as any worthwhile update will trigger queries and callbacks w new info
      return (callId <= latestTriggeredCallId) // cancel further callbacks on this callId if its no longer the latest call
   }
   
   def handleWindowsFgndHwndReport (hwnd:Int):Unit = { //println(s"fgnd report: $hwnd")
      if (hMapCur != hMapPrior) return; // if in middle of updating full report, can ignore indiv fgnd change updates
      // do note that this will update all new/changed fgnd windows, but cant clear dead windows etc, so gotta keep refresh handy for those
      val dat = hMapCur .get(hwnd) .orElse (Some(WinDatEntry (hwnd))) .map { curDat =>
         val isVis = curDat .isVis .orElse { setAsnycQVisCheck(20,hwnd); None }
         val winText = curDat .winText .map {wt => setAsnycQWindowText(20,hwnd); wt }
            .orElse { if (isVis.isDefined && true==isVis.get) {setAsnycQWindowText(20,hwnd)}; None }
         WinDatEntry (hwnd, isVis, winText, curDat.exePathName, curDat.shouldExclude)
      } .get
      if (!dat.shouldExclude.getOrElse(false)) { // just reducing work a little where its pointless
         val hMapUpdated = LinkedHashMap[Int, WinDatEntry]();
         hMapUpdated.put(hwnd, dat);
         hMapCur.filterKeys(_ != hwnd).foreach {case (k, v) => hMapUpdated.put(k, v)}
         hMapCur = hMapUpdated; hMapPrior = hMapCur;
         RenderSpacer.queueSpacedRender()
      }
   }
   def handleWindowsObjDestroyedReport (hwnd:Int):Unit = {
      if (hMapCur != hMapPrior) return; // ignore if in middle of updating full report
      if (hMapCur.contains(hwnd)) {
         if (!hMapCur.get(hwnd).map(_.shouldExclude).flatten.getOrElse(false)) {RenderSpacer.queueSpacedRender()}
         hMapCur.remove(hwnd); hMapPrior.remove(hwnd)
      }
   }
   
   def setAsnycQVisCheck (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcVisCheck(hwnd, WinapiLocal.checkWindowVisible(hwnd))}
   def setAsnycQWindowText (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcWindowText(hwnd, WinapiLocal.getWindowText(hwnd))}
   def setAsnycQModuleFile (t:Int, hwnd:Int) = js.timers.setTimeout(t) {cbProcProcId(hwnd, WinapiLocal.getWindowThreadProcessId(hwnd))}
   
   def cbProcVisCheck (hwnd:Int, isVis:Int) = {
      // update the map data, also if we're here then it was previously unknown, so if now its true, then queue query for winText
      hMapCur .get(hwnd) .foreach { d =>
         hMapCur .put (hwnd, d.copy(isVis = Some(isVis>0), shouldExclude = Some(isVis<=0).filter(identity)))
         if (isVis>0) {setAsnycQWindowText(0,hwnd)}
         //ElemsDisplay.queueRender() // no point ordering a render here as anything vis here always triggers a title query
   } }
   
   def cbProcWindowText (hwnd:Int, winText:String) = {
      // update the map data, also if its displayable title (non-empty), queue render, then if exePath is undefined, queue that query too
      hMapCur .get(hwnd) .foreach {d =>
         if (!winText.isEmpty) {
            if (d.winText!=Some(winText)) { RenderSpacer.queueSpacedRender() }
            if (hMapCur.get(hwnd).map(_.exePathName.isEmpty).getOrElse(true)) { setAsnycQModuleFile(20,hwnd) }
         }
         hMapCur .put (hwnd, d.copy(winText = Some(winText)))
   } }
   
   def cbProcProcId (hwnd:Int, pid:Int) = {
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
   
   def cbFgndWindowChangeListener (hook:Int, event:Int, hwnd:Int, idObj:Long, idChild:Long, idThread:Int, evTime:Int ):Unit = {
      //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
      //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )
      // EVENT_SYSTEM_FOREGROUND 0x0003
      println (s"..Foreground window change listener: new fgnd window ${hwnd}")
      handleWindowsFgndHwndReport(hwnd)
      // ughh, this win fn callback ^^ seems to require the thread to be waiting on GetMessageA to get this callback.. cant do from here!
      // as workaround, ended up doing a npm 'cluster' based thread in main that sits around listening for this and calls us w new hwnd upon fgnd change!
      // .. note that could use 'worker_threads', but elect doesnt seem to take the old node's experimental flag, nor did node-gyp play well w updated node.. oh well
   }
   
   def delayedTaskListCleanup (cbCountSnapshot:Int) = {
      if (cbCountSnapshot == cbCountForCallId) {
         //println (s"triggering delayed cleanup of prior call cache.. cbCount for this call was $cbCountForCallId")
         hMapPrior = hMapCur;
      }
   }
   
   object RenderReadyListsManager {
      var (renderList, groupedRenderList) = calcRenderReadyLists()
      def calcRenderReadyLists() = {
         val hListComb = hMapCur.values.toSeq .++ ( hMapPrior.values.filterNot{d => hMapCur.contains(d.hwnd)}.toSeq )
         val renderList = hListComb .filterNot(e => ExclusionsManager.shouldExclude(e,latestTriggeredCallId))
         val groupedRenderList = renderList.zipWithIndex.groupBy(_._1.exePathName.map(_.fullPath)).values.map(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2).map(_._1)
         //println((renderList.size, groupedRenderList.size))
         (renderList,groupedRenderList)
      }
      def updateRenderReadyLists() = {val t = calcRenderReadyLists(); renderList = t._1; groupedRenderList = t._2}
   }
   def updateRenderReadyLists() = RenderReadyListsManager.updateRenderReadyLists()
   def getRenderList() = RenderReadyListsManager.renderList
   def getGroupedRenderList() = RenderReadyListsManager.groupedRenderList
   
   def handleRefreshRequest():Unit = { //println (s"refresh called! @${js.Date.now()}")
      latestTriggeredCallId += 1
      prepForNewEnumWindowsCall(latestTriggeredCallId)
      js.timers.setTimeout(250) {RenderSpacer.queueSpacedRender()} // mandatory repaint per refresh, simpler this way to catch only ordering changes
      WinapiLocal.streamWindowsQuery (cbStreamWinQueryCallback _, latestTriggeredCallId)
   }
   def backgroundOnlyRefreshReq() = {
      // the windows fgnd/close listeners should handle most change, but this is useful periodically for some clean sweeps?
      //if (isDismissed) handleRefreshRequest()
   }
   
   def handleWindowActivationReq(hwnd:Int):Unit = {
      // note that win rules to allow switching require the os to register our switche app processing the most recent ui input (which would've triggered this)
      // hence calling this immediately here can actually be flaky, but putting it on a small timeout seems to make it a LOT more reliable!
      // note also, that the set foreground doesnt bring back minimized windows, which requires showWindow, currently handled by js
      //WinapiLocal.activateWindow(hwnd)
      js.timers.setTimeout(25) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      if (SwitchePageState.inSearchState) {SwitchePageState.exitSearchState()}
      js.timers.setTimeout(80) {isDismissed = true; getSelfWindowOpt.map(WinapiLocal.hideWindow)}
      //js.timers.setTimeout(150) {handleRefreshRequest()}
   }
   def getSelfWindowOpt() = {
      hMapPrior.values.filter(ExclusionsManager.selfSelector).headOption.map(_.hwnd)
   }
   def handleSelfWindowHideReq() = {
      // want to make sure focus is returned to the window we were supposed to have active
      js.timers.setTimeout(50) {SwitchePageState.recentsIdsVec.headOption.map(SwitchePageState.idToHwnd).map(WinapiLocal.activateWindow)}
      js.timers.setTimeout(20) {isDismissed = true; getSelfWindowOpt.map(WinapiLocal.hideWindow)}
      //js.timers.setTimeout(250) {handleRefreshRequest()} // useful to clean up on closed windows etc although the fgnd listener does update the rest
   }
   def handleWindowCloseReq(hwnd:Int) = {
      // we try and activate the window first so it doesnt just die in the bkg, then send close, then after some delay, a refresh to update
      js.timers.setTimeout(30) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(80) {WinapiLocal.closeWindow(hwnd)} // sadly, this can take a while, if the window even agrees to close!
      js.timers.setTimeout(120) {WinapiLocal.closeWindow(hwnd)}
      //js.timers.setTimeout(250) {handleRefreshRequest()}
      //js.timers.setTimeout(400) {handleRefreshRequest()} // and it can take even longer for it to get picked up from win calls, esp for some os windows
      //js.timers.setTimeout(600) {handleRefreshRequest()}
      // making this ^ call and elsewhere to clear out closed windows, but maybe could get rid of these if could setup a window destroyed listener
   }
   def handleWindowShowReq(hwnd:Int) = { // useful for inspection/closing.. brings that window to top, then brings ourselves back
      js.timers.setTimeout(30) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(800) {getSelfWindowOpt.map(WinapiLocal.activateWindow)}
   }
   
   def handleExclPrintReq() = {
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
   
   def handleGroupModeToggleReq() = { inGroupedMode = !inGroupedMode; SwitcheFacePage.render() }

   def handleElectronHotkeyCall() = { //println ("..electron global hotkey press reported!")
      if (isDismissed || !isAppForeground) {
         isDismissed=false; isAppForeground = true; //SwitcheFacePage.render();
         SwitchePageState.triggerHoverLockTimeout(); SwitchePageState.resetFocus();
         //handleRefreshRequest()
      }
      else { SwitchePageState.focusNextElem() }
   }
   def handleElectronFocusEvent() = { //println(s"app is focused! @${js.Date.now()}")
      if (!isAppForeground) {
         isDismissed=false; isAppForeground = true; //SwitcheFacePage.render();
         SwitchePageState.triggerHoverLockTimeout();
         //handleRefreshRequest()
      }
   }
   def handleElectronBlurEvent() = {isAppForeground = false;}
   def handleElectronShowEvent() = {}
   def handleElectronHideEvent() = {}
   
   def init() {
      js.Dynamic.global.updateDynamic("handleElectronHotkeyCall")(SwitcheState.handleElectronHotkeyCall _)
      js.Dynamic.global.updateDynamic("handleElectronFocusEvent")(SwitcheState.handleElectronFocusEvent _)
      js.Dynamic.global.updateDynamic("handleElectronBlurEvent")(SwitcheState.handleElectronBlurEvent _)
      js.Dynamic.global.updateDynamic("handleElectronShowEvent")(SwitcheState.handleElectronShowEvent _)
      js.Dynamic.global.updateDynamic("handleElectronHideEvent")(SwitcheState.handleElectronHideEvent _)
      //WinapiLocal.hookFgndWindowChangeListener (cbFgndWindowChangeListener _)
      js.Dynamic.global.updateDynamic("handleWindowsFgndHwndReport")(SwitcheState.handleWindowsFgndHwndReport _)
      js.Dynamic.global.updateDynamic("handleWindowsObjDestroyedReport")(SwitcheState.handleWindowsObjDestroyedReport _)
   }
   init()
}



object RenderSpacer {
   // so to many requestAnimationFrame interspersed are taking a lot of time, as they each are upto 100ms, so gonna bunch them up too
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d // js.Date.now()
   def queueSpacedRender () = {
      val targStamp = js.Date.now + minRenderSpacing
      if ( js.Date.now >= (lastRenderTargStamp-slop)) {
         lastRenderTargStamp = js.Date.now + minRenderSpacing
         js.timers.setTimeout (minRenderSpacing) {js.Dynamic.global.window.requestAnimationFrame({t:js.Any => SwitcheFacePage.render()})}

   } }
}
