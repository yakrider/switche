package electron

import nodejs.EventEmitter

import scala.scalajs.js
import scala.scalajs.js.Function

object BrowserWindow {
  def apply(options: js.Object)(implicit electron: Electron): BrowserWindow =
    new BrowserWindow(js.Dynamic.newInstance(electron.BrowserWindow)(options).asInstanceOf[raw.BrowserWindow])
}

class BrowserWindow private[electron](window: raw.BrowserWindow) extends EventEmitter {
  override def emit(event: String, args: Any*): Unit = window.emit(event, args)
  override def getMaxListeners: Long = window.getMaxListeners
  override def once(event: String)(callback: Function): Unit = window.once(event)(callback)
  override def setMaxListeners(count: Long): Unit = window.setMaxListeners(count)
  override def removeListener(event: String, listener: Function): Unit = window.removeListener(event, listener)
  override def on(event: String)(callback: Function): Unit = window.on(event)(callback)
  override def removeAllListeners(): Unit = window.removeAllListeners()
  override def removeAllListeners(event: String): Unit = window.removeAllListeners(event)
  override def listeners(event: String): Array[Function] = window.listeners(event).toArray
  override def listenerCount(event: String): Long = window.listenerCount(event)
  def loadURL(url: String): Unit = window.loadURL(url)

  val webContents: WebContents = window.webContents
}
