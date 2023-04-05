package switche

import scala.collection.mutable
import scala.collection.mutable.{HashMap, LinkedHashMap}
import scala.language.postfixOps
import scala.scalajs.js
import scala.scalajs.js.|.fromTypeConstructor

import upickle._
import upickle.default._



type Hwnd = Int


// define various payloads that can be recieved in events too
case class RenderListEntry ( hwnd:Int, y:Int ) derives ReadWriter
case class RenderList_P    ( rl: Seq[RenderListEntry], grl: Seq[Seq[RenderListEntry]] ) derives ReadWriter

case class ExePathName   (full_path:String, name:String ) derives ReadWriter
case class WinDatEntry   (hwnd:Hwnd, win_text:Option[String], exe_path_name:Option[ExePathName], icon_cache_idx:Int )  // derives ReadWriter
case class WinDatEntry_P ( msg:String, wde:WinDatEntry_wNull )   derives ReadWriter
case class WinDatEntry_wNull (hwnd:Hwnd, win_text:String, exe_path_name:ExePathName, icon_cache_idx:Int) derives ReadWriter {
   def conv() = WinDatEntry (hwnd, Option(win_text), Option(exe_path_name), icon_cache_idx)
}

case class IconEntry_P      ( iid:String, icon:String )           derives ReadWriter
case class IconLookup_P     ( hwnd:Int, exe:String, iid:String )  derives ReadWriter
case class BackendNotice_P  ( msg:String )                        derives ReadWriter
case class HotKey_P         ( req:String )                        derives ReadWriter
case class AppWindowEvent_P ( app_win_ev:String )                 derives ReadWriter



@js.native
trait front_end_req extends js.Object {
   var req : String = js.native
   var hwnd : Int = js.native
}
object front_end_req {
   def apply (_req:String, _hwnd:Option[Int]=None) : front_end_req = {
      val jdl = js.Dynamic.literal().asInstanceOf[front_end_req]
      jdl.req  = _req
      _hwnd .foreach (v => jdl.hwnd = v)     // only set if available, else shouldnt be in json at all
      jdl
   }
}



object SendMsgToBack {
   
   def send(fer:front_end_req) : Unit = {
      println (s"frontend emitting request: ${fer.req}")
      TauriEvent .emit ("frontend_request", fer) //.toFuture.onComplete(v => println(v));
   }
   
   def FE_Req_WindowActivate (hwnd:Int) = send ( front_end_req ( "fe_req_window_activate", Some(hwnd) ) )
   def FE_Req_WindowPeek     (hwnd:Int) = send ( front_end_req ( "fe_req_window_peek",     Some(hwnd) ) )
   def FE_Req_WindowMinimize (hwnd:Int) = send ( front_end_req ( "fe_req_window_minimize", Some(hwnd) ) )
   def FE_Req_WindowClose    (hwnd:Int) = send ( front_end_req ( "fe_req_window_close",    Some(hwnd) ) )
   
   def FE_Req_Data_Load       () = send ( front_end_req ( "fe_req_data_load"        ) )
   def FE_Req_Refresh         () = send ( front_end_req ( "fe_req_refresh"          ) )
   def FE_Req_Get_RenderLists () = send ( front_end_req ( "fe_req_get_render_lists" ) )
   def FE_Req_SwitcheEscape   () = send ( front_end_req ( "fe_req_switche_escape"   ) )
   def FE_Req_DebugPrint      () = send ( front_end_req ( "fe_req_debug_print"      ) )
   
   def FE_Req_Switch_Last         () = send ( front_end_req ( "fe_req_switch_tabs_last"     ) )
   def FE_Req_Switch_TabsOutliner () = send ( front_end_req ( "fe_req_switch_tabs_outliner" ) )
   def FE_Req_Switch_NotepadPP    () = send ( front_end_req ( "fe_req_switch_notepad_pp"    ) )
   def FE_Req_Switch_IDE          () = send ( front_end_req ( "fe_req_switch_ide"           ) )
   def FE_Req_Switch_Winamp       () = send ( front_end_req ( "fe_req_switch_winamp"        ) )
   def FE_Req_Switch_Browser      () = send ( front_end_req ( "fe_req_switch_browser"       ) )
}





object SwitcheState {
   
   val hwndMap = new mutable.HashMap[Hwnd,WinDatEntry] ()
   
   var inElectronDevMode = false;
   var inGroupedMode = true;
   var isDismissed = false;

   var renderList : Seq[RenderListEntry] = Seq()
   var groupedRenderList : Seq[Seq[RenderListEntry]] = Seq()
   var wdesUpdatedSinceRender : Boolean = false;

   
   //def getRenderList() = renderList
   //def getGroupedRenderList() = groupedRenderList
   //def getRenderListTopMost() = renderList.headOption.map(_.hwnd)


   def handleReq_GroupModeToggle() = {
      inGroupedMode = !inGroupedMode;
      RenderSpacer.immdtRender()
   }
   def setDismissed() = { isDismissed = false }
   def clearDismissed() = { isDismissed = true }

   def procAppWindowEvent_Focus() = { }
   def procAppWindowEvent_Blur() = { }
   def procAppWindowEvent_Show() = { isDismissed = false } // this is called for every hotkey, so dont want to refresh top icon etc here
   def procAppWindowEvent_Hide() = { isDismissed = true }

   def procHotkey_Invoke() = {
      // note: for all practical purposes, invoke must be scroll-down because successive invokes should scroll down the list
      // (unless ofc we started tracking if switche is topmost, in which case, we could resetFocus upon invoke if not-topmost .. meh)
      procHotkey_ScrollDown()
   }
   def procHotkey_ScrollDown() = {
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { SwitchePageState.resetFocus(); isDismissed = false; }
      else { SwitchePageState.focusElem_Next() }
   }
   def procHotkey_ScrollUp() = {
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed) { SwitchePageState.focusElem_Bottom(); isDismissed = false; }
      else { SwitchePageState.focusElem_Prev() }
   }
   def procHotkey_ScrollEnd() = {
      SwitchePageState.triggerHoverLockTimeout()
      // note below that a scroll-end only has meaning if we're scrolling (and hence already active)
      if (!isDismissed) { SwitchePageState.handleReq_CurElemActivation(); isDismissed = true; }
   }
  
   def setTauriEventListeners() : Unit = {
      // todo : the unlisten fns returned by these prob needs to be stored and used to unlisten during unmount (e.g. page refresh?)
      TauriEvent .listen ( "backend_notice",            backendNoticeListener _      )
      TauriEvent .listen ( "updated_render_list",       updateListener_RenderList _  )
      TauriEvent .listen ( "updated_win_dat_entry",     updateListener_WinDatEntry _ )
      TauriEvent .listen ( "updated_icon_entry",        updateListener_IconEntry _   )
      TauriEvent .listen ( "updated_icon_lookup_entry", updateListener_IconLookupE _ )
   }
   
   def updateListener_RenderList  (e:BackendPacket) : Unit = {
      println ("received render_list")
      val ep:RenderList_P = upickle.default.read[RenderList_P](e.payload);
      //println(ee.payload); println (ep.rl)
      val doRender = wdesUpdatedSinceRender || ep.rl != renderList || ep.grl != groupedRenderList
      renderList = ep.rl;  groupedRenderList = ep.grl
      wdesUpdatedSinceRender = false
      if (doRender) { RenderSpacer.immdtRender() }
      
      // todo how about when rls are the same but wdes have changed .. should those be rendered too?
   }
   
   def updateListener_WinDatEntry (e:BackendPacket) : Unit = {
      // note: since win-dat-entry might be updated for various reasons, we have the 'notice' field to distinguish between them
      // .. in particular, the only change that wont result in list-change is title-update
      println ("received win_dat_entry")
      //println(e.payload);
      val ep:Option[WinDatEntry_P] = scala.util.Try {
         upickle.default.read[WinDatEntry_P](e.payload)
      } .fold ((err => {println(err); None}), (pl=>Some(pl)))
      ep .map (pl => s"${pl.wde.hwnd} : ${pl.wde.win_text}") .foreach(println)
      val wde = ep.map(_.wde.conv())
      val wde_old = wde.map(_.hwnd).flatMap(hwndMap.get)
      wde.foreach (wde => hwndMap.update (wde.hwnd, wde))
      if ( ep.exists(_.msg == "title_changed") && wde_old.isDefined ) {
         // surgically update title for that elem
         if (wde_old.flatMap(_.win_text) != wde.flatMap(_.win_text)) {
            wde.foreach(SwitchePageState.handle_TitleUpdate)
         }
      } else { wdesUpdatedSinceRender = true } // flags for rendering upon new list arrival even if the list is identical!
   }
   
   def updateListener_IconEntry (e:BackendPacket) : Unit = {
   
   }
   
   def updateListener_IconLookupE (e:BackendPacket) : Unit = {
   
   }
   
   def backendNoticeListener (e:BackendPacket) : Unit = {
      println (s"got backend_notice payload: ${e.payload}")
      val ep:BackendNotice_P = upickle.default.read[BackendNotice_P](e.payload);
      println (s"backend_notice: ${ep.msg}")
      ep.msg match {
         case "hotkey_req__app_invoke"  =>  procHotkey_Invoke()
         case "hotkey_req__scroll_down" =>  procHotkey_ScrollDown()
         case "hotkey_req__scroll_up"   =>  procHotkey_ScrollUp()
         case "hotkey_req__scroll_end"  =>  procHotkey_ScrollEnd()
         case _ => { }
      }
   }
   
}



object RenderSpacer {
   // so too many requestAnimationFrame interspersed are taking a lot of time, as they each are upto 100ms, so gonna bunch them up too
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d;
   def immdtRender() : Unit = {
      lastRenderTargStamp = js.Date.now()
      SwitcheFacePage.updatePageElems()
   }
   def queueSpacedRender():Unit = {
      // main idea .. if its past reqd spacing, req frame now and update stamp
      // else if its not yet time, but nothing queued already, queue with reqd delay
      // else if last queued still in future, can just ignore it! -d
      if ( js.Date.now() + slop > lastRenderTargStamp ) {
         // i.e nothing queued is still in the future, so lets setup a delayed req w appropriate spacing
         val waitDur =  math.max (1, lastRenderTargStamp + minRenderSpacing - js.Date.now() - slop)
         lastRenderTargStamp = js.Date.now() + waitDur
         js.timers.setTimeout (waitDur) {
            // note that in theory, animation frames might not trigger when browser minimized etc, but didnt seem to matter w our show/hide mechanism
            //if (!SwitcheState.isDismissed) { // coz when browser minimized/hidden animation frame calls are disabled!
            //   js.Dynamic.global.window.requestAnimationFrame ({t:js.Any => SwitcheFacePage.render()})
            //} else { SwitcheFacePage.render() }
            // note that ^^ at 60Hz, repaints trigger ~16ms, so w our 50ms spacing, prob no longer much point in doing this .. so do it direct:
            SwitcheFacePage.updatePageElems()
      } }
   }

}
