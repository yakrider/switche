package nodejs.raw

import scala.scalajs.js

/**
  * @see https://nodejs.org/api/events.html
  */
@js.native
trait EventEmitter extends js.Any {
  def emit(event: String, args: js.Object*): Unit = js.native
  def on(event: String)(callback: js.Function): Unit = js.native
  def once(event: String)(callback: js.Function): Unit = js.native

  def removeListener(event: String, listener: js.Function): Unit = js.native
  def removeAllListeners(): Unit = js.native
  def removeAllListeners(event: String): Unit = js.native

  def getMaxListeners: Long = js.native
  def setMaxListeners(count: Long): Unit = js.native
  def listenerCount(event: String): Long = js.native
  def listeners(event: String): js.Array[js.Function] = js.native
}