package electron

import nodejs.Require
import scala.scalajs.js

//import scala.scalajs.js.annotation.JSExportDescendentObjects
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

//@JSExportDescendentObjects
@EnableReflectiveInstantiation
abstract class RendererProcess(require: Require) {
  val rawElectron = require("electron").asInstanceOf[raw.Electron]
  implicit val electron = new Electron(rawElectron)
}
