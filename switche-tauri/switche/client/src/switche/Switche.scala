package switche

import scala.collection.mutable
import scala.language.postfixOps
import scala.scalajs.js
import scala.scalajs.js.|.fromTypeConstructor
import upickle._
import upickle.default._

import scala.scalajs.js.annotation.JSGlobal
import org.scalajs.dom
import org.scalajs.dom.{KeyboardEvent, document => doc}

import scala.scalajs.js.JSConverters.JSRichIterableOnce

// this is to allow invoke returned Promise[_] to be converted/treated as Future[_]
import scala.scalajs.js.Thenable.Implicits._


type Hwnd = Int


// define various payloads that can be recieved in events too
case class RenderListEntry ( hwnd:Int, y:Int ) derives ReadWriter
case class RenderList_P    ( rl: Seq[RenderListEntry], grl: Seq[Seq[RenderListEntry]] ) derives ReadWriter

case class ExePathName   (full_path:String, name:String ) derives ReadWriter
case class WinDatEntry   (hwnd:Hwnd, win_text:Option[String], exe_path_name:Option[ExePathName], icon_cache_idx:Int )  // derives ReadWriter
//case class WinDatEntry_P ( msg:String, wde:WinDatEntry_wNull )   derives ReadWriter
case class WinDatEntry_wNull (hwnd:Hwnd, win_text:String, exe_path_name:ExePathName, icon_cache_idx:Int) derives ReadWriter {
   def conv() = WinDatEntry (hwnd, Option(win_text), Option(exe_path_name), icon_cache_idx)
}

case class IconEntry_P      ( ico_id:Int, ico_str:String )     derives ReadWriter
case class BackendNotice_P  ( msg:String )                     derives ReadWriter

case class Configs (
   auto_hide_enabled      : Boolean,
   grp_mode_is_default    : Boolean,
   n_grp_mode_top_recents : Int,
) derives ReadWriter

object Configs {
   def default() = Configs (
      auto_hide_enabled       =  true,
      grp_mode_is_default     =  true,
      n_grp_mode_top_recents  =  9,
   )
}




@js.native @JSGlobal("window.__TAURI__.tauri")
object TauriCommand extends js.Object {
   def invoke (cmd:String, args:js.Object) : js.Promise[String] = js.native
}

@js.native @JSGlobal("window.__TAURI__.event")
object TauriEvent extends js.Object {
   def emit   ( event: String, payload: js.Object ) : js.Promise[() => Unit] = js.native
   def listen ( event: String, handler: js.Function1[BackendPacket,_] ) : js.Promise[() => Unit] = js.native
}

@js.native
trait BackendPacket extends js.Object {
   val event:String = js.native
   val windowLabel:Option[String] = js.native
   val payload:String = js.native
   val id:Int = js.native
}



@js.native
trait front_end_req extends js.Object {
   var req : String = js.native
   var hwnd : Int = js.native
   var params : js.Array[String] = js.native
}
object front_end_req {
   def apply (_req:String, _hwnd:Option[Int]=None, _params:Seq[String]=Seq()) : front_end_req = { println((_req, _hwnd, _params))
      val jdl = js.Dynamic.literal().asInstanceOf[front_end_req]
      jdl.req  = _req
      _hwnd .foreach (v => jdl.hwnd = v)     // only set if available, else shouldnt be in json at all
      jdl.params = _params.toJSArray
      jdl
   }
}


object SendMsgToBack {
   
   def send(fer:front_end_req) : Unit = {
      //println (s"frontend emitting request: ${fer.req}")
      TauriEvent .emit ("frontend_request", fer) //.toFuture.onComplete(v => println(v));
   }
   
   def FE_Req_WindowActivate (hwnd:Int) = send ( front_end_req ( "fe_req_window_activate", Some(hwnd) ) )
   def FE_Req_WindowPeek     (hwnd:Int) = send ( front_end_req ( "fe_req_window_peek",     Some(hwnd) ) )
   def FE_Req_WindowMinimize (hwnd:Int) = send ( front_end_req ( "fe_req_window_minimize", Some(hwnd) ) )
   def FE_Req_WindowMaximize (hwnd:Int) = send ( front_end_req ( "fe_req_window_maximize", Some(hwnd) ) )
   def FE_Req_WindowClose    (hwnd:Int) = send ( front_end_req ( "fe_req_window_close",    Some(hwnd) ) )
   
   def FE_Req_Data_Load       () = send ( front_end_req ( "fe_req_data_load"        ) )
   def FE_Req_Refresh         () = send ( front_end_req ( "fe_req_refresh"          ) )
   def FE_Req_Get_RenderLists () = send ( front_end_req ( "fe_req_get_render_lists" ) )
   def FE_Req_SwitcheEscape   () = send ( front_end_req ( "fe_req_switche_escape"   ) )
   def FE_Req_SwitcheQuit     () = send ( front_end_req ( "fe_req_switche_quit"     ) )
   def FE_Req_DebugPrint      () = send ( front_end_req ( "fe_req_debug_print"      ) )
   
   def FE_Req_Switch_Last         () = send ( front_end_req ( "fe_req_switch_tabs_last"     ) )
   
   def FE_Req_Switch_TabsOutliner () = send ( front_end_req ( "fe_req_switch_tabs_outliner" ) )
   def FE_Req_Switch_NotepadPP    () = send ( front_end_req ( "fe_req_switch_notepad_pp"    ) )
   def FE_Req_Switch_IDE          () = send ( front_end_req ( "fe_req_switch_ide"           ) )
   def FE_Req_Switch_Music        () = send ( front_end_req ( "fe_req_switch_music"         ) )
   def FE_Req_Switch_Browser      () = send ( front_end_req ( "fe_req_switch_browser"       ) )
   
   
   def _send_FE_Req_Activate_Matching (exe:String, title:String = "") = {
      send ( front_end_req ( "fe_req_activate_matching", _hwnd = None, _params = Seq(exe, title).filterNot(_.isEmpty) ) )
   }
}





object Switche {
   
   val hwndMap = new mutable.HashMap[Hwnd,WinDatEntry] ()
   
   var inElectronDevMode  = false
   var inGroupedMode      = true
   var isDismissed        = false
   var isFgnd             = false
   var scrollEnd_armed    = false

   var renderList : Seq[RenderListEntry] = Seq()
   var groupedRenderList : Seq[Seq[RenderListEntry]] = Seq()

   val iconsCache = mutable.HashMap[Int,String]()
   
   var configs = Configs.default()
   
   def setDismissed()    = { isDismissed = true }
   def setNotDismissed() = { isDismissed = false }
   
   def setIsFgnd()  = { isFgnd = true }
   def setNotFgnd() = { isFgnd = false }
  
   def scrollEnd_arm()    = { scrollEnd_armed = true;  RibbonDisplay.setArmedIndicator_On() }
   def scrollEnd_disarm() = { scrollEnd_armed = false; RibbonDisplay.setArmedIndicator_Off() }
   
   def main (args: Array[String]): Unit = {
      doc.addEventListener ( "DOMContentLoaded", { (e: dom.Event) =>
         doc.body.appendChild (SwitcheFacePage.getShellPage())
         Switche.setTauriEventListeners()
         
         js.timers.setTimeout (300) { SendMsgToBack.FE_Req_Data_Load() }
         // ^^ in case this is cold restart for the backend too, lets give it some time to marshall its data
         
         // the fgnd/close/title change listeners should in theory cover everything, but might be useful to periodically clean up random things that might fall through
         //js.timers.setInterval(30*1000) { SendMsgToBack.FE_Req_Refresh() }
         
      } )
   }
   
   
   def getCachedIcon (iid:Int) : Option[String] = { iconsCache.get(iid) }
   def updateIconCache (iid:Int, ico_str:String): Unit = { iconsCache.update (iid, ico_str) }
   def deleteCachedIcon (iid:Int): Unit = { iconsCache.remove(iid); }
   
   def printIconCaches() = {
      println (s"Printing icon caches.. (${iconsCache.size}):");
      iconsCache.foreach(println)
   }
   

   def handleReq_GroupModeToggle() = {
      inGroupedMode = !inGroupedMode;
      RenderSpacer.immdtRender()
   }
   def procHotkey_Invoke() = {
      // should be same as scroll-down, except we won't arm the scroll-end behavior (as its F1 invocation)
      SwitchePageState.triggerHoverLockTimeout()
      if (isDismissed || !isFgnd) {
         setNotDismissed(); setIsFgnd()
         SwitchePageState.resetFocus()
      }
      else { SwitchePageState.focusElem_Next() }
   }
   def procHotkey_ScrollDown() : Unit = {
      // is the same as scrolling down upon invoke, except we'll arm scroll-end for alt-tab and right-mouse-scroll
      if (!SwitchePageState.verifyActionRepeatSpacing()) { return }
      scrollEnd_arm()
      procHotkey_Invoke()
   }
   def procHotkey_ScrollUp() : Unit = {
      if (!SwitchePageState.verifyActionRepeatSpacing()) { return }
      SwitchePageState.triggerHoverLockTimeout()
      scrollEnd_arm()
      if (isDismissed || !isFgnd) {
         setNotDismissed(); setIsFgnd()
         SwitchePageState.resetFocus()
      }
      else { SwitchePageState.focusElem_Prev() }
   }
   def procHotkey_ScrollEnd() = { println (s"scroll-end-armed-state: ${scrollEnd_armed}")
      SwitchePageState.triggerHoverLockTimeout()
      // note below that a scroll-end only has meaning if we're scrolling (and hence already active)
      if (!isDismissed && isFgnd && scrollEnd_armed) {
         //setDismissed(); scrollEnd_disarm()             // auto happens on cur-elem-activation below
         SwitchePageState.handleReq_CurElemActivation()
      }
   }
   def procHotkey_ScrollEnd_Disarm() = {
      if (!isDismissed && isFgnd && scrollEnd_armed) { scrollEnd_disarm() }
   }
   def procHotkey_SwitcheEscape() = {
      SwitchePageState.handleReq_SwitcheEscape (fromBkndHotkey = true)
   }
   def procBkndEvent_SwitcheFgnd() = {
      setIsFgnd(); setNotDismissed()
   }
   def procBkndEvent_FgndLost() = {
      setNotFgnd(); scrollEnd_disarm()
      SwitchePageState.resetFocus()
      SwitchePageState.exitSearchState()
      if (configs.auto_hide_enabled) { setDismissed() }
      RenderSpacer.queueSpacedRender()
   }
  
   def setTauriEventListeners() : Unit = {
      // todo : the unlisten fns returned by these prob could be stored and used to unlisten during unmount (e.g. page refresh?)
      TauriEvent .listen ( "backend_notice",            backendNoticeListener _      )
      TauriEvent .listen ( "updated_render_list",       updateListener_RenderList _  )
      TauriEvent .listen ( "updated_win_dat_entry",     updateListener_WinDatEntry _ )
      TauriEvent .listen ( "updated_icon_entry",        updateListener_IconEntry _   )
      TauriEvent .listen ( "updated_configs",           updateListener_Configs _     )
      //TauriEvent .listen ( "updated_icon_lookup_entry", updateListener_IconLookupE _ )
   }
   
   def updateListener_RenderList  (e:BackendPacket) : Unit = {
      //println ("received render_list")
      val ep:RenderList_P = upickle.default.read[RenderList_P](e.payload);
      //println(ee.payload); println (ep.rl)
      val rl_same = ep.rl == renderList && ep.grl == groupedRenderList
      renderList = ep.rl;  groupedRenderList = ep.grl
      if (!rl_same) { RenderSpacer.queueSpacedRender() }
   }
   
   def updateListener_WinDatEntry (e:BackendPacket) : Unit = {
      
      val ep : Option [WinDatEntry_wNull] = scala.util.Try {
         upickle.default .read[WinDatEntry_wNull] (e.payload)
      } .fold (
         err => { println(err); None },
         wde => Some(wde)
      )
      //ep .map (wde => s"${wde.hwnd} : ${wde.win_text}") .foreach(println)
      
      val wde : Option[WinDatEntry] = ep.map(_.conv())
      val wde_old = wde.map(_.hwnd).flatMap(hwndMap.get)
      
      wde.foreach (wde => hwndMap.update (wde.hwnd, wde))
      
      // for mere title-updates, since they can come w high frequency (e.g. title scrobbling), we dont want to re-render
      // (esp since re-rendering can introduce missed mouse events whlie the event targets are being swapped out)
      if ( wde_old.isDefined
         && wde_old.flatMap(_.win_text) != wde.flatMap(_.win_text)                  // title changed
         && wde_old.map(_.copy(win_text=None)) == wde.map(_.copy(win_text=None))    // but everything else is same
      ) {
         wde.foreach (SwitchePageState.handle_TitleUpdate)   // surgically update title for that elem
      } else {
         RenderSpacer.queueSpacedRender()   // for everything else, we'll just queue a render
      }
   }
   
   def updateListener_IconEntry (e:BackendPacket) : Unit = {
      //println (s"got icon entry: ${e.payload}");
      val ep:IconEntry_P = upickle.default.read[IconEntry_P](e.payload)
      updateIconCache (ep.ico_id, ep.ico_str);
   }
   
   def updateListener_Configs (e:BackendPacket) : Unit = {
      println (s"got configs: ${e.payload}");
      val confs_old = configs;
      configs = upickle.default.read[Configs](e.payload)
      if (confs_old.grp_mode_is_default != configs.grp_mode_is_default) {
         inGroupedMode = configs.grp_mode_is_default
         RenderSpacer.immdtRender()
      }
      if (confs_old.auto_hide_enabled != configs.auto_hide_enabled) {
         if (configs.auto_hide_enabled && !isDismissed && !isFgnd) {
            SwitchePageState.handleReq_SwitcheEscape()
         }
      }
   }
   
   def backendNoticeListener (e:BackendPacket) : Unit = {
      println (s"got backend_notice payload: ${e.payload}")
      val ep:BackendNotice_P = upickle.default.read[BackendNotice_P](e.payload);
      ep.msg match {
         case "hotkey_req__app_invoke"        =>  procHotkey_Invoke()
         case "hotkey_req__scroll_down"       =>  procHotkey_ScrollDown()
         case "hotkey_req__scroll_up"         =>  procHotkey_ScrollUp()
         case "hotkey_req__scroll_end"        =>  procHotkey_ScrollEnd()
         case "hotkey_req__scroll_end_disarm" =>  procHotkey_ScrollEnd_Disarm()
         case "hotkey_req__switche_escape"    =>  procHotkey_SwitcheEscape()
         case "switche_event__in_fgnd"        =>  procBkndEvent_SwitcheFgnd()
         case "switche_event__fgnd_lost"      =>  procBkndEvent_FgndLost()
         case _ => { }
      }
   }
   
}



object RenderSpacer {
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d;
   def immdtRender() : Unit = {
      lastRenderTargStamp = js.Date.now()
      SwitcheFacePage.updatePageElems()
   }
   def queueSpacedRender():Unit = { // println("rendering queued")
      // main idea .. if its past reqd spacing, rebuild view now and update stamp
      // else if its not yet time, but nothing queued already, queue with required delay
      // else if last queued still in future, can just ignore it
      if ( js.Date.now() + slop > lastRenderTargStamp ) {
         // i.e nothing queued is still in the future, so lets setup a delayed req w appropriate spacing
         val waitDur =  math.max (1, lastRenderTargStamp + minRenderSpacing - js.Date.now() - slop)
         lastRenderTargStamp = js.Date.now() + waitDur
         js.timers.setTimeout (waitDur) {
            // note that ^^ at 60Hz, repaints trigger ~16ms, so w our 50ms spacing, no longer much point in animation frames .. so do it direct:
            SwitcheFacePage.updatePageElems()
      } }
   }

}


