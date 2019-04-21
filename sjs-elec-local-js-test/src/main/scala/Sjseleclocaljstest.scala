
import scala.scalajs.js
import scala.scalajs.js.DynamicImplicits._
import scala.scalajs.js.Dynamic.{global => g, literal => JsObject}
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel, JSGlobal, JSImport}
import org.scalajs.dom
import scalatags.JsDom.all._

object Sjseleclocaljstest extends js.JSApp {
    def main(): Unit = {
        println("Hello from sjseApp..")
        g.console.log("uhh, from sjse main for now...")
        //g.console.log(g.document)
        //g.console.log(g.document.getElementById("scala-js-root-div"))
        //g.console.log(SwitchFacePage.getShellPage())
        g.document.getElementById("scala-js-root-div").appendChild (SwitchFacePage.getShellPage())
    }
}


@JSExportTopLevel("SwitchFacePage")
object SwitchFacePage {
   def getHelloOutput() = {
      s"Hello! from node ${g.process.versions.node}, Chromium ${g.process.versions.chrome}, and Electron ${g.process.versions.electron}."
   }
   def getShellPage () = {
      val helloOutDiv = div (getHelloOutput)
      //val testOutDiv = div ( getTestOutput )
      //val page = div ( helloOutDiv, br, testOutDiv, br, s" **done!** " ).render
      val page = div ( helloOutDiv, br, s" **done!** " ).render
      //val page = div ("dammit, yet another hello, from shell page").render
      page
   }
   //def getTestOutput() = { div ( testOutListFiles, testGetActiveWindow ) }
   //def testOutListFiles() = SwitchNativeTests.listFiles(".").map(div(_))
   //def testGetActiveWindow() = div ( SwitchNativeTests.getActiveWindow().toString )
}


/*
object SwitchNativeTests {


  def runQuickTestCode(app:SwitchFaceApp) = {
    g.console.log("uhh, from SwitchNativeTests for now..")
    g.console.log("node version:", g.process.versions.node)
    g.console.log("chrome version:",  g.process.versions.chrome)
    g.console.log("electron version:", g.process.versions.electron)
    //g.console.log("g is:", g) // will print out a whole pile btw!
    //g.console.log("g.WinHelper is:", g.WinHelper) // says undefined
    //g.console.log("WinHelper is:", WinHelper) //wont compile
    //g.console.log("hello is:", g.hello) // says undefined
    //println(s"WinHelper from println is: ${WinHelper}")
    //println(g)
    //println(WinHelper)
    //println(hello)
    //println (hello)
    //println(s"hello from println is: ${hello}")
    //println(s"hello from println is: ${hello.hello}")
    //println(s"WinHelper.hello() from println is : ${WinHelper.hello()}") // throws error if WinHelper is undefined
    //g.console.log(WinHelper.hello())
    //g.console.log(g.require("WinHelper"))
    //g.console.log(g.require("win-helper"))

    app.quit()
  }
   //val fs = g.require("fs")
   //val wa = g.require ("win32-api")
   //g.require ("win32-api")


   //val u32 = win32api.U.load()
   //val u32 = g.User32.load()
   //val u32 = g.User32
   //g.console.log(u32)
   //g.console.log(g.require("fs"))
   //g.console.log(g.require("win32-api"))

   //val locJsTest = g.require("win-helper")
   //object WinApiHelper { val winHelper = js.Dynamic.global.require("win-helper") }
   //object WinApiHelper { val winHelper = js.Dynamic.global.require("./win-helper.js") }
   //g.console.log(g.hello)
   //import WinHelper._
   //g.console.log(g.WinHelper.hello)
   //g.console.log(WinHelper.hello)
   //g.console.log(g.require("win-helper").hello)
   //g.console.log(g.require("win-helper").hello)
   //g.console.log(g.require("./win-helper").hello)
   //g.console.log(g.require("win-helper.js").hello)
   //g.console.log(g.require("./win-helper.js").hello)


   //def getActiveWindow() = u32.GetActiveWindow()}
   def getActiveWindow() = {"disabled"}//u32.GetActiveWindow()}

   //def listFiles(path:String):Seq[String] = {fs.readdirSync(path).asInstanceOf[js.Array[String]]}
   def listFiles(path:String):Seq[String] = Seq("disabled") // {fs.readdirSync(path).asInstanceOf[js.Array[String]]}

}
*/




