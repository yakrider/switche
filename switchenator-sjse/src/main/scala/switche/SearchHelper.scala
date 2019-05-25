package switche

import org.scalajs.dom.html.Span
import scalatags.JsDom.all._

case class CheckSearchExeTitleRes (chkPassed:Boolean, exeSpan:Span, titleSpan:Span)

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
      val spans = res.zip(lowerRes).map {case (r,l) => if(l.startsWith("¶¶")) {span(`class`:="mtxt",r)} else {span(r)} }
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
   
   def checkSearchExeTitle (exeName:String, title:String, toMatch:String) : CheckSearchExeTitleRes = {
      val chkFrags = toMatch.split(" +").filterNot(_.isEmpty).map(_.toLowerCase).toList
      if (chkFrags.isEmpty) return CheckSearchExeTitleRes (true, span(exeName).render, span(title).render)
      // regardless of pass or fail etc, we always highlight matches in both exes and titles, but if chk fails, can bail early
      // for exe match to be true, only one of the terms need to match exe, BUT, any other non-exe match terms must match in title
      // for title match, all of the terms need to match
      val exeLower = exeName.toLowerCase(); val titleLower = title.toLowerCase()
      val exeMatchChecks = chkFrags.map(f => f -> exeLower.contains(f))
      val didExeTitleMatch = exeMatchChecks.filterNot(_._2).map(_._1).forall(titleLower.contains)
      
      if (didExeTitleMatch) {
         val exeSpan = Some(exeMatchChecks.filter(_._2).map(_._1)).filterNot(_.isEmpty).map(f => tagSearchMatches(exeName,f)).getOrElse(span(exeName).render)
         val titleSpan = tagSearchMatches (title,chkFrags)
         CheckSearchExeTitleRes (true, exeSpan, titleSpan)
      } else {
         CheckSearchExeTitleRes (false, span(exeName).render, span(title).render)
      }
   }
}

