import scala.scalajs.js

package object nodejs {
  type Require = js.Function1[String, js.Any]
}