Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"

!define APP_NAME "Wireline"
!ifndef APP_VERSION
!define APP_VERSION "0.1.0"
!endif
!define DOWNLOAD_URL "https://github.com/skibidiboydawg/wireline/releases/latest/download/Wireline-Setup.exe"

Name "${APP_NAME} Web Installer"
OutFile "..\dist\Wireline-WebSetup.exe"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ManifestDPIAware true

!define MUI_ICON "wireline.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section "Download & Install ${APP_NAME}"
  DetailPrint "Downloading the latest ${APP_NAME} installer..."
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri ''${DOWNLOAD_URL}'' -OutFile ''$TEMP\Wireline-Setup.exe''"'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "Could not download ${APP_NAME} from GitHub Releases.$\r$\nCheck your internet connection or download Wireline-Setup.exe manually."
    Abort
  ${EndIf}

  DetailPrint "Launching ${APP_NAME} setup..."
  ExecWait '"$TEMP\Wireline-Setup.exe"' $1
  Delete "$TEMP\Wireline-Setup.exe"
  ${If} $1 != 0
    SetErrorLevel $1
  ${EndIf}
SectionEnd
