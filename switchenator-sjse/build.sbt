enablePlugins(ScalaJSPlugin, ScalaJSBundlerPlugin)

// project name/version
name := "Sjseleclocaljstest"
version := "0.1.0"

// what version of scala to use
//scalaVersion := "2.11.8"
scalaVersion := "2.12.8"

// compiler flags
scalacOptions ++= Seq(
  "-P:scalajs:sjsDefinedByDefault",
  "-feature"
)

// add repositories to pull from
resolvers ++= Seq(
  //"jitpack" at "https://jitpack.io"
)

// libraries
libraryDependencies ++= Seq(
  "org.scala-js" %%% "scalajs-dom" % "0.9.1",
  "com.lihaoyi" %%% "scalatags" % "0.6.7"
)

// nodejs sources
npmDependencies in Compile ++= Seq(
  // e.g. "snabbdom" -> "0.5.3"
)

// root webpack config file
webpackConfigFile := Some(baseDirectory.value / "webpack.config.js")

// optionally use yarn over npm
useYarn := false

// put all js dependencies into a single output file
skip in packageJSDependencies := false

// call the `main` method after the js is loaded
scalaJSUseMainModuleInitializer := true

// do not emit source maps in production
emitSourceMaps in fullOptJS := false
