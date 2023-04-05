package switche

import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.JSGlobal
import org.scalajs.dom
import org.scalajs.dom.html.Span
import org.scalajs.dom.{KeyboardEvent, document => doc}
import scalatags.JsDom.all._

import scala.collection.mutable
import scala.scalajs.concurrent.JSExecutionContext.Implicits.queue

// this is to allow invoke returned Promise[_] to be converted/treated as Future[_]
import scala.scalajs.js.Thenable.Implicits._


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



object SwitcheTauri {
   
   def main (args: Array[String]): Unit = {
      
      doc.addEventListener ( "DOMContentLoaded", { (e: dom.Event) =>
         
         doc.body.appendChild (SwitcheFacePage.getShellPage())
         
         SwitcheState.setTauriEventListeners()
         
         js.timers.setTimeout (50) { SendMsgToBack.FE_Req_Data_Load() }
         
         // the fgnd/close/title change listeners should in theory cover everything, but might be useful to periodically clean up random things that might fall through
         js.timers.setInterval(30*1000) { SendMsgToBack.FE_Req_Refresh() }
         
      } )
   }
   
}


