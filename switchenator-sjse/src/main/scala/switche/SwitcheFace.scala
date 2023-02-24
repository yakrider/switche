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
      setPageEventHandlers()
      div ( RibbonDisplay.getTopRibbonDiv(), ElemsDisplay.getElemsDiv ) .render
   }
   //def queuePageUpdate() = { g.window.requestAnimationFrame({t:js.Any => render()}) }  // used spaced render call instead
   // NOTE that the intention now is to always call this via RenderSpacer (spaced or immdt) instead of directly, so the render time can be recorded
   def updatePageElems () : Unit = { //println(s"rendering @${js.Date.now()}")
      // note that although this seems expensive to call rebuild on every render, unchanged cases get diffed and ignored by browser engine keeping it cheap
      ElemsDisplay.updateElemsDiv()
      RibbonDisplay.updateCountsSpan()
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
      e.target.closest(".elemBox") .foreach {e => handleMouseEnter (e.asInstanceOf[Div]) }
   }
   def capturePhaseKeyupHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"up")
      // note: escape can cause app hide, and when doing that, we dont want that to leak outside app, hence on keyup
      if (inSearchState) {
         handleSearchModeKeyup(e)   // let it recalc matches if necessary etc
      } else { // not in search state
         if (e.key == "Escape")  SwitcheState.handleReq_SelfWindowHide()
      }
   }
   val modifierKeys = Set("Meta","Alt","Control","Shift")
   def capturePhaseKeydownHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"down")
      var doStopProp = true
      @inline def setupSearchboxPassthrough() = {
         inSearchState = true; doStopProp = false;
         if (!modifierKeys.contains(e.key)) { activateSearchBox(); }
      }
      @inline def eventPassthroughGuarded() = {
         if (doStopProp) { e.stopPropagation(); e.preventDefault(); }
      }
      // ^^ setup here is to allow propagation only to some selective cases that need to go to search-box

      triggerHoverLockTimeout()
      if (e.altKey) {
         if (e.key == "m")  handleReq_CurElemMinimize()
         else { } // we'll ignore alt-combos for the searchbox
      }
      else if (e.ctrlKey) {
         if      (e.key == " ")  handleReq_CurElemActivation()
         else if (e.key == "g")  handleReq_GroupModeToggle()
         else if (e.key == "t")  handleReq_ChromeTabsListActivation (doTog=false)   // t for tabs (but is two hand key)
         else if (e.key == "c")  handleReq_ChromeTabsListActivation (doTog=false)   // c for chrome-tabs .. we'll see which we use more
         else if (e.key == "w")  handleReq_CurElemClose()
         else if (e.key == "v")  handleReq_CurElemShow()
         //else if (e.key == "r") RibbonDisplay.handleRefreshBtnClick()  // ctrl-r is not available for us because of the way we overload it in ahk remapping
         else if (e.key == "f")  RibbonDisplay.handleRefreshBtnClick()   // so instead of ^, we'll use f for 'fresh'.. meh
         else { setupSearchboxPassthrough() }
      }
      else {
         // these are enabled for both normal and  search-state
         if      (e.key == "F1")         focusElem_Next()         // note: not really needed, registered as global hotkey, set electron to forwards it as a call
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
            else { setupSearchboxPassthrough() }
         } else { setupSearchboxPassthrough() }
      }
      eventPassthroughGuarded()
      // ^^ basically all key-down events other than for propagation to searchbox should end here!!
   }

}

object SwitchePageState {
   import SwitcheFaceConfig._
   // doing recents and grouped elems separately as they literally are different divs (w/ diff styles etc)
   case class OrderedElemsEntry (y:Int, elem:Div, yg:Int=(-1))
   var recentsElemsMap:   mutable.LinkedHashMap[String,OrderedElemsEntry] = mutable.LinkedHashMap()
   var groupedElemsMap:   mutable.LinkedHashMap[String,OrderedElemsEntry] = mutable.LinkedHashMap()
   var searchElemsMap:    mutable.LinkedHashMap[String,OrderedElemsEntry] = mutable.LinkedHashMap()
   var recentsIdsVec:     Vector[String] = Vector()
   var searchIdsVec:      Vector[String] = Vector()
   var groupedIdsVec:     Vector[String] = Vector()
   var groupsHeadsIdsVec: mutable.ArrayBuffer[String] = mutable.ArrayBuffer()
   var curElemId = ""; var inSearchState = false;
   var isHoverLocked = false; var lastActionStamp = 0d;
   // ^^ hover-lock flag locks-out mouseover, and is intended to be set (w small timeout) while mouse scrolling/clicks etc
   // .. and that prevents mouse jiggles from screwing up any in-preogress mouse scrolls, clicks, key-nav etc

   def rebuildElems() = {} // todo: instead of full render, consider surgical updates to divs directly w/o waiting for global render etc

   def recentsId  (hwnd:Int) = s"${hwnd}_r"
   def groupedId  (hwnd:Int) = s"${hwnd}_g"
   def searchedId (hwnd:Int) = s"${hwnd}_s"
   def idToHwnd (idStr:String) = idStr.split("_").head.toInt // if fails, meh, its js!
   def isIdGrp (idStr:String) = { idStr.split("_").head != idStr }

   def setCurElem (id:String) = { curElemId = id; }
   def setCurElemHighlight (newFocusElem:Div) = {
      // we manage 'focus' ourselves so that remains even when actual focus is moved to search-box etc
      clearCurElemHighlight()       // note that this will clear curElemId too
      setCurElem (newFocusElem.id)
      newFocusElem.classList.add("curElem")
      if (inSearchState) { // in search state see if we can find another in recents to highlight too
         recentsElemsMap .get (recentsId (idToHwnd (newFocusElem.id))) .foreach (_.elem.classList.add("curElem"))
      }
   }
   def clearCurElemHighlight () = {
      curElemId = ""
      //doc.querySelectorAll(s".curElem") .foreach(_.classList.remove("curElem"))
      // ^^ our (old) sjs versions doesnt have NodeList conversion to scala iterable ..
      // .. so for now, we'll just try it twice, as there are at most two of these (if there's one in recents too during search)
      Option (doc.querySelector(s".curElem")) .foreach(_.classList.remove("curElem"))
      if (inSearchState) { // if in search mode try to clear one more
         Option (doc.querySelector(s".curElem")) .foreach(_.classList.remove("curElem"))
      }
   }

   def focusElem (isReverseDir:Boolean=false, isGrpNext:Boolean=false) = {
      type Wrapper = Vector[String] => Option[String]
      val (incr, vecWrap) = { if (!isReverseDir) { (1, (_.headOption):Wrapper) } else { (-1, (_.lastOption):Wrapper) } }
      // first we'll pick curElem from whichever map the it is in
      recentsElemsMap.get(curElemId).map(e => (false,e))
      .orElse { groupedElemsMap .get(curElemId) .map(e => (true,e)) }
      .orElse { searchElemsMap.get(curElemId).map(e => (false,e)) }
      // now that we have the cur-entry, lets try to find the next-entry option
      .flatMap { case (hasGrps, e) =>
         if (inSearchState) {        // search mode is non-grouped
            searchIdsVec .lift(e.y+incr) .orElse (vecWrap(searchIdsVec)) .flatMap(searchElemsMap.get)
         } else if (!SwitcheState.inGroupedMode) {    // this is recents mode, and is non-grouped
            recentsIdsVec .lift(e.y+incr) .orElse (vecWrap(recentsIdsVec)) .flatMap(recentsElemsMap.get)
         } else {              // i.e. in grouped mode (which has a recents block followed by grouped block!)
            if (!isGrpNext) {  // this is regular next/prev
               if (!hasGrps) {  // but cur-item is in recents block .. move to prev/next in recents, but wrap over to grp
                  recentsIdsVec .lift(e.y+incr) .flatMap(recentsElemsMap.get) .orElse ( vecWrap(groupedIdsVec) .flatMap(groupedElemsMap.get) )
               } else {        // ok, cur-item is already in grpd block .. move to prev/next in grp, but wrap over to recents
                  groupedIdsVec .lift(e.y+incr) .flatMap(groupedElemsMap.get) .orElse ( vecWrap(recentsIdsVec) .flatMap(recentsElemsMap.get) )
               }
            } else {           // this is group-next/prev
               if (!hasGrps) {  // but cur-item is in recents block .. so just move to the grp block
                  vecWrap (groupedIdsVec) .flatMap(groupedElemsMap.get)
               } else {        // ok, cur-item is in grpd-block .. move to next/prev by grp, but wrap over to recents head
                  groupsHeadsIdsVec .lift(e.yg+incr) .flatMap(groupedElemsMap.get) .orElse ( recentsIdsVec.headOption.flatMap(recentsElemsMap.get) )
               }
      }  }  } // then finally, we can make that current (if we found one)
      .map(_.elem) .foreach (setCurElemHighlight)
   }

   def focusElem_Next()  = focusElem (isReverseDir=false, isGrpNext=false)
   def focusElem_Prev()  = focusElem (isReverseDir=true,  isGrpNext=false)
   def focusGroup_Next() = focusElem (isReverseDir=false, isGrpNext=true)
   def focusGroup_Prev() = focusElem (isReverseDir=true,  isGrpNext=true)

   def focusElem_Top() = {
      if (inSearchState) { searchIdsVec.headOption.flatMap(searchElemsMap.get) }
      else { recentsIdsVec.headOption.flatMap(recentsElemsMap.get) }
   } .map(_.elem) .foreach (setCurElemHighlight)

   def focusElem_Bottom() = {
      if (inSearchState) { searchIdsVec.lastOption.flatMap(searchElemsMap.get) }
      else if (SwitcheState.inGroupedMode) { groupedIdsVec.lastOption.flatMap(groupedElemsMap.get) }
      else { recentsIdsVec.lastOption.flatMap(recentsElemsMap.get) }
   } .map(_.elem) .foreach (setCurElemHighlight)

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

   def handleMouseEnter (elem:Div) = {
      if (!isHoverLocked) { setCurElemHighlight(elem) }
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
      val ico = IconsManager.getCachedIcon(e.dat) .map (icoStr => img(`class`:="ico", src:=icoStr)) .getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val elem = div (`class`:="elemBox", id:=idStr, tabindex:=0, exeSpan, nbsp(2), ySpan, nbsp(2), icoSpan, nbsp(), titleSpan).render
      //elem.onclick = {ev:MouseEvent => SwitcheState.handleReq_WindowActivation(e.hwnd)}
      // ^^ moved most handlers to single document-level events handler
      elem.onmouseenter = {ev:MouseEvent => handleMouseEnter(elem)}
      // ^^ but we left mouseenter here, as doing that globally is pointlessly inefficient
      elem
   }

   def rebuildRecentsElems() = {
      recentsElemsMap.clear()
      SwitcheState.getRenderList .take (
         if (SwitcheState.inGroupedMode) groupedModeTopRecentsCount else SwitcheState.getRenderList().length
      ) .zipWithIndex .foreach { case (d,i) =>
         val id = recentsId (d.dat.hwnd)
         val elem = makeElemBox (id, d, isDim=false, isRecents=true)
         recentsElemsMap.put (id, OrderedElemsEntry (i, elem))
      }
      recentsIdsVec = recentsElemsMap.keys.toVector
   }
   def rebuildGroupedElems() = {
      groupedElemsMap.clear()
      groupsHeadsIdsVec = mutable.ArrayBuffer()
      case class PartOrderedElem (id:String, d:Div, grpIdx:Int)
      def getIdElemH (d:RenderListEntry, isExeDim:Boolean, grpIdx:Int) = {
         val id = groupedId (d.dat.hwnd)
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
      searchElemsMap.clear(); recentsElemsMap.clear();   // we'll also build recents here w/ match highlighting like for search
      val matchStr = RibbonDisplay.searchBox.value.trim
      case class SearchedElem (id:String, elem:Div, chkPassed:Boolean)
      def getSearchElem (e:RenderListEntry, isRecents:Boolean, isExeDim:Boolean, r:CheckSearchExeTitleRes) = {
         val id = if (isRecents) recentsId(e.dat.hwnd) else searchedId(e.dat.hwnd)
         val elem = makeElemBox (id, e, isExeDim, isRecents, r.exeSpan, r.ySpan, r.titleSpan)
         SearchedElem (id, elem, r.chkPassed)
      }
      def getSearchMatchRes (e:RenderListEntry) = {
         SearchHelper.checkSearchExeTitle (e.dat.exePathName.map(_.name).getOrElse(""), e.dat.winText.getOrElse(""), matchStr, e.y)
      }
      // the searchElemsMap idxs need to have not the zipWithIndex, but sequential idxs of only matching elems ..
      // .. so we'll do it in stages, first lets make the elems
      val sElems = SwitcheState.getGroupedRenderList() .map {_ .map { e => e -> getSearchMatchRes(e) } } .map { ll =>
         Seq ( ll.take(1).map {case (e,r) => getSearchElem(e,false,false,r)}, ll.tail.map {case (e,r) => getSearchElem(e,false,true,r)} ) .flatten
      } .flatten
      // then we'll create a separate matchIdxs for only the matching elems ids
      val matchIdxs = sElems .filter(_.chkPassed) .zipWithIndex .map { case (e,i) => e.id -> i } (breakOut):mutable.LinkedHashMap[String,Int]
      // and using that, we'll build the searchElemsMap and the searchIdsVec
      sElems .zipWithIndex .foreach { case (e,i) =>
         val y = matchIdxs.get(e.id).getOrElse(-1)
         searchElemsMap .put (e.id, OrderedElemsEntry(y,e.elem))
      }
      searchIdsVec = matchIdxs.keys.toVector

      // now lets build the recents w similar search-match highlighting as well .. (it is simpler since we dont need to next/prev through it)
      SwitcheState.getRenderList() .take (
         if (SwitcheState.inGroupedMode) groupedModeTopRecentsCount else SwitcheState.getRenderList().length
      ) .zipWithIndex .foreach { case (e,i) =>
         val se = getSearchElem(e,true,true,getSearchMatchRes(e))
         recentsElemsMap .put (se.id, OrderedElemsEntry (i, se.elem))
      }
      recentsIdsVec = recentsElemsMap.keys.toVector
   }

   def reSyncCurFocusIdAfterRebuild() = {
      { if (inSearchState) { // try search-elems match or else search-elems top
            searchElemsMap .get (searchedId (idToHwnd(curElemId))) .orElse (searchIdsVec.headOption.flatMap(searchElemsMap.get))
         } else if (SwitcheState.inGroupedMode) { // try grouped-elems match, or else recents above it, (or else recents-top fallback below)
            groupedElemsMap.get(curElemId) .orElse (recentsElemsMap.get(curElemId))
         } else { // try recents elems match, (or else recents-top in fallback further below)
            recentsElemsMap.get(curElemId)
         } // if still not found default to top of recents list (e.g. at initial load)
      } .orElse (recentsIdsVec.headOption.flatMap(recentsElemsMap.get)) .map(_.elem) .foreach (setCurElemHighlight)
   }

   def activateSearchBox () = { RibbonDisplay.searchBox.focus() }
   def exitSearchState() = {
      inSearchState = false; RibbonDisplay.searchBox.value = ""; RibbonDisplay.searchBox.blur();
      RenderSpacer.immdtRender()
   }
   def resetSearchMatchFocus() : Unit = {
      searchIdsVec.headOption .flatMap(searchElemsMap.get) .map(_.elem) .map(setCurElemHighlight) .getOrElse(clearCurElemHighlight)
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
            RenderSpacer.immdtRender()
            resetSearchMatchFocus()
      } }
   }

   def replaceTitleInnerSpan (elem:Div, newSpan:Span) = {
      clearedElem (elem.getElementsByClassName("titleSpan").apply(0)) .appendChild (newSpan)
   }
   def handle_TitleUpdate (hwnd:Int, dat:WinDatEntry) = {
      def titleUpdate_NonSearchDiv (oe:OrderedElemsEntry) : Unit = {
         replaceTitleInnerSpan ( oe.elem, span (dat.winText.getOrElse("title.."):String).render )
      }
      def titleUpdate_SearchDiv (oe:OrderedElemsEntry) : Unit = {
         val sRes = SearchHelper.checkSearchExeTitle (
            dat.exePathName.map(_.name).getOrElse(""), dat.winText.getOrElse(""), RibbonDisplay.searchBox.value.trim, oe.y
         )
         replaceTitleInnerSpan (oe.elem, sRes.titleSpan)
      }
      if (inSearchState) {
         // note that in search mode, even recents elems are search match highlighted etc
         recentsElemsMap .get(recentsId(hwnd))  .foreach (titleUpdate_SearchDiv)
         searchElemsMap  .get(searchedId(hwnd)) .foreach (titleUpdate_SearchDiv)
      } else {
         recentsElemsMap .get(recentsId(hwnd)) .foreach (titleUpdate_NonSearchDiv)
         groupedElemsMap .get(groupedId(hwnd)) .foreach (titleUpdate_NonSearchDiv)
      }
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
      //SwitchePageState.rebuildRecentsElems() // recents will also be specially rebuilt by search rebuild below
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
   def updateCountsSpan () : Unit = {
      val count = SwitcheState.getRenderList().length
      clearedElem(countSpan) .appendChild ( span ( nbsp(3), s"($count) ※", nbsp(3) ).render )
   }
   def updateDebugLinks() : Unit = {
      clearElem(debugLinks)
      if (SwitcheState.inElectronDevMode) {
         val printExclLink =  a ( href:="#", "DebugPrint", onclick:={e:MouseEvent => handleReq_DebugPrint()} ).render
         debugLinks.appendChild ( printExclLink )
   } }
   def handleRefreshBtnClick() : Unit = {
      // we want to refresh too (as do periodic calls elsewhere), but here we want to also force icons-refresh
      IconsManager.clearCachedIconMappings()
      handleReq_Refresh()
   }
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={e:MouseEvent => handleRefreshBtnClick()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleReq_GroupModeToggle()} )
      //val dragSpot = span (style:="-webkit-app-region:drag", nbsp(3), "※", nbsp(3)) // combined with count instead
      div ( id:="top-ribbon",
         nbsp(0), reloadLink, nbsp(4), refreshLink, nbsp(4), groupModeLink, countSpan, debugLinks, nbsp(4), searchBox
      ).render
   }
}

