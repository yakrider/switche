package yakrider.switchenator.switchback



object JnaTaskSwitcherExploration extends App {

   import com.sun.jna.platform.WindowUtils
   import com.sun.jna.platform.win32
   import scala.collection.JavaConversions._
   import com.sun.jna.platform.DesktopWindow
   import scala.util.Try
   import com.sun.jna.platform.win32.WinDef.HWND

   case class TaskEntry (title:String, path:String, hwnd:win32.WinDef.HWND, loc:java.awt.Rectangle) {
      override def toString () = s"$title \n\t $path \n\t $loc \n\t ${hwnd.toString}"
   }
   def getWinPrintString (w:DesktopWindow) = {
      s"${w.getTitle} \n\t ${w.getFilePath} \n\t ${w.getHWND} @ ${w.getLocAndSize}"
      //s"${w.getTitle} \n\t ${w.getFilePath} \n\t ${w.getHWND} @ ${w.getLocAndSize} \n\t ico:${WindowUtils.}"
   }

   val ws = WindowUtils.getAllWindows(true)
   ws.size
   ws.map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)

   val chs = ws.filter(_.getFilePath.contains("Chrome"))
   chs.map{w => s"${w.getTitle} \n\t\t ${w.getFilePath}"} .foreach(println)

   ws.map{w => s"${w.getTitle} \n\t\t ${w.getFilePath}"} .foreach(println)
   ws.filterNot(_.getTitle.isEmpty).size
   ws.filterNot(_.getTitle.isEmpty).map{w => s"${w.getTitle} \n\t\t ${w.getFilePath}"} .foreach(println)

   val tes = ws. map { w => TaskEntry (w.getTitle, w.getFilePath, w.getHWND, w.getLocAndSize) }
   tes.zipWithIndex.map{case(k,v)=>v->k}.foreach(println)

   tes.filter(_.title.isEmpty).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   tes.filter(_.loc.isEmpty).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)

   val viz = ws.filter {w => !w.getTitle.isEmpty && !w.getLocAndSize.isEmpty }
   viz.size
   viz.map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)

   ws.filter(_.getTitle.isEmpty).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   ws.filter(_.getLocAndSize.isEmpty).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   ws.filterNot(_.getLocAndSize.isEmpty).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   ws.filter(_.getFilePath.contains("eclipse")).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)

   /* Observations :
    * - everything w zero rects can be ignored (afterall they cant be seen, cant switch to them)
    *    - there's some w titles in there! (eg Backup and Sync), others are besclient, isamtray, viber, eclipse-launchers etc
    * - things w/o titles can usually be ignored too, but most of them ALSO are zero rects, except for the singular explorer window..
    *    - so, assuming others might come up w untitled windows, prob SHOULD NOT use that as a hard filtering rule
    * -  of things w non-zero rects, there were just a few extraneous, presumably hidden windows etc
    *    - 'Start' for 'Explorer.EXE' usually in position 0, - an untitled explorer.exe in pos 1, - a 'Program Manager' for explorer
    *    - a 'VirtualWinMainClass', - apparently second copy of winamp,
    */

   val atest = ws.filter(_.getTitle.toLowerCase.contains("winamp")).last
   win32.User32.INSTANCE.SetForegroundWindow(atest.getHWND)

   val ser = atest.getHWND.getPointer.hashCode()
   val ser2 = com.sun.jna.Pointer.nativeValue(atest.getHWND.getPointer)
   val p = com.sun.jna.Pointer.createConstant(ser)
   val p2 = com.sun.jna.Pointer.createConstant(ser2)
   new com.sun.jna.platform.win32.WinDef.HWND(p).getPointer.hashCode()
   new com.sun.jna.platform.win32.WinDef.HWND(p2).getPointer.hashCode()

   val icon = WindowUtils.getWindowIcon(atest.getHWND)
   //val imv = QuickImageViewer.viewImage(icon,"some icon")


   ws.map(w => getWinPrintString(w) -> Option(WindowUtils.getWindowIcon(w.getHWND)).map(_.getWidth)).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   ws.filter(w => Option(WindowUtils.getWindowIcon(w.getHWND)).isDefined).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   ws.filterNot(w => Option(WindowUtils.getWindowIcon(w.getHWND)).isDefined).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v->k}.foreach(println)
   // ughh, so only 13 of 50 produced icons.. rest nulled.. most regular (non-appified) chromes, eclipse, electron/node stuff
   // .. so will have to figure out something for that later, presumably maybe digging through some other way of extracting icons to push from server

   trait AppIgnoreRule {
      def process (ws:Seq[DesktopWindow]) : Seq[DesktopWindow]
   }
   abstract class AppIgnoreRuleStringMatcher (matchStr:String, doExactMatch:Boolean, doExcludeNotInclude:Boolean=true) extends AppIgnoreRule {
      def getMatcherString (w:DesktopWindow) : String
      val matchStrLower = matchStr.toLowerCase()
      def process (ws:Seq[DesktopWindow]) = { ws .filter { w =>
         val s = getMatcherString(w).trim.toLowerCase
         val matched = if (doExactMatch) s.equals(matchStrLower) else s.contains(matchStrLower)
         if (doExcludeNotInclude) matched else !matched
      } }
   }
   case class AppIgnoreRuleTitle (
         matchStr:String, doExactMatch:Boolean, doExcludeNotInclude:Boolean
   ) extends AppIgnoreRuleStringMatcher (matchStr, doExactMatch, doExcludeNotInclude) {
      def getMatcherString (w:DesktopWindow) = w.getTitle
   }
   case class AppIgnoreRulePath (
         matchStr:String, doExactMatch:Boolean, doExcludeNotInclude:Boolean
   ) extends AppIgnoreRuleStringMatcher (matchStr, doExactMatch, doExcludeNotInclude) {
      def getMatcherString (w:DesktopWindow) = w.getFilePath
   }
   case object AppIgnoreRuleEmptyTitles extends AppIgnoreRule { // dont use, not very useful and can be misleading
      def process (ws:Seq[DesktopWindow]) = ws.filterNot (w => w.getTitle.isEmpty)
   }
   case object AppIgnoreRuleZeroRects extends AppIgnoreRule { // pretty robustly can use this one
      def process (ws:Seq[DesktopWindow]) = ws.filterNot(_.getLocAndSize.isEmpty)
   }
   ws.size
   ws.filterNot (w => w.getTitle.isEmpty).size
   ws.filterNot (w => w.getLocAndSize.isEmpty).size
   val r1:AppIgnoreRule = AppIgnoreRuleTitle ("chrome",false,true)

   r1.process(ws).map(getWinPrintString).foreach(println)

   val appIgnoreRules:Seq[AppIgnoreRule] = Seq ( AppIgnoreRuleZeroRects )

   case class AppIgnoreRuleEntry ( matchStr:String, doFullMatch:Boolean)
   case class AppIgnoreEntry (
         titleStr:Option[String]=None, titleFragStr:Option[String]=None, exeLocStr:Option[String]=None, execLocFragStr:Option[String]=None
   )

   val appIgnoreListExact = Seq () // Seq ( AppIgnoreEntry (??) )

   val appIgnoreCache = scala.collection.mutable.HashSet[TaskEntry]()

   ws.tail.head.getTitle

   def shouldPruneWindow (w:DesktopWindow, i:Int, iMax:Int) : Boolean = {
      val isExplorer = w.getFilePath.contains("Explorer.EXE") // common case
      if (i==0 && isExplorer && w.getTitle.equals("Start")) return true
      if (i<=1 && isExplorer && w.getTitle.isEmpty) return true
      if (i==iMax && isExplorer && w.getTitle.equals("Program Manager")) return true
      if (w.getTitle.equals("VirtuaWinMainClass") && w.getFilePath.contains("VirtuaWin")) return true
      return false
   }
   def pruneAppList (ws:Seq[DesktopWindow]) = {
      val f1 = ws .filterNot(_.getLocAndSize.isEmpty)
      val len = f1.size
      f1 .zipWithIndex .filterNot{case(w,i) => shouldPruneWindow(w,i,len-1)}.map(_._1)
   }

   ws.map(getWinPrintString).zipWithIndex.map{case(k,v)=>v+1->k}.foreach(println)
   ws.filterNot(_.getLocAndSize.isEmpty).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v+1->k}.foreach(println)
   pruneAppList(ws).map(getWinPrintString).zipWithIndex.map{case(k,v)=>v+1->k}.foreach(println)


   com.sun.jna.platform.WindowUtils.getWindowTitle(com.sun.jna.platform.win32.User32.INSTANCE.GetForegroundWindow())
   com.sun.jna.platform.win32.User32.INSTANCE
   yakrider.scala.utils.UtilFns.exec(s"curl http://localhost:7090/switchback/api/v0?cmd=activateHwndTest")

}


import com.sun.jna._
import com.sun.jna.ptr._

trait Kernel32 extends Library {
  def GetDiskFreeSpaceA(lpRootPathName: String,
       lpSectorsPerCluster: IntByReference, lpBytesPerSector: IntByReference,
        lpNumberOfFreeClusters: IntByReference, lpTotalNumberOfClusters: IntByReference): Boolean
  def GetComputerNameW(outName: Array[Char], size: IntByReference): Boolean
  def GetTempPathA(size: Integer, outName: Array[Byte]): Integer
}

import com.sun.jna.platform.win32.WinDef.DWORD
trait User32Ext extends Library {
   def LockSetForegroundWindow (uLockCode:Int):Boolean;
   def AllowSetForegroundWindow (dwProcessId:DWORD):Boolean;
}

object JnaExtTest {
   val libKernel32 = Native.loadLibrary("kernel32", classOf[Kernel32]).asInstanceOf[Kernel32]
   val libUser32Ext = Native.loadLibrary("user32", classOf[User32Ext]).asInstanceOf[User32Ext]

   libUser32Ext.LockSetForegroundWindow(1)
   libUser32Ext.LockSetForegroundWindow(2)

   // so a roundabout way of making win let the bknd process set foreground window..
   // lets say we interpret these lines together, then first we allow this foreground process to let any other process set foreground,
   // and before any other user input etc, we trigger via api call the server to set winamp foreground, and that finally seems to work!
   libUser32Ext.AllowSetForegroundWindow(new DWORD(-1L))
   yakrider.scala.utils.UtilFns.exec(s"curl http://localhost:7090/switchback/api/v0?cmd=activateHwndTest")
   // .. uhh, buts its a major pain still.. will have to figure out how to do eqv in electron/node, but if figuring that out, could just directly write all there

   // also, just noting, but damn apparently setting up our own jna wrappers for sys libraries was apparently really straightforward, even in scala!!

}


object SwitchBackJsonHelper {
   import com.sun.jna.Pointer
   import com.sun.jna.platform.DesktopWindow
   import com.sun.jna.platform.win32.WinDef.HWND

   import upickle.default.{ReadWriter, macroRW}

   case class AppListItem (hwndLong:String, exePath:String, title:String)
   object AppListItem { implicit def rw: ReadWriter[AppListItem] = macroRW }
   case class AppList (apps:Seq[AppListItem])
   object AppList { implicit def rw: ReadWriter[AppList] = macroRW }

   def serializeLocalPointer (hwnd:HWND) = Pointer.nativeValue(hwnd.getPointer).toString
   def deSerializeLocalPointer (hwndStr:String) = new HWND (Pointer.createConstant(hwndStr.toLong))

   def getAppListsJson (ws:Seq[DesktopWindow]) = {
      val listItems = ws .map (w => AppListItem (serializeLocalPointer(w.getHWND), w.getFilePath.replace("""\""","/"), w.getTitle))
      upickle.default.write (AppList(listItems))
   }
}

object SwitchBackJnaCore {

   import com.sun.jna.platform.WindowUtils
   import scala.collection.JavaConversions._
   import com.sun.jna.platform.DesktopWindow
   import com.sun.jna.platform.win32.WinDef.HWND

   def getWinPrintString (w:DesktopWindow) = {
      s"${w.getTitle} \n\t ${w.getFilePath} \n\t ${w.getHWND} @ ${w.getLocAndSize}"
   }
   def shouldPruneWindow (w:DesktopWindow, i:Int, iMax:Int) : Boolean = {
      val isExplorer = w.getFilePath.contains("Explorer.EXE") // common case
      if (i==0 && isExplorer && w.getTitle.equals("Start")) return true
      if (i<=1 && isExplorer && w.getTitle.isEmpty) return true
      if (i==iMax && isExplorer && w.getTitle.equals("Program Manager")) return true
      if (w.getTitle.equals("VirtuaWinMainClass") && w.getFilePath.contains("VirtuaWin")) return true
      return false
   }
   def pruneAppList (ws:Seq[DesktopWindow]) = {
      val f1 = ws .filterNot(_.getLocAndSize.isEmpty)
      val len = f1.size
      f1 .zipWithIndex .filterNot{case(w,i) => shouldPruneWindow(w,i,len-1)}.map(_._1)
   }
   def getTasksList() =  pruneAppList ( WindowUtils.getAllWindows(true) )
   def getTasksListJson() = SwitchBackJsonHelper.getAppListsJson(getTasksList())

   def activateApp (hwndStr:String):String = { println(s"got act call, hwndStr:${hwndStr}")
      scala.util.Try {
         val hwnd = SwitchBackJsonHelper.deSerializeLocalPointer(hwndStr)
         println (s"for hwnd ${hwnd}, got title: ${WindowUtils.getWindowTitle(hwnd)}")
         var res = com.sun.jna.platform.win32.User32.INSTANCE.SetForegroundWindow(hwnd)
         com.sun.jna.platform.win32.User32.INSTANCE.ShowWindow(hwnd,1)
         com.sun.jna.platform.win32.User32.INSTANCE.SetFocus(hwnd)
         var x = com.sun.jna.platform.win32.User32.INSTANCE.FindWindow(null,"Untitled - Notepad")
         x = com.sun.jna.platform.win32.User32.INSTANCE.FindWindow(null,"Rapid Environment Editor")
         SwitchBackJsonHelper.serializeLocalPointer(x)
         println (s"result on trying to set foreground window : ${res}")
         res
      } .toOption .map(_=>s"""{"result":"success"}""") .getOrElse ("""{"result":"failure"}""")
   }
}

object SwitchBackJnaCoreTest {
   //import yakrider.switchenator.switchback.{SwitchBackJnaCore, SwitchBackJsonHelper} // for interpreter exec
   def testPrintCurTasks() = { SwitchBackJnaCore.getTasksList().foreach(println) }
   def testPrintCurTasksJson() = { SwitchBackJnaCore.getTasksList().map (e => SwitchBackJsonHelper.getAppListsJson(Seq(e))) .foreach(println) }
   def testActivateWinamp() = {
      val winampHwnd = SwitchBackJnaCore .getTasksList .filter(_.getFilePath.toLowerCase.contains("winamp"))
         .headOption .map(_.getHWND) .map(SwitchBackJsonHelper.serializeLocalPointer) ;
      winampHwnd .map (SwitchBackJnaCore.activateApp)
   }
   def testDirHwndStr() = {
      val hwndStr = "2232222"
      SwitchBackJnaCore.activateApp(hwndStr)
   }


}
































