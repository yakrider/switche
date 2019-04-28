package switche


import org.scalajs.dom
import org.scalajs.dom.raw.MouseEvent
import scalatags.JsDom.all._

import scala.scalajs.js
import scala.scalajs.js.Dynamic.{global => g}


object SwitcheFaceConfig {
   def nbsp(n:Int=1) = raw((1 to n).map(i=>"&nbsp;").mkString)
   def clearElem (e:dom.raw.Element) { e.innerHTML = ""}
   def clearedElem (e:dom.raw.Element) = { e.innerHTML = ""; e }
}

object SwitchFacePage {
   
   def getShellPage () = {
      val topRibbon = RibbonDisplay.getTopRibbonDiv()
      val elemsDiv = ElemsDisplay.getElemsDiv
      val page = div (topRibbon, elemsDiv)
      page.render
   }
   //def queueRender() = g.window.requestAnimationFrame({t:js.Any => render()}) // used spaced render call instead
   def render() = { println("rendering")
      val renderList = SwitcheState.getRenderList()
      ElemsDisplay.updateElemsDiv(renderList)
      RibbonDisplay.updateCountsSpan(renderList.size)
   }
}

object ElemsDisplay {
   import SwitcheFaceConfig._
   val elemsDiv = div (id:="elemsDiv").render
   var elemsCount = 0

   def getElemsDiv = elemsDiv
   def makeElemBox (e:WinDatEntry, dimExeSpan:Boolean=false) = {
      val exeSpanClass = s"exeSpan${if (dimExeSpan) " dim" else ""}"
      val exeSpan = span (`class`:=exeSpanClass, e.exePathName.map(_.name).getOrElse("exe..").toString)
      val ico = Some(SwitcheState.okToRenderImages).filter(identity).flatMap(_=> e.exePathName.map(_.fullPath) .map {path =>
         IconsManager.getCachedIcon(path) .map (ico => img(`class`:="ico", src:=s"data:image/png;base64, $ico"))
      }).flatten.getOrElse(span("ico"))
      val icoSpan = span (`class`:="exeIcoSpan", ico)
      val titleSpan = span (`class`:="titleSpan", e.winText.getOrElse("title").toString)
      div (`class`:="elemBox", exeSpan, nbsp(3), icoSpan, nbsp(), titleSpan, onclick:= {ev:MouseEvent => SwitcheState.handleWindowActivationRequest(e.hwnd)})
   }
   def updateElemsDiv (renderList:Seq[WinDatEntry]) = {
      val elemsList = if (SwitcheState.inGroupedMode) {
         //val groupedList = renderList.zipWithIndex.groupBy(_._1.exeName).mapValues(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2._2).map(_._2._1)
         val groupedList = renderList.zipWithIndex.groupBy(_._1.exePathName.map(_.fullPath)).values.map(l => l.map(_._1)->l.map(_._2).min).toSeq.sortBy(_._2).map(_._1)
         groupedList .map (l => Seq ( l.take(1).map(makeElemBox(_,false)), l.tail.map(makeElemBox(_,true)) ).flatten ).flatten
      } else { renderList.map(makeElemBox(_)) }
      clearedElem(elemsDiv) .appendChild (div(elemsList).render)
   }
}

object RibbonDisplay {
   import SwitcheFaceConfig._
   import SwitcheState._
   val countSpan = span (id:="countSpan").render
   def updateCountsSpan (n:Int) = clearedElem(countSpan).appendChild(span(s"($n)").render)
   def getTopRibbonDiv() = {
      val reloadLink = a ( href:="#", "Reload", onclick:={e:MouseEvent => g.window.location.reload()} )
      val refreshLink = a ( href:="#", "Refresh", onclick:={e:MouseEvent => handleRefreshRequest()} )
      val printExclLink = a (href:="#", "ExclPrint", onclick:={e:MouseEvent => handleExclPrintRequest()} )
      val groupModeLink = a (href:="#", "ToggleGrouping", onclick:={e:MouseEvent => handleGroupModeRequest()} )
      div (id:="top-ribbon", reloadLink, nbsp(4), refreshLink, nbsp(4), printExclLink, nbsp(4), countSpan, nbsp(4), groupModeLink).render
   }
}

