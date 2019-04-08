
import java.nio.charset.Charset
import sbt.Keys._
import sbt.Project.projectToRef

//import org.scalajs.core.tools.io.{IO => toolsIO}

enablePlugins(ScalaJSPlugin)
// Use Node.
scalaJSUseRhino in Global := false

lazy val SjseQuickStart = (project in file(".")). settings(
  name := "scalajs-electron-quick-start",
  version := "1.0.0",
  scalaVersion := "2.11.8",

  resolvers += Resolver.sonatypeRepo("public"),
  libraryDependencies ++= Seq(
    "org.scala-js"  %%% "scalajs-dom"      % "0.9.0",
    "com.mscharley" %%% "scalajs-electron" % "0.1.1",
    "com.mscharley" %%% "scalajs-nodejs"   % "0.1.0"
  )
)



lazy val elPack = taskKey[Unit]("electronPackage/elPack")
lazy val elPackProd = taskKey[Unit]("electronPackage/elPackProd")


lazy val elecProjAlias = SjseQuickStart

lazy val electronPackage = project.in(file("target/electronPackage")) settings(
   name:="electronPackage",
   version:="0.0.1",
   scalaVersion:="2.11.8",
  elPack := {
    val appTarget = (fastOptJS in elecProjAlias in Compile).value
    val resourceDir = (resourceDirectory in elecProjAlias in Compile).value
    // build the electron package in same target parent location
    val electronPackageLoc = new File ((target in elecProjAlias in Compile).value, "electronPackage")
    makeElectronPackage (appTarget, resourceDir, electronPackageLoc)
  },
  elPackProd := {
    val appTarget = (fullOptJS in elecProjAlias in Compile).value
    val resourceDir = (resourceDirectory in elecProjAlias in Compile).value
    // build the electron package in same target parent location
    val electronPackageLoc = new File ((target in elecProjAlias in Compile).value, "electronPackage")
    makeElectronPackage (appTarget, resourceDir, electronPackageLoc)
  }
) dependsOn(elecProjAlias)



def makeElectronPackage (appTarget:Attributed[File], resourcesLoc:File, outTargetLoc:File) = {
  appTarget.map { t =>
    //streams.value.log.info(s"<electronTargeter>: processing file ${t.getName}")
    println(s"<electronTargeter>: processing file ${t.getName}")
    // making all file names uniform either for fast or full opt so the serving html does not have to change
    val (jsInName, jsOutName) = (t.getName, t.getName.replace("-fastopt.js", ".js").replace("-fullopt.js", ".js"))
    val (srcMapInName, srcMapOutName) = (jsInName.+(".map"), jsOutName.+(".map"))
    val depsInName = jsInName.replace("fastopt.js", "jsdeps.js").replace("fullopt.js", "jsdeps.min.js")
    val depsOutName = depsInName.replace("jsdeps.js","deps.js").replace("jsdeps.min.js","deps.js")    
    recursiveCopy (new File(resourcesLoc, "electron"), outTargetLoc)
    recursiveCopy (new File(resourcesLoc, "jslocal"), outTargetLoc)
    recursiveCopy (t, new File (outTargetLoc, jsOutName))
    recursiveCopy (new File (t.getParent, srcMapInName), new File (outTargetLoc, srcMapOutName))
    recursiveCopy (new File (t.getParent, depsInName), new File (outTargetLoc, depsOutName))
  }
}

def recursiveCopy(from: File, to: File): Unit = {
  if (from.isDirectory) {
    to.mkdirs()
    for {
      f ← from.listFiles()
    } recursiveCopy(f, new File(to, f.getName))
  }
  else if (!to.exists() || from.lastModified() > to.lastModified) {
    println(s"Copy file $from to $to ")
    from.getParentFile.mkdirs
    IO.copyFile(from, to, preserveLastModified = true)
  }
}

