; Switche NSIS installer
;
; per-user install (no admin prompt) into %LOCALAPPDATA%\Switche, which is the same
; location the old Tauri NSIS installer used .. upgrading in place keeps the user's
; switche.conf.toml and logs, both of which live next to the exe
;
; built via installer\build-installer.bat, which passes:
;   /DAPP_VERSION=x.y.z /DAPP_SIZE_KB=n /DSRC_EXE=.. /DICON=.. /DOUTFILE=..
; optionally /DPRODUCT=Name to build a side-by-side test variant

Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef APP_VERSION
  !error "APP_VERSION must be defined (e.g. /DAPP_VERSION=4.0.0)"
!endif
!ifndef APP_SIZE_KB
  !define APP_SIZE_KB 0
!endif
!ifndef SRC_EXE
  !error "SRC_EXE must be defined"
!endif
!ifndef ICON
  !error "ICON must be defined"
!endif
!ifndef OUTFILE
  !error "OUTFILE must be defined"
!endif
!ifndef PRODUCT
  !define PRODUCT "Switche"
!endif
!define PUBLISHER "yakrider"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT}"

Name "${PRODUCT}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\${PRODUCT}"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${PRODUCT} ${APP_VERSION}"

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName"     "${PRODUCT}"
VIAddVersionKey "FileDescription" "${PRODUCT} Setup"
VIAddVersionKey "FileVersion"     "${APP_VERSION}"
VIAddVersionKey "ProductVersion"  "${APP_VERSION}"
VIAddVersionKey "LegalCopyright"  "Copyright (C) 2024 yakrider"
VIAddVersionKey "CompanyName"     "${PUBLISHER}"

!define MUI_ICON   "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN          "$INSTDIR\Switche.exe"
!define MUI_FINISHPAGE_RUN_TEXT     "Launch ${PRODUCT} now"
!define MUI_FINISHPAGE_RUN_NOTCHECKED

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"


; closes any running instance, and aborts if one survives (e.g. an elevated instance
; .. that a per-user installer can neither kill nor overwrite)
!macro StopRunningInstance
  nsExec::ExecToLog 'taskkill /IM Switche.exe /F'
  Pop $0
  Sleep 700
  nsExec::ExecToStack 'cmd /C tasklist /FI "IMAGENAME eq Switche.exe" /NH | find /I "Switche.exe"'
  Pop $0
  Pop $1
  ${If} $0 == 0
    MessageBox MB_OK|MB_ICONEXCLAMATION "Switche is still running (it may be running elevated).$\r$\nPlease exit it and run this again."
    Abort
  ${EndIf}
!macroend


Section "!${PRODUCT} (required)" SEC_MAIN
  SectionIn RO

  !insertmacro StopRunningInstance

  SetOutPath "$INSTDIR"
  File "${SRC_EXE}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS"
  CreateShortCut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\Switche.exe" "" "$INSTDIR\Switche.exe" 0

  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName"     "${PRODUCT}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion"  "${APP_VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher"       "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon"     '"$INSTDIR\Switche.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "${UNINST_KEY}" "MainBinaryName"  "Switche.exe"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify"  1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair"  1
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" ${APP_SIZE_KB}
SectionEnd

Section "Desktop shortcut" SEC_DESKTOP
  SetShellVarContext current
  CreateShortCut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\Switche.exe" "" "$INSTDIR\Switche.exe" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_MAIN}    "The ${PRODUCT} application and its Start Menu shortcut."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} "Also place a shortcut on the desktop."
!insertmacro MUI_FUNCTION_DESCRIPTION_END


Section "Uninstall"
  !insertmacro StopRunningInstance

  SetShellVarContext current
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  Delete "$DESKTOP\${PRODUCT}.lnk"
  Delete "$INSTDIR\Switche.exe"
  Delete "$INSTDIR\uninstall.exe"

  ; deliberately keep switche.conf.toml and the log files so a reinstall keeps settings
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINST_KEY}"
SectionEnd
