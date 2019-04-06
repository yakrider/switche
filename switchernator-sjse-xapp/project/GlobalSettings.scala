import sbt._
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport._

object GlobalSettings {
  val name = "switchenator"
  val organization = "com.yakrider"
  val version = "0.0.1"
  val scalacOptions = Seq( "-unchecked", "-deprecation", "-feature" )
  val Resolvers = Seq(Resolver.sonatypeRepo("snapshots"),
    "Typesafe repository" at "http://repo.typesafe.com/typesafe/releases/"
  )

  /** Declare global dependency versions here to avoid mismatches in multi part dependencies */
  object versions {
    val scala = "2.11.8"
    
    val scalaDom = "0.9.3"
    val scalaJsElectron = "0.1.1"
    val scalaJsNodeJs = "0.1.0"
    
    val scalatags = "0.6.7"
    val upickle = "0.6.6"
    //val scalarx = "0.2.8"
    //val autowire = "0.2.5"
    
    val scalatra = "2.5.3"
    val jetty = "9.4.7.v20170914"
    val jna = "4.5.1"
  }

  /** dependencies shared between JS and JVM projects */
  val sharedDependencies = Def.setting(Seq(
     "com.lihaoyi" %%% "upickle" % versions.upickle,
     //"com.lihaoyi" %%% "scalarx" % versions.scalarx,
     //"com.lihaoyi" %%% "autowire" % versions.autowire,
     "com.lihaoyi" %%% "scalatags" % versions.scalatags
  ))
  
  /** Dependencies only used by the JVM project */
  val jvmDependencies = Def.setting(Seq(     
     "org.scalatra" %%% "scalatra" % versions.scalatra,
     "org.scalatra" %%% "scalatra-scalate" % versions.scalatra,
     "ch.qos.logback" % "logback-classic" % "1.1.5" % "runtime",
     "com.typesafe.scala-logging" %%% "scala-logging" % "3.1.0",
     "org.apache.commons" % "commons-lang3" % "3.3.1",
     "commons-io" % "commons-io" % "2.4",

     "net.java.dev.jna" % "jna" % versions.jna,
     "net.java.dev.jna" % "jna-platform" % versions.jna,     
     
     //"javax.servlet" % "javax.servlet-api" % "3.1.0" % "provided",
     "javax.servlet" % "javax.servlet-api" % "4.0.0" % "container;provided;test",
     //"org.eclipse.jetty" % "jetty-util" % versions.jetty,
     "org.eclipse.jetty" % "jetty-webapp" % versions.jetty % "container;provided",
     "org.eclipse.jetty" % "jetty-server" % versions.jetty
  ))

  /** Dependencies only used by the JS project (note the use of %%% instead of %%) */
  val scalajsDependencies = Def.setting(Seq(     
      "com.mscharley"      %%% "scalajs-electron"     % versions.scalaJsElectron,
      "com.mscharley"      %%% "scalajs-nodejs"       % versions.scalaJsNodeJs,
      "org.scala-js"       %%% "scalajs-dom"          % versions.scalaDom     
      
  ))

  /** Dependencies for external JS libs that are bundled into a single .js file according to dependency order */
  val jsDependencies = Def.setting ( Seq(
      //"org.webjars.npm" % "js-joda" % versions.jsJoda / "dist/js-joda.js" minified "dist/js-joda.min.js"
      //"org.webjars" % "requirejs" % "2.3.6" / "require.js",

      // the following does work, at least minimally as changing the name will complain of js file not found
      // also, this pattern doesnt seem to work above 0.6.17 sbt-scalajs, and def not beyond 0.6xx
      ProvidedJS/"win-helper.js" commonJSName "WinHelper"
  ))
  
}

