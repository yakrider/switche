package switche


object ExclusionsManager {
   //type ExclFnType = (Int,String,Option[String]) => Boolean
   type ExclFnType = WinDatEntry => Boolean
   
   private object RulesExcluder {
      // NOTE: design is for these to return true IF the entries ARE to be excluded
      val exclInvis: ExclFnType = {!_.isVis.getOrElse(false)}
      val exclEmptyTitles: ExclFnType = {_.winText.getOrElse("").isEmpty}
      
      val titleMatchExclusions = Set[String]()
      val exclTitleMatches: ExclFnType = {_.winText.map(titleMatchExclusions.contains).getOrElse(true)}
      
      val exeMatchExclusions = Set[String](
         "SearchUI.exe", "shakenMouseEnlarger.exe", "ShellExperienceHost.exe", "LockApp.exe",
         "MicrosoftEdgeCP.exe", "MicrosoftEdge.exe", // microshit seems to put all these almost-there windows while actual stuff comes under ApplicationFrameHost
         "WindowsInternal.ComposableShell.Experiences.TextInput.InputApp.exe"
      )
      val exclExeMatches: ExclFnType = {_.exePathName.map(_.name).map(exeMatchExclusions.contains).getOrElse(true) }
      
      val exeAndTitleMatchExclusions = Set[(Option[String],Option[String])] (
         //(Some("ShellExperienceHost.exe"), Some("Start"))
         //(Some("ShellExperienceHost.exe"), Some("Windows Shell Experience Host"))
         //(Some("ShellExperienceHost.exe"), Some("New notification"))
         (Some("explorer.exe"), Some("Program Manager")),
         (Some("explorer.exe"), Some("SubFolderTipWindow")),
         (Some("electron.exe"), Some("Sjs-Electron-Local-JS-Test")),
         (Some("atmgr.exe"), Some("FloatActionBar")),
         (Some("SystemSettings.exe"), Some("Settings")),
         (Some("ApplicationFrameHost.exe"), Some("Microsoft Edge"))
      )
      val exclExeAndTitleMatches: ExclFnType = {e => exeAndTitleMatchExclusions.contains((e.exePathName.map(_.name),e.winText)) }
      
      val exclusions = List[ExclFnType] (
         // NOTE: exclInvis and exclEmptyTitles have also been partially built into upstream processing to eliminate pointless winapi queries etc
         //exclInvis, exclEmptyTitles, exclTitleMatches, exclExeMatches, exclTitleAndExeMatches
         exclInvis, exclEmptyTitles, exclExeMatches, exclExeAndTitleMatches
      )
      //def shouldExclude (e:WinDatEntry) = exclInvis(e) || exclEmptyTitles(e) || exclExeMatches(e) || exclTitleMatches(e) || exclExeAndTitleMatches(e)
      def shouldExclude(e:WinDatEntry) = exclusions.exists(_(e))
   }
   object WinampDupExcluder {
      var curCallId = -1; var alreadySeen = false;
      def exclWinampDup (e:WinDatEntry, callId:Int): Boolean = { //println("winamp checked!")
         var shouldExclude = false
         if (callId < 0) {return false} // exclusions for listener based updates
         if (curCallId != callId) { curCallId = callId; alreadySeen = false } // reset upon new callId
         if (e.exePathName.map(_.name).map(_=="winamp.exe").getOrElse(false)) { shouldExclude = alreadySeen; alreadySeen = true } // update if see winamp
         shouldExclude
      }
   }
   def shouldExclude (e:WinDatEntry, callId:Int = -1) = {
      // if value already calculated, use that, else check other excluders in a short-circuiting manner
      e.shouldExclude.getOrElse ( RulesExcluder.shouldExclude(e) || WinampDupExcluder.exclWinampDup(e,callId) )
   }
   def selfSelector (e:WinDatEntry):Boolean = {
      e.exePathName.map(_.name).contains("electron.exe") && e.winText.contains("Sjs-Electron-Local-JS-Test")
   }
   
}
