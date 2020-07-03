package switche


object ExclusionsManager {
   //type ExclFnType = (Int,String,Option[String]) => Boolean
   type ExclFnType = WinDatEntry => Boolean

   def selfSelector (e:WinDatEntry):Boolean = {
      e.exePathName.map(_.name).contains("electron.exe") && e.winText.contains("Sjs-Electron-Local-JS-Test")
   }

   private object RulesExcluder {
      // NOTE: design is for these to return true IF the entries ARE to be excluded
      val exclInvis: ExclFnType = {!_.isVis.getOrElse(false)}
      val exclEmptyTitles: ExclFnType = {_.winText.getOrElse("").isEmpty}

      val titleMatchExclusions = Set[String]()
      val exclTitleMatches: ExclFnType = {_.winText.map(titleMatchExclusions.contains).getOrElse(true)}

      val exeMatchExclusions = Set[String](
         "SearchUI.exe", "shakenMouseEnlarger.exe", "ShellExperienceHost.exe", "LockApp.exe",
         //"MicrosoftEdgeCP.exe", "MicrosoftEdge.exe", // microshit seems to put all these almost-there windows while actual stuff comes under ApplicationFrameHost
         "WindowsInternal.ComposableShell.Experiences.TextInput.InputApp.exe",
         // added the following after updating from win 17763 builds to 19401+ build
         "SearchApp.exe", "StartMenuExperienceHost.exe", "TextInputHost.exe"
      )
      val exclExeMatches: ExclFnType = {_.exePathName.map(_.name).map(exeMatchExclusions.contains).getOrElse(true) }

      val exeAndTitleMatchExclusions = Set[(Option[String],Option[String])] (
         //(Some("ShellExperienceHost.exe"), Some("Start"))
         //(Some("ShellExperienceHost.exe"), Some("Windows Shell Experience Host"))
         //(Some("ShellExperienceHost.exe"), Some("New notification"))
         (Some("explorer.exe"), Some("Program Manager")),
         (Some("explorer.exe"), Some("SubFolderTipWindow")),
         //(Some("electron.exe"), Some("Sjs-Electron-Local-JS-Test")), // covererd separately as self exclusion so externalized configs wont need this
         (Some("atmgr.exe"), Some("FloatActionBar")),
         //(Some("SystemSettings.exe"), Some("Settings")),
         //(Some("ApplicationFrameHost.exe"), Some("Settings")),
         //(Some("ApplicationFrameHost.exe"), Some("Microsoft Edge"))
      )
      val exclExeAndTitleMatches: ExclFnType = {e => exeAndTitleMatchExclusions.contains((e.exePathName.map(_.name),e.winText)) }

      val exclSelf: ExclFnType = {e => selfSelector(e)}

      val exclusions = List[ExclFnType] (
         // NOTE: exclInvis and exclEmptyTitles have also been partially built into upstream processing to eliminate pointless winapi queries etc
         //exclInvis, exclEmptyTitles, exclTitleMatches, exclExeMatches, exclTitleAndExeMatches
         exclInvis, exclEmptyTitles, exclExeMatches, exclExeAndTitleMatches, exclSelf
      )
      //def shouldExclude (e:WinDatEntry) = exclInvis(e) || exclEmptyTitles(e) || exclExeMatches(e) || exclTitleMatches(e) || exclExeAndTitleMatches(e)
      def shouldExclude(e:WinDatEntry) = exclusions.exists(_(e))
   }

   def filterRuleExclusions (wdes:Seq[WinDatEntry]) = wdes.filterNot(RulesExcluder.shouldExclude)

   // though here is many windows apps show up with two entries, one of which is the ApplicationFrameHost.. and mostly share title
   // so if find AFH entry with matching title to something else, prob safe to exclude one of those .. should cut down on a bunch
   def filterApplicationFrameHostDups (wdes:Seq[WinDatEntry]) = {
      //val nonAfhTitleSet = wdes .filterNot(_.exePathName.map(_.name).contains("ApplicationFrameHost.exe")) .map(_.winText) .filter(_.exists(!_.isEmpty)) .toSet
      //wdes .filterNot (e => e.exePathName.map(_.name).contains("ApplicationFrameHost.exe") && nonAfhTitleSet.contains(e.winText))
      // not doing this ^^ as the non-frame-host dups dont seem to respond well to closing etc, while the app-frame-host ones work well
      val afhTitleSet = wdes .filter(_.exePathName.map(_.name).contains("ApplicationFrameHost.exe")) .map(_.winText) .filter(_.exists(!_.isEmpty)) .toSet
      wdes .filterNot (e => !e.exePathName.map(_.name).contains("ApplicationFrameHost.exe") && afhTitleSet.contains(e.winText))
   }

   def filterWinampDups (wdes:Seq[WinDatEntry]) = {
      val winampTitleHwnds = wdes .filter(_.exePathName.map(_.name).contains("winamp.exe")) .map(e => e.winText -> e.hwnd) .groupBy(_._1) .mapValues(_.map(_._2))
      wdes .filterNot (e => e.exePathName.map(_.name).contains("winamp.exe") && winampTitleHwnds.get(e.winText).exists(_.drop(1).contains(e.hwnd)))
   }

   def shouldExclude (e:WinDatEntry, callId:Int = -1) = {
      // call id was used to track dups between same set of winapi streamWindows query, but now we just filter for dups on full render list
      e.shouldExclude .getOrElse ( RulesExcluder.shouldExclude(e) )
   }

   def filterExclusions (wdes:Seq[WinDatEntry]) = {
      Some(wdes) .map (filterRuleExclusions) .map (filterApplicationFrameHostDups) .map (filterWinampDups) .get
   }

}
