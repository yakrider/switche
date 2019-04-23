package nodejs

import scala.scalajs.js

trait EventEmitter {
  def emit(event: String, args: Any*): Unit
  def on(event: String)(callback: js.Function): Unit
  def once(event: String)(callback: js.Function): Unit

  def removeListener(event: String, listener: js.Function): Unit
  def removeAllListeners(): Unit
  def removeAllListeners(event: String): Unit

  def getMaxListeners: Long
  def setMaxListeners(count: Long): Unit
  def listenerCount(event: String): Long
  def listeners(event: String): Array[js.Function]
}