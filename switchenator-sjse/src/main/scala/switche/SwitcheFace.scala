package switche

import org.scalajs.dom
import org.scalajs.dom.{Element, EventTarget}
import org.scalajs.dom.html.{Div, Span}
import org.scalajs.dom.raw.{KeyboardEvent, MouseEvent, WheelEvent}
import org.scalajs.dom.{document => doc}
import scalatags.JsDom.all._

import scala.collection.{breakOut, mutable}
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}


object SwitcheFaceConfig {
   val groupedModeTopRecentsCount = 9
   val hoverLockTime = 300 // ms
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.raw.Element) { e.innerHTML = ""}
   def clearedElem (e:dom.raw.Element) = { e.innerHTML = ""; e }
}

object DomExts {
   @js.native trait ElementWithClosest extends js.Object {
      def closest(selector:String): Element = js.native
   }
   implicit class ElementExtender (val elem:Element) extends AnyVal {
      def closest(selector:String):Option[Element] = Option(elem.asInstanceOf[ElementWithClosest].closest(selector))
   }
   implicit class EventTargetExtender (val target:EventTarget) extends AnyVal {
      def closest(selector:String):Option[Element] = { target match {
         case elem:Element => elem.closest(selector)
         case _ => None
   } } }

}

object SwitcheFacePage {
   import SwitchePageState._
   import SwitcheState._
   import DomExts._

   def getShellPage () = {
      val topRibbon = RibbonDisplay.getTopRibbonDiv()
      val elemsDiv = ElemsDisplay.getElemsDiv
      val page = div (topRibbon, elemsDiv)
      setPageEventHandlers()
      page.render
   }
   //def queueRender() = g.window.requestAnimationFrame({t:js.Any => render()}) // used spaced render call instead
   def render() = { //println(s"rendering @${js.Date.now()}")
      // note that although this seems expensive to call rebuild on every render, unchanged cases get diffed and ignored by browser engine keeping it cheap
      SwitcheState.updateRenderReadyLists()
      ElemsDisplay.updateElemsDiv()
      RibbonDisplay.updateCountsSpan(SwitcheState.getRenderList.size)
   }
   def printKeyDebugInfo (e:KeyboardEvent, evType:String) = {
      println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, modCtrl:${e.getModifierState("Control")}, modCaps:${e.getModifierState("CapsLock")}")
   }
   def setPageEventHandlers() = {
      // reminder here.. capture phase means its just going down from top level to target, after that bubble phase goes from target upwards
      // intercepting here at the 'capture' phase allows us to use e.stopPropagation() to prevent event from ever reaching target
      doc.addEventListener ("click",       procMouse_Click _)
      doc.addEventListener ("contextmenu", procMouse_ContextMenu _)
      doc.addEventListener ("auxclick",    procMouse_AuxClick _)
      doc.addEventListener ("mouseup",     procMouse_Up _)
      doc.addEventListener ("wheel",       procMouse_Wheel _)
      doc.addEventListener ("keyup",       capturePhaseKeyupHandler _, useCapture=true)
      doc.addEventListener ("keydown",     capturePhaseKeydownHandler _, useCapture=true)
      //dom.document.addEventListener ("mouseenter", procMouse_Enter _, useCapture=true) // done from element for efficiency
   }
   def procMouse_Click (e:MouseEvent) = {
      triggerHoverLockTimeout()
      e.target.closest(".elemBox").foreach(_=>handleReq_CurElemActivation())
   }
   def procMouse_AuxClick (e:MouseEvent) = { //println (s"got auxclick, btn:${e.button}")
      if (e.button == 1) {
         // disabling this as auxclick only seems to be triggered when pane doesnt have scrollable content, else the OS seems to make that into the
         // funky round scroll icon w no auxclick registered, and no easy way around it other than way fiddling w mouse drivers etc
         // instead gonna use mouse-up which seems to reliably be generated.. note that mouse-down also only seems to be generated when NOT in that
         // scroll-circle mode.. sucky part is that regardless, the mouse pointer icon will change if so, and nothing to be done about it.. oh well
         //procMouse_MiddleClick(e)
      } else if (e.button == 2) {
         procMouse_RightClick(e)
      } else {
         e.preventDefault(); e.stopPropagation() // ignore any other buttons
      }
   }
   def procMouse_MiddleClick (e:MouseEvent) = { //println("got middle click!")
      triggerHoverLockTimeout(); e.preventDefault(); e.stopPropagation()
      e.target.closest(".elemBox").foreach(_=>handleReq_CurElemClose())
   }
   def procMouse_RightClick (e:MouseEvent) = {
      // eventually could consider supporting more native right-click+wheel global combo here
      // but for now, we're using ahk to send separate hotkeys for right-mouse + wheel-down and enc scroll, so can use this for closing windows
      triggerHoverLockTimeout()
      //e.target.closest(".elemBox").foreach(_=>handleReq_CurElemClose())
      // ^ disabling, as middle click seems to gets used exclusively, and right click mostly only seems to trigger accidentally
   }
   def procMouse_ContextMenu (e:MouseEvent) = { //println (s"got context menu click, btn:${e.button}")
      // this fires separately from the auxclick 2 report on right-click
      //e.preventDefault(); e.stopPropagation()
   }
   def procMouse_Up (e:MouseEvent) = {
      if (e.button == 1) {procMouse_MiddleClick(e)}
   }
   def procMouse_Wheel (e:WheelEvent) = {
      if (verifyActionRepeatSpacing(20d)) {  // enforced spacing (in ms) between consecutive mouse scroll action handling
         triggerHoverLockTimeout()
         if (e.deltaY > 0 || e.deltaX > 0) { focusElem_Next() } else { focusElem_Prev() }
   } }
   def procMouse_Enter (e:MouseEvent) = {
      e.target.closest(".elemBox") .foreach {e => handleMouseEnter (e.id, e.asInstanceOf[Div]) }
   }
   def capturePhaseKeyupHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"up")
      // note: escape can cause app hide, and when doing that, we dont want that to leak outside app, hence on keyup
      if (inSearchState) {
         handleSearchModeKeyup(e)   // let it recalc matches if necessary etc
      } else { // not in search state
         if (e.key == "Escape")  SwitcheState.handleReq_SelfWindowHide()
      }
   }
   def capturePhaseKeydownHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"down")
      var doStopProp = true
      // ^^ setup here is to allow propagation only to some selective cases that need to go to search-box!
      triggerHoverLockTimeout()
      if (e.altKey) {
         if (e.key == "m")  handleReq_CurElemMinimize()
      }
      else if (e.ctrlKey) {
         if      (e.key == " ")  handleReq_CurElemActivation()
         else if (e.key == "g")  handleReq_GroupModeToggle()
         else if (e.key == "t")  handleReq_ChromeTabsListActivation (doTog=false)   // t for tabs (but is two hand key)
         else if (e.key == "c")  handleReq_ChromeTabsListActivation (doTog=false)   // c for chrome-tabs .. we'll see which we use more
         else if (e.key == "w")  handleReq_CurElemClose()
         else if (e.key == "v")  handleReq_CurElemShow()
         //else if (e.key == "r") handleReq_Refresh()  // ctrl-r is not available for us because of the way we overload it in ahk remapping
         else if (e.key == "f")  handleReq_Refresh()   // so instead of ^, we'll use f for 'fresh'.. meh
         //else if (e.key == "Backspace") { if (SwitchePageState.inSearchState) exitSearchState() }
         else { inSearchState = true; activateSearchBox(); doStopProp = false; }
      }
      else {
         // these are enabled for both normal and  search-state
         if      (e.key == "F1")         focusElem_Next()       // note: not really needed, registered as global hotkey, set electron to forwards it as a call
         else if (e.key == "F2")         focusElem_Prev()
         else if (e.key == "Tab")        if (e.shiftKey) focusElem_Prev() else focusElem_Next()
         else if (e.key == "ArrowDown")  focusElem_Next()
         else if (e.key == "ArrowUp")    focusElem_Prev()
         else if (e.key == "PageUp")     focusElem_Top()
         else if (e.key == "PageDown")   focusElem_Bottom()
         else if (e.key == "Enter")      handleReq_CurElemActivation()
         else if (e.key == "Escape")    {/*handleEscapeKeyDown()*/}        // moved to keyup as dont want its keyup leaking outside app if we use it hide app
         else if (e.key == "F5")        dom.window.location.reload(true)   // the true means full reload ignoring cache
         else if (!inSearchState) {
            // these are enabled in normal mode but disabled in search-state
            if      (e.key == " ")           handleReq_CurElemActivation()
            else if (e.key == "ArrowRight")  focusGroup_Next()
            else if (e.key == "ArrowLeft")   focusGroup_Prev()
            // for other keys while not in search-state, we'll activate search-state and let it propagate to searchbox
            else { inSearchState = true; activateSearchBox(); doStopProp = false; }
         } else { // if we're here, we must be in search-state, so we'll just let it go through to search-box
            activateSearchBox(); doStopProp = false;
      } }
      if (doStopProp) { e.stopPropagation(); e.preventDefault(); }
      // ^^ basically all key-down events other than for propagation to searchbox should end here!!
   }

}

object SwitchePageState {
   import SwitcheFaceConfig._
   // doing recents and grouped elems separately as they literally are different divs (w/ diff styles etc)
   case class OrderedElemsEntry (y:Int, elem:Div, yg:Int=(-1))
   var recentsElemsMap:   mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var groupedElemsMap:   mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var searchElemsMap:    mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var recentsIdsVec:     Vector[String] = _
   var searchIdsVec:      Vector[String] = _
   var groupedIdsVec:     Vector[String] = _
   var groupsHeadsIdsVec: mutable.ArrayBuffer[String] = _
   var curElemId = ""; var inSearchState = false;
   var isHoverLocked = false; var lastActionStamp = 0d;
   // ^^ hover-lock flag locks-out mouseover, and is intended to be set (w small timeout) while mouse scrolling/clicks etc
   // .. and that prevents mouse jiggles from screwing up any in-preogress mouse scrolls, clicks, key-nav etc

   def rebuildElems() = {} // todo: instead of full render, consider surgical updates to divs directly w/o waiting for global render etc

   def getElemId (hwnd:Int,isGrpElem:Boolean) = s"${hwnd}${if (isGrpElem) "_g" else ""}"
   def idToHwnd (idStr:String) = idStr.split("_").head.toInt // if fails, meh, its js!
   def isIdGrp (idStr:String) = {idStr.split("_").head != idStr}

   def setCurElem (id:String) = { curElemId = id; }
   def setCurElemHighlight (newFocusElem:Div) = {
      // we manage 'focus' ourselves so that remains even when actual focus is moved to search-box etc
      clearCurElemHighlight()
      newFocusElem.classList.add("curElem")
   }
   def clearCurElemHighlight () = {
      Option (doc.querySelector(s".curElem")) .foreach(_.classList.remove("curElem"))
   }

   private def focusElemId_Recents (id:String) = recentsElemsMap.get(id) .foreach {e => setCurElem(id); setCurElemHighlight(e.elem)}
   private def focusElemId_Grouped (id:String) = groupedElemsMap.get(id) .foreach {e => setCurElem(id); setCurElemHighlight(e.elem)}
   private def focusElemId_Search  (id:String) = searchElemsMap .get(id) .foreach {e => setCurElem(id); setCurElemHighlight(e.elem)}

   def focusElem (isReverseDir:Boolean=false, isGrpNext:Boolean=false) = {
      type Wrapper = Vector[String] => Option[String]
      val (incr, vecWrap) = if (!isReverseDir) { (1, (_.headOption):Wrapper) } else { (-1, (_.lastOption):Wrapper) }
      groupedElemsMap .get(curElemId) .map(e => (true,e))
      .orElse { recentsElemsMap.get(curElemId).map(e => (false,e)) }
      .orElse { searchElemsMap.get(curElemId).map(e => (false,e)) }
      .foreach { case (isGrpd, e) =>
         if (SwitchePageState.inSearchState) {        // search mode is non-grouped
            searchIdsVec .lift(e.y+incr) .orElse (vecWrap(searchIdsVec)) .map(focusElemId_Search)
         } else if (!SwitcheState.inGroupedMode) {    // this is recents mode, and is non-grouped
            recentsIdsVec .lift(e.y+incr) .orElse (vecWrap(recentsIdsVec)) .map(focusElemId_Recents)
         } else {              // i.e. in grouped mode (which has a recents block followed by grouped block!)
            if (!isGrpNext) {    // this is regular next/prev
               if (!isGrpd) {  // but cur-item is in recents block .. move to prev/next in recents, but wrap over to grp
                  recentsIdsVec .lift(e.y+incr) .map(focusElemId_Recents) .orElse (vecWrap(groupedIdsVec).map(focusElemId_Grouped))
               } else {        // ok, cur-item is already in grpd block .. move to prev/next in grp, but wrap over to recents
                  groupedIdsVec .lift(e.y+incr) .map(focusElemId_Grouped) .orElse (vecWrap(recentsIdsVec).map(focusElemId_Recents))
               }
            } else {           // this is group-next/prev
               if (!isGrpd) {  // but cur-item is in recents block .. so just move to the grp block
                  vecWrap(groupedIdsVec).map(focusElemId_Grouped)
               } else {        // ok, cur-item is in grpd-block .. move to next/prev by grp, but wrap over to recents head
                  groupsHeadsIdsVec .lift(e.yg+incr) .map(focusElemId_Grouped) .orElse ( recentsIdsVec.headOption.map(focusElemId_Recents))
               }
         } }
   } }
   def focusElem_Next()  = focusElem (isReverseDir=false, isGrpNext=false)
   def focusElem_Prev()  = focusElem (isReverseDir=true,  isGrpNext=false)
   def focusGroup_Next() = focusElem (isReverseDir=false, isGrpNext=true)
   def focusGroup_Prev() = focusElem (isReverseDir=true,  isGrpNext=true)

   def focusElem_Top() = {
      if (SwitchePageState.inSearchState) { searchIdsVec.headOption.map(focusElemId_Search) }
      else { recentsIdsVec.headOption.map(focusElemId_Recents) }
   }
   def focusElem_Bottom() = {
      if (SwitchePageState.inSearchState) { searchIdsVec.lastOption.map(focusElemId_Search) }
      else if (SwitcheState.inGroupedMode) { groupedIdsVec.lastOption.map(focusElemId_Grouped) }
      else { recentsIdsVec.lastOption.map(focusElemId_Recents) }
   }

   def resetFocus() = {
      //js.Dynamic.global.document.activeElement.blur()
      setCurElem(recentsIdsVec.head); focusElem_Next()
   }

   def handleReq_CurElemActivation() = { SwitcheState.handleReq_WindowActivation (idToHwnd(curElemId)) }
   def handleReq_CurElemMinimize()   = { SwitcheState.handleReq_WindowMinimize   (idToHwnd(curElemId)) }
   def handleReq_CurElemClose()      = { SwitcheState.handleReq_WindowClose      (idToHwnd(curElemId)) }
   def handleReq_CurElemShow()       = { SwitcheState.handleReq_WindowShow       (idToHwnd(curElemId)) }

   def handleReq_SecondRecentActivation() = {
      recentsIdsVec.lift(1) .map(idToHwnd) .foreach(SwitcheState.handleReq_WindowActivation)
   }
   def handleReq_ChromeTabsListActivation(doTog:Boolean=true) = {
      val hwnd = SwitcheState.getRenderList() .filter { e =>
         e.dat.winText.contains("Tabs Outliner") && e.dat.exePathName.map(_.name).contains("chrome.exe") &&
            e.dat.isVis.contains(true) && e.dat.isUnCloaked.contains(true) // these two are redundant checks, just for peace of mind
      } .map(_.dat.hwnd)
      // if we found the hwnd, if its not already active, activate it, else switch to next window
      if (hwnd.isEmpty) {
         // didnt find it, do nothing .. (certainly dont switch to second window!)
      } else if (doTog && recentsIdsVec.headOption.map(idToHwnd).exists(hwnd.contains)) {
         // found it, its already at top, so toggle to the next-top window
         handleReq_SecondRecentActivation()
      } else { // aight, found it, and its not top, switch to it
         hwnd.foreach(SwitcheState.handleReq_WindowActivation)
      }
   }

   def handleMouseEnter (idStr:String, elem:Div) = {
      if (!isHoverLocked) { setCurElem(idStr); setCurElemHighlight(elem) }
   }
   def triggerHoverLockTimeout() = {
      isHoverLocked = true; val t = js.Date.now(); lastActionStamp = t;
      js.timers.setTimeout(hoverLockTime) {handleHoverLockTimeout(t)}
   }
   def handleHoverLockTimeout(kickerStamp:Double) = {
      if (lastActionStamp == kickerStamp) { isHoverLocked = false }
   }
   def verifyActionRepeatSpacing (minRepeatSpacingMs:Double) : Boolean = {
      val t = scalajs.js.Date.now()
      if ((t - lastActionStamp) < minRepeatSpacingMs) { return false }
      lastActionStamp = t
      return true
   }

   def makeElemBox (idStr:String, e:RenderListEntry, isDim:Boolean, isRecents:Boolean):Div = {
      val exeInnerSpan = span ( e.dat.exePathName.map(_.name).getOrElse("exe..").toString ).render
      val yInnerSpan = span (`class`:="ySpan", f"${e.y}%2d" ).render
      val titleInnerSpan = span ( e.dat.winText.getOrElse("title").toString ).render
      makeElemBox ( idStr, e, isDim, isRecents, exeInnerSpan, yInnerSpan, titleInnerSpan )
   }
   def makeElemBox (idStr:String, e:RenderListEntry, isDim:Boolean, isRecents:Boolean,
                    exeInnerSpan:Span, yInnerSpan:Span, titleInnerSpan:Span
    ) : Div = {
      val recentsCls = if (isRecents) " recents" else ""
      val dimCls = if (isDim) " dim" else ""
      val exeSpan = span (`class`:=s"exeSpan$recentsCls$dimCls", exeInnerSpan)
      val ySpan = span (`class`:=s"ySpan$recentsCls", yInnerSpan)
      val titleSpan = span (`class`:=s"titleSpan$recentsCls", titleInnerSpan)
      val ico = e.dat.iconOverrideLoc .orElse (e.dat.exePathName.map(_.fullPath)) .map ( path =>
         IconsManager.getCachedIcon(e.dat.hwnd,path) .map (icoStr => img(`class`:="ico", src:=icoStr))
      ).flatten.getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val elem = div (`class`:="elemBox", id:=idStr, tabindex:=0, exeSpan, nbsp(2), ySpan, nbsp(2), icoSpan, nbsp(), titleSpan).render
      //elem.onclick = {ev:MouseEvent => SwitcheState.handleReq_WindowActivation(e.hwnd)}
      // ^^ moved most handlers to single document-level events handler
      elem.onmouseenter = {ev:MouseEvent => handleMouseEnter(idStr,elem)}
      // ^^ but we left mouseenter here, as doing that globally is pointlessly inefficient
      elem
   }

   def rebuildRecentsElems() = {
      recentsElemsMap = mutable.LinkedHashMap()
      val recentsToTake = if (SwitcheState.inGroupedMode) {SwitcheState.getRenderList.take(groupedModeTopRecentsCount)} else {SwitcheState.getRenderList()}
      recentsToTake .zipWithIndex .foreach {case (d,i) =>
         val id = s"${d.dat.hwnd}"
         val elem = makeElemBox (id, d, isDim=false, isRecents=true)
         recentsElemsMap.put (id, OrderedElemsEntry (i, elem))
      }
      recentsIdsVec = recentsElemsMap.keys.toVector
   }
   def rebuildGroupedElems() = {
      groupedElemsMap = mutable.LinkedHashMap()
      groupsHeadsIdsVec = mutable.ArrayBuffer()
      case class PartOrderedElem (id:String, d:Div, grpIdx:Int)
      def getIdElemH(d:RenderListEntry,isExeDim:Boolean,grpIdx:Int) = {
         val id = s"${d.dat.hwnd}_g"
         val elem = makeElemBox (id, d, isExeDim, isRecents=false)
         PartOrderedElem (id, elem, grpIdx)
      }
      SwitcheState.getGroupedRenderList .zipWithIndex .map {case (ll,gi) =>
         Seq ( ll.take(1).map(d => getIdElemH(d,false,gi)), ll.tail.map(d => getIdElemH(d,true,gi)) ).flatten
      } .map { ll => ll.headOption.map(_.id).foreach(groupsHeadsIdsVec.+=(_)); ll } .flatten .zipWithIndex
      .foreach {case (e,i) => groupedElemsMap.put (e.id, OrderedElemsEntry(i,e.d,e.grpIdx)) }
      groupedIdsVec = groupedElemsMap.keys.toVector
   }
   def rebuildSearchElems() : Unit = {
      searchElemsMap = mutable.LinkedHashMap()
      val matchStr = RibbonDisplay.searchBox.value.trim
      case class SearchedElem (id:String, d:Div, chkPassed:Boolean)
      def getIdElem(d:RenderListEntry, isExeDim:Boolean, r:CheckSearchExeTitleRes) = {
         val id = s"${d.dat.hwnd}_s"
         val elem = makeElemBox (id, d, isExeDim, isRecents=false, r.exeSpan, r.ySpan, r.titleSpan)
         SearchedElem (id, elem, r.chkPassed)
      }
      val sElems = SwitcheState.getGroupedRenderList() .map {_ .map { d =>
         d -> SearchHelper.checkSearchExeTitle (d.dat.exePathName.map(_.name).getOrElse(""), d.dat.winText.getOrElse(""), matchStr, d.y)
      } } .filterNot(_.isEmpty) .map { ll =>
         Seq ( ll.take(1).map {case (d,r) => getIdElem(d,false,r)}, ll.tail.map {case (d,r) => getIdElem(d,true,r)} ) .flatten
      } .flatten
      val matchIdxs = sElems.filter(_.chkPassed).zipWithIndex .map {case (e,i) => e.id -> i}(breakOut):mutable.LinkedHashMap[String,Int]
      sElems .zipWithIndex .foreach { case (e,i) =>
         val y = matchIdxs.get(e.id).getOrElse(-1)
         searchElemsMap .put (e.id, OrderedElemsEntry(y,e.d))
      }
      searchIdsVec = matchIdxs.keys.toVector
   }

   def reSyncCurFocusIdAfterRebuild() = {
      def checkResyncIdRecents(idStr:String) = { recentsElemsMap.get(idStr).map {o => recentsIdsVec.lift(o.y).map(id => (id,o.elem))}.flatten }
      def checkResyncIdGrouped(idStr:String) = { groupedElemsMap.get(idStr).map {o => groupedIdsVec.lift(o.y).map(id => (id,o.elem))} .flatten }
      def checkResyncIdSearch (idStr:String) = { searchElemsMap.get(idStr).map {o => searchIdsVec.lift(o.y).map(id => (id,o.elem))} .flatten }
      {  if (SwitchePageState.inSearchState) {
            checkResyncIdSearch(s"${curElemId.split("_").head}_s") .orElse(searchIdsVec.headOption.map(checkResyncIdSearch).flatten)
         } else if (SwitcheState.inGroupedMode) {
            checkResyncIdGrouped(curElemId) .orElse (checkResyncIdRecents(curElemId)) orElse (checkResyncIdGrouped(s"${curElemId}_g"))
         } else { checkResyncIdRecents(curElemId.split("_").head) }
      }.orElse { recentsElemsMap.headOption.map {case (id, o) => (id, o.elem)} }
      .foreach { case (id,elem) => setCurElem(id); setCurElemHighlight(elem) }
   }

   def activateSearchBox () = { RibbonDisplay.searchBox.focus() }
   def exitSearchState() = {
      inSearchState = false; RibbonDisplay.searchBox.value = ""; RibbonDisplay.searchBox.blur();
      ElemsDisplay.updateElemsDiv()
   }
   def resetSearchMatchFocus() : Unit = {
      searchIdsVec.headOption .map { id =>
         searchElemsMap.get(id) .foreach { o => setCurElem(id); setCurElemHighlight(o.elem); }
      } .getOrElse { setCurElem(""); clearCurElemHighlight(); }
   }

   val handleSearchModeKeyup: KeyboardEvent => Unit = {
      var cachedSearchBoxTxt = ""
      // ^^ we only need to cache it for this fn, so we're wrapping the whole thing into a closured val (instead of a fn)
      (e: KeyboardEvent) => {
         val curSearchBoxTxt = RibbonDisplay.searchBox.value.trim
         if (curSearchBoxTxt.isEmpty || e.key == "Escape") {
            cachedSearchBoxTxt = ""
            exitSearchState()
         } else if (curSearchBoxTxt != cachedSearchBoxTxt) {
            cachedSearchBoxTxt = curSearchBoxTxt
            ElemsDisplay.updateElemsDiv()
            resetSearchMatchFocus()
      } }
   }

   def replaceTitleInnerSpan (elem:Div, newSpan:Span) = {
      clearedElem (elem.getElementsByClassName("titleSpan").apply(0)) .appendChild (newSpan)
   }
   def handle_TitleUpdate(hwnd:Int, dat:WinDatEntry) = {
      recentsElemsMap.get(s"${hwnd}"  ).map(_.elem).foreach{elem => replaceTitleInnerSpan (elem, span((dat.winText.getOrElse("title..")):String).render)}
      groupedElemsMap.get(s"${hwnd}_g").map(_.elem).foreach{elem => replaceTitleInnerSpan (elem, span((dat.winText.getOrElse("title..")):String).render)}
      if (inSearchState) {
         searchElemsMap .get(s"${hwnd}_s") .foreach { oe =>
            val sRes = SearchHelper.checkSearchExeTitle (
               dat.exePathName.map(_.name).getOrElse(""), dat.winText.getOrElse(""), RibbonDisplay.searchBox.value.trim, oe.y
            )
            replaceTitleInnerSpan(oe.elem,sRes.titleSpan)
      } }
   }

}

object ElemsDisplay {
   import SwitcheFaceConfig._
   val elemsDiv = div (id:="elemsDiv").render
   def getElemsDiv = elemsDiv

   def header(txt:String, mode:String) = {
      div (`class`:=s"modeHeader $mode", nbsp(1), txt) .render
   }
   def makeRecentsDiv (isDim:Boolean=false) = {
      val cls = s"recentsDiv${ if (isDim) " dim" else "" }"
      div (`class`:=cls, header("Recents:","r"), SwitchePageState.recentsElemsMap.values.map(_.elem).toSeq) .render
   }
   def makeGroupedDiv() = {
      div (`class`:="groupedDiv",  header("Grouped:","g"), SwitchePageState.groupedElemsMap.values.map(_.elem).toSeq) .render
   }
   def makeSearchedDiv()  = {
      //div (`class`:="searchedDiv", header("Searched:","s"), SwitchePageState.searchElemsMap .values.map(_.elem).toSeq) .render
      div (`class`:="searchedDiv", header("Grouped:","g"), SwitchePageState.searchElemsMap .values.map(_.elem).toSeq) .render
   }

   def updateElemsDiv_RecentsMode() = {
      SwitchePageState.rebuildRecentsElems()
      clearedElem(elemsDiv) .appendChild (makeRecentsDiv())
   }
   def updateElemsDiv_GroupedMode() = {
      SwitchePageState.rebuildRecentsElems()
      SwitchePageState.rebuildGroupedElems()
      clearedElem(elemsDiv) .appendChild ( div ( makeRecentsDiv(), makeGroupedDiv() ).render )
   }
   def updateElemsDiv_SearchState() = {
      SwitchePageState.rebuildRecentsElems()
      SwitchePageState.rebuildSearchElems()
      clearedElem(elemsDiv) .appendChild ( div ( makeRecentsDiv(isDim=true), makeSearchedDiv() ).render )
   }
   def updateElemsDiv () = {
      SwitcheState.updateRenderReadyLists()
      if (SwitchePageState.inSearchState) { updateElemsDiv_SearchState }
      else if (SwitcheState.inGroupedMode) { updateElemsDiv_GroupedMode }
      else { updateElemsDiv_RecentsMode }
      SwitchePageState.reSyncCurFocusIdAfterRebuild()
   }
}

object RibbonDisplay {
   import SwitcheFaceConfig._
   import SwitcheState._
   val countSpan = span (`class`:="dragSpan").render
   val debugLinks = span ().render
   val searchBox = input (`type`:="text", id:="searchBox", placeholder:="").render
   // note: ^^ all key handling is now done at doc level capture phase (which selectively allows char updates etc to filter down to searchBox)
   def updateCountsSpan (n:Int) = { clearedElem(countSpan) .appendChild ( span ( nbsp(3), s"($n) ※", nbsp(3) ).render ) }
   def updateDebugLinks() = {
      clearElem(debugLinks)
      if (SwitcheState.inElectronDevMode) {
         val printExclLink =  a ( href:="#", "DebugPrint", onclick:={e:MouseEvent => handleReq_DebugPrint()} ).render
         debugLinks.appendChild ( printExclLink )
   } }
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={e:MouseEvent => handleReq_Refresh()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleReq_GroupModeToggle()} )
      //val dragSpot = span (style:="-webkit-app-region:drag", nbsp(3), "※", nbsp(3)) // combined with count instead
      div ( id:="top-ribbon",
         nbsp(0), reloadLink, nbsp(4), refreshLink, nbsp(4), groupModeLink, countSpan, debugLinks, nbsp(4), searchBox
      ).render
   }
}

