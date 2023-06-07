package switche

import org.scalajs.dom
import org.scalajs.dom.{Element, EventTarget, MouseEvent, KeyboardEvent, WheelEvent}
import org.scalajs.dom.html.{Div, Span}
import org.scalajs.dom.{document => doc}
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
      println (s"key:${e.key}, code:${e.keyCode}, ev:${evType}, ctrl:${e.ctrlKey}, alt:${e.altKey}")
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
      triggerHoverLockTimeout(); e.preventDefault(); e.stopPropagation()
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
      if (verifyActionRepeatSpacing(20d)) {  // enforced spacing (in ms) between consecutive mouse scroll action handling
         triggerHoverLockTimeout(); //scrollEnd_arm();
         if (e.deltaY > 0 || e.deltaX > 0) { focusElem_Next() } else { focusElem_Prev() }
   } }
   def procMouse_Enter (e:MouseEvent) = {
      // NOTE: this is deprecated in favor of directly setting mouse-enter in the div boxes
      //handleMouseEnter ( e.target.asInstanceOf[Element].closest(".elemBox").asInstanceOf[Div] )
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e .asInstanceOf[Element] .closest(".elemBox") .asInstanceOf[Div] )
      } .foreach (handleMouseEnter)
   }
   
   def capturePhaseKeyupHandler (e:KeyboardEvent) = {    //printKeyDebugInfo(e,"up")
      // note: escape can cause app hide, and when doing that, we dont want that to leak outside app, hence on keyup
      if (inSearchState) { // && RibbonDisplay.searchBox.value.nonEmpty) {
         handle_SearchModeKeyup(e)   // let it recalc matches if necessary etc
      } else { // not in search state
         if (e.key == "Escape")  handleReq_SwitcheEscape()
      }
   }
   
   val modifierKeys = Set("Meta","Alt","Control","Shift")
   
   def capturePhaseKeydownHandler (e:KeyboardEvent) = {    printKeyDebugInfo(e,"down")
      
      var doStopProp = true
      @inline def setupSearchbox (doPassthrough:Boolean) = {
         doStopProp = modifierKeys.contains(e.key) || (!doPassthrough && !inSearchState)
         if (!modifierKeys.contains(e.key)) { inSearchState = true; activateSearchBox() }
      }
      @inline def eventPassthroughGuarded() = {
         if (doStopProp) { e.stopPropagation(); e.preventDefault(); }
      }
      // ^^ setup here is to allow propagation only to some selective cases that need to go to search-box
      
      // todo: wth .. this is ridiculous .. the alt-tab repl via krusty-to-switche is non-viable long run ..
      // .. so doesnt make sense to keep fiddling with this to get all niggles there smoothed .. (besides needing crap on krusty)
      // .. this should just directly be impld in code here .. just add a kbd hook if want alt-tab handling
      // .. and until there's config, could setup some combo to toggle that .. at which point we can hook/unhook
      // besides .. really intend to have that anyway, so doesnt make sense to waste time in round-about stuff
      //
      // todo: ^^ however .. even after direct alt-tab handling would have similar issue? nah we'd get events on alt dn/up ..
      // .. indeed we could even make a different msg-to-front for alt-tab vs just invoke, and keep flags updated that way
      // .. and that'd make it easy to check both the armed flag and alt-dn for the unarm operation that doesnt go to search
      // .. indeed, we could even make it such that any key while alt is down will disarm the dismissal
      //
      // hmm, that leaves the mouse scroll, and prob should impl that directly to switche too .. afterall we already woudl be
      // .. using hooks, whats another one for mouse .. and then again we'd know mouse btn down state just like alt down state!
      //
      // .. potentially all this would also make krusty a bit cleaner .. and ofc make switche a lot more stand-alone capable
      //
      // todo .. should consider ways to make up/down nav to work during alt-tab usage ..
      // - best again to only spend time on it after doing native alt-tab impl here instead of w krusty (to avoid wasted effort)
      // - if doing as is, prob could impl alt-j/i/k/,/u/m to left/right/top/btm/pgup/pgdn explicitly here ..
      //    .. and disable them from activating l/o/n/i/m/b via just alt on F1 usage, and maybe make eqv of ctrl-alt-hotkey instead?
      
      triggerHoverLockTimeout()
      
      // first, keys that are enabled for both normal and  search-state, and with or without alt/ctrl etc :
      if      (e.key == "Enter")      handleReq_CurElemActivation()
      else if (e.key == "Escape")    {/*handleEscapeKeyDown()*/}      // moved to keyup as dont want its keyup leaking outside app if we use it hide app
      else if (e.key == "F5")        dom.window.location.reload()
      // scroll/invoke hotkeys nav
      else if (e.key == "F1")         focusElem_Next()   // note: not really needed, registered as global hotkey, set electron to forwards it as a call
      else if (e.key == "F2")         focusElem_Prev()
      else if (e.key == "F16")        { scrollEnd_arm();  if (e.shiftKey) focusElem_Prev() else focusElem_Next(); }
      else if (e.key == "F17")        { scrollEnd_arm();  if (e.shiftKey) focusElem_Next() else focusElem_Prev(); }
      else if (e.key == "Tab")        { scrollEnd_arm();  if (e.shiftKey) focusElem_Prev() else focusElem_Next(); }
      // arrow nav
      else if (e.key == "ArrowUp")    focusElem_Prev()
      else if (e.key == "ArrowDown")  focusElem_Next()
      else if (e.key == "PageUp")     focusElem_Top()
      else if (e.key == "PageDown")   focusElem_Bottom()

      else if (!inSearchState) {
         // these are enabled in normal mode but disabled in search-state (with or without alt)
         if      (e.key == " ")              handleReq_CurElemActivation()
         // arrow group-nav
         else if (e.key == "ArrowLeft")      focusGroup_Prev()
         else if (e.key == "ArrowRight")     focusGroup_Next()
          // for other keys while not in search-state, we'll activate search-state and let it propagate to searchbox
         else if (!e.altKey) { setupSearchbox (doPassthrough = true) } // for search state
      }
      else if (!e.altKey) { setupSearchbox (doPassthrough = true) } // for non-search state
      
      // global ctrl hotkeys
      if (e.ctrlKey) {
         if      (e.key == "r")  RibbonDisplay.handleRefreshBtnClick()   // note: ctrl-r for refresh is not available w krusty (due to l2 overloading)
         else if (e.key == "f")  RibbonDisplay.handleRefreshBtnClick()   // so we'll also allow ctrl-f for 'fresh'.. meh
         else if (e.key == "g")  handleReq_GroupModeToggle()
         else if (e.key == "w")  handleReq_CurElemClose()
         else if (e.key == "v")  handleReq_CurElemPeek()
         else if (e.key == "z")  handleReq_CurElemMinimize()
         else if (e.key == "x")  handleReq_CurElemMaximize()
         // ^^ note that some of these e.g. z/x are awkward but worth doing it that way to keep them left handed (to work well w krusty l2)
         else { setupSearchbox (doPassthrough = true) }
      }
      
      // global alt (and alt-ctrl) hotkeys
      if (e.altKey) { // special handling ONLY for alt keys (the above apply whether alt or not)
         if (e.ctrlKey || e.shiftKey) {
            val key = e.key.toLowerCase()
            // ^^ ctrl works if not w/ krusty, but else we need shift (as caps-as-ctrl for some of these maps to arrow keys)
            if      (key == "l" ) { SendMsgToBack.FE_Req_Switch_Last()         }
            else if (key == "o" ) { SendMsgToBack.FE_Req_Switch_TabsOutliner() }
            else if (key == "n" ) { SendMsgToBack.FE_Req_Switch_NotepadPP()    }
            else if (key == "i" ) { SendMsgToBack.FE_Req_Switch_IDE()          }
            else if (key == "m" ) { SendMsgToBack.FE_Req_Switch_Music()        }
            else if (key == "b" ) { SendMsgToBack.FE_Req_Switch_Browser()      }
         }
         // krusty eqv nav for alt (to support alt tab usage) (note that some of these will overlap the alt-ctrl(/shift above)
         else if (e.key == "i")  focusElem_Prev()
         else if (e.key == ",")  focusElem_Next()
         else if (e.key == "u")  focusElem_Top()
         else if (e.key == "m")  focusElem_Bottom()
         else if (e.key == "j")  focusGroup_Prev()
         else if (e.key == "k")  focusGroup_Next()
         // alt-specific hotkeys
         else if (e.key == "F4") { handleReq_SwitcheQuit() }
         //else if (e.key == " ")  { scrollEnd_disarm(); setupSearchbox (doPassthrough = false); }
         // ^^ cant do alt-space as in windows OS intercepts it before it reaches the browser .. so we'll set up alternatives:
         else if (e.key == "s")  { scrollEnd_disarm(); setupSearchbox (doPassthrough = false); }
         else if (e.key == "l")  { scrollEnd_disarm(); setupSearchbox (doPassthrough = false); }
         else { } // we'll ignore alt-combos for the searchbox
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


   def recentsId (hwnd:Int) = s"${hwnd}_r"
   def groupedId (hwnd:Int) = s"${hwnd}_g"
   def idToHwnd (idStr:String) = idStr.split("_") .headOption .flatMap (s => Try(s.toInt).toOption)


   def handleReq_SwitcheEscape () = { //println("dismissed")
      isDismissed = true;
      SendMsgToBack.FE_Req_SwitcheEscape()
      // we'll do a delayed focus-reset so the visual flip happens out of sight after switche window is gone
      js.timers.setTimeout(300) {  SwitchePageState.resetFocus() }
   }
   def handleReq_SwitcheQuit () = {
      SendMsgToBack.FE_Req_SwitcheQuit()
   }
   
   def handleReq_CurElemActivation() : Unit = {
      idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowActivate )
      isDismissed = true;
      js.timers.setTimeout(300) {   // again, small delay to avoid visible change
         if (SwitchePageState.inSearchState) { SwitchePageState.exitSearchState() }
         SwitchePageState.resetFocus()
      }
   }
   def handleReq_CurElemMinimize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMinimize ) }
   def handleReq_CurElemMaximize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMaximize ) }
   def handleReq_CurElemClose()    = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowClose    ) }
   def handleReq_CurElemPeek()     = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowPeek     ) }
   

   def handleMouseEnter (elem:Div) = {
      if (!isHoverLocked) { setCurElemHighlight(elem) }
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

   def activateSearchBox () = { RibbonDisplay.searchBox.focus() }
   def exitSearchState() = {
      inSearchState = false; RibbonDisplay.searchBox.value = ""; RibbonDisplay.searchBox.blur();
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
   def resetFocus() = { //println("reset-focus")
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
   }
}


object RibbonDisplay {
   import SwitcheFaceConfig._
   import Switche._
   val countSpan = span (`class`:="dragSpan").render
   val debugLinks = span ().render
   val searchBox = input (`type`:="text", id:="searchBox", placeholder:="").render
   // note: ^^ all key handling is now done at doc level capture phase (which selectively allows char updates etc to filter down to searchBox)
   def updateCountsSpan () : Unit = {
      val count = renderList.length
      clearedElem(countSpan) .appendChild ( span ( nbsp(3), s"($count) ※", nbsp(3) ).render )
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
   def getTopRibbonDiv() = {
      val reloadLink = a (href:="#", "Reload", onclick:={(e:MouseEvent) => g.window.location.reload()} )
      val refreshLink = a (href:="#", "Refresh", onclick:={(e:MouseEvent) => handleRefreshBtnClick()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={(e:MouseEvent) => handleReq_GroupModeToggle()} )
      //val dragSpot = span (style:="-webkit-app-region:drag", nbsp(3), "※", nbsp(3)) // combined with count instead
      div ( id:="top-ribbon",
         nbsp(0), reloadLink, nbsp(4), refreshLink, nbsp(4), groupModeLink, countSpan, debugLinks, nbsp(4), searchBox
      ).render
   }
}

