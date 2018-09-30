import org.scalatra.LifeCycle
import javax.servlet.ServletContext
import yakrider.switchenator.switchback.SwitchBackWebApp

class ScalatraBootstrap extends LifeCycle {

   override def init(context: ServletContext) {

      // should be a temp thing as this seems to be preventing allow-origin * from being put to headers
      // -- and should not be necessary if client uses proper appUrl instead of trying to use local/lan host, except for workbench
      context.initParameters("org.scalatra.cors.allowCredentials") = "false"

      // this prevents listing of webapp contents outside the '/nuncer' mount point
      context.initParameters("org.eclipse.jetty.servlet.Default.dirAllowed") = "false"

      context mount (new SwitchBackWebApp, "/switchback/*")

  }

}
