Unicode true

!include "MUI2.nsh"

!define APP_NAME "Wireline"
!ifndef APP_VERSION
!define APP_VERSION "0.1.0"
!endif
!define APP_EXE "Wireline.exe"
!define REG_KEY "Software\Wireline"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Wireline"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "..\dist\Wireline-${APP_VERSION}-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${REG_KEY}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ManifestDPIAware true

VIProductVersion "0.1.0.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileVersion" "0.1.0.0"
VIAddVersionKey "FileDescription" "${APP_NAME} desktop client installer"
VIAddVersionKey "LegalCopyright" "Wireline"

!define MUI_ICON "wireline.ico"
!define MUI_UNICON "wireline.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Wireline now"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

LangString DESC_SEC_APP ${LANG_ENGLISH} "Install the Wireline desktop client and its Start Menu shortcut."
LangString DESC_SEC_DESKTOP ${LANG_ENGLISH} "Add a Wireline shortcut to the desktop."

Section "${APP_NAME}" SEC_APP
  SetOutPath "$INSTDIR"
  File /oname=${APP_EXE} "..\target\release\skypeline-client.exe"
  File "wireline.ico"
  File "..\dist\portable\Wireline\README.txt"

  WriteRegStr HKCU "${REG_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\wireline.ico"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\wireline.ico" 0 SW_SHOWNORMAL "" "Launch ${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe" "" "$INSTDIR\wireline.ico" 0 SW_SHOWNORMAL "" "Uninstall ${APP_NAME}"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section /o "Desktop shortcut" SEC_DESKTOP
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\wireline.ico" 0 SW_SHOWNORMAL "" "Launch ${APP_NAME}"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\wireline.ico"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\config.json"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "${REG_KEY}"
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_APP} $(DESC_SEC_APP)
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} $(DESC_SEC_DESKTOP)
!insertmacro MUI_FUNCTION_DESCRIPTION_END
