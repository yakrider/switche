




import mill._, scalalib._, scalajslib._, scalajslib.api._, scalalib.api._

//import $exec.plugins
//import com.github.lolgab.mill.scalablytyped._

import $file.scalablytyped


object switche extends JavaModule {

   def scalaVersion = "3.2.2" // "2.13.10"

   // override def ivyDeps = Acc("??")

   
   object client extends ScalaJSModule {
     
      def scalaVersion = "3.2.2" // "2.13.10"
      def scalaJSVersion = "1.13.0"
     
      //override def moduleKind = T { ModuleKind.CommonJSModule }
      override def moduleKind = T { ModuleKind.ESModule }
     
      override def ivyDeps = Agg(
         ivy"org.scala-js::scalajs-dom::2.4.0",
         ivy"com.lihaoyi::scalatags::0.12.0",
         ivy"com.lihaoyi::upickle::3.0.0",
         //ivy"com.lihaoyi::ujson::3.0.0"
      )
      
      def moduleDeps = Seq(
         scalablytyped.`scalablytyped-module`
      )
      
      def scalacOptions = Seq (
         "-deprecation",
         "-feature",
         "-unchecked"
      )
     
      def prepare() = T.command {
         os.proc("npm", "install").call()
         ()
      }
      
   }
   
   /* NOTE :
      The following copy fns are only here because the RootModule support in mill isnt released yet
      Once that is out, we can follow the updated examples there that should simplify setting webapp folder like here
    */
   
   
   def copyStatics = T {
      super.resources() .foreach {p =>
         os.copy.over (p.path / "webapp", T.workspace / "out" / "webapp")
      }
      ()
   }
   def copyFastLinkOut = T {
      val jsPath = switche.client.fastLinkJS().dest.path
      val webappPath = T.workspace / "out" / "webapp"
      os.copy.over (jsPath / "main.js", webappPath / "main.js")
      os.copy.over (jsPath / "main.js.map", webappPath / "main.js.map")
      
      //val touchFile = T.workspace / "src-tauri" / "src" / "main.rs"
      //os.proc("touch", touchFile.wrapped.toString).call()
      // ^^ shouldnt need this now that we're using vite for hot-reload dev server
   }
   def webapp = T {
      switche.copyStatics()
      //switche.client.fastLinkJS()
      copyFastLinkOut()
      ()
   }
   
}
