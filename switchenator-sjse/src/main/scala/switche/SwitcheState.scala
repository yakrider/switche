package switche

import scala.collection.mutable
import scala.collection.mutable.{HashMap, LinkedHashMap}
import scala.scalajs.js


case class ExePathName (fullPath:String, name:String)
case class WinDatEntry (
   hwnd:Int, isVis:Option[Boolean]=None, isUnCloaked:Option[Boolean]=None, winText:Option[String]=None,
   exePathName:Option[ExePathName]=None, shouldExclude:Option[Boolean]=None, everFgnd:Option[Boolean]=None,
   iconOverrideLoc:Option[String]=None
)
case class RenderListEntry (dat:WinDatEntry, y:Int)


object SwitcheState {

   var latestTriggeredCallId = 0; var cbCountForCallId = 0;
   var hMapCur = LinkedHashMap[Int,WinDatEntry]();
   var hMapPrior = LinkedHashMap[Int,WinDatEntry]();
   var inElectronDevMode = false; var inGroupedMode = true; var isDismissed = false;

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
      val isUnCloaked = isVis.filter(identity).map(_ => WinapiLocal.checkWindowCloaked(dat.hwnd)).map(_==0)
      val winText = isUnCloaked.filter(identity).map(_ => WinapiLocal.getWindowText(dat.hwnd)).orElse(dat.winText)
      val exePathName = winText.filterNot(_.isEmpty).map(_ => dat.exePathName.orElse(getExePathName(dat.hwnd))).flatten
      val preExclDat = WinDatEntry (dat.hwnd, isVis, isUnCloaked, winText, exePathName, None)
      val shouldExclFlag = Some ( ExclusionsManager.shouldExclude (preExclDat, if(isListenedUpdate){-1} else{latestTriggeredCallId}) )
      val iconOverrideLoc = shouldExclFlag.filter(_!=true) .flatMap(_ => IconPathOverridesManager.getIconOverridePath(preExclDat))
      preExclDat.copy (shouldExclude = shouldExclFlag, iconOverrideLoc = iconOverrideLoc)
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
            IconsManager.processFoundHwndExePath(dat)
            RenderSpacer.queueSpacedRender()
         }
      }
      return (callId <= latestTriggeredCallId) // cancel further callbacks on this callId if its no longer the latest call
   }
   def procWinReport_FgndHwnd(hwnd:Int):Unit = { //println(s"fgnd report: $hwnd")
      val dat = hMapCur .get(hwnd) .orElse (hMapPrior.get(hwnd)) .orElse (Some(WinDatEntry (hwnd))) .map(getUpdatedDat(_,true)) .get .copy(everFgnd = Some(true))
      val hMapUpdated = LinkedHashMap[Int, WinDatEntry]();
      hMapUpdated.put(hwnd, dat); hMapCur.filterKeys(_ != hwnd).foreach {case (k, v) => hMapUpdated.put(k, v)}
      // if not in middle of query, can resync priorMap, else can just add it there and either it will update in place or append, nbd
      if (hMapCur == hMapPrior) {hMapPrior = hMapUpdated} else {hMapPrior.put(hwnd,dat)}
      hMapCur = hMapUpdated
      if (!dat.shouldExclude.contains(true)) {
         IconsManager.processFoundHwndExePath(dat)
         RenderSpacer.queueSpacedRender() // no point being more surgical, as for grouped stuff, everything might have to be reordered anyway
      }
      // ugh, this sucks, but looks like alt-esc which sends cur win to z-order back has no triggered event other than the next win coming up ..
      // so instead, we'll just try and queue a (relatively light) refresh-request on every fgnd change .. oh well
      //handleReq_Refresh()
      // ^^ otoh, even alt-tab doesnt update z-order on that, and calling it here occasionally causes update races disrupting quick switches (F20)
      //  .. as compromise, we'll call w delay to avoid that .. render is almost 100ms, 50ms spacing, plus other proc .. so 250ms seems ok
      js.timers.setTimeout (250) { handleReq_Refresh() }
   }
   def procWinReport_ObjShown(hwnd:Int):Unit = { //println(s"fgnd report: $hwnd")
      if ( !hMapCur.contains(hwnd) || hMapCur(hwnd).isVis.contains(true) || !hMapCur(hwnd).everFgnd.contains(true) ) return;
      val dat = getUpdatedDat(hMapCur(hwnd), true)
      hMapCur.put(hwnd,dat); hMapPrior.put(hwnd,dat)
      if (!dat.shouldExclude.contains(true)) {
         IconsManager.processFoundHwndExePath(dat)
         RenderSpacer.queueSpacedRender() // no point being more surgical, as for grouped stuff, everything might have to be reordered anyway
      }
   }
   def procWinReport_ObjDestroyed(hwnd:Int):Unit = {
      // note that we've made reports of windows being hidden (go to tray etc) come here too
      if (!hMapCur.contains(hwnd) || !hMapCur(hwnd).isVis.contains(true)) return;
      if (ExclusionsManager.selfSelector(hMapCur(hwnd))) return;
      if (!hMapCur(hwnd).shouldExclude.contains(true)) { RenderSpacer.queueSpacedRender() }
      hMapCur.remove(hwnd); hMapPrior.remove(hwnd)
   }
   def procWinReport_TitleChanged(hwnd:Int):Unit = {
      if (!hMapCur.contains(hwnd) || !hMapCur(hwnd).isVis.contains(true)) return;
      val winText = WinapiLocal.getWindowText(hwnd)
      val dat = hMapCur(hwnd)
      var updatedDat = dat.copy(winText = Some(winText))
      updatedDat = updatedDat.copy(shouldExclude = Some(ExclusionsManager.shouldExclude(updatedDat)))
      if (updatedDat!=dat) {
         hMapCur.put(hwnd,updatedDat); hMapPrior.put(hwnd,updatedDat)
         if (!updatedDat.shouldExclude.contains(true)) { SwitchePageState.handle_TitleUpdate (hwnd,updatedDat) }
      }
   }

   object RenderReadyListsManager {
      case class GroupSortingEntry (seenCount:Int, meanPercIdx:Double)
      val grpSortingMap = mutable.HashMap[String,GroupSortingEntry]()
      var (renderList:Seq[RenderListEntry], groupedRenderList:Seq[Seq[RenderListEntry]]) = calcRenderReadyLists()

      def registerEntry (exePath:String, idx:Int, listSize:Int) = {
         val percIdx = idx.toDouble./(listSize)
         grpSortingMap.get(exePath).orElse(Some(GroupSortingEntry(0,0.0))) .foreach { case(ge) =>
            grpSortingMap.put (exePath, GroupSortingEntry (ge.seenCount+1, ge.meanPercIdx.*(ge.seenCount).+(percIdx)./(ge.seenCount+1)) )
      } }
      private def calcRenderReadyLists() = {
         val renderList = hMapCur.values.toSeq .++ ( hMapPrior.values.filterNot{d => hMapCur.contains(d.hwnd)}.toSeq )
         val filtRenderList = ExclusionsManager.filterExclusions (renderList) .zipWithIndex .map {case (d,i) => RenderListEntry(d,i+1)}
         // v1: this bunches groups while keeping ordering of highest in list member, but causes groups to move around
         //val groupedRenderList = filtRenderList.groupBy(_._1.exePathName.map(_.fullPath)).values .map(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2).map(_._1)
         // v2: this orders by exePath only, but at least wont cause groups jumping around all the time
         //val groupedRenderList = filtRenderList.groupBy(_._1.exePathName.map(_.fullPath)).toSeq.sortBy(_._1).map(_._2)
         // v3: this will build a pretty stable but responsive ordering for groups by tracking recents index percentile averages
         filtRenderList .foreach { e => e.dat.exePathName.map(_.fullPath).foreach {p => registerEntry(p,e.y,filtRenderList.size)} }
         val groupedRenderList = filtRenderList.groupBy(_.dat.exePathName.map(_.fullPath)).toSeq
            .sortBy { case (po,l) => po.map(grpSortingMap.get).flatten.map(_.meanPercIdx) -> po } .map(_._2)
         (filtRenderList, groupedRenderList)
      }
      def updateRenderReadyLists() = {val t = calcRenderReadyLists(); renderList = t._1; groupedRenderList = t._2}
   }
   def updateRenderReadyLists() = RenderReadyListsManager.updateRenderReadyLists()
   def getRenderList() = RenderReadyListsManager.renderList
   def getGroupedRenderList() = RenderReadyListsManager.groupedRenderList

   def handleReq_Refresh():Unit = { //println (s"refresh called! @${js.Date.now()}")
      latestTriggeredCallId += 1
      prepForNewEnumWindowsCall(latestTriggeredCallId)
      js.timers.setTimeout(250) {RenderSpacer.queueSpacedRender()} // mandatory repaint per refresh, simpler this way to catch only ordering changes
      WinapiLocal.streamWindowsQuery (cbStreamWinQueryCallback _, latestTriggeredCallId)
   }
   def handleReq_RefreshIdle() = {
      // the windows event listeners should handle most change, but this is useful periodically for some clean sweeps?
      if (isDismissed) handleReq_Refresh()
   }

   def handleReq_WindowActivation(hwnd:Int):Unit = {
      // note that win rules to allow switching require the os to register our switche app processing the most recent ui input (which would've triggered this)
      // hence calling this immediately here can actually be flaky, but putting it on a small timeout seems to make it a LOT more reliable!
      // note also, that the set foreground doesnt bring back minimized windows, which requires showWindow, currently handled by js
      //WinapiLocal.activateWindow(hwnd)
      js.timers.setTimeout(25) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      if (SwitchePageState.inSearchState) {SwitchePageState.exitSearchState()}
      js.timers.setTimeout(80) {
         isDismissed = true;  SwitchePageState.resetFocus(); // resetting focus now makes it less flickery when its called back up
         getSelfWindowOpt.map(WinapiLocal.hideWindow);
      }
      //js.timers.setTimeout(150) {handleReq_Refresh()}
   }
   def handleReq_WindowMinimize(hwnd:Int):Unit = {
      js.timers.setTimeout(25) {WinapiLocal.minimizeWindow(hwnd)}
      js.timers.setTimeout(60) {WinapiLocal.minimizeWindow(hwnd)}
   }
   def getSelfWindowOpt() = {
      hMapPrior.values.filter(ExclusionsManager.selfSelector).headOption.map(_.hwnd)
   }
   def handleReq_SelfWindowHide() = {
      // want to make sure focus is returned to the window we were supposed to have active
      js.timers.setTimeout(40) {
         SwitchePageState.recentsIdsVec.headOption .map (SwitchePageState.idToHwnd) .foreach (WinapiLocal.activateWindow)
      }
      isDismissed = true; SwitchePageState.resetFocus(); // resetting focus now makes it less flickery when its called back up
      getSelfWindowOpt.map(WinapiLocal.hideWindow)
   }
   def handleReq_WindowClose(hwnd:Int) = {
      // we try and activate the window first so it doesnt just die in the bkg, then send close, then after some delay, a refresh to update
      js.timers.setTimeout(30) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(80) {WinapiLocal.closeWindow(hwnd)} // sadly, this can take a while, if the window even agrees to close!
      js.timers.setTimeout(120) {WinapiLocal.closeWindow(hwnd)}
      //js.timers.setTimeout(250) {handleReq_Refresh()}
      //js.timers.setTimeout(400) {handleReq_Refresh()} // and it can take even longer for it to get picked up from win calls, esp for some os windows
      //js.timers.setTimeout(600) {handleReq_Refresh()}
      // making this ^ call and elsewhere to clear out closed windows, but maybe could get rid of these if could setup a window destroyed listener
   }
   def handleReq_WindowShow(hwnd:Int) = { // useful for inspection/closing.. brings that window to top, then brings ourselves back
      js.timers.setTimeout(30) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(50) {WinapiLocal.activateWindow(hwnd)}
      js.timers.setTimeout(1000) {getSelfWindowOpt.map(WinapiLocal.activateWindow)}
   }

   def handleReq_DebugPrint() = {
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

   def handleReq_GroupModeToggle() = { inGroupedMode = !inGroupedMode; SwitcheFacePage.render() }


   def procAppEvent_DevMode() = {
      //println("heard dev-mode notification call!")
      inElectronDevMode = true
      RibbonDisplay.updateDebugLinks()
   }
   def procAppEvent_Focus() = {}
   def procAppEvent_Blur() = {}
   def procAppEvent_Show() = {}
   def procAppEvent_Hide() = {}

   def procHotkey_Invoke() = { //println ("..electron global hotkey press reported!")
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { isDismissed=false; SwitchePageState.resetFocus(); }
      else { SwitchePageState.focusElem_Next() }
   }
   def procHotkey_ScrollDown() = { procHotkey_Invoke }
   def procHotkey_ScrollUp() = {
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { isDismissed=false; SwitchePageState.focusElem_Bottom() }
      else { SwitchePageState.focusElem_Prev() }
   }
   def procHotkey_ScrollEnd() = {
      if (!isDismissed) { SwitchePageState.handleReq_CurElemActivation() }
   }
   def procHotkey_SilentTabSwitch() = {
      SwitchePageState.handleReq_SecondRecentActivation()
   }
   def procHotkey_ChromeTabsList() = {
      SwitchePageState.handleReq_ChromeTabsListActivation (doTog=true)
   }

   def init() {

      js.Dynamic.global.updateDynamic ("procHotkey_Invoke")           (SwitcheState.procHotkey_Invoke _)
      js.Dynamic.global.updateDynamic ("procHotkey_SilentTabSwitch")  (SwitcheState.procHotkey_SilentTabSwitch _)
      js.Dynamic.global.updateDynamic ("procHotkey_ChromeTabsList")   (SwitcheState.procHotkey_ChromeTabsList _)
      js.Dynamic.global.updateDynamic ("procHotkey_ScrollDown")       (SwitcheState.procHotkey_ScrollDown _)
      js.Dynamic.global.updateDynamic ("procHotkey_ScrollUp")         (SwitcheState.procHotkey_ScrollUp _)
      js.Dynamic.global.updateDynamic ("procHotkey_ScrollEnd")        (SwitcheState.procHotkey_ScrollEnd _)

      js.Dynamic.global.updateDynamic ("procAppEvent_DevMode")        (SwitcheState.procAppEvent_DevMode _)
      js.Dynamic.global.updateDynamic ("procAppEvent_Focus")          (SwitcheState.procAppEvent_Focus _)
      js.Dynamic.global.updateDynamic ("procAppEvent_Blur")           (SwitcheState.procAppEvent_Blur _)
      js.Dynamic.global.updateDynamic ("procAppEvent_Show")           (SwitcheState.procAppEvent_Show _)
      js.Dynamic.global.updateDynamic ("procAppEvent_Hide")           (SwitcheState.procAppEvent_Hide _)

      js.Dynamic.global.updateDynamic ("procWinReport_FgndHwnd")      (SwitcheState.procWinReport_FgndHwnd _)
      js.Dynamic.global.updateDynamic ("procWinReport_ObjDestroyed")  (SwitcheState.procWinReport_ObjDestroyed _)
      js.Dynamic.global.updateDynamic ("procWinReport_ObjShown")      (SwitcheState.procWinReport_ObjShown _)
      js.Dynamic.global.updateDynamic ("procWinReport_TitleChanged")  (SwitcheState.procWinReport_TitleChanged _)

   }
   init()
}



object RenderSpacer {
   // so too many requestAnimationFrame interspersed are taking a lot of time, as they each are upto 100ms, so gonna bunch them up too
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d;
   def queueSpacedRender():Unit = {
      // main idea .. if its past reqd spacing, req frame now and update stamp
      // else if its not yet time, but nothing queued already, queue with reqd delay
      // else if last queued still in future, can just ignore it! -d
      if ( js.Date.now() + slop > lastRenderTargStamp ) {
         // i.e nothing queued is still in the future, so lets setup a delayed req w appropriate spacing
         val waitDur =  math.max (0, lastRenderTargStamp + minRenderSpacing - js.Date.now() - slop)
         lastRenderTargStamp = js.Date.now() + waitDur
         js.timers.setTimeout (waitDur) {
            // note that in theory, animation frames might not trigger when browser minimized etc, but didnt seem to matter w our show/hide mechanism
            //if (!SwitcheState.isDismissed) { // coz when browser minimized/hidden animation frame calls are disabled!
            //   js.Dynamic.global.window.requestAnimationFrame ({t:js.Any => SwitcheFacePage.render()})
            //} else { SwitcheFacePage.render() }
            // note that ^^ at 60Hz, repaints trigger ~16ms, so w our 50ms spacing, prob no longer much point in doing this .. so do it direct:
            SwitcheFacePage.render()
         }
      }
   }

}
