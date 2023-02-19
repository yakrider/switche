package switche


object ExclusionsManager {
   //type ExclFnType = (Int,String,Option[String]) => Boolean
   type ExclFnType = WinDatEntry => Boolean

   def selfSelector (e:WinDatEntry):Boolean = {
      e.exePathName.map(_.name).contains("electron.exe") && e.winText.contains("Switche - Searchable Task Switcher")
   }

   private object RulesExcluder {
      // NOTE: design is for these to return true IF the entries ARE to be excluded
      val exclInvis: ExclFnType = {!_.isVis.getOrElse(false)}
      val exclCloaked: ExclFnType = {!_.isUnCloaked.getOrElse(false)}
      val exclEmptyTitles: ExclFnType = {_.winText.getOrElse("").isEmpty}

      val titleMatchExclusions = Set[String]()
      val exclTitleMatches: ExclFnType = {_.winText.map(titleMatchExclusions.contains).getOrElse(true)}

      val exeMatchExclusions = Set[String](
         "SearchUI.exe", "shakenMouseEnlarger.exe", "ShellExperienceHost.exe", "LockApp.exe",
         //"MicrosoftEdgeCP.exe", "MicrosoftEdge.exe", // microshit seems to put all these almost-there windows while actual stuff comes under ApplicationFrameHost
         "WindowsInternal.ComposableShell.Experiences.TextInput.InputApp.exe",
         // added the following after updating from win 17763 builds to 19401+ build
         "SearchApp.exe", "StartMenuExperienceHost.exe", "TextInputHost.exe",
         // had to add following after activating wireless display adapter to cast/project to/from TVs/Android etc
         "WDADesktopService.exe"
      )
      val exclExeMatches: ExclFnType = {_.exePathName.map(_.name).map(exeMatchExclusions.contains).getOrElse(true) }

      val exeAndTitleMatchExclusions = Map[String, Set[String]] ( // exact exe-name and full-title matches
         "ShellExperienceHost.exe" -> Set ("Start", "Windows Shell Experience Host", "New notification"),
         "explorer.exe" -> Set ("Program Manager", "SubFolderTipWindow"),
         //"electron.exe" -> Set ("Switche - Searchable Task Switcher"), // covererd separately as self exclusion so externalized configs wont need this
         "atmgr.exe" -> Set ("FloatActionBar")
         //"SystemSettings.exe" -> Set("Settings"),
         //"ApplicationFrameHost.exe" -> Set("Settings", "Microsoft Edge"),
      )
      val exclExeAndTitleMatches: ExclFnType = { e =>
         e.exePathName.map(_.name) .flatMap(exeAndTitleMatchExclusions.get) .exists (s => e.winText.exists(s.contains))
      }

      val exeAndPartTitleMatchExclusions = Map[String,Set[String]] ( // exact exe-name and partial-title matches
         "Dimmer.exe" -> Set("#"),
      )
      val exclExeAndPartTitleMatches: ExclFnType = { e =>
         e.exePathName.map(_.name) .flatMap(exeAndPartTitleMatchExclusions.get) .exists (s => e.winText.exists(wt => s.exists(wt.contains)))
      }

      val exclSelf: ExclFnType = {e => selfSelector(e)}

      val exclusions = List[ExclFnType] (
         // NOTE: exclInvis and exclEmptyTitles have also been partially built into upstream processing to eliminate pointless winapi queries etc
         //exclInvis, exclEmptyTitles, exclTitleMatches, exclExeMatches, exclTitleAndExeMatches
         exclInvis, exclCloaked, exclEmptyTitles, exclExeMatches, exclExeAndTitleMatches, exclExeAndPartTitleMatches, exclSelf
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




object IconPathOverridesManager {

   type IconOvFnType = WinDatEntry => Option[String]

   private object IconPathOverrider {

      val icOvCacheLoc = "C:\\yakdat\\misc\\switche-icons-override-cache"
      val exeMatchOverrides = Map[String,String] (
         "krustyboard.exe" -> s"$icOvCacheLoc\\krustyboard_logo-512.ico"
      )
      val iconOvs_ExeMatches: IconOvFnType = {_.exePathName.map(_.name).flatMap(exeMatchOverrides.get) }

      val exeAndTitleMatchOverrides = Map[Tuple2[String,String],String] (// mapping is : Map[(exePath, title) -> iconPath]
         ("ApplicationFrameHost.exe" -> "Settings") -> s"$icOvCacheLoc\\SystemSettings_logo.scale-400.ico"
      )
      val iconOvs_ExeAndTitleMatches: IconOvFnType = { e =>
         e.exePathName.map(_.name) .flatMap(n => e.winText.map(n -> _)) .flatMap(exeAndTitleMatchOverrides.get)
      }

      val exeAndPartialTitleMatchOverrides = Map[String, Map[String,String]] ( // Map[exePath -> Map[(part-title, iconPath)..] ..]
         "ApplicationFrameHost.exe" -> Map (
            "Drawboard PDF" -> s"$icOvCacheLoc\\drawboardPDF_Logo.targetsize-256_altform-unplated.ico",
            "Graphics Command Center" -> s"$icOvCacheLoc\\igcc_StoreLogo.scale-400.ico" )
      )
      val iconOvs_ExeAndPartTitleMatches: IconOvFnType = { e =>
         e.exePathName.map(_.name) .flatMap(exeAndPartialTitleMatchOverrides.get)
            .flatMap ( _.find { case (k,v) => e.winText.exists(_.contains(k)) } .map(_._2) )
      }

      val iconOvFns = List[IconOvFnType] (
         iconOvs_ExeMatches, iconOvs_ExeAndTitleMatches, iconOvs_ExeAndPartTitleMatches
      )

      def getIconOverridePath (e:WinDatEntry) : Option[String] = {
         iconOvFns.flatMap(_(e)).headOption
      }
   }
   def getIconOverridePath (e:WinDatEntry) = IconPathOverrider.getIconOverridePath(e)

}


