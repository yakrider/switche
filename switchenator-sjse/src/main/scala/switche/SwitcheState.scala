package switche

import scala.collection.mutable
import scala.collection.mutable.{HashMap, LinkedHashMap}
import scala.scalajs.js


case class ExePathName (fullPath:String, name:String)
case class WinDatEntry (
   hwnd:Int, isVis:Option[Boolean]=None, winText:Option[String]=None,
   exePathName:Option[ExePathName]=None, shouldExclude:Option[Boolean]=None, everFgnd:Option[Boolean]=None
)


object SwitcheState {

   var latestTriggeredCallId = 0; var cbCountForCallId = 0;
   var hMapCur = LinkedHashMap[Int,WinDatEntry]();
   var hMapPrior = LinkedHashMap[Int,WinDatEntry]();
   var inGroupedMode = true; var isDismissed = false;
   var curElemId = ""

   def prepForNewEnumWindowsCall (callId:Int) = {
      latestTriggeredCallId = callId; cbCountForCallId = 0;
      hMapPrior = hMapCur; hMapCur = LinkedHashMap[Int,WinDatEntry]();
   }
   def kickPostCbCleanup() = {
      cbCountForCallId += 1; val ccSnap = cbCountForCallId;
      js.timers.setTimeout(80) {delayedTaskListCleanup(ccSnap)}
   }
   def delayedTaskListCleanup (cbCountSnapshot:Int) = {
      if (cbCountSnapshot == cbCountForCallId) { //println (s"delayed prior cache cleanup.. cbCount: $cbCountForCallId")
         hMapPrior = hMapCur;
   } }
   def getExePathName(hwnd:Int) = {
      val pid = WinapiLocal.getWindowThreadProcessId(hwnd)
      val exePath = WinapiLocal.getProcessExeFromPid(pid)
      Some (ExePathName(exePath, exePath.split("""\\""").lastOption.getOrElse("")))
   }
   def getUpdatedDat (dat:WinDatEntry, isListenedUpdate:Boolean=false) = {
      val isVis = Some (WinapiLocal.checkWindowVisible(dat.hwnd)).map(_>0)
      val winText = isVis.filter(identity).map(_ => WinapiLocal.getWindowText(dat.hwnd)).orElse(dat.winText)
      val exePathName = winText.filterNot(_.isEmpty).map(_ => dat.exePathName.orElse(getExePathName(dat.hwnd))).flatten
      val preExclDat = WinDatEntry (dat.hwnd, isVis, winText, exePathName, None)
      val shouldExclFlag = Some ( ExclusionsManager.shouldExclude (preExclDat, if(isListenedUpdate){-1} else{latestTriggeredCallId}) )
      preExclDat.copy (shouldExclude = shouldExclFlag)
   }

   def cbStreamWinQueryCallback (hwnd:Int, callId:Int):Boolean = {
      if (callId > latestTriggeredCallId) {
         println (s"something went screwy.. got win api callback with callId higher than latest sent! .. treating as latest!")
         prepForNewEnumWindowsCall(callId)
      }
      kickPostCbCleanup()
      if (callId == latestTriggeredCallId && !hMapCur.contains(hwnd)) {
         val dat = hMapPrior .get(hwnd) .orElse (Some(WinDatEntry (hwnd))) .map(getUpdatedDat(_)) .get
         hMapCur .put (hwnd, dat)
         if (!dat.shouldExclude.getOrElse(false)) {
            dat.exePathName.map(_.fullPath).foreach(ep => IconsManager.processFoundHwndExePath(hwnd,ep))
            RenderSpacer.queueSpacedRender()
         }
      }
      return (callId <= latestTriggeredCallId) // cancel further callbacks on this callId if its no longer the latest call
   }
   def handleWindowsFgndHwndReport (hwnd:Int):Unit = { //println(s"fgnd report: $hwnd")
      val dat = hMapCur .get(hwnd) .orElse (hMapPrior.get(hwnd)) .orElse (Some(WinDatEntry (hwnd))) .map(getUpdatedDat(_,true)) .get .copy(everFgnd = Some(true))
      val hMapUpdated = LinkedHashMap[Int, WinDatEntry]();
      hMapUpdated.put(hwnd, dat); hMapCur.filterKeys(_ != hwnd).foreach {case (k, v) => hMapUpdated.put(k, v)}
      // if not in middle of query, can resync priorMap, else can just add it there and either it will update in place or append, nbd
      if (hMapCur == hMapPrior) {hMapPrior = hMapUpdated} else {hMapPrior.put(hwnd,dat)}
      hMapCur = hMapUpdated
      if (!dat.shouldExclude.contains(true)) {
         dat.exePathName.map(_.fullPath).foreach(ep => IconsManager.processFoundHwndExePath(hwnd,ep))
         RenderSpacer.queueSpacedRender() // no point being more surgical, as for grouped stuff, everything might have to be reordered anyway
      }
   }
   def handleWindowsObjShownReport (hwnd:Int):Unit = { //println(s"fgnd report: $hwnd")
      if ( !hMapCur.contains(hwnd) || hMapCur(hwnd).isVis.contains(true) || !hMapCur(hwnd).everFgnd.contains(true) ) return;
      var dat = getUpdatedDat(hMapCur(hwnd),true)
      hMapCur.put(hwnd,dat); hMapPrior.put(hwnd,dat)
      if (!dat.shouldExclude.contains(true)) {
         dat.exePathName.map(_.fullPath).foreach(ep => IconsManager.processFoundHwndExePath(hwnd,ep))
         RenderSpacer.queueSpacedRender() // no point being more surgical, as for grouped stuff, everything might have to be reordered anyway
      }
   }
   def handleWindowsObjDestroyedReport (hwnd:Int):Unit = {
      // note that we've made reports of windows being hidden (go to tray etc) come here too
      if (!hMapCur.contains(hwnd) || !hMapCur(hwnd).isVis.contains(true)) return;
      if (ExclusionsManager.selfSelector(hMapCur(hwnd))) return;
      if (!hMapCur(hwnd).shouldExclude.contains(true)) { RenderSpacer.queueSpacedRender() }
      hMapCur.remove(hwnd); hMapPrior.remove(hwnd)
   }
   def handleWindowsTitleChangedReport (hwnd:Int):Unit = {
      if (!hMapCur.contains(hwnd) || !hMapCur(hwnd).isVis.contains(true)) return;
      val winText = WinapiLocal.getWindowText(hwnd)
      val dat = hMapCur(hwnd)
      var updatedDat = dat.copy(winText = Some(winText))
      updatedDat = updatedDat.copy(shouldExclude = Some(ExclusionsManager.shouldExclude(updatedDat)))
      if (updatedDat!=dat) {
         hMapCur.put(hwnd,updatedDat); hMapPrior.put(hwnd,updatedDat)
         if (!updatedDat.shouldExclude.contains(true)) { SwitchePageState.handleTitleUpdate (hwnd,updatedDat) }
      }
   }

   def cbFgndWindowChangeListener (hook:Int, event:Int, hwnd:Int, idObj:Long, idChild:Long, idThread:Int, evTime:Int ):Unit = {
      //HWINEVENTHOOK SetWinEventHook (DWORD eventMin, DWORD eventMax, HMODULE hmodWinEventProc, WINEVENTPROC pfnWinEventProc, DWORD idProcess, DWORD idThread, DWORD dwFlags );
      //WINEVENTPROC void Wineventproc( HWINEVENTHOOK hWinEventHook, DWORD event, HWND hwnd, LONG idObject, LONG idChild, DWORD idEventThread, DWORD dwmsEventTime )
      // EVENT_SYSTEM_FOREGROUND 0x0003
      println (s"...Foreground window change listener: new fgnd window ${hwnd}")
      handleWindowsFgndHwndReport(hwnd)
      // ughh, this win fn callback ^^ seems to require the thread to be waiting on GetMessageA to get this callback.. cant do from here!
      // as workaround, ended up doing a npm 'cluster' based thread in main that sits around listening for this and calls us w new hwnd upon fgnd change!
      // .. note that could use 'worker_threads', but elect doesnt seem to take the old node's experimental flag, nor did node-gyp play well w updated node.. oh well
   }

   object RenderReadyListsManager {
      case class GroupSortingEntry (seenCount:Int, meanPercIdx:Double)
      val grpSortingMap = mutable.HashMap[String,GroupSortingEntry]()
      var (renderList:Seq[WinDatEntry], groupedRenderList:Seq[Seq[WinDatEntry]]) = calcRenderReadyLists()

      def registerEntry (exePath:String, idx:Int, listSize:Int) = {
         val percIdx = idx.toDouble./(listSize)
         grpSortingMap.get(exePath).orElse(Some(GroupSortingEntry(0,0.0))) .foreach { case(ge) =>
            grpSortingMap.put (exePath, GroupSortingEntry (ge.seenCount+1, ge.meanPercIdx.*(ge.seenCount).+(percIdx)./(ge.seenCount+1)) )
      } }
      private def calcRenderReadyLists() = {
         val renderList = hMapCur.values.toSeq .++ ( hMapPrior.values.filterNot{d => hMapCur.contains(d.hwnd)}.toSeq )
         val filtRenderList = ExclusionsManager.filterExclusions (renderList)
         // v1: this bunches groups while keeping ordering of highest in list member, but causes groups to move around
         //val groupedRenderList = filtRenderList.zipWithIndex.groupBy(_._1.exePathName.map(_.fullPath)).values .map(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2).map(_._1)
         // v2: this orders by exePath only, but at least wont cause groups jumping around all the time
         //val groupedRenderList = filtRenderList.groupBy(_.exePathName.map(_.fullPath)).toSeq.sortBy(_._1).map(_._2)
         // v3: this will build a pretty stable but responsive ordering for groups by tracking recents index percentile averages
         filtRenderList .zipWithIndex .foreach { case(d,i) => d.exePathName.map(_.fullPath).foreach {p => registerEntry(p,i,filtRenderList.size)} }
         val groupedRenderList = filtRenderList.groupBy(_.exePathName.map(_.fullPath)).toSeq .sortBy{case(po,l) => (po.map(grpSortingMap.get).flatten.map(_.meanPercIdx), po)}.map(_._2)
         (filtRenderList, groupedRenderList)
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
      // the windows event listeners should handle most change, but this is useful periodically for some clean sweeps?
      if (isDismissed) handleRefreshRequest()
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
      js.timers.setTimeout(1000) {getSelfWindowOpt.map(WinapiLocal.activateWindow)}
   }

   def handleDebugPrintReq() = {
      val nonVisEs = hMapCur.values.filter(!_.isVis.getOrElse(false))
      println (s"Printing non-vis entries (${nonVisEs.size}) :")
      nonVisEs.foreach(println); println()
      val emptyTextEs = hMapCur.values.filter(e => e.isVis==Some(true) && e.winText==Some(""))
      println (s"isVis true empty title entries: (${emptyTextEs.size}) :")
      emptyTextEs.foreach (e => println(e.toString)); println();
      println (s"Printing full data incl excl flags for titled Vis entries:")
      hMapCur.values.filter(e => e.isVis.filter(identity).isDefined && e.winText.filterNot(_.isEmpty).isDefined).foreach(println); println();
      println (s"Printing Group Sorting Entries:")
      RenderReadyListsManager.grpSortingMap.toSeq.sortBy(_._2.meanPercIdx).foreach(println); println();
      IconsManager.printIconCaches(); println()
   }

   def handleGroupModeToggleReq() = { inGroupedMode = !inGroupedMode; SwitcheFacePage.render() }

   def handleElectronHotkeyCall() = { //println ("..electron global hotkey press reported!")
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { isDismissed=false; SwitchePageState.resetFocus(); }
      else { SwitchePageState.focusNextElem() }
   }
   def handleElectronHotkeyGlobalScrollDownCall() = { handleElectronHotkeyCall }
   def handleElectronHotkeyGlobalScrollUpCall() = {
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { isDismissed=false; SwitchePageState.focusBottomElem() }
      else { SwitchePageState.focusPrevElem() }
   }
   def handleElectronHotkeyGlobalScrollEndCall() = {
      if (!isDismissed) { SwitchePageState.handleCurElemActivationReq() }
   }
   def handleElectronFocusEvent() = {}
   def handleElectronBlurEvent() = {}
   def handleElectronShowEvent() = {}
   def handleElectronHideEvent() = {}

   def init() {
      js.Dynamic.global.updateDynamic("handleElectronHotkeyCall")(SwitcheState.handleElectronHotkeyCall _)
      js.Dynamic.global.updateDynamic("handleElectronHotkeyGlobalScrollDownCall")(SwitcheState.handleElectronHotkeyGlobalScrollDownCall _)
      js.Dynamic.global.updateDynamic("handleElectronHotkeyGlobalScrollUpCall")(SwitcheState.handleElectronHotkeyGlobalScrollUpCall _)
      js.Dynamic.global.updateDynamic("handleElectronHotkeyGlobalScrollEndCall")(SwitcheState.handleElectronHotkeyGlobalScrollEndCall _)
      js.Dynamic.global.updateDynamic("handleElectronFocusEvent")(SwitcheState.handleElectronFocusEvent _)
      js.Dynamic.global.updateDynamic("handleElectronBlurEvent")(SwitcheState.handleElectronBlurEvent _)
      js.Dynamic.global.updateDynamic("handleElectronShowEvent")(SwitcheState.handleElectronShowEvent _)
      js.Dynamic.global.updateDynamic("handleElectronHideEvent")(SwitcheState.handleElectronHideEvent _)
      //WinapiLocal.hookFgndWindowChangeListener (cbFgndWindowChangeListener _)
      js.Dynamic.global.updateDynamic("handleWindowsFgndHwndReport")(SwitcheState.handleWindowsFgndHwndReport _)
      js.Dynamic.global.updateDynamic("handleWindowsObjDestroyedReport")(SwitcheState.handleWindowsObjDestroyedReport _)
      js.Dynamic.global.updateDynamic("handleWindowsObjShownReport")(SwitcheState.handleWindowsObjShownReport _)
      js.Dynamic.global.updateDynamic("handleWindowsTitleChangedReport")(SwitcheState.handleWindowsTitleChangedReport _)
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
