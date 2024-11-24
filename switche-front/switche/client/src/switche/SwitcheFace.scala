package switche

import scala.collection.{IndexedSeq, Map, mutable}
import scala.util.Try

import scala.scalajs.js
import org.scalajs.dom
import org.scalajs.dom.{Element, KeyboardEvent, MouseEvent, WheelEvent, window, document => doc}
import org.scalajs.dom.html.{Div, Span}
import scalatags.JsDom.all._



// some type defs for the various modes and states for various ui sections

/** GrpType represents whether the element is of type non-grouped, head-of-grouped, or rest-of-grouped */
sealed abstract class GrpT (val cls:String)
object GrpTs {
   /** Non-Grouped */
   case object NG extends GrpT ("ng")
   /** Non-Grouped Last-Recents Head */
   case object LH extends GrpT ("lh")
   /** Group-Head */
   case object GH extends GrpT ("gh")
   /** Group-Tail */
   case object GT extends GrpT ("gt")
}

/** ElemType represents whether we are part of recents, grouped-recetns, or grouped sections */
sealed abstract class ElemT (val cls:String)
object ElemTs {
   /** recents-mode recents block */
   case object R  extends ElemT ("r")
   /** grouped mode grouped block */
   case object G  extends ElemT ("g")
   /** grouped mode recents block */
   case object GR extends ElemT ("gr")
}

/** StateT represents whether we are in search state or not */
sealed abstract class StateT (val cls:String)
object StateTs {
   /** listing state (search not activated) */
   case object L  extends StateT ("l")
   /** searching state */
   case object S  extends StateT ("s")
}



object SwitcheFaceConfig {
   val hoverLockTime = 200 // ms
   val minScrollSpacingMs = 15d // ms
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.Element): Unit = { e.innerHTML = ""}
   def clearedElem (e:dom.Element) = { e.innerHTML = ""; e }
}



object SwitcheFacePage {
   import Switche._
   import SwitchePageState._
   import ElemsDisplay._

   def getShellPage () = {
      setPageEventHandlers()
      div ( id:="scala-js-root-div", RibbonDisplay.topRibbon, ElemsDisplay.elemsDiv ) .render
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
      //doc.addEventListener ("wheel",       procMouse_Wheel _, )        // see below
      doc.addEventListener ("keyup",       capturePhaseKeyUpHandler _,   useCapture=true)
      doc.addEventListener ("keydown",     capturePhaseKeyDownHandler _, useCapture=true)
      //doc.addEventListener ("keypress",    capturePhaseKeyPressHandler _, useCapture=true)
      //dom.document.addEventListener ("mouseenter", procMouse_Enter _, useCapture=true) // done from element for efficiency
      
      // looks like chrome has an intervention for doc/window level wheel listener, where passive:false MUST be specified for preventDefault to work
      // also, doing window below makes it exclude the scrollbar, which would be nice .. (but maybe doesnt work w custom scrollbar)
      val eventListenerOptions = js.Dynamic.literal ( "passive" -> false.asInstanceOf[js.Any] )
      window .asInstanceOf[js.Dynamic] .addEventListener ("wheel", procMouse_Wheel _, eventListenerOptions)
   }
   
   def procMouse_Click (e:MouseEvent) = { //println ("mouse lbtn click!")
      triggerHoverLockTimeout(); scrollEnd_disarm(); RibbonDisplay.closeDropdowns()
      e.preventDefault(); e.stopPropagation()
      //e.target.closest(".elemBox").foreach(_=>handleReq_CurElemActivation())
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e.asInstanceOf[Element] .closest(".elemBox") .asInstanceOf[Div] )  // can return a null so wrapping into option
      } .foreach ( b => { setCurElemHighlight(b); handleReq_CurElemActivation(); } )
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
         Option ( e.asInstanceOf[Element] .closest(".elemBox") .asInstanceOf[Div] )  // can return a null so wrapping into option
      } .foreach ( b => { setCurElemHighlight(b); handleReq_CurElemClose(); } )
      // ^^ we dont actually need the closest, we're just filtering by whether closest can find an elemBox in its ancestors
      // .. this makes it such that in rows the mouse click is active while elsewhere in empty body it is not!
      // ^^ update: started doing setCurElemHighlight on click location in case the highlight had moved elsewhere from pointer loc
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
      if (verifyActionRepeatSpacing()) {  // enforced spacing (in ms) between consecutive mouse scroll action handling
         triggerHoverLockTimeout(); //scrollEnd_arm();
         if (e.deltaY > 0 || e.deltaX > 0) {
            if (e.ctrlKey) focusGroup_Next() else if (e.shiftKey) focusBlockElem_Next() else focusElem_Next()
         } else {
            if (e.ctrlKey) focusGroup_Prev() else if (e.shiftKey) focusBlockElem_Prev() else focusElem_Prev()
         }
   } }
   def procMouse_Enter (e:MouseEvent) = {
      // NOTE: this is deprecated in favor of directly setting mouse-enter in the div boxes
      //handleMouseEnter ( e.target.asInstanceOf[Element].closest(".elemBox").asInstanceOf[Div] )
      Some(e.target) .filter(_.isInstanceOf[Element]) .flatMap { e =>
         Option ( e .asInstanceOf[Element] .closest(".elemBox") .asInstanceOf[Div] )
      } .foreach {e => /*handleMouseEnter*/ }
   }
   
   def capturePhaseKeyUpHandler (e:KeyboardEvent) = {    printKeyDebugInfo(e,"up")
      // note: escape can cause app hide, and when doing that, we dont want that to leak outside app, hence on keyup
      if (inSearchState) { // && RibbonDisplay.searchBox.value.nonEmpty) {
         SearchDisplay.handle_SearchModeKeyup(e)   // let it recalc matches if necessary etc
      } else { // not in search state
         if (e.key == "Escape")  handleReq_SwitcheEscape()
      }
   }
   
   //val modifierKeys = Set ("Meta","Alt","Control","Shift")
   val passthroughKeys = Set ("MediaPlayPause","MediaTrackNext","MediaTrackPrevious","AudioVolumeUp","AudioVolumeDown")
   
   
   def capturePhaseKeyDownHandler (e:KeyboardEvent) = {    printKeyDebugInfo(e,"down")
      
      var (doStopProp, preventDefault) = (true, false)
      // ^^ note that we're setting default to stop further propagation (but preventing default is uncommon)
      

      // at the end, this can setup event prop and default based on flags we'll have updated
      @inline def eventPassthroughGuarded() = { //println(s"doStopProp=$doStopProp")
         if (preventDefault) { e.preventDefault() }
         if (!passthroughKeys.contains(e.key) && doStopProp) { e.stopPropagation(); e.preventDefault(); }
      }
    
      triggerHoverLockTimeout()
      
      (scrollEnd_armed, inSearchState, e.altKey, e.ctrlKey, e.key) match {
         
         // first, keys that are enabled for both normal and  search-state, and with or without alt/ctrl etc :
         case (_, _, _, _, "Enter")      => handleReq_CurElemActivation()
         case (_, _, _, _, "Escape")     =>  //handleReq_SwitcheEscape()   // moved to keyup to avoid leakage of keyup after we use it hide app
         case (_, _, _, _, "F5")         => handleReq_Reload()
         
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
         case (_, _, _, _, "ArrowUp")    => if (e.shiftKey) focusBlockElem_Prev()   else focusElem_Prev()
         case (_, _, _, _, "ArrowDown")  => if (e.shiftKey) focusBlockElem_Next()   else focusElem_Next()
         case (_, _, _, _, "PageUp")     => if (e.shiftKey) focusBlockElem_Top()    else focusElem_Top()
         case (_, _, _, _, "PageDown")   => if (e.shiftKey) focusBlockElem_Bottom() else focusElem_Bottom()
         
         // arrow left-right group nav while armed, w alt, or non-search .. else it'll just pass on to searchbox
         case (_, _, _, _, "ArrowLeft")   if (!inSearchState || scrollEnd_armed || e.altKey)  => focusGroup_Prev()
         case (_, _, _, _, "ArrowRight")  if (!inSearchState || scrollEnd_armed || e.altKey)  => focusGroup_Next()
         
         
         // space key .. usually for cur elem activation .. but in search state, it'll pass through to searchbox
         // note that the alt versions are for reference, as OS never lets those through (and we do the eqv via LL-hook callbacks)
         //   (arm,    srch,   alt,   ctrl,  key)
         case (true,   _,      _,     _,     " ")  => scrollEnd_disarm()               // space disarms if scroll-armed
         case (false,  _,      true,  _,     " ")  => handleReq_CurElemActivation()    // alt-space activates if NOT scroll-armed
         case (_,      false,  _,     _,     " ")  => handleReq_CurElemActivation()    // activation in non-search state
         case (_,      true,   _,     true,  " ")  => handleReq_CurElemActivation()    // activation in search-state w/ ctrl key
         
         
         // alt-f4 .. we'll directly exit app ..  (although just doing doStopProp = false would also work indirectly)
         case (_, _, true, _, "F4")   =>  handleReq_SwitcheQuit()
         
         // alt-ctrl-a .. toggle auto-hide mode .. (for easy switch during dev, else via configs is sufficient)
         case (_, _, true, true, "a") =>  SendMsgToBack.FE_Req_AutoHideToggle()
        
         // alt-key or scroll-end-armed ..  we'll setup alt-tab state key nav options
         //   (arm, srch, alt, ctrl, key)
         case (_, _, true, _, _) | (true, _, _, _, _) => {
            e.key match {
               case "i"  => focusElem_Prev()
               case ","  => focusElem_Next()
               case "u"  => focusElem_Top()
               case "m"  => focusElem_Bottom()
               case "j"  => focusGroup_Prev()
               case "k"  => focusGroup_Next()
               case "r"  => handleReq_Refresh()
               
               case " " | "s" | "l" | "S" | "L" =>
                  e.preventDefault(); scrollEnd_disarm();
                  doStopProp = SearchDisplay.checkSearchBox_StopProp (e, doPassthrough = false)
                  
               case _ => // all other alt combos, or while dis-armed can be ignored
            }
         }
         
         
         // ctrl specific hotkeys .. typically for switche specific actions
         //   (arm, srch,  alt,  ctrl,  key)
         case (_,   _,    false, true,  _) => {
            preventDefault = true
            e.key.toLowerCase match {
               case "r"  => handleReq_Refresh()
               case "f"  => handleReq_Refresh()
               case "g"  => handleReq_GroupModeToggle()
               case "w"  => handleReq_CurElemClose()
               case "p"  => handleReq_CurElemPeek()
               //case "z"  => handleReq_CurElemMinimize()   // disabled to allow for use in search-box instead
               //case "x"  => handleReq_CurElemMaximize()
               
               case _  if (SearchDisplay.searchBoxCtrlKeys.contains(e.key)) => {
                  // ^^ we'll support some ctrl-combos for searchbox .. e.g left-right arrows, delete/bksp, ctr-a/x/c/v/z/y
                  preventDefault = false
                  doStopProp = SearchDisplay.checkSearchBox_StopProp (e, doPassthrough = true)
               }
               case _ =>   // other ctrl hotkeys can be ignored
            }
         }
         
         // all other non-alt non-ctrl events can be passed through to searchbox
         case (_, _, false, false, _) => {
            doStopProp = SearchDisplay.checkSearchBox_StopProp (e, doPassthrough = true)
         }
         
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
   var last_mouse_enter_xy : Option[(Double,Double)] = None
   // ^^ mouse-enter retriggers after repaint, so we'll keep track of mouse pos at last mouseenter and ignore if same pos
   var fresh_render_under_mouse_elem_bounds : Option[dom.DOMRect] = None
   // ^^ otoh, we'll want to trigger selection when user moves mouse even in the ignored first mouse position ..
   // .. so, we'll capture the bounds of the elem under mouse on first render, and use that to trigger mouse-enter ourselves if needed
   

   def recentsId (hwnd:Int) = s"${hwnd}_r"
   def groupedId (hwnd:Int) = s"${hwnd}_g"
   def idToHwnd (idStr:String) = idStr.split("_") .headOption .flatMap (s => Try(s.toInt).toOption)

   def getCappedRecents() : Seq[(RenderListEntry, GrpT)] = {
      // we'll compile the capped-recents list (to incl both the top-recents and last-recents) first ..
      // .. then we'll tag the beginning (if any) of the last-recents block, so we can mark it up for css
      val cappedRecents = if (inGroupedMode) {
         val (nfirst, nlast) = (configs.n_grp_mode_top_recents, configs.n_grp_mode_last_recents)
         val ngap = renderList.size - nfirst - nlast    // we rely on this possibly being negative in which case the drop does nothing
         renderList.take(nfirst) ++ renderList.drop(nfirst).drop(ngap)
      } else { renderList }
      
      var last_y = cappedRecents.headOption.map(_.y).getOrElse(0)
      cappedRecents .map { rle => {
         val diff = rle.y - last_y
         last_y = rle.y
         val tag = if (diff > 1) GrpTs.LH else GrpTs.NG
         (rle, tag)
      } }
   }
   
   def setCurElem (id:String) = {
      curElemId = id;
   }
   def resetFocus() = { //println("reset-focus")
      RibbonDisplay.closeDropdowns()
      recentsIdsVec.headOption.foreach(setCurElem); ElemsDisplay.focusElem_Next()
   }

   def handleReq_SwitcheEscape (fromBkndHotkey:Boolean = false) = { //println("dismissed")
      scrollEnd_disarm(); setDismissed()
      if (!fromBkndHotkey) { SendMsgToBack.FE_Req_SwitcheEscape() }
      // we'll do delayed ui updates so the visual flip happens out of sight after switche window is gone
      js.timers.setTimeout(300) {
         //SwitchePageState.resetFocus();
         // ^^ reset will auto happen when search-state exit triggers rendering and it sees switche is dismissedd
         SearchDisplay.exitSearchState()
      }
   }
   def handleReq_SwitcheQuit () = {
      SendMsgToBack.FE_Req_SwitcheQuit()
   }
   def handleReq_Refresh () = {
      SendMsgToBack.FE_Req_Refresh()
      RibbonDisplay.blipArmedIndicator()
   }
   def handleReq_Reload() = {
      dom.window.location.reload()
   }
   def handleReq_GroupModeToggle() = {
      inGroupedMode = !inGroupedMode;
      SendMsgToBack.FE_Req_GrpModeEnabled(inGroupedMode)
      RenderSpacer.queueSpacedRender()
   }
   
   def handleReq_CurElemActivation() : Unit = {
      scrollEnd_disarm(); setDismissed(); RibbonDisplay.closeDropdowns()
      idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowActivate )
      // we'll reset focus w small delay to avoid visible change
      //js.timers.setTimeout(300) { SwitchePageState.exitSearchState(); SwitchePageState.resetFocus(); }
      // ^^ this will happen on swi-fgnd-lost report anyway, putting this on a timer just invites races etc
   }
   def handleReq_CurElemMinimize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMinimize ) }
   def handleReq_CurElemMaximize() = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowMaximize ) }
   def handleReq_CurElemPeek()     = { idToHwnd (curElemId) .foreach ( SendMsgToBack.FE_Req_WindowPeek     ) }
   def handleReq_CurElemClose()    = {
      val toClose = idToHwnd(curElemId)
      //ElemsDisplay.focusElem_Next()
      ElemsDisplay.pickNextElem (isReverseDir=false, isGrpNext=false, wrapBlocks=true) .map(_.id) .foreach(setCurElem)
      // ^^ instead of immdtly focusing on next elem, we'll just pick it out and let it be highligted upon next repaint
      toClose .foreach ( SendMsgToBack.FE_Req_WindowClose    )
   }
   

   def handleHoverLockTimeout (kickerStamp:Double) = {
      if (lastActionStamp == kickerStamp) { isHoverLocked = false }
   }
   def triggerHoverLockTimeout() = {
      isHoverLocked = true; val t = js.Date.now(); lastActionStamp = t;
      js.timers.setTimeout(hoverLockTime) {handleHoverLockTimeout(t)}
   }
   def verifyActionRepeatSpacing (minRepeatSpacingMs:Double = minScrollSpacingMs) : Boolean = {
      val t = scalajs.js.Date.now()
      if ((t - lastActionStamp) < minRepeatSpacingMs) { return false }
      lastActionStamp = t
      return true
   }
   
   def handleElemMouseEnter (elem:Div, ev:MouseEvent) = { //println (s"mouse enter on elem: ${elem.id}")
      if (last_mouse_enter_xy.isEmpty) {
         fresh_render_under_mouse_elem_bounds = Some(elem.getBoundingClientRect())
         // ^^ for first render, we want to capture bounds of elem under mouse, so we can react to mousemove even w/o mouseenter
      } else if (last_mouse_enter_xy == Some(ev.clientX, ev.clientY)) {
         // we'll ignore these spurious mouse-enters triggered due to elem repaints (but we'll invalidate first render bounds)
         fresh_render_under_mouse_elem_bounds = None
      } else if (!isHoverLocked) {
         ElemsDisplay.setCurElemHighlight(elem)
      }
      last_mouse_enter_xy = Some (ev.clientX, ev.clientY)
   }
   def handleElemMouseMove  (elem:Div, ev:MouseEvent) = {
      // in the corner case after first repaint, when we want to ignore the first mouse-enter, but still want to respond to the user
      //    actually moving the mouse, we'll use the bounds we captured on repaint-fresh mouse-enter to trigger a mouse-enter ourselves
      if ( !isHoverLocked &&
            last_mouse_enter_xy != Some(ev.clientX, ev.clientY) &&
            fresh_render_under_mouse_elem_bounds.exists { bounds =>
               ev.clientX >= bounds.left && ev.clientX <= bounds.right && ev.clientY >= bounds.top && ev.clientY <= bounds.bottom
            }
      ) { ElemsDisplay.setCurElemHighlight(elem) }
   }

}




object ElemsDisplay {
   import SwitcheFaceConfig._
   import SwitchePageState._
   import Switche._

   val elemsDiv = div (id:="elemsDivs").render
   
   // note the use of elemsDiv for each section and elevsDivs for the outer div

   def makeElemsDiv (elemT:ElemT, stateT:StateT) = {
      val headerTxt = if (elemT == ElemTs.G) "Grouped:" else "Recents:"
      val header = div (`class`:=s"modeHeader ${elemT.cls}", nbsp(1), headerTxt) .render
      val elemsMap = if (elemT == ElemTs.G) groupedElemsMap else recentsElemsMap
      
      // instead of simply putting the elems into a div, we want to introduce a spacer before recents-list second half (if any)
      val (block_1, block_2) = elemsMap .values .map(_.elem) .span (!_.classList.contains(GrpTs.LH.cls))
      val spacingDiv = div (`class`:="spacingDiv") .render
      val spacedElems = Seq (block_1, block_2) .filter(_.nonEmpty) .flatMap (_ ++ Seq(spacingDiv)) .dropRight(1)
      
      div ( `class`:=s"elemsDiv ${elemT.cls} ${stateT.cls}", header, spacedElems  ) .render
   }
   def updateElemsDiv () = {
      //updateRenderReadyLists()
      // ^^ no longer relevant as we get latest built renderlists from backend instead
      val searchedDiv : Div = {
         if (inSearchState) {
            SearchDisplay.rebuildSearchStateElems()
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
      if (isFgnd && !isDismissed) { reSyncCurFocusIdAfterRebuild() } else { resetFocus() }
      // ^^ we typically want to resync on rebuild so our selection remains where we placed it despite z-order changing ..
      // however, while dismissed, we'll want to always have the second-top selected, as thats what we want when swi is invoked next
      last_mouse_enter_xy = None
      // ^^ this helps avoid spurious mouse-enter triggering elem-highlight upon repaint
   }
   

   def makeElemBox (idStr:String, wde:WinDatEntry, y:Int, elemT:ElemT, grpT:GrpT) : Div = {
      val exeInnerSpan = span ( wde.exe_path_name.map(_.name).getOrElse("exe..") ).render
      val yInnerSpan = span (`class`:="ySpan", f"${y}%2d" ).render
      val titleInnerSpan = span ( wde.win_text.getOrElse("-- no title --") ).render
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
      val elem = div (`class`:=s"elemBox ${grpT.cls}", id:=idStr, tabindex:=0, exeSpan, nbsp(2), ySpan, nbsp(2), icoSpan, nbsp(), titleSpan).render
      //elem.onclick = {ev:MouseEvent => SwitcheState.handleReq_WindowActivation(e.hwnd)}
      // ^^ moved most handlers to single document-level events handler
      // .. but we left mouseenter etc here, as doing that globally is a little wasteful
      elem.onmouseenter = { (ev:MouseEvent) => handleElemMouseEnter (elem,ev) }
      elem.onmousemove  = { (ev:MouseEvent) => handleElemMouseMove  (elem,ev) }
      elem
   }


   def rebuildRecentsElems() = {
      // this needs the elems table, and a vec to navigate through it
      val elemsMap = mutable.LinkedHashMap[String,OrderedElemsEntry]()
      getCappedRecents() .flatMap {case (e,gt) => hwndMap.get(e.hwnd).map(d => (d,e,gt)) } .zipWithIndex .foreach { case ((wde,rle,gt),i) =>
         val id = recentsId (wde.hwnd)
         val elemT = if (inGroupedMode) ElemTs.GR else ElemTs.R
         val elem = makeElemBox (id, wde, rle.y, elemT, gt)
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
   
   
   def setCurElemHighlight (newFocusElem:Div) = {
      // we manage 'focus' ourselves so that remains even when actual focus is moved to search-box etc
      clearCurElemHighlight()       // note that this will clear curElemId too
      setCurElem (newFocusElem.id)
      newFocusElem.classList.add("curElem")
      // we'll want to bring this into view if it is not in view area
      if (shouldScrollIntoView(newFocusElem)) { //println("scrolling-into-view")
         //newFocusElem.scrollIntoView()
         // ^^ in theory works, but its more pleasant if it doesnt scroll to the default top
         import typings.std.{ ScrollLogicalPosition => slp, ScrollBehavior => slb }
         val options = typings.std.ScrollIntoViewOptions() .setBlock(slp.center) .setBehavior(slb.smooth)
         newFocusElem .asInstanceOf[Element] .asInstanceOf[js.Dynamic] .scrollIntoView (options)
         triggerHoverLockTimeout()
         // ^^ for multiple scroll-into-views, we dont want the mouse being at say bottom to keep triggering it
      }
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

 
   def shouldScrollIntoView (elem:Element) = {
      val scrolledTop = ElemsDisplay.elemsDiv.scrollTop == 0
      
      lazy val edRect = ElemsDisplay.elemsDiv.getBoundingClientRect()
      lazy val scrolledBtm = 1 > math.abs (ElemsDisplay.elemsDiv.scrollHeight - (edRect.bottom - edRect.top) - ElemsDisplay.elemsDiv.scrollTop )
      // the abs is because decimals dont show up in scrolltop, but they do in abs positions, so we want some slop
      
      lazy val elRect = elem.getBoundingClientRect()
      lazy val edViewHeight = edRect.bottom - edRect.top
      lazy val scrollViewThresh = 8 // 0.05 * edViewHeight   // 5% of view height
      // ^^ note that we do a hover-lock upon scroll-to-top, so triggering it unnecessarily causes ui to feel slower

      //lazy val (topTooHigh, btmTooLow) = ( elRect.top < 0.1 * viewHeight,  elRect.bottom > 0.9 * viewHeight )
      lazy val (topTooHigh, btmTooLow) = ( elRect.top - edRect.top < scrollViewThresh,  edRect.bottom - elRect.bottom < scrollViewThresh)

      //println ((topTooHigh, btmTooLow, scrolledTop, scrolledBtm))
      //println (((elRect.top, elRect.bottom), ElemsDisplay.elemsDiv.scrollTop, ElemsDisplay.elemsDiv.scrollHeight, (edRect.top, edRect.bottom)))
      
      // we should only scroll-into-view if either top or bottom is too high/low, and not already scrolled to top/bottom
      (!scrolledTop && topTooHigh) || (!scrolledBtm && btmTooLow)
   }

   def handle_TitleUpdate (dat:WinDatEntry) = {
      def replaceTitleSpan (oe:OrderedElemsEntry) : Unit = {
         val titleSpan = if (!inSearchState) {
            span ( dat.win_text.getOrElse("title.."):String ).render
         } else {
            SearchHelper.checkSearchExeTitle (
               dat.exe_path_name.map(_.name).getOrElse(""),
               dat.win_text.getOrElse(""),
               SearchDisplay.getSearchBoxText(),
               oe.y
            ) .titleSpan
         }
         clearedElem (oe.elem.getElementsByClassName("titleSpan").item(0)) .appendChild (titleSpan)
      }
      recentsElemsMap .get(recentsId(dat.hwnd)) .foreach (replaceTitleSpan)
      groupedElemsMap .get(groupedId(dat.hwnd)) .foreach (replaceTitleSpan)
   }

   def getIdfnVecAndMap(elemT:ElemT) = {
      if (elemT == ElemTs.G) { (groupedId _, groupedIdsVec, groupedElemsMap) } else { (recentsId _, recentsIdsVec, recentsElemsMap) }
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


   def pickNextElem (isReverseDir:Boolean, isGrpNext:Boolean, wrapBlocks:Boolean) = {
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
         (inSearchState, inGroupedMode, isGrpNext, curInGrpd, wrapBlocks) match {
            // in recents-mode, always stay within recents (in both regular and search-state)
            case (     _, false,     _,     _,     _ ) => { pickNext (oe, ElemTs.R, ElemTs.R) }
            // in grouped-mode, if in search-state, always stay within grouped (recents is dimmed out, and non navigable)
            case (  true,  true,     _,     _,     _ ) => { pickNext (oe, ElemTs.G, ElemTs.G) }
            // non-search recents, for regular-nav (not grp-next) .. if cur in recents nav there w rollover to grpd, and vice-versa
            case ( false,  true, false, false,  true ) => { pickNext (oe, ElemTs.R, ElemTs.G) }
            case ( false,  true, false,  true,  true ) => { pickNext (oe, ElemTs.G, ElemTs.R) }
            // however, if specifed to not wrap across recents/groups blocks, we just stay within the blocks
            case ( false,  true, false, false, false ) => { pickNext (oe, ElemTs.R, ElemTs.R) }
            case ( false,  true, false,  true, false ) => { pickNext (oe, ElemTs.G, ElemTs.G) }
            // non-search grouped, for grp-next, cur in recents .. if recents top, nav groups-head, else do recents top or first grp-head
            case ( false,  true,  true, false,     _ ) => {
               if (recentsIdsVec.headOption.contains(oe.elem.id)) { vecWrap (groupsHeadsIdsVec) .flatMap(groupedElemsMap.get) }
               else if (isReverseDir) { recentsIdsVec.headOption .flatMap(recentsElemsMap.get) } // reversing from rec middle .. do rec top
               else { groupsHeadsIdsVec.headOption .flatMap(groupedElemsMap.get) }  // but for fwd do first grp-head
            }
            // non-search grouped, grp-next, cur in grouped .. if grp-head or nav-fwd, nav grp heads w wrap to recents, else move to grp-head
            case ( false,  true,  true,  true,     _ ) => {
               if (groupsHeadsIdsVec.lift(oe.yg).contains(oe.elem.id) || !isReverseDir) {
                  groupsHeadsIdsVec .lift(oe.yg+incr) .flatMap(groupedElemsMap.get)
                     .orElse ( recentsIdsVec.headOption.flatMap(recentsElemsMap.get) )
               } else { groupsHeadsIdsVec .lift(oe.yg) .flatMap(groupedElemsMap.get) }
            }
         }
      } .map(_.elem)
   }
   def focusElem (isReverseDir:Boolean, isGrpNext:Boolean, wrapBlocks:Boolean) = {
      pickNextElem (isReverseDir, isGrpNext, wrapBlocks) .foreach(setCurElemHighlight)
   }
   def focusElem_Next()       = focusElem ( isReverseDir = false, isGrpNext = false, wrapBlocks =  true )
   def focusElem_Prev()       = focusElem ( isReverseDir =  true, isGrpNext = false, wrapBlocks =  true )
   def focusGroup_Next()      = focusElem ( isReverseDir = false, isGrpNext =  true, wrapBlocks =  true )
   def focusGroup_Prev()      = focusElem ( isReverseDir =  true, isGrpNext =  true, wrapBlocks =  true )
   def focusBlockElem_Next()  = focusElem ( isReverseDir = false, isGrpNext = false, wrapBlocks = false )
   def focusBlockElem_Prev()  = focusElem ( isReverseDir =  true, isGrpNext = false, wrapBlocks = false )
   
   def focusElem_TopBtm (toTop:Boolean, withinBlock:Boolean) = {
      val pickElem : IndexedSeq[String] => Option[String] = if (toTop) _.headOption else _.lastOption
      val curInGrpd = groupedElemsMap.contains(curElemId)
      // lets enumerate all ways we can end up in grouped block top/btm (which appears underneath recents block)
      if (inGroupedMode &&                     // we must be in grouped mode
            (inSearchState ||                  // then either we could be in search state (in grpd mode) ..
               (withinBlock && curInGrpd) ||   // or we're doing within-block nav and already in group mode ..
               (!withinBlock && !toTop)        // or we're doing cross-block nav and going to bottom
      ) ) {
         pickElem(groupedIdsVec).flatMap(groupedElemsMap.get)
      } else {
         pickElem(recentsIdsVec).flatMap(recentsElemsMap.get)
      }
   } .map(_.elem) .foreach (setCurElemHighlight)

   def focusElem_Top()         = focusElem_TopBtm ( toTop =  true, withinBlock = false )
   def focusElem_Bottom()      = focusElem_TopBtm ( toTop = false, withinBlock = false )
   def focusBlockElem_Top()    = focusElem_TopBtm ( toTop =  true, withinBlock = true  )
   def focusBlockElem_Bottom() = focusElem_TopBtm ( toTop = false, withinBlock = true  )

}





object SearchDisplay {
   import SwitchePageState._
   import Switche._
   
   
   val searchBox      = input (
      `type`:="text", autocomplete:="off", id:="searchBox", placeholder:=""
   ).render
   
   var cachedSearchBoxTxt = ""
   
   val searchBoxCtrlKeys = Set ("ArrowLeft","ArrowRight","Delete","Backspace", "a", "x", "c", "v", "z", "y")
   val searchBoxExclKeys = Set ("Meta","Alt","Control","Shift", "AudioVolumeUp","AudioVolumeDown")
   
   
   def getSearchBoxText () = {
      searchBox.value.trim
   }
   def activateSearchBox () = {
      searchBox.disabled = false;
      searchBox.focus()
   }
   def blurSearchBox() = {
      //searchBox.blur()
      //Try { doc.activeElement.asInstanceOf[HTMLElement] } .toOption .foreach(_.blur())
      //Option (doc.querySelector(s".curElem")) .flatMap (e => Try { e.asInstanceOf[HTMLElement] }.toOption) .foreach(_.blur())
      // gaah .. in theory, any of ^^ these should work, and they do remove the blinking cursor from there ..
      // however, in our particular case, we had added text cursor highlight in windows, and that seems to persist even w/o focus!
      // .. so we'll just disable the whole searchbox instead .. oh well
      searchBox.disabled = true
   }

   def handle_SearchModeKeyup (e:KeyboardEvent) = { //println(s"searchbox keyup: ${e.key}")
      // Note: all search-box key handling is now done at doc level capture phase
      // (which selectively allows char updates etc to filter down to searchBox)
      RibbonDisplay.closeDropdowns()
      val curSearchBoxTxt = searchBox.value.trim
      if (curSearchBoxTxt.isEmpty || e.key == "Escape") {
         cachedSearchBoxTxt = ""
         exitSearchState()
      } else if (curSearchBoxTxt != cachedSearchBoxTxt) {
         cachedSearchBoxTxt = curSearchBoxTxt
         RenderSpacer.immdtRender()     // using immdt-render as we want to reset-search-match-focus after it's done
         resetSearchMatchFocus()
      }
   }
   // this is called whenever we want to setup selective propagation to searchbox
   def checkSearchBox_StopProp(e:KeyboardEvent, doPassthrough:Boolean): Boolean = {
      val doStopProp = searchBoxExclKeys.contains(e.key) || (!doPassthrough && !inSearchState)
      if (!searchBoxExclKeys.contains(e.key)) { inSearchState = true; activateSearchBox() }
      doStopProp
   }
   def resetSearchMatchFocus() : Unit = {
      import ElemsDisplay._;
      val (_, idsVec, elemsMap) = getIdfnVecAndMap (if (inGroupedMode) ElemTs.G else ElemTs.R)
      idsVec.headOption .flatMap(elemsMap.get) .map(_.elem) .map(setCurElemHighlight) .getOrElse(clearCurElemHighlight())
   }
   def exitSearchState() = { //println("exit-search-state")
      inSearchState = false
      searchBox.value = ""
      blurSearchBox()
      RenderSpacer.queueSpacedRender()
   }

   def rebuildSearchStateElems() : Unit = {
      // we'll build both recents and grouped for search-state together to reuse common mechanisms
      // note that in search state, there will be no group nav, nav will be restricted to search matches, and if grouped, nav will use that block
      val matchStr = SearchDisplay.searchBox.value.trim
      case class SearchedElem (id:String, elem:Div, chkPassed:Boolean)
      def getSearchElem (rle:RenderListEntry, elemT:ElemT, grpT:GrpT, r:CheckSearchExeTitleRes) = {
         hwndMap .get(rle.hwnd) .map { wde =>
            val id = if (elemT == ElemTs.G) groupedId(wde.hwnd) else recentsId(wde.hwnd)
            val elem = ElemsDisplay.makeElemBox (id, wde, rle.y, elemT, grpT, r.exeSpan, r.ySpan, r.titleSpan)
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
         getCappedRecents() .zipWithIndex .foreach { case ((e,gt),i) =>
            getSearchMatchRes(e) .flatMap (res => getSearchElem (e, ElemTs.GR, gt, res)) .foreach { se =>
               elemsMap .put (se.id, OrderedElemsEntry (i, se.elem))
         } }
         recentsElemsMap = elemsMap
         recentsIdsVec = elemsMap.keys.toVector
      }
   }
   
}





object RibbonDisplay {
   import SwitcheFaceConfig._
   import Switche._
   import SwitchePageState._
  
   
   // -- setup for all the various indicators and their tooltips --
   class Indicator (cls:String) {
      
      val content = div (`class`:=s"content $cls").render   // gets setup with a symbol in css
      def setContent (e:Element) = { clearedElem(content).appendChild(e); this }
      
      val tooltip = div (`class`:="tooltip left" ).render
      var isToggledOn = false
      
      def showTooltip () = { tooltip.classList.add ("show") }
      def hideTooltip () = { if (!isToggledOn) { tooltip.classList.remove ("show") } }
      
      def toggleTooltip () = {
         isToggledOn = !isToggledOn;
         if (isToggledOn) { showTooltip() } else { hideTooltip() }
      }
      def clearToggle () = { isToggledOn = false; hideTooltip() }
      
      def setTooltip (e:Element)  = { tooltip .replaceChildren (e); this }
      def setTooltip (tip:String) = { tooltip .replaceChildren (span(tip).render); this }
      
      
      val indicator = div ( `class`:="indicator left",
         content, tooltip, onmouseover:={() => showTooltip()}, onmouseout:={() => hideTooltip()}
      ).render
      def alignRight() = {
         indicator.classList.remove("left"); tooltip.classList.remove("left");
         indicator.classList.add("right");   tooltip.classList.add("right");
         this
      }
      def set (turn_on:Boolean) = {
         if (turn_on) { content.classList.add("on") }
         else         { content.classList.remove("on") }
      }
      def blip () = {
         js.timers.setTimeout(50 ) { set (true ) }
         js.timers.setTimeout(250) { set (false) }
      }
      
   }
   
   private val altTabIndicator    = new Indicator ("altTabIndicator" )
   private val rbtnWhlIndicator   = new Indicator ("rbtnWheelIndicator" )
   private val autoOrderIndicator = new Indicator ("autoOrderIndicator" )
   private val elevIndicator      = new Indicator ("elevIndicator" )
   
   private val armedIndicator    = new Indicator ("armedIndicator")
   armedIndicator .setTooltip ("Key Release Activation : Inactive")
   
   private val dragIndicator  = new Indicator ("dragIndicator" )
   //dragIndicator.content.setAttribute("data-tauri-drag-region", "")
   // ^^ tauri drag region by default maximizes on double-click simulating the titlebar .. ofc that wouldnt work for us
   // .. instead, we'll set this as -webkit-app-region: drag in css .. but that makes double click not work, so we'll specify triple-click
   dragIndicator.content.ondblclick = {_ => SendMsgToBack.FE_Req_SelfAutoResize()}
   dragIndicator .setTooltip ("Click and hold here to drag. \nTriple-click to Auto-Size")
   
   
   private val countSpan = span (`class`:="countSpan").render
   private val countsIndicator = new Indicator ("countsIndicator")
   countsIndicator .setContent (countSpan)
   countsIndicator .setTooltip ("Listed Windows Counts")
   def updateCountsSpan () : Unit = { countSpan .replaceChildren ( span ( s"(${renderList.length})" ).render ) }
   
   private val warnIndicator  = new Indicator ("warnIndicator" ) .alignRight()
   warnIndicator .setTooltip ("No Warnings or Errors")
   
   private val helpIndicator  = new Indicator ("helpIndicator" ) .alignRight()
   helpIndicator.content.onclick = {(e:MouseEvent) => { helpIndicator.toggleTooltip(); e.stopPropagation() } }
   helpIndicator .setTooltip (HelpText.helpText)
   
   
   def setAltTabEnabled (isEnabled:Boolean) = {
      altTabIndicator.set(isEnabled)
      altTabIndicator.setTooltip ( s"Alt-Tab Replacement : ${if (isEnabled) "Enabled" else "Disabled"}" )
   }
   def setRbtnWheelEnabled (isEnabled:Boolean) = {
      rbtnWhlIndicator.set(isEnabled)
      rbtnWhlIndicator.setTooltip ( s"Mouse Right Btn with Wheel Activation : ${if (isEnabled) "Enabled" else "Disabled"}" )
   }
   def setGrpOrderingAuto (isEnabled:Boolean) = {
      autoOrderIndicator.set(isEnabled)
      autoOrderIndicator.setTooltip ( s"Groups Ordering is : ${if (isEnabled) "Auto" else "From-Configs (Not Auto)"}" )
   }
   def setElevated (isElevated:Boolean) = {
      elevIndicator.set(isElevated)
      elevIndicator.setTooltip ( s"Running Elevated : ${if (isElevated) "Yes" else "No"}")
   }
   def setReleaseArmed(isArmed:Boolean) = {
      armedIndicator.set(isArmed)
      armedIndicator.setTooltip ( s"Key Release Activation : ${if (isArmed) "Active" else "Inactive"}" )
   }
   
   
   private val debugLinks = span () .render
   def updateDebugLinks() : Unit = {
      clearElem(debugLinks)
      if (inElectronDevMode) {
         val printExclLink =  a ( href:="#", "DebugPrint", onclick:={(e:MouseEvent) => SendMsgToBack.FE_Req_DebugPrint()} ).render
         debugLinks.appendChild ( printExclLink )
   } }
   def debugDisplayMsg (msg:String) = { debugLinks.innerHTML = s"$msg (${js.Date.now().toString})"; }
   
   
   
   // -- menu dropdown management --
   object MenuDropdown {
      
      def menuItem (text:String, hotkey:String, action:() => Unit) = {
         val menuSpan = span (`class`:="menuItem", text, span (`class`:="menuHotkey", hotkey) ).render
         a (menuSpan, onclick:= {() => blipArmedIndicator(); action()} ).render
      }
      
      val refresh    = menuItem ("Refresh"      ,   "Ctrl+R" ,   {() => SendMsgToBack.FE_Req_Refresh()      } )
      val reload     = menuItem ("Reload"       ,   "F5"     ,   {() => handleReq_Reload()                  } )
      val grp_tgl    = menuItem ("Group Mode"   ,   "Ctrl+G" ,   {() => handleReq_GroupModeToggle()         } )
      val conf_edit  = menuItem ("Edit Config"  ,   ""       ,   {() => SendMsgToBack.FE_Req_EditConfig()   } )
      val conf_reset = menuItem ("Reset Config" ,   ""       ,   {() => SendMsgToBack.FE_Req_ResetConfig()  } )
      val quit       = menuItem ("Quit"         ,   "Alt+F4" ,   {() => SendMsgToBack.FE_Req_SwitcheQuit()  } )
      
      val dropdown = div (`class`:="dropdown", refresh, reload, grp_tgl, conf_edit, conf_reset, quit).render
      val menu_link = span (`class`:="menulink", onclick := {()=> dropdown.classList.toggle("show")}, nbsp(4) ).render
      val menu_dropdown = div (`class`:="menubox", onclick := {(e:MouseEvent) => e.stopPropagation()}, menu_link, dropdown ).render
      // ^^ the stop propagation prevents the click from bubbling up to the document where we have set clicks to close the dropdown
   }
   
   def closeDropdowns () = {
      MenuDropdown.dropdown.classList.remove("show")
      helpIndicator.clearToggle()
   }
   def blipArmedIndicator () = { armedIndicator.blip() }
   
   
   
   // -- pulling together all the pieces of the top ribbon --
   val topRibbon =  div (
      id:="top-ribbon", MenuDropdown.menu_dropdown,
      altTabIndicator.indicator, rbtnWhlIndicator.indicator, autoOrderIndicator.indicator,
      elevIndicator.indicator, armedIndicator.indicator, dragIndicator.indicator, countsIndicator.indicator,
      debugLinks, SearchDisplay.searchBox, warnIndicator.indicator, helpIndicator.indicator
   ).render
   
}

