package electron

import nodejs.EventEmitter

import scala.scalajs.js.Function

class Application private[electron](application: raw.Application) extends EventEmitter {
  override def emit(event: String, args: Any*): Unit = application.emit(event, args)
  override def getMaxListeners: Long = application.getMaxListeners
  override def once(event: String)(callback: Function): Unit = application.once(event)(callback)
  override def setMaxListeners(count: Long): Unit = application.setMaxListeners(count)
  override def removeListener(event: String, listener: Function): Unit = application.removeListener(event, listener)
  override def on(event: String)(callback: Function): Unit = application.on(event)(callback)
  override def removeAllListeners(): Unit = application.removeAllListeners()
  override def removeAllListeners(event: String): Unit = application.removeAllListeners(event)
  override def listeners(event: String): Array[Function] = application.listeners(event).toArray
  override def listenerCount(event: String): Long = application.listenerCount(event)
  def quit(): Unit = application.quit()

  def onActivate = on("activate")_
  def onReady = on("ready")_
  def onWindowAllClosed = on("window-all-closed")_
  def onceActivate = once("activate")_
  def onceReady = once("ready")_
  def onceWindowAllClosed = once("window-all-closed")_
}
