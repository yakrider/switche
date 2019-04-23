package electron.ipc

import scala.scalajs.js

@js.native
trait IpcRenderer extends nodejs.raw.EventEmitter {
  def send(event: String, args: Any*): Unit
  def sendSync(event: String, args: Any*): js.Any
  def sendToHost(event: String, args: Any*): Unit
}
