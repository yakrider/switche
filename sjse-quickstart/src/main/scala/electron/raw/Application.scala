package electron.raw

import nodejs.raw.EventEmitter

import scala.scalajs.js

@js.native
@js.annotation.JSImport("electron", "Application")
private[electron] abstract class Application extends js.Object with EventEmitter {
  def quit(): Unit = js.native
}
