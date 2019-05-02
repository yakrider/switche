package switche

import org.scalajs.dom
import org.scalajs.dom.FocusEvent
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

object SwitcheFacePage {
   
   def getShellPage () = {
      val topRibbon = RibbonDisplay.getTopRibbonDiv()
      val elemsDiv = ElemsDisplay.getElemsDiv
      val page = div (topRibbon, elemsDiv)
      setPageEventHandlers()
      page.render
   }
   //def queueRender() = g.window.requestAnimationFrame({t:js.Any => render()}) // used spaced render call instead
   def render() = { //println("rendering")
      SwitcheState.updateRenderReadyLists()
      ElemsDisplay.updateElemsDiv()
      RibbonDisplay.updateCountsSpan(SwitcheState.getRenderList.size)
   }
   def printKeyDebugInfo (e:KeyboardEvent,evType:String) = {
      println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, modCtrl:${e.getModifierState("Control")}, modCaps:${e.getModifierState("CapsLock")}")
   }
   def setPageEventHandlers() = {
      import SwitchePageState._
      import SwitcheState._
      // note that Escape/Tab only give key down/up.. all kbd actions should trigger a short hover lock to avoid mouse clashes
      dom.document.onkeydown = (e:KeyboardEvent) => { //printKeyDebugInfo(e,"down")
         triggerHoverLockTimeout()
         if (e.ctrlKey) {
            if (e.key == " ") {handleCurElemActivationReq()}
            else if (e.key == "t") {handleGroupModeToggleReq()}
            else if (e.key == "w") {handleCurElemCloseReq()}
            else if (e.key == "v") {handleCurElemShowReq()}
            else if (e.key == "r") {handleRefreshRequest()}
            else if (e.key == "f") {activateSearchBox()}
         }
         else {
            if (e.key == " ") {handleSpaceKey()} // special as we overload it for activation ONLY if search hasnt started, else use ctrl
            else if (e.key == "F1") {focusNextElem()} // note: not really needed, registered as global hotkey, set electron to forwards it as a call
            else if (e.key == "F2") {focusPrevElem()}
            else if (e.key == "Tab") { e.stopPropagation(); e.preventDefault();  focusNextElem() }
            else if (e.key == "ArrowDown") {focusNextElem()}
            else if (e.key == "ArrowUp") {focusPrevElem()}
            else if (e.key == "PageUp") {focusTopElem()}
            else if (e.key == "PageDown") {focusBottomElem()}
            else if (e.key == "Enter") {handleCurElemActivationReq()}
            else if (e.key == "Escape") {handleEscapeKey()}
            else if (e.key == "F5") {dom.window.location.reload()}
            else {activateSearchBox()}
      } }
      //dom.document.onkeypress = (e:KeyboardEvent) => { printKeyDebugInfo(e,"press") }
      //dom.document.onkeyup = (e:KeyboardEvent) => { }//printKeyDebugInfo(e,"up")
      dom.document.onmousewheel = (e:WheelEvent) => {
         triggerHoverLockTimeout()
         if (e.deltaY > 0) { focusNextElem() } else { focusPrevElem() }
      }
      dom.document.oncontextmenu = (e:MouseEvent) => { triggerHoverLockTimeout(); handleCurElemCloseReq() }
   }
}

object SwitchePageState {
   import SwitcheFaceConfig._
   // doing recents and grouped elems separately as they literally are different divs (w/ diff styles etc)
   case class OrderedElemsEntry (y:Int, elem:Div)
   var recentsElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var groupedElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var searchElemsMap: mutable.LinkedHashMap[String,OrderedElemsEntry] = _
   var recentsIdsVec: Vector[String] = _
   var groupedIdsVec: Vector[String] = _
   var searchIdsVec: Vector[String] = _
   var curFocusId:String = ""; //var curFocusVecIdx:Int = _; // the idx is to be able to check after we rebuild if we can maintain id?
   var isHoverLocked:Boolean = false; var lastActionStamp:Double = 0 //js .Date.now()
   var inSearchState:Boolean = false;
   
   def rebuildElems() = {} // instead of full render, consider surgical updates to divs directly w/o waiting for global render etc
   
   def getElemId (hwnd:Int,isGrpElem:Boolean) = s"${hwnd}${if (isGrpElem) "_g" else ""}"
   def idToHwnd (idStr:String) = idStr.split("_").head.toInt // if fails, meh, its js!
   def isIdGrp (idStr:String) = {idStr.split("_").head != idStr}
   
   private def focusElemIdRecents(id:String) = recentsElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus()}
   private def focusElemIdGrouped(id:String) = groupedElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus()}
   private def focusElemIdSearch (id:String) = searchElemsMap.get(id) .foreach {e => curFocusId = id; e.elem.focus()}
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
   def handleCurElemCloseReq() = { SwitcheState.handleWindowCloseReq(idToHwnd(curFocusId)) }
   def handleCurElemShowReq() = { SwitcheState.handleWindowShowReq(idToHwnd(curFocusId)) }
   def handleMouseEnter (idStr:String, elem:Div) = { if (!isHoverLocked) { curFocusId = idStr; elem.focus() } }
   def triggerHoverLockTimeout() = {
      isHoverLocked = true; val t = js.Date.now(); lastActionStamp = t;
      js.timers.setTimeout(hoverLockTime){checkHoverLockTimeout(t)}
   }
   def checkHoverLockTimeout(kickerStamp:Double) = { if (lastActionStamp == kickerStamp) isHoverLocked = false; }
   
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
         IconsManager.getCachedIcon(path) .map (ico => img(`class`:="ico", src:=s"data:image/png;base64, $ico"))
      ).flatten.getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val elem = div (`class`:="elemBox", id:=idStr, tabindex:=0, exeSpan, nbsp(3), icoSpan, nbsp(), titleSpan).render
      elem.onclick = {ev:MouseEvent => SwitcheState.handleWindowActivationReq(e.hwnd)}
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
      def getIdElem(d:WinDatEntry, isExeDim:Boolean) = {val id = getElemId(d.hwnd,true); (id,makeElemBox(id,d,isExeDim))}
      SwitcheState.getGroupedRenderList .map { ll =>
         Seq ( ll.take(1).map(d => getIdElem(d,false)), ll.tail.map(d => getIdElem(d,true)) ).flatten
      }.flatten .zipWithIndex .foreach {case ((id,e),i) => groupedElemsMap.put (id, OrderedElemsEntry(i,e)) }
      groupedIdsVec = groupedElemsMap.keys.toVector
   }
   def rebuildSearchElems() : Unit = {
      searchElemsMap = mutable.LinkedHashMap()
      val matchStr = RibbonDisplay.searchBox.value.trim
      def getIdElem(d:WinDatEntry, isExeDim:Boolean, r:CheckSearchExeTitleRes) = {
         val id = s"${d.hwnd}_s"; val elem = makeElemBox(id,d,isExeDim,r.exeSpan,r.titleSpan); (id,elem)
      }
      val x = SwitcheState.getGroupedRenderList() .map {_ .map { d =>
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
   /*
   def focusElem (hwnd:Int, isGrpElem:Boolean=false) = {
      val elem = dom.document.querySelector(s"#${getElemId(hwnd,isGrpElem)}").asInstanceOf[dom.raw.HTMLElement]
      elem.focus()
   }*/
   
   def handleSpaceKey() = {
      if (!inSearchState) { handleCurElemActivationReq() }
      else {RibbonDisplay.searchBox.focus()} // nothing, as the box auto takes care of its kbd events
   }
   def handleEscapeKey() = {
      if (inSearchState) {RibbonDisplay.searchBox.focus()} // passes on to searchbox handler
      else {SwitcheState.handleSelfWindowHideReq()}
   }
   def activateSearchBox () = {
      if (!inSearchState) { inSearchState = true; }
      RibbonDisplay.searchBox.focus()
   }
   def exitSearchState() = {
      inSearchState = false; RibbonDisplay.searchBox.value = ""; ElemsDisplay.updateElemsDiv()
   }
   def handleSearchBoxKeyUp(e:KeyboardEvent) = {
      if (e.key == "Escape") { e.stopPropagation(); e.preventDefault(); exitSearchState() }
      ElemsDisplay.updateElemsDiv() // gotta update on every keyup
   }
   
}

object ElemsDisplay {
   import SwitcheFaceConfig._
   val elemsDiv = div (id:="elemsDiv").render
   def getElemsDiv = elemsDiv
   
   def updateElemsDivRecentsMode = {
      SwitchePageState.rebuildRecentsElems()
      val elemsDivList = div(SwitchePageState.recentsElemsMap.values.map(_.elem).toSeq).render
      clearedElem(elemsDiv) .appendChild (elemsDivList)
   }
   def updateElemsDivGroupedMode = {
      SwitchePageState.rebuildGroupedElems()
      val recentElemsHeader = div (`class`:="groupedModeHeader", "Recents:")
      val recentElemsDivList = div (SwitchePageState.recentsElemsMap.values.map(_.elem).toSeq).render
      val groupedElemsHeader = div (`class`:="groupedModeHeader", "Grouped:")
      val groupedElemsDivList = div (SwitchePageState.groupedElemsMap.values.map(_.elem).toSeq).render
      val elemsDivBlock = div (recentElemsHeader, recentElemsDivList, groupedElemsHeader, groupedElemsDivList)
      clearedElem(elemsDiv) .appendChild (elemsDivBlock.render)
   }
   def updateElemsDivSearchState = {
      SwitchePageState.rebuildSearchElems()
      val searchElemsHeader = div (`class`:="groupedModeHeader", "Searched::")
      val searchElemsDivList = div (SwitchePageState.searchElemsMap.values.map(_.elem).toSeq).render
      val elemsDivBlock = div (searchElemsHeader, searchElemsDivList)
      clearedElem(elemsDiv) .appendChild (elemsDivBlock.render)
   }
   def updateElemsDiv () = {
      if (SwitchePageState.inSearchState) { updateElemsDivSearchState }
      else if (SwitcheState.inGroupedMode) { updateElemsDivGroupedMode }
      else { updateElemsDivRecentsMode }
      SwitchePageState.reSyncCurFocusIdAfterRebuild()
   }
}

object RibbonDisplay {
   import SwitcheFaceConfig._
   import SwitcheState._
   val countSpan = span (id:="countSpan").render
   val searchBox = input (`type`:="text", id:="searchBox", placeholder:="").render
   searchBox.onkeyup = (e:KeyboardEvent) => {SwitchePageState.handleSearchBoxKeyUp(e)}
   // the box itself takes chars on keypress, do consider interactions here ^ w broader handling.. e.g handling keyup here w keydown elsewhere etc
   def updateCountsSpan (n:Int) = { clearedElem(countSpan).appendChild( span(s"($n)").render ) }
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={e:MouseEvent => handleRefreshRequest()} )
      val printExclLink = a (href:="#", "ExclPrint", onclick:={e:MouseEvent => handleExclPrintReq()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleGroupModeToggleReq()} )
      div ( id:="top-ribbon", reloadLink, nbsp(4), refreshLink, nbsp(4), printExclLink, nbsp(4),
         countSpan, nbsp(4), groupModeLink, nbsp(8), searchBox
      ).render
   }
}

