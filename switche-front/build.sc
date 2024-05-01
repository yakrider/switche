




import mill._, scalalib._, scalajslib._, scalajslib.api._, scalalib.api._

//import $exec.plugins
//import com.github.lolgab.mill.scalablytyped._

import $file.scalablytyped


object switche extends JavaModule {
   
   object client extends ScalaJSModule {
     
      //def scalaVersion = "2.13.13"
      def scalaVersion = "3.3.3"
      
      def scalaJSVersion = "1.16.0"
     
      //override def moduleKind = T { ModuleKind.CommonJSModule }
      override def moduleKind = T { ModuleKind.ESModule }
     
      override def ivyDeps = Agg(
         ivy"org.scala-js::scalajs-dom::2.8.0",
         ivy"com.lihaoyi::scalatags::0.13.1",
         ivy"com.lihaoyi::upickle::3.3.0",
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
      Since our backend is a separate (non-scala) tauri app, which we run for dev w vite, we want to prep a webapp folder it can watch
      .. hence the webapp target to copy over of statics and generated js to a serve-ready webapp folder
    */
   
   def webapp = T {
      // the goal here is to have the webapp statics and generated js copied everytime we rebuild/update (as the webapp folder gets cleaned)
      
      // first we'll copy the statics (and so changing things like css will trigger rebuild while watching switche.webapp)
      super.resources() .foreach {p =>
         os.copy.over (p.path / "webapp", T.workspace / "out" / "webapp")
      }
      
      // then we'll copy the generated js (and since it depends on fastLinkJS, it will trigger when any code file changes)
      val jsPath = switche.client.fastLinkJS().dest.path
      val webappPath = T.workspace / "out" / "webapp"
      os.copy.over (jsPath / "main.js", webappPath / "main.js")
      os.copy.over (jsPath / "main.js.map", webappPath / "main.js.map")
      
      //val touchFile = T.workspace / "src-tauri" / "src" / "main.rs"
      //os.proc("touch", touchFile.wrapped.toString).call()
      // ^^ shouldnt need this now that we're using vite for hot-reload dev server (gotta usePolling in watch there)
   }
   
}
