package electron

import scala.scalajs.js

@js.native
@js.annotation.JSImport("electron", "WebContents")
abstract class WebContents extends js.Any {
  val debugger: js.Dynamic = js.native
  val devToolsWebContents: WebContents = js.native
  val hostWebContents: WebContents = js.native
  val session: js.Dynamic = js.native

  def loadURL(url: String): Unit = js.native
  def loadURL(url: String, options: js.Object): Unit = js.native
  def downloadURL(url: String): Unit = js.native
  def getURL: String = js.native
  def getTitle: String = js.native
  def isLoading: Boolean = js.native
  def isWaitingForResponse: Boolean = js.native
  def stop(): Unit = js.native
  def reload(): Unit = js.native
  def reloadIgnoringCache(): Unit = js.native
  def canGoBack: Boolean = js.native
  def canGoForward: Boolean = js.native
  def canGoToOffset(offset: Long): Boolean = js.native
  def clearHistory(): Unit = js.native
  def goBack(): Unit = js.native
  def goForward(): Unit = js.native
  def goToIndex(index: Long): Unit = js.native
  def goToOffset(offset: Long): Unit = js.native
  def isCrashed: Boolean = js.native
  def setUserAgent(userAgent: String): Unit = js.native
  def getUserAgent: String = js.native
  def insertCSS(css: String): Unit = js.native
  def executeJavaScript(code: String): Unit = js.native
  def executeJavaScript(code: String, userGesture: Boolean, callback: js.Function1[js.Any, Unit]): Unit = js.native
  def setAudioMuted(muted: Boolean): Unit = js.native
  def isAudioMuted: Boolean = js.native
  def undo(): Unit = js.native
  def redo(): Unit = js.native
  def cut(): Unit = js.native
  def copy(): Unit = js.native
  def paste(): Unit = js.native
  def pasteAndMatchStyle(): Unit = js.native
  def delete(): Unit = js.native
  def selectAll(): Unit = js.native
  def unselect(): Unit = js.native
  def replace(text: String): Unit = js.native
  def replaceMisspelling(text: String): Unit = js.native
  def insertText(text: String): Unit = js.native
  def findInPage(text: String): Unit = js.native
  def findInPage(text: String, options: js.Object): Unit = js.native
  def stopFindInPage(action: String): Unit = js.native
  //noinspection AccessorLikeMethodIsUnit
  def hasServiceWorker(callback: js.Function1[Boolean, Unit]): Unit = js.native
  def unregisterServiceWorker(callback: js.Function1[Boolean, Unit]): Unit = js.native
  def print(): Unit = js.native
  def print(options: js.Object): Unit = js.native
  def printToPDF(options: js.Object, callback: js.Function2[js.Error, nodejs.Buffer, Unit]): Unit = js.native
  def addWorkSpace(path: String): Unit = js.native
  def removeWorkSpace(path: String): Unit = js.native
  def openDevTools(): Unit = js.native
  def openDevTools(options: js.Object): Unit = js.native
  def closeDevTools(): Unit = js.native
  def isDevToolsOpened: Boolean = js.native
  def isDevToolsFocused: Boolean = js.native
  def toggleDevTools(): Unit = js.native
  def inspectElement(x: Long, y: Long): Unit = js.native
  def inspectServiceWorker(): Unit = js.native
  def send(channel: String, args: js.Any*): Unit = js.native
  def enableDeviceEmulation(parameters: js.Object): Unit = js.native
  def disableDeviceEmulation(): Unit = js.native
  def sendInputEvent(event: js.Object): Unit = js.native
  def beginFrameSubscription(callback: js.Function1[nodejs.Buffer, Unit]): Unit = js.native
  def endFrameSubscription(): Unit = js.native
  def savePage(fullPath: String, saveType: String, callback: js.Function1[js.Error, Unit]): Unit = js.native
}
