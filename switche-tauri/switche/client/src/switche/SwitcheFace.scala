package switche

import org.scalajs.dom
import org.scalajs.dom.{Element, EventTarget, HTMLElement, KeyboardEvent, MouseEvent, WheelEvent, window, document => doc}
import org.scalajs.dom.html.{Div, Span}
import scalatags.JsDom.all._

import scala.collection.{IndexedSeq, Map, mutable}
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}
import scala.util.Try


object SwitcheFaceConfig {
   val groupedModeTopRecentsCount = 9
   val hoverLockTime = 300 // ms
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.Element): Unit = { e.innerHTML = ""}
   def clearedElem (e:dom.Element) = { e.innerHTML = ""; e }
}


// some type defs for the various modes and states for various ui sections
sealed abstract class GrpT (val cls:String)
object GrpTs {
   case object NG extends GrpT ("ng")   // Non-Grouped
   case object GH extends GrpT ("gh")   // Group-Head
   case object GT extends GrpT ("gt")   // Group-Tail
}

sealed abstract class ElemT (val cls:String)
object ElemTs {
   case object R  extends ElemT ("r")     // recents-mode recents block
   case object G  extends ElemT ("g")     // grouped mode grouped block
   case object GR extends ElemT ("gr")    // grouped mode recents block
}

sealed abstract class StateT (val cls:String)
object StateTs {
   case object L  extends StateT ("l")    // listing state (search not activated)
   case object S  extends StateT ("s")    // searching state
}


object SwitcheFacePage {
   import SwitchePageState._
   import Switche._

   def getShellPage () = {
      setPageEventHandlers()
      div ( id:="scala-js-root-div", RibbonDisplay.getTopRibbonDiv(), ElemsDisplay.getElemsDiv ) .render
   }
   // NOTE that the intention now is to always call this via RenderSpacer (spaced or immdt) instead of directly, so the render time can be recorded
   def updatePageElems () : Unit = { //println(s"rendering @${js.Date.now()}")
      // note that although this seems expensive to call rebuild on every render, unchanged cases get diffed and ignored by browser engine keeping it cheap
      ElemsDisplay.updateElemsDiv()
      RibbonDisplay.updateCountsSpan()
   }
   def printKeyDebugInfo (e:KeyboardEvent, evType:String) = {
      //println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, modCtrl:${e.getModifierState("Control")}, modCaps:${e.getModifierState("CapsLock")}")
      println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, alt:${e.altKey}, shift:${e.shiftKey}")
   }
   
   def setPageEventHandlers() = {
      // reminder here.. capture phase means its just going down from top level to target, after that bubble phase goes from target upwards
      // intercepting here at the 'capture' phase allows us to use e.stopPropagation() to prevent event from ever reaching target
      doc.addEventListener ("click",       procMouse_Click _)
      doc.addEventListener ("contextmenu", procMouse_ContextMenu _)
      doc.addEventListener ("auxclick",    procMouse_AuxClick _)
      doc.addEventListener ("mouseup",     procMouse_Up _)
      doc.addEventListener ("wheel",       procMouse_Wheel _)
      doc.addEventListener ("keyup",       capturePhaseKeyUpHandler _, useCapture=true)
      doc.addEventListener ("keydown",     capturePhaseKeyDownHandler _, useCapture=true)
      //doc.addEventListener ("keypress",    capturePhaseKeyPressHandler _, useCapture=true)
      //dom.document.addEventListener ("mouseenter", procMouse_Enter _, useCapture=true) // done from element for efficiency
   }
   
   def procMouse_Click (e:MouseEvent) = {
      triggerHoverLockTimeout()
      scrollEnd_disarm()
      //e.target.closest(".elemBox").foreach(_=>handleReq_CurElemActivation())
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e.asInstanceOf[Element] .closest(".elemBox") )  // can return a null so wrapping into option
      } .foreach ( _ => handleReq_CurElemActivation())
      // ^^ we dont actually need the closest, we're just filtering by whether closest can find an elemBox in its ancestors
      // .. this makes it such that in rows the mouse click is active while elsewhere in empty body it is not!
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
      //e.target.closest(".elemBox").foreach(_=>handleReq_CurElemClose())
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e.asInstanceOf[Element] .closest(".elemBox") )  // can return a null so wrapping into option
      } .foreach ( _ => handleReq_CurElemClose())
      // ^^ we dont actually need the closest, we're just filtering by whether closest can find an elemBox in its ancestors
      // .. this makes it such that in rows the mouse click is active while elsewhere in empty body it is not!
   }
   def procMouse_RightClick (e:MouseEvent) = {
      // eventually could consider supporting more native right-click+wheel global combo here
      // but for now, we're using ahk to send separate hotkeys for right-mouse + wheel-down and enc scroll, so can use this for closing windows
      triggerHoverLockTimeout()
      //scrollEnd_disarm()
      e.preventDefault(); e.stopPropagation()
      //handleReq_CurElemClose()
      // ^ disabling, as middle click seems to gets used exclusively, and right click mostly only seems to trigger accidentally
   }
   def procMouse_ContextMenu (e:MouseEvent) = { //println (s"got context menu click, btn:${e.button}")
      // this fires separately from the auxclick 2 report on right-click
      triggerHoverLockTimeout(); e.preventDefault(); e.stopPropagation()
   }
   def procMouse_Up (e:MouseEvent) = {
      if (e.button == 1) {procMouse_MiddleClick(e)}
   }
   def procMouse_Wheel (e:WheelEvent) = {
      e.preventDefault()
      if (verifyActionRepeatSpacing(20d)) {  // enforced spacing (in ms) between consecutive mouse scroll action handling
         triggerHoverLockTimeout(); //scrollEnd_arm();
         if (e.deltaY > 0 || e.deltaX > 0) {
            if (e.ctrlKey) focusGroup_Next() else  focusElem_Next()
         } else {
            if (e.ctrlKey) focusGroup_Prev() else focusElem_Prev()
         }
   } }
   def procMouse_Enter (e:MouseEvent) = {
      // NOTE: this is deprecated in favor of directly setting mouse-enter in the div boxes
      //handleMouseEnter ( e.target.asInstanceOf[Element].closest(".elemBox").asInstanceOf[Div] )
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e .asInstanceOf[Element] .closest(".elemBox") .asInstanceOf[Div] )
      } .foreach (handleMouseEnter)
   }
   
   def capturePhaseKeyUpHandler (e:KeyboardEvent) = {    printKeyDebugInfo(e,"up")
      // note: escape can cause app hide, and when doing that, we dont want that to leak outside app, hence on keyup
      if (inSearchState) { // && RibbonDisplay.searchBox.value.nonEmpty) {
         handle_SearchModeKeyup(e)   // let it recalc matches if necessary etc
      } else { // not in search state
         if (e.key == "Escape")  handleReq_SwitcheEscape()
      }
   }
   
   //val modifierKeys = Set ("Meta","Alt","Control","Shift")
   val passthroughKeys = Set ("MediaPlayPause","MediaTrackNext","MediaTrackPrevious","AudioVolumeUp","AudioVolumeDown")
   val searchBoxCtrlKeys = Set ("ArrowLeft","ArrowRight","Delete","Backspace", "a", "x", "c", "v", "z", "y")
   val searchBoxExclKeys = Set ("Meta","Alt","Control","Shift", "AudioVolumeUp","AudioVolumeDown")
   
   
   def capturePhaseKeyDownHandler (e:KeyboardEvent) = {    printKeyDebugInfo(e,"down")
      
      var (doStopProp, preventDefault) = (true, false)
      // ^^ note that we're setting default to stop further propagation (but preventing default is uncommon)
      
      // we'll call this whenever we want to setup selective propagation to searchbox
      @inline def setupSearchbox (doPassthrough:Boolean) = {
         doStopProp = searchBoxExclKeys.contains(e.key) || (!doPassthrough && !inSearchState)
         if (!searchBoxExclKeys.contains(e.key)) { inSearchState = true; activateSearchBox() }
      }
      // and at the end, this can setup event prop and default based on flags we'll have updated
      @inline def eventPassthroughGuarded() = { //println(s"doStopProp=$doStopProp")
         if (preventDefault) { e.preventDefault() }
         if (!passthroughKeys.contains(e.key) && doStopProp) { e.stopPropagation(); e.preventDefault(); }
      }
    
      triggerHoverLockTimeout()
      
      (scrollEnd_armed, inSearchState, e.altKey, e.ctrlKey, e.key) match {
         
         // first, keys that are enabled for both normal and  search-state, and with or without alt/ctrl etc :
         case (_, _, _, _, "Enter")      => handleReq_CurElemActivation()
         case (_, _, _, _, "Escape")     =>  //handleReq_SwitcheEscape()   // moved to keyup to avoid leakage of keyup after we use it hide app
         case (_, _, _, _, "F5")         => dom.window.location.reload()
         
         // scroll/invoke hotkeys nav
         case (_, _, _, _, "F1")         => focusElem_Next()
         case (_, _, _, _, "F15")        => focusElem_Next()
         case (_, _, _, _, "F2")         => focusElem_Prev()
         // ^^ these are regular invocation (not using alt-tab or rht-srcoll, so we dont arm scroll-end activation
         
         case (_, _, _, _, "F16")        => scrollEnd_arm(); if (e.shiftKey) focusElem_Prev() else focusElem_Next()
         case (_, _, _, _, "F17")        => scrollEnd_arm(); if (e.shiftKey) focusElem_Next() else focusElem_Prev()
         // ^^ these are scroll specific nav, so even w/o alt, we'll arm scroll-end .. (e.g. right-mouse-scroll or alt-tab)
         
         case (_, _, _, _, "Tab")        => if (e.shiftKey) focusElem_Prev() else focusElem_Next()
         // ^^ tab w/o alt should be regular next-nav, but w/o arming scroll-end .. (and w/ alt, shouldnt get here w hook interception)
         
         // arrow up/down nav .. consistent regardless of everything else
         case (_, _, _, _, "ArrowUp")    => focusElem_Prev()
         case (_, _, _, _, "ArrowDown")  => focusElem_Next()
         case (_, _, _, _, "PageUp")     => focusElem_Top()
         case (_, _, _, _, "PageDown")   => focusElem_Bottom()
         
         // arrow left-right group nav while armed, w alt, or non-search .. else it'll just pass on to searchbox
         case (_, _, _, _, "ArrowLeft")   if (!inSearchState || scrollEnd_armed || e.altKey)  => focusGroup_Prev()
         case (_, _, _, _, "ArrowRight")  if (!inSearchState || scrollEnd_armed || e.altKey)  => focusGroup_Next()
         
         
         // space key .. usually for cur elem activation .. but in search state, it'll pass through to searchbox
         case (_, false, _, _,     " ")   => handleReq_CurElemActivation()    // non-search state
         case (_, true,  _, true,  " ")   => handleReq_CurElemActivation()    // search-state w/ ctrl key
         
         
         // alt-f4 .. we'll directly exit app ..  (although just doing doStopProp = false would also work indirectly)
         case (_, _, true, _, "F4")   =>  handleReq_SwitcheQuit()
         
         // alt w ctrl/shift ... used for specific app switching
         case (_, _, true, _, _)  if (e.ctrlKey || e.shiftKey) => {
            preventDefault = true
            e.key.toLowerCase match {
               case "l"  => SendMsgToBack.FE_Req_Switch_Last()
               case "o"  => SendMsgToBack.FE_Req_Switch_TabsOutliner()
               case "n"  => SendMsgToBack.FE_Req_Switch_NotepadPP()
               case "i"  => SendMsgToBack.FE_Req_Switch_IDE()
               case "m"  => SendMsgToBack.FE_Req_Switch_Music()
               case "b"  => SendMsgToBack.FE_Req_Switch_Browser()
               case _ =>
            }
         }
         
         // alt-key or scroll-end-armed ..  we'll setup alt-tab state key nav options
         case (_, _, true, _, _) | (true, _, _, _, _) => {
            e.key match {
               case "i"  => focusElem_Prev()
               case ","  => focusElem_Next()
               case "u"  => focusElem_Top()
               case "m"  => focusElem_Bottom()
               case "j"  => focusGroup_Prev()
               case "k"  => focusGroup_Next()
               
               case "s" | "l" | "S" | "L" =>
                  e.preventDefault(); scrollEnd_disarm(); setupSearchbox (doPassthrough = false)
                  
               case _ => // all other alt combos, or while armed can be ignored
            }
         }
         
         
         // ctrl specific hotkeys .. typically for switche specific actions
         case (_, _, false, true, _) => {
            preventDefault = true
            e.key.toLowerCase match {
               case "r"  => RibbonDisplay.handleRefreshBtnClick()
               case "f"  => RibbonDisplay.handleRefreshBtnClick()
               case "g"  => handleReq_GroupModeToggle()
               case "w"  => handleReq_CurElemClose()
               case "p"  => handleReq_CurElemPeek()
               //case "z"  => handleReq_CurElemMinimize()   // disabled to allow for use in search-box instead
               //case "x"  => handleReq_CurElemMaximize()
               
               case _  if (searchBoxCtrlKeys.contains(e.key)) => {
                  // ^^ we'll support some ctrl-combos for searchbox .. e.g left-right arrows, delete/bksp, ctr-a/x/c/v/z/y
                  preventDefault = false
                  setupSearchbox (doPassthrough = true)
               }
               case _ =>   // other ctrl hotkeys can be ignored
            }
         }
         
         // all other non-alt non-ctrl events can be passed through to searchbox
         case (_, _, false, false, _) => setupSearchbox (doPassthrough = true)
         
         case _ =>   // we shouldnt even get this far, but either way we'd just drop it

       }
      
      eventPassthroughGuarded()
      // ^^ basically all key-down events other than for propagation to searchbox should end here!!

   }

}



object SwitchePageState {
   import SwitcheFaceConfig._
   import Switche._

   // doing recents and grouped elems separately as they literally are different divs (w/ diff styles etc)
   // note that in search mode, both of these containers will be updated with search style elems and search-filtered id-vecs
   case class OrderedElemsEntry (y:Int, elem:Div, yg:Int=(-1))
   var recentsElemsMap:   Map[String,OrderedElemsEntry] = Map()
   var groupedElemsMap:   Map[String,OrderedElemsEntry] = Map()
   var recentsIdsVec:     IndexedSeq[String] = Vector()
   var groupedIdsVec:     IndexedSeq[String] = Vector()
   var groupsHeadsIdsVec: IndexedSeq[String] = Vector()
   var curElemId = ""; var inSearchState = false;
   var isHoverLocked = false; var lastActionStamp = 0d;
   // ^^ hover-lock flag locks-out mouseover, and is intended to be set (w small timeout) while mouse scrolling/clicks etc
   // .. and that prevents mouse jiggles from screwing up any in-preogress mouse scrolls, clicks, key-nav etc
   var isFreshRendered = true;
   // ^^ mouse-enter retriggers after repaint w/o mouse movement, so we'll use this flag to ignore the first event after repaint


   def recentsId (hwnd:Int) = s"${hwnd}_r"
   def groupedId (hwnd:Int) = s"${hwnd}_g"
   def idToHwnd (idStr:String) = idStr.split("_") .headOption .flatMap (s => Try(s.toInt).toOption)


   def handleReq_SwitcheEscape (fromBkndHotkey:Boolean = false) = { //println("dismissed")
      scrollEnd_disarm(); setDismissed()
      if (!fromBkndHotkey) { SendMsgToBack.FE_Req_SwitcheEscape() }
      // we'll do a delayed focus-reset so the visual flip happens out of sight after switche window is gone
      js.timers.setTimeout(300) {  SwitchePageState.resetFocus() }
   }
   def handleReq_SwitcheQuit () = {
      SendMsgToBack.FE_Req_SwitcheQuit()
   }
   
   def handleReq_CurElemActivation() : Unit = {
      scrollEnd_disarm(); setDismissed()
      idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowActivate )
      js.timers.setTimeout(300) {   // again, small delay to avoid visible change
         if (SwitchePageState.inSearchState) { SwitchePageState.exitSearchState() }
         SwitchePageState.resetFocus()
      }
   }
   def handleReq_CurElemMinimize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMinimize ) }
   def handleReq_CurElemMaximize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMaximize ) }
   def handleReq_CurElemClose()    = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowClose    ) }
   def handleReq_CurElemPeek()     = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowPeek     ) }
   

   def handleMouseEnter (elem:Div) = { println("mouse enter!")
      if (!isHoverLocked && !isFreshRendered) { setCurElemHighlight(elem) }
      isFreshRendered = false
   }
   def handleHoverLockTimeout (kickerStamp:Double) = {
      if (lastActionStamp == kickerStamp) { isHoverLocked = false }
   }
   def triggerHoverLockTimeout() = {
      isHoverLocked = true; val t = js.Date.now(); lastActionStamp = t;
      js.timers.setTimeout(hoverLockTime) {handleHoverLockTimeout(t)}
   }
   def verifyActionRepeatSpacing (minRepeatSpacingMs:Double) : Boolean = {
      val t = scalajs.js.Date.now()
      if ((t - lastActionStamp) < minRepeatSpacingMs) { return false }
      lastActionStamp = t
      return true
   }


   def makeElemBox (idStr:String, wde:WinDatEntry, y:Int, elemT:ElemT, grpT:GrpT) : Div = {
      val exeInnerSpan = span ( wde.exe_path_name.map(_.name).getOrElse("exe..").toString ).render
      val yInnerSpan = span (`class`:="ySpan", f"${y}%2d" ).render
      val titleInnerSpan = span ( wde.win_text.getOrElse("title").toString ).render
      makeElemBox ( idStr, wde, y, elemT, grpT, exeInnerSpan, yInnerSpan, titleInnerSpan )
   }
   def makeElemBox (
      idStr:String, wde:WinDatEntry, y:Int, elemT:ElemT, grpT:GrpT,
      exeInnerSpan:Span, yInnerSpan:Span, titleInnerSpan:Span
   ) : Div = {
      val exeSpan = span (`class`:=s"exeSpan ${elemT.cls} ${grpT.cls}", exeInnerSpan)
      val ySpan = span (`class`:=s"ySpan ${elemT.cls}", yInnerSpan)
      val titleSpan = span (`class`:=s"titleSpan ${elemT.cls}", titleInnerSpan)
      val ico = Switche.getCachedIcon(wde.icon_cache_idx) .map (icoStr => img(`class`:="ico", src:=icoStr)) .getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val elem = div (`class`:="elemBox", id:=idStr, tabindex:=0, exeSpan, nbsp(2), ySpan, nbsp(2), icoSpan, nbsp(), titleSpan).render
      //elem.onclick = {ev:MouseEvent => SwitcheState.handleReq_WindowActivation(e.hwnd)}
      // ^^ moved most handlers to single document-level events handler
      elem.onmouseenter = {(ev:MouseEvent) => handleMouseEnter(elem)}
      // ^^ but we left mouseenter here, as doing that globally is pointlessly inefficient
      elem
   }

   def rebuildRecentsElems() = {
      // this needs the elems table, and a vec to navigate through it
      val elemsMap = mutable.LinkedHashMap[String,OrderedElemsEntry]()
      val cappedRecents = {
         if (!inGroupedMode) renderList else renderList.take(groupedModeTopRecentsCount)
      }
      cappedRecents .flatMap(e => hwndMap.get(e.hwnd).map(d => (d, e))) .zipWithIndex .foreach { case ((wde,rle),i) =>
         val id = recentsId (wde.hwnd)
         val elemT = if (inGroupedMode) ElemTs.GR else ElemTs.R
         val elem = makeElemBox (id, wde, rle.y, elemT, GrpTs.NG)
         elemsMap.put (id, OrderedElemsEntry (i, elem))
      }
      recentsElemsMap = elemsMap                // will cast it to immutable trait
      recentsIdsVec = elemsMap.keys.toVector
   }

   def rebuildGroupedElems() = {
      // this additionally needs a group-heads vec to nav across groups (and has non-grp-head exes dimmed)
      val elemsMap = mutable.LinkedHashMap[String,OrderedElemsEntry]()
      val groupsHeadsIdsBuf = mutable.ArrayBuffer[String]()
      case class GrpIdxdElem (id:String, d:Div, grpIdx:Int)
      def getGrpIdxdElem (rle:RenderListEntry, grpT:GrpT, grpIdx:Int) = {
         val id = groupedId (rle.hwnd)
         val elemOpt = hwndMap.get(rle.hwnd) .map (wde => makeElemBox (id, wde, rle.y, ElemTs.G, grpT))
         elemOpt .map (elem => GrpIdxdElem (id, elem, grpIdx))
      }
      groupedRenderList .zipWithIndex .map { case (ll, gi) => Seq (
         // we wanted to set the first (if any) in group to group-head type, and rest (if any) to group-tail type (dimmed)
         ll .take(1) .flatMap (rle => getGrpIdxdElem (rle, GrpTs.GH, gi)),
         ll .tail    .flatMap (rle => getGrpIdxdElem (rle, GrpTs.GT, gi))
      ) .flatten } .flatMap { ll =>     // also register each group head to build out the group-heads idx for group-nav
         ll.headOption .map(_.id) .foreach (groupsHeadsIdsBuf.+=); ll
      } .zipWithIndex .foreach { case (e,i) => // and finally we can build out the flattened elems-map
         elemsMap.put (e.id, OrderedElemsEntry(i,e.d,e.grpIdx))
      }
      groupedElemsMap = elemsMap
      groupedIdsVec = elemsMap.keys.toVector
      groupsHeadsIdsVec = groupsHeadsIdsBuf
   }

   def rebuildSearchStateElems() : Unit = {
      // we'll build both recents and grouped for search-state together to reuse common mechanisms
      // note that in search state, there will be no group nav, nav will be restricted to search matches, and if grouped, nav will use that block
      val matchStr = RibbonDisplay.searchBox.value.trim
      case class SearchedElem (id:String, elem:Div, chkPassed:Boolean)
      def getSearchElem (rle:RenderListEntry, elemT:ElemT, grpT:GrpT, r:CheckSearchExeTitleRes) = {
         hwndMap .get(rle.hwnd) .map { wde =>
            val id = if (elemT == ElemTs.G) groupedId(wde.hwnd) else recentsId(wde.hwnd)
            val elem = makeElemBox (id, wde, rle.y, elemT, grpT, r.exeSpan, r.ySpan, r.titleSpan)
            SearchedElem (id, elem, r.chkPassed)
      } }
      def getSearchMatchRes (rle:RenderListEntry) = {
         hwndMap .get(rle.hwnd) .map { wde =>
            SearchHelper.checkSearchExeTitle (wde.exe_path_name.map(_.name).getOrElse(""), wde.win_text.getOrElse(""), matchStr, rle.y)
      } }
      def getSearchStateMapAndVec (sElems:Seq[SearchedElem]) = {
         val searchedElemsMap = mutable.LinkedHashMap[String,OrderedElemsEntry]()
         // this creates a separate matchIdxs table for only the matching elems ids (in place of regular recents/grouped ids nav vec)
         val matchIdxs = sElems .filter(_.chkPassed) .zipWithIndex .view .map { case (e,i) => e.id -> i } .to(mutable.LinkedHashMap)
         // then using that we can build out the search-state elems-map and nav-ids-vec
         sElems .foreach { e =>
            val y = matchIdxs.getOrElse(e.id, -1)
            searchedElemsMap .put (e.id, OrderedElemsEntry(y,e.elem))
         }
         (searchedElemsMap, matchIdxs.keys.toVector)
      }
      if (!inGroupedMode) {
         // lets do the simpler case of non-grouped mode
         val sElems = renderList .flatMap (e => getSearchMatchRes(e) .flatMap (res => getSearchElem (e, ElemTs.R, GrpTs.NG, res)))
         getSearchStateMapAndVec(sElems) match { case (m,v) => recentsElemsMap = m; recentsIdsVec = v }
      } else {
         // in grouped mode, we'll do navs in grouped block, but do simpler eqv match highlighting in dimmed top-recents where available
         // the searchElemsMap idxs need to have not the zipWithIndex, but sequential idxs of only matching elems ..
         val sElems = groupedRenderList .map {_ .flatMap {e => getSearchMatchRes(e).map(res => e -> res) } } .flatMap {ll => Seq (
            ll .take(1) .flatMap { case (e,r) => getSearchElem (e, ElemTs.G, GrpTs.GH, r) },
            ll .tail    .flatMap { case (e,r) => getSearchElem (e, ElemTs.G, GrpTs.GT, r) }
         ) } .flatten
         getSearchStateMapAndVec(sElems) match { case (m,v) => groupedElemsMap = m; groupedIdsVec = v }

         // now lets build the recents w similar search-match highlighting as well .. (we wont need nav idxs for it)
         val elemsMap = mutable.LinkedHashMap[String,OrderedElemsEntry]()
         renderList .take(groupedModeTopRecentsCount) .zipWithIndex .foreach { case (e,i) =>
            getSearchMatchRes(e) .flatMap (res => getSearchElem (e, ElemTs.GR, GrpTs.NG, res)) .foreach { se =>
               elemsMap .put (se.id, OrderedElemsEntry (i, se.elem))
         } }
         recentsElemsMap = elemsMap
         recentsIdsVec = elemsMap.keys.toVector
      }
   }

   def activateSearchBox () = {
      RibbonDisplay.searchBox.disabled = false;
      RibbonDisplay.searchBox.focus()
   }
   def exitSearchState() = {
      inSearchState = false; RibbonDisplay.searchBox.value = "";
      RibbonDisplay.blurSearchBox()
      RenderSpacer.immdtRender()
   }

   val handle_SearchModeKeyup: KeyboardEvent => Unit = {
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

   def handle_TitleUpdate (dat:WinDatEntry) = {
      def replaceTitleSpan (oe:OrderedElemsEntry) : Unit = {
         val titleSpan = if (!inSearchState) {
            span ( dat.win_text.getOrElse("title.."):String ).render
         } else {
            SearchHelper.checkSearchExeTitle (
               dat.exe_path_name.map(_.name).getOrElse(""), dat.win_text.getOrElse(""), RibbonDisplay.searchBox.value.trim, oe.y
            ) .titleSpan
         }
         clearedElem (oe.elem.getElementsByClassName("titleSpan").item(0)) .appendChild (titleSpan)
      }
      recentsElemsMap .get(recentsId(dat.hwnd)) .foreach (replaceTitleSpan)
      groupedElemsMap .get(groupedId(dat.hwnd)) .foreach (replaceTitleSpan)
   }



   def setCurElem (id:String) = { curElemId = id; }
   def setCurElemHighlight (newFocusElem:Div) = {
      // we manage 'focus' ourselves so that remains even when actual focus is moved to search-box etc
      clearCurElemHighlight()       // note that this will clear curElemId too
      setCurElem (newFocusElem.id)
      newFocusElem.classList.add("curElem")
      if (inSearchState && inGroupedMode) { // in group-mode search-state see if we can find another in recents to highlight too
         idToHwnd (newFocusElem.id) .map (recentsId) .flatMap (recentsElemsMap.get) .foreach (_.elem.classList.add("curElem"))
      }
   }
   def clearCurElemHighlight () = {
      curElemId = ""
      //doc.querySelectorAll(s".curElem") .foreach(_.classList.remove("curElem"))
      // ^^ our (old) sjs versions doesnt have NodeList conversion to scala iterable ..
      // .. so for now, we'll just try it twice, as there are at most two of these (if there's one in recents too during search)
      Option (doc.querySelector(s".curElem")) .foreach(_.classList.remove("curElem"))
      if (inSearchState && inGroupedMode) { // if in grouped-mode search-state try to clear one more
         Option (doc.querySelector(s".curElem")) .foreach(_.classList.remove("curElem"))
      }
   }

   def getIdfnVecAndMap(elemT:ElemT) = {
      if (elemT == ElemTs.G) { (groupedId _, groupedIdsVec, groupedElemsMap) } else { (recentsId _, recentsIdsVec, recentsElemsMap) }
   }
   def resetFocus() = { println("reset-focus")
      recentsIdsVec.headOption.foreach(setCurElem); focusElem_Next()
   }
   def resetSearchMatchFocus() : Unit = {
      val (_, idsVec, elemsMap) = getIdfnVecAndMap (if (inGroupedMode) ElemTs.G else ElemTs.R)
      idsVec.headOption .flatMap(elemsMap.get) .map(_.elem) .map(setCurElemHighlight) .getOrElse(clearCurElemHighlight())
   }

   def reSyncCurFocusIdAfterRebuild() = {
      // note that doing id-conversion checks helps sync up even in cases when we're toggling between recents and grouped modes!
      def getSyncElem (curBlock:ElemT, wrapBlock:ElemT) = {
         val ((curIdm,_,curMap),(_,wrapVec,wrapMap)) = getIdfnVecAndMap(curBlock) -> getIdfnVecAndMap(wrapBlock)
         idToHwnd(curElemId) .map(curIdm) .flatMap(curMap.get) .orElse ( wrapVec.headOption.flatMap(wrapMap.get) )
      }
      (inSearchState, inGroupedMode) match {
         // in recents-mode, whether search or not, we try to sync up within recents-block (falling back to its top)
         case (     _, false ) => { getSyncElem (ElemTs.R, ElemTs.R) }
         // in grouped-mode search-state, we can only sync within the grouped-block (recents is dimmed out and non-navigable)
         case (  true,  true ) => { getSyncElem (ElemTs.G, ElemTs.G) }
         // in grouped-mode non-search-state, we can sync either recents or grpd, but if fails, fall back to recents-top
         case ( false,  true ) => { recentsElemsMap.get(curElemId) .orElse ( getSyncElem (ElemTs.G, ElemTs.R) ) }
      }
   } .map(_.elem) .foreach (setCurElemHighlight)      // finally do the focus syncing


   def focusElem (isReverseDir:Boolean=false, isGrpNext:Boolean=false) = {
      // we'll setup closures to nav and wrap-over forwards or backwards so it can handle both with same logic block below
      type Wrapper = IndexedSeq[String] => Option[String]
      val (incr, vecWrap) = {
         if (!isReverseDir) { (1, (_.headOption):Wrapper) } else { (-1, (_.lastOption):Wrapper) }
      }
      // then w/ those, setup the common case nav fn that operates on either recents/grpd block, and any specified block to wrap-over to
      def pickNext (oe:OrderedElemsEntry, curBlock:ElemT, wrapBlock:ElemT) = {
         val ((_,curVec,curMap),(_,wrapVec,wrapMap)) = getIdfnVecAndMap(curBlock) -> getIdfnVecAndMap(wrapBlock)
         curVec .lift(oe.y+incr) .flatMap(curMap.get) .orElse ( vecWrap(wrapVec).flatMap(wrapMap.get) )
      }
      // now we'll pick curElem from whichever (recents/grouped) map it currently happens to be in
      recentsElemsMap .get(curElemId) .map (oe => (false, oe))
      .orElse { groupedElemsMap .get(curElemId) .map (oe => (true, oe)) }
      // now lets try to find the next-entry option for various state/mode/nav-type/curElem combinations
      .flatMap { case (curInGrpd, oe) =>
         (inSearchState, inGroupedMode, isGrpNext, curInGrpd) match {
            // in recents-mode, always stay within recents (in both regular and search-state)
            case (     _, false,     _,     _ ) => { pickNext (oe, ElemTs.R, ElemTs.R) }
            // in grouped-mode, if in search-state, always stay within grouped (recents is dimmed out, and non navigable)
            case (  true,  true,     _,     _ ) => { pickNext (oe, ElemTs.G, ElemTs.G) }
            // non-search recents, for regular-nav (not grp-next) .. if cur in recents nav there w fallback to grpd, and vice-versa
            case ( false,  true, false, false ) => { pickNext (oe, ElemTs.R, ElemTs.G) }
            case ( false,  true, false,  true ) => { pickNext (oe, ElemTs.G, ElemTs.R) }
            // non-search grouped, for grp-next, cur in recents .. if recents top, nav groups-head, else do recents top or first grp-head
            case ( false,  true,  true, false ) => {
               if (recentsIdsVec.headOption.contains(oe.elem.id)) { vecWrap (groupsHeadsIdsVec) .flatMap(groupedElemsMap.get) }
               else if (isReverseDir) { recentsIdsVec.headOption .flatMap(recentsElemsMap.get) } // reversing from rec middle .. do rec top
               else { groupsHeadsIdsVec.headOption .flatMap(groupedElemsMap.get) }  // but for fwd do first grp-head
            }
            // non-search grouped, grp-next, cur in grouped .. if grp-head or nav-fwd, nav grp heads w wrap to recents, else move to grp-head
            case ( false,  true,  true,  true ) => {
               if (groupsHeadsIdsVec.lift(oe.yg).contains(oe.elem.id) || !isReverseDir) {
                  groupsHeadsIdsVec .lift(oe.yg+incr) .flatMap(groupedElemsMap.get)
                     .orElse ( recentsIdsVec.headOption.flatMap(recentsElemsMap.get) )
               } else { groupsHeadsIdsVec .lift(oe.yg) .flatMap(groupedElemsMap.get) }
            }
         }
      } .map(_.elem) .foreach (setCurElemHighlight)  // finally, can make that current (if we found one)
   }
   def focusElem_Next()  = focusElem (isReverseDir=false, isGrpNext=false)
   def focusElem_Prev()  = focusElem (isReverseDir=true,  isGrpNext=false)
   def focusGroup_Next() = focusElem (isReverseDir=false, isGrpNext=true )
   def focusGroup_Prev() = focusElem (isReverseDir=true,  isGrpNext=true )

   def focusElem_Top() = {
      // top is usually recents top, except for search during grpd mode, when we dim out recents block
      if (inGroupedMode && inSearchState) { groupedIdsVec.headOption.flatMap(groupedElemsMap.get) }
      else                                { recentsIdsVec.headOption.flatMap(recentsElemsMap.get) }
   } .map(_.elem) .foreach (setCurElemHighlight)

   def focusElem_Bottom() = {
      // regardless of search mode, in grp mode, btm is grp-nav-vec btm, and ditto for recents
      if (inGroupedMode) { groupedIdsVec.lastOption.flatMap(groupedElemsMap.get) }
      else               { recentsIdsVec.lastOption.flatMap(recentsElemsMap.get) }
   } .map(_.elem) .foreach (setCurElemHighlight)

}



object ElemsDisplay {
   import SwitcheFaceConfig._
   import SwitchePageState._
   import Switche._

   val elemsDiv = div (id:="elemsDiv").render
   def getElemsDiv = elemsDiv

   def makeElemsDiv (elemT:ElemT, stateT:StateT) = {
      val headerTxt = if (elemT == ElemTs.G) "Grouped:" else "Recents:"
      val header = div (`class`:=s"modeHeader ${elemT.cls}", nbsp(1), headerTxt) .render
      val elemsMap = if (elemT == ElemTs.G) groupedElemsMap else recentsElemsMap
      div ( `class`:=s"elemsDiv ${elemT.cls} ${stateT.cls}", header, elemsMap.values.map(_.elem).toSeq ) .render
   }
   def updateElemsDiv () = {
      //updateRenderReadyLists()
      // ^^ no longer relevant as we get latest built renderlists from backend instead
      val searchedDiv : Div = {
         if (inSearchState) {
            rebuildSearchStateElems()
            if (inGroupedMode) {
               //makeElemsDiv (ElemTs.G, StateTs.S)      // uncommenting this instead of below will remove top-recents in grpd search state
               div ( makeElemsDiv (ElemTs.GR, StateTs.S), makeElemsDiv (ElemTs.G, StateTs.S) ) .render
            } else { makeElemsDiv (ElemTs.R,  StateTs.S) }
         } else {
            rebuildRecentsElems()
            if (inGroupedMode) {
               rebuildGroupedElems()
               div ( makeElemsDiv (ElemTs.GR, StateTs.L), makeElemsDiv (ElemTs.G, StateTs.L) ) .render
            } else { makeElemsDiv (ElemTs.R,  StateTs.L) }
      } }
      clearedElem(elemsDiv) .appendChild (searchedDiv.render)
      reSyncCurFocusIdAfterRebuild()
      //triggerHoverLockTimeout()     // coz newly built elems seem to get the first mouse-enter w/o any mouse movement
      // ^^ gaah that makes ui a bit janky, so we moved to using a special first-render-after-repaint flag
      isFreshRendered = true
   }
}


object RibbonDisplay {
   import SwitcheFaceConfig._
   import Switche._
   val countSpan = span (`class`:="dragSpan").render
   val debugLinks = span ().render
   val armedIndicator = span (`class`:="armedIndicator", "").render   // content is set in css
   val searchBox = input (`type`:="text", autocomplete:="off", id:="searchBox", placeholder:="").render
   def blurSearchBox() = {
      //searchBox.blur()
      //Try { doc.activeElement.asInstanceOf[HTMLElement] } .toOption .foreach(_.blur())
      //Option (doc.querySelector(s".curElem")) .flatMap (e => Try { e.asInstanceOf[HTMLElement] }.toOption) .foreach(_.blur())
      //window.focus()
      //countSpan.focus()
      
      // gaah .. in theory, any of ^^ these should work, and they do remove the blinking cursor from there ..
      // however, in our particular case, we had added text cursor highlight in windows, and that seems to persist even w/o focus!
      // .. so we'll just disable the whole searchbox instead .. oh well
      searchBox.disabled = true
   }
   // note: ^^ all key handling is now done at doc level capture phase (which selectively allows char updates etc to filter down to searchBox)
   def updateCountsSpan () : Unit = {
      val count = renderList.length
      clearedElem(countSpan) .appendChild ( span ( nbsp(3), s"($count)", nbsp(2), " ※ ", nbsp(2) ).render )
   }
   def updateDebugLinks() : Unit = {
      clearElem(debugLinks)
      if (inElectronDevMode) {
         val printExclLink =  a ( href:="#", "DebugPrint", onclick:={(e:MouseEvent) => SendMsgToBack.FE_Req_DebugPrint()} ).render
         debugLinks.appendChild ( printExclLink )
   } }
   def debugDisplayMsg (msg:String) = { debugLinks.innerHTML = s"$msg (${js.Date.now().toString})"; }
   def handleRefreshBtnClick() : Unit = {
      // we want to refresh too (as do periodic calls elsewhere), but here we want to also force icons-refresh
      SendMsgToBack.FE_Req_Refresh()
   }
   def setArmedIndicator_On  () = { armedIndicator.classList.add("armed") }
   def setArmedIndicator_Off () = { armedIndicator.classList.remove("armed") }
   
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={(e:MouseEvent) => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={(e:MouseEvent) => handleRefreshBtnClick()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={(e:MouseEvent) => handleReq_GroupModeToggle()} )
      //val dragSpot = span (style:="-webkit-app-region:drag", nbsp(3), "※", nbsp(3)) // combined with count instead
      div ( id:="top-ribbon",
         nbsp(0), reloadLink, nbsp(4), refreshLink, nbsp(4), groupModeLink,
         countSpan, armedIndicator, debugLinks, nbsp(2), searchBox
      ).render
   }
}

