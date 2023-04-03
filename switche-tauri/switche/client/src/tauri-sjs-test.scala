
import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.{JSGlobal}
import org.scalajs.dom
import org.scalajs.dom.html.Span
import org.scalajs.dom.{KeyboardEvent, document => doc}
import scalatags.JsDom.all._

import scala.collection.mutable
import scala.scalajs.concurrent.JSExecutionContext.Implicits.queue

// this is to allow invoke returned Promise[_] to be converted/treated as Future[_]
import scala.scalajs.js.Thenable.Implicits._


@js.native @JSGlobal("window.__TAURI__.tauri")
object Tauri extends js.Object {
   def invoke (cmd:String, args:js.Object) : js.Promise[String] = js.native
}

@js.native
trait Event[T] extends js.Object {
   val event:String = js.native
   val windowLabel:Option[String] = js.native
   val payload:T = js.native
   val id:Int = js.native
}

@js.native @JSGlobal("window.__TAURI__.event")
object TauEv extends js.Object {
   def emit[T]   ( event: String, payload: T                        ) : js.Promise[() => Unit] = js.native
   def listen[T] ( event: String, handler: js.Function1[Event[T],_] ) : js.Promise[() => Unit] = js.native
}


object TauriSjsTest {
   
   
   val evSpansBuffer = mutable.ListBuffer[dom.html.Span]();
   
   val logDiv = div (`class`:="logs").render
   val clearBtn = span (`class`:="dimBtn", "clear", onclick:= {() => { clearPage(); evSpansBuffer.clear(); }} )
   val reloadBtn = span (`class`:="dimBtn", "reload", onclick:= {() => reload()} )
   
  
   val header = div (`class`:="header", clearBtn, nbsp(2), reloadBtn)
   
   var unlistener: Option[()=>Unit] = None;
   
   def main (args: Array[String]): Unit = {
      doc.addEventListener ("DOMContentLoaded", { (e: dom.Event) =>
         doc.body.appendChild (div (id:="pageDiv", div (header, logDiv)).render);
         setPageEventHandlers();
      } );
   }

   def logAdd (s:String) = logDiv.append (div (`class`:="mk dn", s ).render )

   def setPageEventHandlers () = {
      doc.onkeydown  = (e: KeyboardEvent) => { printKeyEvent (e,"dn") }
      doc.onkeyup    = (e: KeyboardEvent) => { printKeyEvent (e,"up") }
     
      TauEv.listen ("ping", pingL ) .onComplete {ft =>
         unlistener = ft.toOption
         println(unlistener)
      };

   }
   val pingL = (e:Event[String]) => {println(e.payload); logAdd(s"got ${e.payload}");}
   
   //lazy val ll = TauEv.listen ("ping", pingL);
   
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   
   def clearPage (): Unit = {
      logDiv.replaceChildren();
      
      Tauri .invoke ("greet", JsObject(name=s"${System.currentTimeMillis()}")) .onComplete { case str =>
         //str.toOption .foreach (logAdd);
      }
      TauEv .emit ("clear", "sjs-clear-ev");
      //println (ll.value);
   }
   def reload() = { dom.window.location.reload() }

   
   def printKeyEvent (e:KeyboardEvent, evType:String) : Unit =  {
      e.preventDefault(); e.stopPropagation();
      logDiv.append ( div ( s"${e}" ).render )
   }

}



