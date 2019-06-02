enablePlugins(ScalaJSPlugin, ScalaJSBundlerPlugin)

// project name/version
name := "switchenator-sjse"
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
  "org.scala-js" %%% "scalajs-dom" % "0.9.7",
  "com.lihaoyi" %%% "scalatags" % "0.6.7"
)

// nodejs sources
npmDependencies in Compile ++= Seq(
  // e.g. "snabbdom" -> "0.5.3"
  //"electron" -> "2.0.0", // this gives issues, but should run w/o it in sjs
  "ffi" -> "^2.3.0",
  "ref" -> "^1.3.5",
  "ref-struct" -> "^1.1.0",
  "bmp-js" -> "^0.0.3",
  "jimp" -> "^0.2.27"
)

// root webpack config file
webpackConfigFile := Some(baseDirectory.value / "webpack.config.js")
webpackDevServerExtraArgs in fastOptJS ++= Seq(
  "--content-base", (baseDirectory in ThisBuild).value.getAbsolutePath
)
//version in webpack := "4.8.1"
//version in webpack := "3.3.1"
//version in startWebpackDevServer := "3.1.4"
//version in startWebpackDevServer := "3.3.1"

// optionally use yarn over npm
useYarn := false

// put all js dependencies into a single output file
skip in packageJSDependencies := false

// call the `main` method after the js is loaded
scalaJSUseMainModuleInitializer := true

// do not emit source maps in production
emitSourceMaps in fullOptJS := false

// skip in fastOpt too, seldom used and makes build slow
emitSourceMaps in fastOptJS := false
