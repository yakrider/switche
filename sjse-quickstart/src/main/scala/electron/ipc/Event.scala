package electron.ipc

import scala.scalajs.js

@js.native
trait Event extends js.Object {
  var returnValue: js.Any
  val sender: electron.WebContents
}
