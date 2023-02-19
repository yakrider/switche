package switche

import org.scalajs.dom.html.Span
import scalatags.JsDom.all._

case class CheckSearchExeTitleRes (chkPassed:Boolean, exeSpan:Span, ySpan:Span, titleSpan:Span)

object SearchHelper {
   def procSearchStrFragMatch (str:String, frag:String):List[String] = {
      str.replace(frag,s"ØØ¶¶${frag}ØØ").split("ØØ").toList
   }
   def procSearchStrFragsMatch (str:String, frags:Seq[String]): List[String] = {
      val headRes = frags.headOption.map (f => procSearchStrFragMatch(str,f)) .toList .flatten
      if (frags.size==1) headRes else headRes.map {s => procSearchStrFragsMatch(s,frags.drop(1)) }.flatten
   }
   def stringSplits (str:String, idxs:Seq[Int]): List[String] = {
      val headRes = idxs.headOption.map(i=>str.splitAt(i)).map{case(a,b)=>List(a,b)}.toList.flatten
      if (idxs.size<=1) headRes else headRes.headOption.toList ++ headRes.drop(1).map(s=>stringSplits(s,idxs.drop(1))).toList.flatten
   }
   def tagSearchMatches (str:String, chkFrags:Seq[String]):Span = {
      val lowerRes = procSearchStrFragsMatch(str.toLowerCase,chkFrags).filterNot(_.isEmpty)
      val res = stringSplits(str, lowerRes.map(_.replace("¶¶","").size))
      val spans = res.zip(lowerRes).map {case (r,l) => if(l.startsWith("¶¶")) {span(`class`:="searchTxt",r)} else {span(r)} }
      span(spans).render
   }

   // -- relic left behind for possible standalone use reference
   case class CheckSearchTextRes (chkPassed:Boolean, resSpan:Span )
   def checkSearchText (str:String, chkFrags:Seq[String]) = {
      val chkPassed = chkFrags.forall(str.toLowerCase.contains)
      if (chkPassed) { CheckSearchTextRes (true, tagSearchMatches(str,chkFrags))
      } else { CheckSearchTextRes (false, span(str).render) }
   }
   // -- end relic code ---

   def checkSearchExeTitle (exeName:String, title:String, toMatch:String, y:Int) : CheckSearchExeTitleRes = {
      val chkFrags = toMatch.split(" +").filterNot(_.isEmpty).map(_.toLowerCase).toList
      if (chkFrags.isEmpty) return CheckSearchExeTitleRes (true, span(exeName).render, span(y.toString).render, span(title).render)
      // regardless of pass or fail etc, we always highlight matches in both exes and titles, but if chk fails, can bail early
      // for exe match to be true, only one of the terms need to match exe, BUT, any other non-exe match terms must match in title
      // for title match, all of the terms need to match
      val exeMatched = chkFrags .filter (exeName.toLowerCase.contains) .toSet
      val yMatched = chkFrags .filter (f => f == s"${y}i" || f == s"$y") .toSet
      val fullMatched = chkFrags .filterNot(exeMatched.contains) .filterNot(yMatched.contains) .forall(title.toLowerCase.contains)

      if (fullMatched) {
         val exeSpan = Some (exeMatched.toSeq) .filterNot(_.isEmpty) .map (f => tagSearchMatches(exeName,f)) .getOrElse(span(exeName).render)
         val ySpan = Some (yMatched) .filterNot(_.isEmpty) .map (_ => span (`class`:="searchTxt", y.toString).render) .getOrElse(span(s"$y").render)
         val titleSpan = tagSearchMatches (title,chkFrags)
         CheckSearchExeTitleRes (true, exeSpan, ySpan, titleSpan)
      } else {
         CheckSearchExeTitleRes (false, span(exeName).render, span(s"$y").render, span(title).render)
      }
   }
}

