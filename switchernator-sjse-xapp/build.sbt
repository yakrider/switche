import java.nio.charset.Charset
import sbt.Keys._
import sbt.Project.projectToRef
import sbtcrossproject.CrossPlugin.autoImport.{crossProject, CrossType}


scalaVersion := "2.11.8"
scalaVersion in ThisBuild := "2.11.8"


enablePlugins(ScalaJSPlugin)
//scalaJSUseRhino in Global := false //using node

// a special crossProject for configuring a JS/JVM/shared structure
lazy val shared = (crossProject(JSPlatform,JVMPlatform).crossType(CrossType.Pure) in file("switchshared")) .settings (
  scalaVersion := GlobalSettings.versions.scala,
  libraryDependencies ++= GlobalSettings.sharedDependencies.value
)
// set up settings specific to the JS project
//.jsConfigure(_ enablePlugins ScalaJSPlay)


lazy val sharedJVM = shared.jvm.settings(name := "switchenator-sharedJVM")
lazy val sharedJS = shared.js.settings(name := "switchenator-sharedJS")


// instantiate the JVM project for SBT with some additional settings
lazy val switchback = (project in file("switchback"))  .settings( )
/*
lazy val switchback = (project in file("switchback"))  .settings(
    name := "switchback",
    organization := GlobalSettings.organization,
    version := GlobalSettings.version,
    scalaVersion := GlobalSettings.versions.scala,
    scalacOptions ++= GlobalSettings.scalacOptions,
    scalacOptions ++= Seq("-language:reflectiveCalls", "-language:postfixOps"),
    //resolvers in ThisBuild ++= Resolvers,
    libraryDependencies ++= GlobalSettings.sharedDependencies.value,
    libraryDependencies ++= GlobalSettings.jvmDependencies.value,
    
    javacOptions in compile ++= Seq("-target", "8", "-source", "8"),
    packageOptions += Package.MainClass("JettyLauncher"),
    
    containerPort in Jetty := 7090    
)
//.dependsOn(sharedJVM) // disabled until we actually have shared stuff
.enablePlugins (JettyPlugin)
*/

lazy val switchface = (project in file("switchface")) .settings (
    name := "switchface",
    organization := GlobalSettings.organization,
    version := GlobalSettings.version,    
    scalaVersion := GlobalSettings.versions.scala,
    scalacOptions ++= GlobalSettings.scalacOptions,
    scalacOptions += "-P:scalajs:suppressExportDeprecations",
    
    libraryDependencies ++= GlobalSettings.sharedDependencies.value,
    libraryDependencies ++= GlobalSettings.scalajsDependencies.value,    
    jsDependencies ++= GlobalSettings.jsDependencies.value, // disabling this for 1.x builds

    // ECMAScript
    //scalaJSLinkerConfig ~= { _.withModuleKind(ModuleKind.ESModule) },
    // CommonJS
    scalaJSLinkerConfig ~= { _.withModuleKind(ModuleKind.CommonJSModule) },    

    skip in packageJSDependencies := false,   

    resolvers += Resolver.sonatypeRepo("public")

) 
//.dependsOn(sharedJS) // disabled until we have shared stuff
.enablePlugins(ScalaJSPlugin)


//EclipseKeys.skipParents in ThisBuild := false
EclipseKeys.withJavadoc := true
EclipseKeys.withSource := true


lazy val elPack = taskKey[Unit]("electronPackage:elPack")
lazy val elPackProd = taskKey[Unit]("electronPackage:elPackProd")
addCommandAlias("goLiveServer", "; jetty:stop; switchback/compile; jetty:start; jetty:join")


lazy val elecProjAlias = switchface // using alias so can can just update proj name here rather than in code below

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

