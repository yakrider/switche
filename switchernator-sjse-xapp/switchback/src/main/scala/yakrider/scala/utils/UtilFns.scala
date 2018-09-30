package yakrider.scala.utils

import scala.collection.mutable.ListBuffer
import com.typesafe.scalalogging.Logger
import scala.concurrent.Await
import scala.concurrent.Future

object UtilFns {

   import java.io._
   import scala.io._

   import scala.language.implicitConversions
   import scala.language.higherKinds
   import scala.language.reflectiveCalls
   import scala.language.postfixOps

   // NOTE  : breakable should not be used in this utility file, it gets reused by
   // many ppl calling and any utility with breakable doesnt work when nested !!
   //import scala.util.control.Breaks._

   class LazyIfEval  [V, +O1] (v:V, b:Boolean, e1: V => O1) {
      // expected builder sample :
      //def doIf [O] (eChk: V => Boolean) (eOut: V => O) = new LazyIfEval (v, eChk(v), eOut)

      def orElse [O2 >: O1] (e2: V => O2) : O2 = if (b) e1(v) else e2(v)
      def orElseIf [Ox >: O1] (eChk: V => Boolean) (e1New: V => Ox) = {
         if (b) this else new LazyIfEval (v, eChk(v), e1New)
      }
   }

   implicit def boolOptionAdder (b:Boolean) = new {
      def option[A](a: => A): Option[A] = if (b) Some(a) else None
   }

   implicit def universalFunctionsAdder [V] (v:V) = new {

      // SUPER AWESOME GROOVY LIKE WITH FN (sadly, 'with' is reserved, settling for withit)
      // usage : for anything, withit that bugger !! e.g : 9.withit(_*2) or "awe".withit(_+"some")
      def withit [O] (f : V=>O) : O = f(v)


      /* Allows for chained if/else usage. Uses the IfTrue class defined above
       * Sample usage :
       * 5 .doIf (_<0) (_+2) .orElse (_+1)
       * 4 .someIf (_>3) .getOrElse(3)
       * 4 .getIf (_>3) .orElse(_=>3)
       * 4 .doIf (_>3) (_=>1) .orElse (_=>3)
       * List(1,2,3) .map (_ .doIf(_==3)(_+30) .orElseIf(_==1)(_+10) .orElse (_+0) )
      */
      def doIf [O] (eChk: V => Boolean) (eOut: V => O) = new LazyIfEval (v, eChk(v), eOut)
      //def getIf (eChk: V => Boolean) = new LazyIfEval (v, eChk(v), ((_:V)=>v) )


      //def option (f : V=>Boolean) = if (f(v)) Some(v) else None
      def someIf (f : V=>Boolean) = if (f(v)) Some(v) else None
      def noneIf (f : V=>Boolean) = if (!f(v)) Some(v) else None

      def tryTo [O] (f: V=>O) : Option[O] = try { Some(f(v)) } catch { case _:Throwable => None }

      def someIfwTry (f : V=>Boolean) = {
         try { if (f(v)) Some(v) else None }
         catch { case _:Throwable => None }
      }

   }


   // sample usage : (1 to 20) .toSeq .chop {(o,c) => o/3!=c/3}
   implicit def chopAdder [T] (q:Seq[T]) = new {
      def chop (chopIf: (T,T) => Boolean) = {
         if (q.size < 2) Seq(q)
         else q .sliding(2) .foldLeft (ListBuffer(ListBuffer[T](q.head))) { case (a,e) =>
            if ( chopIf(e.head,e.last) ) a .+= ( ListBuffer(e.last) )
            else { a .last .+= (e.last); a }
         } .map (_.toSeq) .toSeq
      }
   }


   implicit def printlnsAdder [A <: {def println(x:Any):Unit}] (a:A) = new {
      def printlns[Any] (b:Any*) = b.foreach(a.println)
   }


   implicit def likeListAdderT2[T, TT <: (T,T)] (t:(T,T)) = new {
      def likeList [B] (f: List[T] => List[B]) : (B,B) = (f(List(t._1,t._2)).withit(l=>(l.head,l.last)))
   }
   implicit def likeListAdderT3[T, TT <: (T,T,T)] (t:(T,T,T)) = new {
      def likeList [B] (f: List[T] => List[B]) : (B,B,B) = (f(List(t._1,t._2,t._3)).withit(l=>(l(0),l(1),l(2))))
   }
   implicit def likeListAdderT4[T, TT <: (T,T,T,T)] (t:(T,T,T,T)) = new {
      def likeList [B] (f: List[T] => List[B]) : (B,B,B,B) = (f(List(t._1,t._2,t._3,t._4)).withit(l=>(l(0),l(1),l(2),l(3))))
   }


   // allow easy way to control parallelism for all parallel constructs
   // usage example : (1 to 8).toVector.par.atPar(2).foreach{i=>println(i);Thread.sleep(1000)}
   import scala.collection.parallel._
   import scala.concurrent.forkjoin.ForkJoinPool
   implicit def atParAdder [V <: {def tasksupport_= (ts:TaskSupport):Unit}] (v:V) = new {
      def atPar (n:Int) : V = {
         v.tasksupport_=(new ForkJoinTaskSupport (new ForkJoinPool(n)))
         v
      }
   }


   // simple implicits to allow flexible use of loggers or println if available
   abstract class LoggerOrPrinter {
      def info (s:String) : Unit;
      def error (s:String) : Unit;
   }

   // use the following directly as an implicit logger no other implicit loggers available via e.g. LazyLogging
   // e.g. : implicit val implicitDummyLogger = UtilFns.dummyLogger
   object dummyLogger extends LoggerOrPrinter {
      // gets used if no implicit logger from LazyLogging
      override def info (s:String) = println (s)
      override def error (s:String) = println (s)
   }
   class CastLogger(logger:Logger) extends LoggerOrPrinter {
      // gets used if implicit logger from LazyLogging can be found
      override def info (s:String) = logger.info(s)
      override def error (s:String) = logger.error(s)
   }
   implicit def toLoggerOrPrinter (logger:Logger) : LoggerOrPrinter = new CastLogger(logger)

   // SLOPPY TRY CATCH WRAPPER : USE VERY SPARINGLY AND ONLY FOR DIRTY WORK
   def quickTry (f: => Unit):Unit = try { f } catch {case _:Throwable => }

   private def twoLines (e:Throwable) = e.toString().split("\n").take(2).mkString("\n")

   // sadly for overloaded fns like below, scala only allows a default value in one of them (!!??), so
   // sadly, means one (w/ default) can be interpreted as-is, other will require defining a dummyLogger
   def printingTry (f: => Unit) (implicit logger:LoggerOrPrinter) : Unit =  {
      try { f } catch {
         case e:Throwable => logger.error ( "Ignoring Exception : " + twoLines(e) )
   } }
   def printingTry (tag:String) (f: => Unit) (implicit logger:LoggerOrPrinter = dummyLogger) : Unit = {
      try { f } catch {
         case e:Throwable => logger.error ( tag + "\n Ignoring Exception : " + twoLines(e) )
   } }

   def tryTo[B] (f: => B) : Option[B] = try{Some(f)} catch{case _:Throwable => None}

   def printingTryTo[B] (f: => B) (implicit logger:LoggerOrPrinter) : Option[B] = {
      try { Some(f) } catch {
         case e:Throwable => {logger.error ( "Ignoring Exception : " + twoLines(e) ); None}
   } }
   def printingTryTo[B] (str:String) (f: => B) (implicit logger:LoggerOrPrinter = dummyLogger) : Option[B] = {
      try { Some(f) } catch {
         case e:Throwable => {logger.error ( str + "\n Ignoring Exception : " + twoLines(e) ); None}
   } }

   def printingLoopingTryTo[B] (tries:Int) (f: => B) (implicit logger:LoggerOrPrinter = dummyLogger) : Option[B] = {
      var out = None:Option[B];
      (1 to tries) .foreach { i =>
         out = printingTryTo ("") (f) (logger);
         if (out.isDefined) return out;
      }
      out
   }
   def printingLoopingTryUntil[B] (tries:Int, cond:B=>Boolean) (f: => B) (implicit logger:LoggerOrPrinter = dummyLogger) : Option[B] = {
      var out = None:Option[B]
      (1 to tries) .foreach { i =>
         out = printingTryTo (s"Attempt $i of $tries:") (f) (logger)
         if (out.isDefined && cond(out.get)==true) {
            //if (i > 1) logger.warn (s"...succeeded only on attempt ${i-1} (out hash : ${out.hashCode})")
            return out;
         }
         if (i == tries) {
            logger.error (s"...failed in all $tries attempts (out hash : ${out.hashCode})")
         } else {
            //logger.info (s"...failed attempt $tries (out hash: ${out.hashCode})")
         }
      }
      out
   }


   def doTimeLimited[B] (errInfo:String, timeLimSecs:Int) (fn : => B) (implicit logger:LoggerOrPrinter = dummyLogger) = {
      import scala.concurrent.ExecutionContext.Implicits.global
      import scala.concurrent.duration._
      printingTryTo (errInfo) { Await.result ( Future(fn), timeLimSecs.seconds ) }
   }


   // GENERIC CLOSE WRAPPER
   def withCloseGuard[A <: {def close():Unit}, B](toGuard:A)(fn: A=>B): B = {
      try { fn(toGuard) } finally { toGuard.close() }
   }

   // CLOSE WRAPPED SCALA BUFFERED SOURCE
   def withBufferedSource[B] (s: BufferedSource) (k: BufferedSource => B) : B = {
      withCloseGuard (s) (k)
   }
   /*def withFileReader[B] (f:String) (k: BufferedSource => B) : B = {
      withCloseGuard ( Source.fromFile(f) ) (k)
   }*/
   def withFileReader[B] (f:String, enc:String="UTF-8") (k: BufferedSource => B) : B = {
      withCloseGuard ( Source.fromFile(new File(f),enc) ) (k)
   }
   /*
   def withBzip2Reader[B] (f:String, enc:String) (k : BufferedSource => B) : B = {
      import org.apache.commons.compress.compressors.bzip2.BZip2CompressorInputStream
      withCloseGuard ( Source.fromInputStream ( new BZip2CompressorInputStream (
            new FileInputStream (f) ), enc ) ) (k)
   }
   def withBzip2PrintWriter[B] (f:String, enc:String="UTF-8") (k: java.io.PrintWriter => B):B = {
      import org.apache.commons.compress.compressors.bzip2.BZip2CompressorOutputStream
      withCloseGuard ( new PrintWriter ( new BufferedWriter ( new OutputStreamWriter (
            new BZip2CompressorOutputStream ( new FileOutputStream (f) ), enc ) ) ) ) (k)
   }
   */
   def withGzipReader[B] (f:String, enc:String="UTF-8") (k : BufferedSource => B) : B = {
      withCloseGuard ( Source.fromInputStream (
            new java.util.zip.GZIPInputStream ( new FileInputStream (f) ), enc ) ) (k)
   }
   def withGzipPrintWriter[B] (f:String, enc:String="UTF-8")(k: java.io.PrintWriter => B):B = {
      withCloseGuard ( new PrintWriter ( new BufferedWriter ( new OutputStreamWriter (
            new java.util.zip.GZIPOutputStream ( new FileOutputStream (f) ), enc ) ) ) ) (k)
   }

   // CLOSE WRAPPED PRINT WRITER FOR FILES
   /*def withPrintWriter[B] (f:String)(k: java.io.PrintWriter => B): B = {
      withCloseGuard (new PrintWriter(new BufferedWriter(new FileWriter(new File(f)))))(k)
   }*/
   def withPrintWriter[B] (f:String, enc:String="UTF-8")(k: java.io.PrintWriter => B): B = {
      withCloseGuard (new PrintWriter(f,enc))(k)
   }
   def withPrintWriterAppending[B] (f:String, enc:String="UTF-8")(k: java.io.PrintWriter => B): B = {
      withCloseGuard (new PrintWriter (new OutputStreamWriter (new FileOutputStream(f,true), enc) ) ) (k)
   }

   // WRITER WITH INCREMENTAL BACKUP FOR FILES
   def withIncBkpWriter[B] (loc:String, tag:String) (k: java.io.PrintWriter => B) : B = {

      val nextFileNum = tryTo ( withFileReader (loc + tag + "-fnum.txt")
            ( _ .getLines .next .toInt ) ) .getOrElse(0) ;

      withPrintWriter ( s"$loc$tag-$nextFileNum.txt", "utf-8" ) ( w => k(w) )

      withPrintWriter (loc + tag + "-fnum.txt") ( _.println (nextFileNum + 1) )

      withPrintWriter (loc + tag + ".txt", "utf-8") ( w => k(w) )
   }



   // CLOSE WRAPPED OBJECT SERIALIZATION WRITER
   def withSerialWriter[B] (f:String) (k: java.io.ObjectOutputStream => B): B = {
      withCloseGuard (new java.io.ObjectOutputStream(new java.io.FileOutputStream(f))) (k)
   }

   // CLOSE WRAPPED OBJECT SERIALIZATION READER
   def withSerialReader[B] (f:String) (k: java.io.ObjectInputStream => B): B = {
      withCloseGuard (new java.io.ObjectInputStream(new java.io.FileInputStream(f))) (k)
   }

   // BASIC WRAPPER TO EXECUTE SYSTEM CALLS
   def exec (cmd:String) = {
      import scala.io.Source
      val proc = Runtime.getRuntime.exec(cmd)
      val procSrc = scala.io.Source.fromInputStream(proc.getInputStream)
      val outputLines = procSrc.getLines.toList
      proc.destroy()
      outputLines
   }

   // SIMPLE FUNCTION TIMER
   def formatMillis (ms:Long) = {
      val mss = ms % 1000
      val secs = (ms / 1000) % 60;
      val mins = (ms / (1000 * 60)) % 60;
      val hrs  = (ms / (1000 * 60 * 60)) % 24;
      val days = (ms / (1000 * 60 * 60 * 24));
      f"$days%dd $hrs%02dh $mins%02dm $secs%02ds $mss%03dms"
   }

   // sadly for overloaded fns like below, scala only allows a default value in one of them (!!??), so
   // sadly, means one (w/ default) can be interpreted as-is, other will require defining a dummyLogger
   def timed[B] (fn: => B) (implicit logger:LoggerOrPrinter) : B =  timed("")(fn)(logger)
   def timed[B] (str:String) (fn: => B) (implicit logger:LoggerOrPrinter = dummyLogger) : B = {
      val stime = System.currentTimeMillis()
      val retval = fn
      logger.info ( s"$str ... took ${formatMillis(System.currentTimeMillis()-stime)}" )
      retval
   }

   // SIMPLE WEB PAGE SCRAPER
   def scrapeUrlToFile ( url:String, dest:String, charset:String )
      (implicit logger:LoggerOrPrinter = dummyLogger) =
   {
      try {
         new File(dest).getParentFile().mkdirs()
         withPrintWriter (dest, charset) { w =>
            Source.fromURL(url, charset) .getLines .foreach ( w.println(_) )
         }
         logger.info (f"Success : scraped $url%s to file $dest%s")
      } catch { case e:Exception =>
         logger.info (f"Error : failed scraping $url%s to file $dest%s\n" + e )
      }
   }

   // ARCHIVE EXTRACTOR WRAPPER
   //def unzip ( loc:String ):Unit = ArchiveExtractor.unzip( loc )

   // WEB DOWNLOADER WRAPPER
   //def download ( url:String, dest:String ) = WebDownloader.download (url, dest)

   def download ( url:String, dest:String, silent:Boolean=true )
      (implicit logger:LoggerOrPrinter = dummyLogger ) : Unit =
   {
      val rbc = java.nio.channels.Channels.newChannel(new java.net.URL(url).openStream())
      val fos = new java.io.FileOutputStream(dest)
      if (!silent) logger.info ("Starting to download from " + url)
      fos.getChannel().transferFrom(rbc, 0, 1L << 36) // upto 8GB
      rbc.close()
      fos.close()
      if (!silent) logger.info ("Download completed and written to " + dest)
   }


}
