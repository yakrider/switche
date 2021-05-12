package switche

import org.scalajs.dom
import org.scalajs.dom.{Element,FocusEvent,EventTarget}
import org.scalajs.dom.html.{Div, Span}
import org.scalajs.dom.raw.{KeyboardEvent, MouseEvent, WheelEvent}
import scalatags.JsDom.all._

import scala.collection.mutable
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
      SwitcheState.updateRenderReadyLists()
      ElemsDisplay.updateElemsDiv()
      RibbonDisplay.updateCountsSpan(SwitcheState.getRenderList.size)
   }
   def printKeyDebugInfo (e:KeyboardEvent,evType:String) = {
      println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, modCtrl:${e.getModifierState("Control")}, modCaps:${e.getModifierState("CapsLock")}")
   }
   def setPageEventHandlers() = {
      // reminder here.. capture phase means its just going down from top level to target, after that bubble phase goes from target upwards
      // intercepting here at the 'capture' phase allows us to use e.stopPropagation() to prevent event from ever reaching target
      dom.document.addEventListener ("click", mouseClickHandler _)
      dom.document.addEventListener ("contextmenu", mouseContextMenuHandler _)
      dom.document.addEventListener ("auxclick", mouseAuxClickHandler _)
      dom.document.addEventListener ("mouseup", mouseUpHandler _)
      dom.document.addEventListener ("wheel", mouseWheelHandler _)
      //dom.document.addEventListener ("mouseenter", mouseEnterHandler _, useCapture=true) // faster if done from element
      dom.document.addEventListener ("keyup", capturePhaseKeyupHandler _, useCapture=true)
      dom.document.addEventListener ("keydown", capturePhaseKeydownHandler _, useCapture=true)
   }
   def mouseClickHandler (e:MouseEvent) = {
      triggerHoverLockTimeout()
      e.target.closest(".elemBox").foreach(_=>handleCurElemActivationReq())
   }
   def mouseAuxClickHandler (e:MouseEvent) = { //println (s"got auxclick, btn:${e.button}")
      if (e.button == 1) {
         // disabling this as auxclick only seems to be triggered when pane doesnt have scrollable content, else the OS seems to make that into the
         // funky round scroll icon w no auxclick registered, and no easy way around it other than way fiddling w mouse drivers etc
         // instead gonna use mouse-up which seems to reliably be generated.. note that mouse-down also only seems to be generated when NOT in that
         // scroll-circle mode.. sucky part is that regardless, the mouse pointer icon will change if so, and nothing to be done about it.. oh well
         //mouseMiddleClickHandler(e)
      } else if (e.button == 2) {
         mouseRightClickHandler(e)
      } else {
         e.preventDefault(); e.stopPropagation() // ignore any other buttons
      }
   }
   def mouseMiddleClickHandler (e:MouseEvent) = { //println("got middle click!")
      triggerHoverLockTimeout(); e.preventDefault(); e.stopPropagation()
      e.target.closest(".elemBox").foreach(_=>handleCurElemCloseReq())
   }
   def mouseRightClickHandler (e:MouseEvent) = {
      // eventually could consider supporting more native right-click+wheel global combo here
      // but for now, we're using ahk to send separate hotkeys for right-mouse + wheel-down and enc scroll, so can use this for closing windows
      triggerHoverLockTimeout()
      e.target.closest(".elemBox").foreach(_=>handleCurElemCloseReq())
   }
   def mouseContextMenuHandler (e:MouseEvent) = { //println (s"got context menu click, btn:${e.button}")
      // this fires separately from the auxclick 2 report on right-click
      //e.preventDefault(); e.stopPropagation()
   }
   def mouseUpHandler (e:MouseEvent) = {
      if (e.button == 1) {mouseMiddleClickHandler(e)}
   }
   def mouseWheelHandler (e:WheelEvent) = {
      if (verifyActionRepeatSpacing(10d)) {  // enforced spacing (in ms) between consecutive mouse scroll action handling
         triggerHoverLockTimeout()
         if (e.deltaY > 0) { focusNextElem() } else { focusPrevElem() }
   } }
   def mouseEnterHandler (e:MouseEvent) = {
      e.target.closest(".elemBox").foreach{e => handleMouseEnter(e.id,e.asInstanceOf[Div])}
   }
   def capturePhaseKeyupHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"up")
      if (e.key == "Escape") {handleEscapeKeyUp()} // escape can cause app hide, we dont want that to leak outside app, hence on keyup
   }
   def capturePhaseKeydownHandler (e:KeyboardEvent) = { //printKeyDebugInfo(e,"down")
      triggerHoverLockTimeout()
      if (e.altKey) {
         if (e.key == "m") {handleCurlElemMinimizeReq()}
      }
      else if (e.ctrlKey) {
         if (e.key == " ") {handleCurElemActivationReq()}
         else if (e.key == "t") {handleGroupModeToggleReq()}
         else if (e.key == "w") {handleCurElemCloseReq()}
         else if (e.key == "v") {handleCurElemShowReq()}
         //else if (e.key == "r") {handleRefreshRequest()} // ctrl-r is not available for us because of the way we overload it in ahk remapping
         else if (e.key == "f") {handleRefreshRequest()} // so instead fo ^, we'll use f for 'fresh'.. meh
      }
      else {
         if (e.key == " ") {handleSpaceKeyDown()} // special as we overload it for activation ONLY if search hasnt started, else use ctrl
         else if (e.key == "F1") {focusNextElem()} // note: not really needed, registered as global hotkey, set electron to forwards it as a call
         else if (e.key == "F2") {focusPrevElem()}
         else if (e.key == "Tab") { e.stopPropagation(); e.preventDefault();  focusNextElem() }
         else if (e.key == "ArrowDown") {focusNextElem()}
         else if (e.key == "ArrowUp") {focusPrevElem()}
         else if (e.key == "ArrowRight") {focusNextGroup()}
         else if (e.key == "ArrowLeft") {focusPrevGroup()}
         else if (e.key == "PageUp") {focusTopElem()}
         else if (e.key == "PageDown") {focusBottomElem()}
         else if (e.key == "Enter") {handleCurElemActivationReq()}
         else if (e.key == "Escape") {/*handleEscapeKeyDown()*/} // moved to keyup as dont want its keyup leaking outside app if we use it hide app
         else if (e.key == "F5") {dom.window.location.reload()}
         else { activateSearchBox(); } //printKeyDebugInfo(e,"down") }
   } }

}

object SwitchePageState {
   import SwitcheFaceConfig._
   // doing recents and grouped elems separately as they literally are different divs (w/ diff styles etc)
   case class OrderedElemsEntry (y:Int, elem:Div, yg:Int=(-1))
   var recentsElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var groupedElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var searchElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var recentsIdsVec: Vector[String] = _
   var groupedIdsVec: Vector[String] = _
   var searchIdsVec: Vector[String] = _
   var groupsHeadsIdsVec: mutable.ArrayBuffer[String] = _
   var curFocusId:String = ""; //var curFocusVecIdx:Int = _; // the idx is to be able to check after we rebuild if we can maintain id?
   var isHoverLocked:Boolean = false; var lastActionStamp:Double = 0 //js .Date.now()
   var inSearchState:Boolean = false; var spaceKeyArmed:Boolean = false;

   def rebuildElems() = {} // instead of full render, consider surgical updates to divs directly w/o waiting for global render etc

   def getElemId (hwnd:Int,isGrpElem:Boolean) = s"${hwnd}${if (isGrpElem) "_g" else ""}"
   def idToHwnd (idStr:String) = idStr.split("_").head.toInt // if fails, meh, its js!
   def isIdGrp (idStr:String) = {idStr.split("_").head != idStr}

   private def focusElemIdRecents(id:String) = recentsElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus()}
   private def focusElemIdGrouped(id:String) = groupedElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus()}
   private def focusElemIdSearch (id:String) = searchElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus(); spaceKeyArmed = true;}
   def resetFocus() = { curFocusId = recentsIdsVec.head; focusNextElem()} //js.Dynamic.global.document.activeElement.blur() }
   def focusNextElem() = {
      groupedElemsMap .get(curFocusId) .map(e => (true,e.y)) .orElse {
         recentsElemsMap.get(curFocusId).map(e => (false,e.y))
      } .orElse { searchElemsMap.get(curFocusId).map(e => (false,e.y))
      } .foreach {case (isGrpd, idx) =>
         if (SwitchePageState.inSearchState) {
            searchIdsVec .lift(idx+1) .orElse (searchIdsVec.headOption) .map(focusElemIdSearch)
         } else if (!SwitcheState.inGroupedMode) {
            recentsIdsVec .lift(idx+1) .orElse (recentsIdsVec.headOption) .map(focusElemIdRecents)
         } else {
            if (!isGrpd) { recentsIdsVec .lift(idx+1) .map(focusElemIdRecents)  .orElse (groupedIdsVec.headOption.map(focusElemIdGrouped)) }
            else { groupedIdsVec .lift(idx+1) .map(focusElemIdGrouped) .orElse (recentsIdsVec.headOption.map(focusElemIdRecents)) }
         }
   } }
   def focusPrevElem() = {
      groupedElemsMap .get(curFocusId) .map(e => (true,e.y)) .orElse {
         recentsElemsMap.get(curFocusId).map(e => (false,e.y))
      } .orElse { searchElemsMap.get(curFocusId).map(e => (true,e.y))
      } .foreach { case (isGrpd, idx) =>
         if (SwitchePageState.inSearchState) {
            searchIdsVec .lift(idx-1) .orElse (searchIdsVec.lastOption) .map(focusElemIdSearch)
         } else if (!SwitcheState.inGroupedMode) {
            recentsIdsVec .lift(idx-1) .orElse (recentsIdsVec.lastOption) .map(focusElemIdRecents)
         } else {
            if (!isGrpd) { recentsIdsVec .lift(idx-1) .map(focusElemIdRecents)  .orElse (groupedIdsVec.lastOption.map(focusElemIdGrouped)) }
            else { groupedIdsVec .lift(idx-1) .map(focusElemIdGrouped) .orElse (recentsIdsVec.lastOption.map(focusElemIdRecents)) }
         }
   } }
   def focusNextGroup() = {
      groupedElemsMap .get(curFocusId) .map(e => (true,e)) .orElse {
         recentsElemsMap.get(curFocusId).map(e => (false,e))
      } .orElse { searchElemsMap.get(curFocusId).map(e => (false,e))
      } .foreach {case (isGrpd, e) =>
         if (SwitchePageState.inSearchState) {
            searchIdsVec .lift(e.y+1) .orElse (searchIdsVec.headOption) .map(focusElemIdSearch)
         } else if (!SwitcheState.inGroupedMode) {
            recentsIdsVec .lift(e.y+1) .orElse (recentsIdsVec.headOption) .map(focusElemIdRecents)
         } else { // i.e. in grp mode
            if (!isGrpd) { groupedIdsVec.headOption.map(focusElemIdGrouped) }
            else { groupsHeadsIdsVec .lift(e.yg+1) .map(focusElemIdGrouped) .orElse (recentsIdsVec.headOption.map(focusElemIdRecents)) }
         }
   } }
   def focusPrevGroup() = {
      groupedElemsMap .get(curFocusId) .map(e => (true,e)) .orElse {
         recentsElemsMap.get(curFocusId).map(e => (false,e))
      } .orElse { searchElemsMap.get(curFocusId).map(e => (true,e))
      } .foreach { case (isGrpd, e) =>
         if (SwitchePageState.inSearchState) {
            searchIdsVec .lift(e.y-1) .orElse (searchIdsVec.lastOption) .map(focusElemIdSearch)
         } else if (!SwitcheState.inGroupedMode) {
            recentsIdsVec .lift(e.y-1) .orElse (recentsIdsVec.lastOption) .map(focusElemIdRecents)
         } else { // i.e. in grp mode
            if (!isGrpd) { groupsHeadsIdsVec.lastOption.map(focusElemIdGrouped) }
            else { groupsHeadsIdsVec .lift(e.yg-1) .map(focusElemIdGrouped) .orElse (recentsIdsVec.headOption.map(focusElemIdRecents)) }
         }
   } }
   def focusTopElem() = {
      if (SwitchePageState.inSearchState) { searchIdsVec.headOption.map(focusElemIdSearch) }
      else { recentsIdsVec.headOption.map(focusElemIdRecents) }
   }
   def focusBottomElem() = {
      if (SwitchePageState.inSearchState) { searchIdsVec.lastOption.map(focusElemIdSearch) }
      else if (SwitcheState.inGroupedMode) { groupedIdsVec.lastOption.map(focusElemIdGrouped) }
      else { recentsIdsVec.lastOption.map(focusElemIdRecents) }
   }

   def handleCurElemActivationReq() = { SwitcheState.handleWindowActivationReq(idToHwnd(curFocusId)) }
   def handleCurlElemMinimizeReq() = { SwitcheState.handleWindowMinimizeReq(idToHwnd(curFocusId)) }
   def handleCurElemCloseReq() = { SwitcheState.handleWindowCloseReq(idToHwnd(curFocusId)) }
   def handleCurElemShowReq() = { SwitcheState.handleWindowShowReq(idToHwnd(curFocusId)) }

   def handleSecondRecentActivationReq() = {
      recentsIdsVec.drop(1).headOption .map(idToHwnd) .foreach(SwitcheState.handleWindowActivationReq)
   }

   def handleMouseEnter (idStr:String, elem:Div) = { if (!isHoverLocked) { curFocusId = idStr; elem.focus() } }
   def triggerHoverLockTimeout() = {
      isHoverLocked = true; val t = js.Date.now(); lastActionStamp = t;
      js.timers.setTimeout(hoverLockTime){checkHoverLockTimeout(t)}
   }
   def checkHoverLockTimeout(kickerStamp:Double) = { if (lastActionStamp == kickerStamp) isHoverLocked = false; }

   def verifyActionRepeatSpacing (minRepeatSpacingMs:Double) : Boolean = {
      val t = scalajs.js.Date.now()
      if ((t - lastActionStamp) < minRepeatSpacingMs) { return false }
      lastActionStamp = t
      return true
   }

   def makeElemBox (idStr:String, e:WinDatEntry, dimExeSpan:Boolean=false):Div = {
      val exeInnerSpan = span ( e.exePathName.map(_.name).getOrElse("exe..").toString ).render
      val titleInnerSpan = span ( e.winText.getOrElse("title").toString ).render
      makeElemBox ( idStr, e, dimExeSpan, exeInnerSpan, titleInnerSpan )
   }
   def makeElemBox (idStr:String, e:WinDatEntry, dimExeSpan:Boolean, exeInnerSpan:Span, titleInnerSpan:Span):Div = {
      val exeSpanClass = s"exeSpan${if (dimExeSpan) " dim" else ""}"
      val exeSpan = span (`class`:=exeSpanClass, exeInnerSpan)
      val titleSpan = span (`class`:="titleSpan", titleInnerSpan)
      val ico = e.exePathName.map(_.fullPath) .map ( path =>
         IconsManager.getCachedIcon(e.hwnd,path) .map (icoStr => img(`class`:="ico", src:=icoStr))
      ).flatten.getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val elem = div (`class`:="elemBox", id:=idStr, tabindex:=0, exeSpan, nbsp(3), icoSpan, nbsp(), titleSpan).render
      //elem.onclick = {ev:MouseEvent => SwitcheState.handleWindowActivationReq(e.hwnd)} // moved handlers to single global js
      elem.onmouseenter = {ev:MouseEvent => handleMouseEnter(idStr,elem)}
      elem
   }
   def rebuildRecentsElems() = {
      recentsElemsMap = mutable.LinkedHashMap()
      val recentsToTake = if (SwitcheState.inGroupedMode) {SwitcheState.getRenderList.take(groupedModeTopRecentsCount)} else {SwitcheState.getRenderList()}
      recentsToTake .zipWithIndex .foreach {case (d,i) =>
         val id = getElemId(d.hwnd,false); recentsElemsMap.put(id, OrderedElemsEntry (i,makeElemBox(id,d)))
      }
      recentsIdsVec = recentsElemsMap.keys.toVector
   }
   def rebuildGroupedElems() = {
      rebuildRecentsElems() // for the topfreqs portion
      groupedElemsMap = mutable.LinkedHashMap()
      groupsHeadsIdsVec = mutable.ArrayBuffer()
      case class PartOrderedElem (id:String, d:Div, grpIdx:Int)
      def getIdElemH(d:WinDatEntry,isExeDim:Boolean,grpIdx:Int) = {
         val id = getElemId(d.hwnd,true); PartOrderedElem (id, makeElemBox(id,d,isExeDim), grpIdx)
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
      def getIdElem(d:WinDatEntry, isExeDim:Boolean, r:CheckSearchExeTitleRes) = {
         val id = s"${d.hwnd}_s"; val elem = makeElemBox(id,d,isExeDim,r.exeSpan,r.titleSpan); (id,elem)
      }
      SwitcheState.getGroupedRenderList() .map {_ .map { d =>
         d -> SearchHelper.checkSearchExeTitle (d.exePathName.map(_.name).getOrElse(""), d.winText.getOrElse(""), matchStr)
      } .filter { case (d,res) => res.chkPassed }
      } .filterNot(_.isEmpty) .map { ll =>
         Seq ( ll.take(1).map{case(d,r) => getIdElem(d,false,r)}, ll.tail.map{case(d,r) => getIdElem(d,true,r)} ).flatten
      }.flatten .zipWithIndex .foreach {case ((id,e),i) => searchElemsMap.put (id, OrderedElemsEntry(i,e)) }
      searchIdsVec = searchElemsMap.keys.toVector
   }
   def reSyncCurFocusIdAfterRebuild() = {
      def checkResyncIdRecents(idStr:String) = { recentsElemsMap.get(idStr).map {o => recentsIdsVec.lift(o.y).map(id => (id,o.elem))}.flatten }
      def checkResyncIdGrouped(idStr:String) = { groupedElemsMap.get(idStr).map {o => groupedIdsVec.lift(o.y).map(id => (id,o.elem))} .flatten }
      def checkResyncIdSearch (idStr:String) = { searchElemsMap.get(idStr).map {o => searchIdsVec.lift(o.y).map(id => (id,o.elem))} .flatten }
      {  if (SwitchePageState.inSearchState) {
            checkResyncIdSearch(s"${curFocusId.split("_").head}_s") .orElse(searchIdsVec.headOption.map(checkResyncIdSearch).flatten)
         } else if (SwitcheState.inGroupedMode) {
            checkResyncIdGrouped(curFocusId) .orElse (checkResyncIdRecents(curFocusId)) orElse (checkResyncIdGrouped(s"${curFocusId}_g"))
         } else { checkResyncIdRecents(curFocusId.split("_").head) }
      }.orElse { recentsElemsMap.headOption.map {case (id, o) => (id, o.elem)} }
      .foreach {case (id,elem) => curFocusId = id; elem.focus() }
   }
   def activateSearchBox () = { RibbonDisplay.searchBox.focus() }
   def exitSearchState() = { inSearchState = false; RibbonDisplay.searchBox.value = ""; ElemsDisplay.updateElemsDiv() }
   def handleSpaceKeyDown() = { if (inSearchState & !spaceKeyArmed) {RibbonDisplay.searchBox.focus()} else {handleCurElemActivationReq()} }
   def handleEscapeKeyUp() = { if (!inSearchState) {SwitcheState.handleSelfWindowHideReq()} else {exitSearchState()} }
   def handleSearchBoxKeyUp(e:KeyboardEvent) = {
      if (e.key == "Escape") {exitSearchState()}
      else { inSearchState = true; ElemsDisplay.updateElemsDiv(); spaceKeyArmed = false; }
   }
   def replaceTitleInnerSpan (elem:Div, newSpan:Span) = {
      clearedElem (elem.getElementsByClassName("titleSpan").apply(0)) .appendChild (newSpan)
   }
   def handleTitleUpdate(hwnd:Int, dat:WinDatEntry) = {
      if (!inSearchState) {
         recentsElemsMap.get(s"${hwnd}").map(_.elem).foreach{elem => replaceTitleInnerSpan (elem, span((dat.winText.getOrElse("title..")):String).render)}
         groupedElemsMap.get(s"${hwnd}_g").map(_.elem).foreach{elem => replaceTitleInnerSpan(elem, span((dat.winText.getOrElse("title..")):String).render)}
      } else {
         val sRes = SearchHelper.checkSearchExeTitle (dat.exePathName.map(_.name).getOrElse(""), dat.winText.getOrElse(""), RibbonDisplay.searchBox.value.trim)
         searchElemsMap.get(s"${hwnd}_s").map(_.elem).foreach{elem => replaceTitleInnerSpan(elem,sRes.titleSpan)}
      }
   }

}

object ElemsDisplay {
   import SwitcheFaceConfig._
   val elemsDiv = div (id:="elemsDiv").render
   def getElemsDiv = elemsDiv

   def updateElemsDivRecentsMode = {
      SwitchePageState.rebuildRecentsElems()
      val recentElemsHeader = div (`class`:="groupedModeHeader", nbsp(1), "Recents:")
      val recentElemsDivList = div (SwitchePageState.recentsElemsMap.values.map(_.elem).toSeq).render
      val elemsDivBlock = div (recentElemsHeader, recentElemsDivList)
      clearedElem(elemsDiv) .appendChild (elemsDivBlock.render)
   }
   def updateElemsDivGroupedMode = {
      SwitchePageState.rebuildGroupedElems()
      val recentElemsHeader = div (`class`:="groupedModeHeader", nbsp(1), "Recents:")
      val recentElemsDivList = div (SwitchePageState.recentsElemsMap.values.map(_.elem).toSeq).render
      val groupedElemsHeader = div (`class`:="groupedModeHeader", nbsp(1), "Grouped:")
      val groupedElemsDivList = div (SwitchePageState.groupedElemsMap.values.map(_.elem).toSeq).render
      val elemsDivBlock = div (recentElemsHeader, recentElemsDivList, groupedElemsHeader, groupedElemsDivList)
      clearedElem(elemsDiv) .appendChild (elemsDivBlock.render)
   }
   def updateElemsDivSearchState = {
      SwitchePageState.rebuildSearchElems()
      val searchElemsHeader = div (`class`:="groupedModeHeader", nbsp(1), "Searched::")
      val searchElemsDivList = div (SwitchePageState.searchElemsMap.values.map(_.elem).toSeq).render
      val elemsDivBlock = div (searchElemsHeader, searchElemsDivList)
      clearedElem(elemsDiv) .appendChild (elemsDivBlock.render)
   }
   def updateElemsDiv () = {
      SwitcheState.updateRenderReadyLists()
      if (SwitchePageState.inSearchState) { updateElemsDivSearchState }
      else if (SwitcheState.inGroupedMode) { updateElemsDivGroupedMode }
      else { updateElemsDivRecentsMode }
      SwitchePageState.reSyncCurFocusIdAfterRebuild()
   }
}

object RibbonDisplay {
   import SwitcheFaceConfig._
   import SwitcheState._
   val countSpan = span (id:="countSpan", style:="-webkit-app-region:drag").render
   val debugLinks = span ().render
   val searchBox = input (`type`:="text", id:="searchBox", placeholder:="").render
   searchBox.onkeyup = (e:KeyboardEvent) => {SwitchePageState.handleSearchBoxKeyUp(e)}
   // the box itself takes chars on keypress, do consider interactions here ^ w broader handling.. e.g handling keyup here w keydown elsewhere etc
   def updateCountsSpan (n:Int) = { clearedElem(countSpan) .appendChild ( span ( nbsp(3), s"($n) ※", nbsp(3) ).render ) }
   def updateDebugLinks() = {
      clearElem(debugLinks)
      if (SwitcheState.inElectronDevMode) {
         val printExclLink =  a ( href:="#", "DebugPrint", onclick:={e:MouseEvent => handleDebugPrintReq()} ).render
         debugLinks.appendChild ( printExclLink )
   } }
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={e:MouseEvent => handleRefreshRequest()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleGroupModeToggleReq()} )
      //val dragSpot = span (style:="-webkit-app-region:drag", nbsp(3), "※", nbsp(3)) // combined with count instead
      div ( id:="top-ribbon",
         nbsp(2), reloadLink, nbsp(4), refreshLink, nbsp(4), groupModeLink, countSpan, debugLinks, nbsp(4), searchBox
      ).render
   }
}

