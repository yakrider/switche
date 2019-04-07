package tutorial.webapp

import scala.scalajs.js
import scala.scalajs.js.annotation._

@js.native
@JSImport("../../src/main/resources/local-js-test.js", JSImport.Default)
object LocalJs extends js.Object {
   def hello():String = js.native
}

object TutorialApp { 
   def main (args:Array[String]) = {
      println ("tada!!")
      println ("from local js : " + LocalJs.hello())
   }
}

