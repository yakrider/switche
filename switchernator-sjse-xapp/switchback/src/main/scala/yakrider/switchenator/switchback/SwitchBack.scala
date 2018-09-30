package yakrider.switchenator.switchback


import org.scalatra.scalate.ScalateSupport
import org.scalatra.CorsSupport
import com.typesafe.scalalogging.LazyLogging
import yakrider.scala.utils.UtilFns._
import org.apache.commons.lang3.time.FastDateFormat
import org.scalatra.ScalatraServlet
import org.scalatra.ContentEncodingSupport

import scala.collection.{mutable => mut}
import scala.util.Try

//import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.concurrent.Await

import scalatags.Text.all._
import scalatags.Text.{all => tags}
import javax.servlet.http.HttpServletRequest
import org.scalatra.SweetCookies


class SwitchBackWebApp extends ScalatraServlet with ScalateSupport with CorsSupport with ContentEncodingSupport with LazyLogging {

   // allowing cross origin access as we want to trigger things from generated static sub-pages
   options("/*"){
      response.setHeader("Access-Control-Allow-Headers", request.getHeader("Access-Control-Request-Headers"));
   }

   error {
      case e: Throwable => {
         Console.err.println (e.toString())
         e.printStackTrace()
         //redirect("/") --> cant do redirects etc.. could be async
         e
      }
   }

   // define available route strings upfront to avoid replication, inaccuracies etc
   val ROOT = "/"
   val ASYNC = "/api/v0"

   def wrappingInfoPrint[B] (fn: => B) = { println (s"\n@${"curStamp-unavailable"} $request}"); fn }

   get (ROOT) { wrappingInfoPrint {
    contentType = "text/json"
    "{err: No html support. Try /api/v0/ for api access}"
   } }

   get (ASYNC) (asyncHandler)
   post (ASYNC) (asyncHandler)

   def asyncHandler = {
      val cmdOpt = params.get("cmd")
      val isJsonCmd = cmdOpt.map(_.startsWith("json")).getOrElse(false)
      contentType = if (isJsonCmd) "application/json" else "text/html"
      val result = cmdOpt .map (runAsyncCmd) .getOrElse(s"Empty cmd : $cmdOpt")
      result
   }

   val runCmdsLock = new Object()

   def runCmd (cmdOpt:Option[String]) = { runCmdsLock.synchronized {
      // run from ScrapeData because that is better set up to handle logging / thread safety etc
      //ScrapeData.runCmd (cmdOpt.getOrElse(""))
   } }

   def jsonError (msg:String) = s"""{"status":500, "message":"$msg"}"""

   import yakrider.switchenator.switchback.{SwitchBackService => sv}
   def runAsyncCmd (cmd:String) = { runCmdsLock.synchronized {
      cmd match {
         case _ => { s"""{"status":200, "response":"Unsupported cmd $cmd}"""" }
      }
   } }

}

object SwitchBackService extends LazyLogging {
   // nothing yet
}















































