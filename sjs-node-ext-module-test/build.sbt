

enablePlugins(ScalaJSPlugin)

name := "sjs node ext module test"
//scalaVersion := "2.12.8"
scalaVersion := "2.11.8"


// indicates this is an app w a main method
scalaJSUseMainModuleInitializer := true

// ECMAScript
//scalaJSLinkerConfig ~= { _.withModuleKind(ModuleKind.ESModule) }
// CommonJS
scalaJSLinkerConfig ~= { _.withModuleKind(ModuleKind.CommonJSModule) }


libraryDependencies += "org.scala-js" %% "scalajs-env-nodejs" % "1.0.0-M6"

//libraryDependencies += "be.doeraene" %%% "scalajs-jquery" % "0.9.1"

skip in packageJSDependencies := false

//jsDependencies += "org.webjars" % "jquery" % "2.2.1" / "jquery.js" minified "jquery.min.js"

//jsDependencies += "org.webjars" % "requirejs" % "2.1.22" / "require.js"




