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
//case class IconLookup_P     ( hwnd:Int, exe:String, iid:String )  derives ReadWriter
case class BackendNotice_P  ( msg:String )                        derives ReadWriter
case class HotKey_P         ( req:String )                        derives ReadWriter
case class AppWindowEvent_P ( app_win_ev:String )                 derives ReadWriter



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
   
   //def FE_Req_Switch_TabsOutliner () = send_FE_Req_Activate_Matching ( "chrome.exe", "Tabs Outliner" )
   //def FE_Req_Switch_NotepadPP    () = send_FE_Req_Activate_Matching ( "notepad++.exe" )
   //def FE_Req_Switch_IDE          () = send_FE_Req_Activate_Matching ( "idea64.exe"    )
   //def FE_Req_Switch_Music        () = send_FE_Req_Activate_Matching ( "MusicBee.exe"  )
   //def FE_Req_Switch_Browser      () = send_FE_Req_Activate_Matching ( "chrome.exe"    )
   
   def send_FE_Req_Activate_Matching (exe:String, title:String = "") = {
      send ( front_end_req ( "fe_req_activate_matching", _hwnd = None, _params = Seq(exe, title).filterNot(_.isEmpty) ) )
   }
}





object Switche {
   
   val hwndMap = new mutable.HashMap[Hwnd,WinDatEntry] ()
   
   var inElectronDevMode = false;
   var inGroupedMode = true;
   var isDismissed = false;
   var scrollEnd_disarmed = false;

   var renderList : Seq[RenderListEntry] = Seq()
   var groupedRenderList : Seq[Seq[RenderListEntry]] = Seq()

   val iconsCache = mutable.HashMap[Int,String]()
  
   def scrollEnd_arm()    = { scrollEnd_disarmed = false }
   def scrollEnd_disarm() = { scrollEnd_disarmed = true }
   
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
      // note: for all practical purposes, invoke must be scroll-down because successive invokes should scroll down the list
      // (unless ofc we started tracking if switche is topmost, in which case, we could resetFocus upon invoke if not-topmost .. meh)
      procHotkey_ScrollDown()
   }
   def procHotkey_ScrollDown() = {
      SwitchePageState.triggerHoverLockTimeout()
      scrollEnd_arm();
      if (isDismissed) { SwitchePageState.resetFocus(); isDismissed = false; }
      else { SwitchePageState.focusElem_Next() }
   }
   def procHotkey_ScrollUp() = {
      SwitchePageState.triggerHoverLockTimeout()
      scrollEnd_arm();
      if (isDismissed) { SwitchePageState.focusElem_Bottom(); isDismissed = false; }
      else { SwitchePageState.focusElem_Prev() }
   }
   def procHotkey_ScrollEnd() = {
      SwitchePageState.triggerHoverLockTimeout()
      // note below that a scroll-end only has meaning if we're scrolling (and hence already active)
      if (!isDismissed && !scrollEnd_disarmed) {
         isDismissed = true
         SwitchePageState.handleReq_CurElemActivation()
      } else { scrollEnd_arm(); }
   }
  
   def setTauriEventListeners() : Unit = {
      // todo : the unlisten fns returned by these prob could be stored and used to unlisten during unmount (e.g. page refresh?)
      TauriEvent .listen ( "backend_notice",            backendNoticeListener _      )
      TauriEvent .listen ( "updated_render_list",       updateListener_RenderList _  )
      TauriEvent .listen ( "updated_win_dat_entry",     updateListener_WinDatEntry _ )
      TauriEvent .listen ( "updated_icon_entry",        updateListener_IconEntry _   )
      //TauriEvent .listen ( "updated_icon_lookup_entry", updateListener_IconLookupE _ )
   }
   
   def updateListener_RenderList  (e:BackendPacket) : Unit = {
      //println ("received render_list")
      val ep:RenderList_P = upickle.default.read[RenderList_P](e.payload);
      //println(ee.payload); println (ep.rl)
      renderList = ep.rl;  groupedRenderList = ep.grl
      // we could try to only render if changed, but rendering is cheap and frequent enough it doesnt matter
      RenderSpacer.queueSpacedRender()
   }
   
   def updateListener_WinDatEntry (e:BackendPacket) : Unit = {
      
      val ep : Option [WinDatEntry_wNull] = scala.util.Try {
         upickle.default .read[WinDatEntry_wNull] (e.payload)
      } .fold (
         err => { println(err); None },
         wde => Some(wde)
      )
      //ep .map (wde => s"${wde.hwnd} : ${wde.win_text}") .foreach(println)
      
      val wde = ep.map(_.conv())
      val wde_old = wde.map(_.hwnd).flatMap(hwndMap.get)
      wde.foreach (wde => hwndMap.update (wde.hwnd, wde))
      if ( wde_old.isDefined && wde_old.flatMap(_.win_text) != wde.flatMap(_.win_text)) {
         // surgically update title for that elem
         wde.foreach (SwitchePageState.handle_TitleUpdate)
      }
      // for everything else, we'll just queue a render .. these things are pretty cheap really
      RenderSpacer.queueSpacedRender();
   }
   
   def updateListener_IconEntry (e:BackendPacket) : Unit = {
      //println (s"got icon entry: ${e.payload}");
      val ep:IconEntry_P = upickle.default.read[IconEntry_P](e.payload)
      updateIconCache (ep.ico_id, ep.ico_str);
   }
   
   def backendNoticeListener (e:BackendPacket) : Unit = {
      println (s"got backend_notice payload: ${e.payload}")
      val ep:BackendNotice_P = upickle.default.read[BackendNotice_P](e.payload);
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
   val minRenderSpacing = 50; val slop = 4; // in ms, slop is there just to catch jitter, delays etc, might not be needed
   var lastRenderTargStamp = 0d;
   def immdtRender() : Unit = {
      lastRenderTargStamp = js.Date.now()
      SwitcheFacePage.updatePageElems()
   }
   def queueSpacedRender():Unit = {
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


