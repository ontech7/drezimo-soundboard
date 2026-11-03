; Tauri's NSIS template (tauri-cli 2.12.4) with Drezimo's own interface: one dark window instead of the wizard pages.
; The install, update and uninstall logic is Tauri's; keep it in step when Tauri's template changes.
Unicode true
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
XPStyle on

!if "{{compression}}" == "none"
  SetCompress off
!else
  SetCompressor /SOLID "{{compression}}"
!endif

{{#if signed_plugins_path}}
!addplugindir "{{signed_plugins_path}}"
{{/if}}

!include LogicLib.nsh
!include nsDialogs.nsh
!include WinMessages.nsh
!include FileFunc.nsh
!include x64.nsh
!include WordFunc.nsh
!include "utils.nsh"
!include "FileAssociation.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "Win\RestartManager.nsh"

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

!define MANUFACTURER "{{manufacturer}}"
!define PRODUCTNAME "{{product_name}}"
!define VERSION "{{version}}"
!define VERSIONWITHBUILD "{{version_with_build}}"
!define HOMEPAGE "{{homepage}}"
!define INSTALLMODE "{{install_mode}}"
!define INSTALLERICON "{{installer_icon}}"
!define UNINSTALLERICON "{{uninstaller_icon}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define ADDITIONALPLUGINSPATH "{{additional_plugins_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define MINIMUMWEBVIEW2VERSION "{{minimum_webview2_version}}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define UNINSTALLERSIGNCOMMAND "{{uninstaller_sign_cmd}}"
!define ESTIMATEDSIZE "{{estimated_size}}"

!if "${INSTALLMODE}" != "currentUser"
  !error "Drezimo's installer is per user: its updates must never ask for administrator rights"
!endif
!if "${ALLOWDOWNGRADES}" == "false"
  !error "The installer replaces a newer version in place: downgrades must stay allowed"
!endif
!if "{{license}}{{start_menu_folder}}{{sidebar_image}}{{header_image}}{{uninstaller_header_image}}" != ""
  !error "The installer has no wizard: no licence page, Start menu folder or wizard images"
!endif
!if "{{display_language_selector}}" == "true"
  !error "The installer follows the system's language: it has no language selector"
!endif
RequestExecutionLevel user

Var PassiveMode
Var UpdateMode
Var NoShortcutMode
Var OldMainBinaryName
; The version already installed, and how this one compares to it: 1 newer, 0 the same, -1 older
Var InstalledVersion
Var Comparison
Var DeleteAppDataState
; The install page's options, 1 when ticked: as things are now, or all on for a new installation
Var DesktopShortcut
Var StartMenuShortcut
Var StartAtLogin
Var OpenWhenDone

Name "${PRODUCTNAME}"
Caption "${PRODUCTNAME}"
UninstallCaption "${PRODUCTNAME}"
SubCaption 3 " "
SubCaption 4 " "
UninstallSubCaption 1 " "
UninstallSubCaption 2 " "
BrandingText " "
OutFile "${OUTFILE}"
ShowInstDetails nevershow
ShowUninstDetails nevershow
InstProgressFlags smooth
AutoCloseWindow true

; Only so NSIS appends the product name to a chosen folder; .onInit sets the real default
!define PLACEHOLDER_INSTALL_DIR "placeholder\${PRODUCTNAME}"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"

VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

!addplugindir "${ADDITIONALPLUGINSPATH}"

!if "${UNINSTALLERSIGNCOMMAND}" != ""
  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}'
!endif

!if "${INSTALLERICON}" != ""
  Icon "${INSTALLERICON}"
!endif
!if "${UNINSTALLERICON}" != ""
  UninstallIcon "${UNINSTALLERICON}"
!endif

{{#each languages}}
LoadLanguageFile "${NSISDIR}\Contrib\Language files\\{{this}}.nlf"
{{/each}}
{{#each language_files}}
!include "{{this}}"
{{/each}}
!include /CHARSET=UTF8 "${DREZIMO_WINDOWS}\installer\texts.nsh"

; The interface: one window in the app's colours, sized in pixels at 96 DPI and scaled to the screen
!define BACKGROUND 0x0E0E10
!define TEXT_COLOR 0xF4F3EF
!define MUTED_COLOR 0x8B8B94
!define AMBER 0xFFB020
!define CARD 0x17171B
!define WINDOW_WIDTH 480
!define WINDOW_HEIGHT 340
!define MARGIN 32
!define TEXT_WIDTH 416
!define BUTTON_TOP 268
!define BUTTON_HEIGHT 40
; WS_CHILD | WS_VISIBLE, with SS_NOPREFIX for text (SS_CENTERIMAGE to centre a line vertically), SS_BITMAP for the
; logo, and centred and clickable (SS_NOTIFY) for buttons
!define TEXT_STYLE 0x50000080
!define LINE_STYLE 0x50000280
!define LOGO_STYLE 0x5000000E
!define BUTTON_STYLE 0x50000381
!define TOGGLE_STYLE 0x50000380
; WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX: Windows' own checkbox, its box drawn at the system's size
!define CHECKBOX_STYLE 0x50010003
!define /math PROGRESS_TOP ${BUTTON_TOP} + 18
!define OPTIONS_TOP 192
!define OPTION_HEIGHT 28
!define /math OPTION_WIDTH ${TEXT_WIDTH} / 2
!define /math OPTION_RIGHT ${MARGIN} + ${OPTION_WIDTH}
; The checkbox, then its label
!define BOX_WIDTH 22

Var Dpi
Var TitleFont
Var BodyFont
Var ButtonFont
Var SmallFont
Var DeleteAppDataBox
Var DeleteAppDataToggle
Var DesktopBox
Var DesktopToggle
Var StartMenuBox
Var StartMenuToggle
Var LoginBox
Var LoginToggle
Var OpenBox
Var OpenToggle

!macro Px var value
  IntOp ${var} ${value} * $Dpi
  IntOp ${var} ${var} / 96
!macroend

; A label on `parent` showing $R5
!macro Label parent style x y w h font color out
  !insertmacro Px $R1 ${x}
  !insertmacro Px $R2 ${y}
  !insertmacro Px $R3 ${w}
  !insertmacro Px $R4 ${h}
  System::Call 'user32::CreateWindowExW(i 0, w "STATIC", w R5, i ${style}, i R1, i R2, i R3, i R4, p ${parent}, p 0, p 0, p 0) p .s'
  Pop ${out}
  SendMessage ${out} ${WM_SETFONT} ${font} 1
  SetCtlColors ${out} ${color} ${BACKGROUND}
!macroend

; A flat button on the current nsDialogs page showing $R5, with rounded corners
!macro Button x w text_color background on_click out
  !insertmacro Px $R1 ${x}
  !insertmacro Px $R2 ${BUTTON_TOP}
  !insertmacro Px $R3 ${w}
  !insertmacro Px $R4 ${BUTTON_HEIGHT}
  nsDialogs::CreateControl STATIC ${BUTTON_STYLE} 0 $R1 $R2 $R3 $R4 $R5
  Pop ${out}
  SendMessage ${out} ${WM_SETFONT} $ButtonFont 1
  SetCtlColors ${out} ${text_color} ${background}
  !insertmacro Px $R6 8
  IntOp $R3 $R3 + 1
  IntOp $R4 $R4 + 1
  System::Call 'gdi32::CreateRoundRectRgn(i 0, i 0, i R3, i R4, i R6, i R6) p .r9'
  System::Call 'user32::SetWindowRgn(p ${out}, p r9, i 1)'
  ${NSD_OnClick} ${out} ${on_click}
!macroend

; An option's label, brighter while it is ticked
!macro ShowOption label state
  ${If} ${state} = 1
    SetCtlColors ${label} ${TEXT_COLOR} ${BACKGROUND}
  ${Else}
    SetCtlColors ${label} ${MUTED_COLOR} ${BACKGROUND}
  ${EndIf}
  System::Call 'user32::InvalidateRect(p ${label}, p 0, i 1)'
!macroend

; An option on the current nsDialogs page, `width` wide: Windows' checkbox in its dark theme, and its text as a label
; of its own, which also switches it: Windows draws a checkbox's own text black whatever the colours set
!macro Option x row width state text on_click box label
  !insertmacro Px $R1 ${x}
  IntOp $R2 ${row} * ${OPTION_HEIGHT}
  IntOp $R2 $R2 + ${OPTIONS_TOP}
  !insertmacro Px $R2 $R2
  !insertmacro Px $R3 ${BOX_WIDTH}
  !insertmacro Px $R4 24
  nsDialogs::CreateControl BUTTON ${CHECKBOX_STYLE} 0 $R1 $R2 $R3 $R4 ""
  Pop ${box}
  System::Call 'uxtheme::SetWindowTheme(p ${box}, w "DarkMode_Explorer", p 0)'
  SetCtlColors ${box} ${TEXT_COLOR} ${BACKGROUND}
  ${If} ${state} = 1
    ${NSD_Check} ${box}
  ${EndIf}
  ${NSD_OnClick} ${box} ${on_click}
  IntOp $R1 $R1 + $R3
  IntOp $R3 ${width} - ${BOX_WIDTH}
  !insertmacro Px $R3 $R3
  nsDialogs::CreateControl STATIC ${TOGGLE_STYLE} 0 $R1 $R2 $R3 $R4 "${text}"
  Pop ${label}
  SendMessage ${label} ${WM_SETFONT} $BodyFont 1
  !insertmacro ShowOption ${label} ${state}
  ${NSD_OnClick} ${label} ${on_click}
!macroend

; A click on an option, with its control on the stack: a click on the label switches the box, the state follows the box
!macro OptionClicked box label state
  Pop $0
  ${If} $0 = ${label}
    ${If} ${state} = 1
      ${NSD_Uncheck} ${box}
    ${Else}
      ${NSD_Check} ${box}
    ${EndIf}
  ${EndIf}
  ${NSD_GetState} ${box} ${state}
  !insertmacro ShowOption ${label} ${state}
!macroend

!macro Font var face size weight
  !insertmacro Px $R1 ${size}
  IntOp $R1 0 - $R1
  System::Call 'gdi32::CreateFontW(i R1, i 0, i 0, i 0, i ${weight}, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "${face}") p .s'
  Pop ${var}
!macroend

; The window and its fonts, for the installer (prefix "") and the uninstaller ("un.")
!macro DREZIMO_INTERFACE un
  Function ${un}StyleWindow
    System::Call 'user32::GetDpiForWindow(p $HWNDPARENT) i .s'
    Pop $Dpi
    ${If} $Dpi = 0
      StrCpy $Dpi 96
    ${EndIf}

    ; Dark title bar (attribute 19 before Windows 10 2004), in the window's own colour on Windows 11
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, *i 1, i 4) i .r0'
    ${If} $0 <> 0
      System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 19, *i 1, i 4)'
    ${EndIf}
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 35, *i 0x00100E0E, i 4)'
    SetCtlColors $HWNDPARENT ${TEXT_COLOR} ${BACKGROUND}

    System::Call '*(i, i, i, i) p .r9'
    System::Call 'user32::GetWindowRect(p $HWNDPARENT, p r9)'
    System::Call '*$9(i .r1, i .r2, i .r3, i .r4)'
    System::Call 'user32::GetClientRect(p $HWNDPARENT, p r9)'
    System::Call '*$9(i, i, i .r5, i .r6)'
    System::Free $9
    IntOp $3 $3 - $1
    IntOp $4 $4 - $2
    !insertmacro Px $7 ${WINDOW_WIDTH}
    !insertmacro Px $8 ${WINDOW_HEIGHT}
    ; The frame around the client area stays as it is
    IntOp $5 $3 - $5
    IntOp $6 $4 - $6
    IntOp $5 $5 + $7
    IntOp $6 $6 + $8
    ; Centred where NSIS placed the window
    IntOp $3 $3 - $5
    IntOp $3 $3 / 2
    IntOp $1 $1 + $3
    IntOp $4 $4 - $6
    IntOp $4 $4 / 2
    IntOp $2 $2 + $4
    System::Call 'user32::SetWindowPos(p $HWNDPARENT, p 0, i r1, i r2, i r5, i r6, i 0x14)'
    GetDlgItem $0 $HWNDPARENT 1018
    System::Call 'user32::SetWindowPos(p r0, p 0, i 0, i 0, i r7, i r8, i 0x14)'

    !insertmacro Font $TitleFont "Segoe UI Semibold" 20 600
    !insertmacro Font $BodyFont "Segoe UI" 14 400
    !insertmacro Font $ButtonFont "Segoe UI Semibold" 14 600
    !insertmacro Font $SmallFont "Segoe UI" 12 400

    ; The logo drawn for the nearest display scale
    InitPluginsDir
    ${If} $Dpi < 108
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-100.bmp"
    ${ElseIf} $Dpi < 132
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-125.bmp"
    ${ElseIf} $Dpi < 156
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-150.bmp"
    ${ElseIf} $Dpi < 180
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-175.bmp"
    ${ElseIf} $Dpi < 216
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-200.bmp"
    ${Else}
      File "/oname=$PLUGINSDIR\logo.bmp" "${DREZIMO_WINDOWS}\installer\logo-250.bmp"
    ${EndIf}
  FunctionEnd

  ; Only the page is kept: NSIS shows its buttons and icon again at each page
  Function ${un}HideFrame
    StrCpy $0 0
    ${Do}
      FindWindow $0 "" "" $HWNDPARENT $0
      ${IfThen} $0 = 0 ${|} ${ExitDo} ${|}
      System::Call 'user32::GetClassNameW(p r0, w .r1, i 16)'
      System::Call 'user32::GetDlgCtrlID(p r0) i .r2'
      ${If} $1 != "#32770"
      ${AndIf} $2 <> 1018
        ShowWindow $0 ${SW_HIDE}
      ${EndIf}
    ${Loop}
  FunctionEnd

  ; Logo, title ($R7) and text ($R8) at the top of the page `$0`
  Function ${un}Header
    SetCtlColors $0 ${TEXT_COLOR} ${BACKGROUND}
    !insertmacro Px $R1 ${MARGIN}
    System::Call 'user32::CreateWindowExW(i 0, w "STATIC", w "", i ${LOGO_STYLE}, i R1, i R1, i 0, i 0, p r0, p 0, p 0, p 0) p .r1'
    System::Call 'user32::LoadImageW(p 0, w "$PLUGINSDIR\logo.bmp", i 0, i 0, i 0, i 0x10) p .r2'
    SendMessage $1 ${STM_SETIMAGE} 0 $2
    StrCpy $R5 $R7
    !insertmacro Label $0 ${TEXT_STYLE} ${MARGIN} 100 ${TEXT_WIDTH} 30 $TitleFont ${TEXT_COLOR} $1
    StrCpy $R5 $R8
    !insertmacro Label $0 ${TEXT_STYLE} ${MARGIN} 136 ${TEXT_WIDTH} 48 $BodyFont ${MUTED_COLOR} $1
  FunctionEnd

  ; The installation's progress page: the header and a thin amber bar
  Function ${un}ProgressPage
    Call ${un}HideFrame
    FindWindow $0 "#32770" "" $HWNDPARENT
    ; NSIS keeps this page at its own size, smaller than the window
    !insertmacro Px $1 ${WINDOW_WIDTH}
    !insertmacro Px $2 ${WINDOW_HEIGHT}
    System::Call 'user32::SetWindowPos(p r0, p 0, i 0, i 0, i r1, i r2, i 0x14)'
    ; Its icon, texts and details go; the progress bar stays, restyled
    StrCpy $1 0
    ${Do}
      FindWindow $1 "" "" $0 $1
      ${IfThen} $1 = 0 ${|} ${ExitDo} ${|}
      ShowWindow $1 ${SW_HIDE}
    ${Loop}
    Call ${un}Header

    GetDlgItem $1 $0 1004
    System::Call 'uxtheme::SetWindowTheme(p r1, w "", w "")'
    ; Without its sunken edge (the installer is 32-bit: SetWindowLongW)
    System::Call 'user32::SetWindowLongW(p r1, i -20, i 0)'
    ; PBM_SETBARCOLOR and PBM_SETBKCOLOR take BGR colours
    SendMessage $1 0x409 0 0x0020B0FF
    SendMessage $1 0x2001 0 0x001B1717
    !insertmacro Px $2 ${MARGIN}
    !insertmacro Px $3 ${PROGRESS_TOP}
    !insertmacro Px $4 ${TEXT_WIDTH}
    !insertmacro Px $5 4
    System::Call 'user32::SetWindowPos(p r1, p 0, i r2, i r3, i r4, i r5, i 0x34)'
    IntOp $4 $4 + 1
    IntOp $5 $5 + 1
    System::Call 'gdi32::CreateRoundRectRgn(i 0, i 0, i r4, i r5, i r5, i r5) p .r9'
    System::Call 'user32::SetWindowRgn(p r1, p r9, i 1)'
    ShowWindow $1 ${SW_SHOW}
  FunctionEnd

  ; Click callbacks get the control on the stack
  Function ${un}Continue
    Pop $0
    SendMessage $HWNDPARENT 0x408 1 0
  FunctionEnd
!macroend
!insertmacro DREZIMO_INTERFACE ""
!insertmacro DREZIMO_INTERFACE "un."

Function .onGUIInit
  Call StyleWindow
FunctionEnd

Function un.onGUIInit
  Call un.StyleWindow
FunctionEnd

; Installer pages
Page custom WelcomePage
Page instfiles "" InstallProgressPage

Function WelcomePage
  ${IfThen} $PassiveMode = 1 ${|} Abort ${|}
  Call HideFrame
  nsDialogs::Create 1018
  Pop $0
  ${If} $Comparison == "0"
    StrCpy $R7 "$(reinstallTitle)"
    StrCpy $R8 "$(reinstallText)"
    StrCpy $R9 "$(reinstallButton)"
  ${ElseIf} $Comparison == "1"
    StrCpy $R7 "$(updateTitle)"
    StrCpy $R8 "$(updateText)"
    StrCpy $R9 "$(updateButton)"
  ${ElseIf} $Comparison == "-1"
    StrCpy $R7 "$(newerInstalledTitle)"
    StrCpy $R8 "$(newerInstalledText)"
    StrCpy $R9 "$(installButton)"
  ${Else}
    StrCpy $R7 "$(installTitle)"
    StrCpy $R8 "$(installText)"
    StrCpy $R9 "$(installButton)"
  ${EndIf}
  Call Header
  !insertmacro Option ${MARGIN} 0 ${OPTION_WIDTH} $DesktopShortcut "$(desktopShortcut)" ToggleDesktopShortcut $DesktopBox $DesktopToggle
  !insertmacro Option ${OPTION_RIGHT} 0 ${OPTION_WIDTH} $StartMenuShortcut "$(startMenuShortcut)" ToggleStartMenuShortcut $StartMenuBox $StartMenuToggle
  !insertmacro Option ${MARGIN} 1 ${OPTION_WIDTH} $StartAtLogin "$(startAtLogin)" ToggleStartAtLogin $LoginBox $LoginToggle
  !insertmacro Option ${OPTION_RIGHT} 1 ${OPTION_WIDTH} $OpenWhenDone "$(openWhenDone)" ToggleOpenWhenDone $OpenBox $OpenToggle
  StrCpy $R5 "$(versionLabel)"
  !insertmacro Label $0 ${LINE_STYLE} ${MARGIN} ${BUTTON_TOP} 200 ${BUTTON_HEIGHT} $SmallFont ${MUTED_COLOR} $1
  StrCpy $R5 $R9
  !insertmacro Button 316 132 ${BACKGROUND} ${AMBER} Continue $1
  nsDialogs::Show
FunctionEnd

Function ToggleDesktopShortcut
  !insertmacro OptionClicked $DesktopBox $DesktopToggle $DesktopShortcut
FunctionEnd

Function ToggleStartMenuShortcut
  !insertmacro OptionClicked $StartMenuBox $StartMenuToggle $StartMenuShortcut
FunctionEnd

Function ToggleStartAtLogin
  !insertmacro OptionClicked $LoginBox $LoginToggle $StartAtLogin
FunctionEnd

Function ToggleOpenWhenDone
  !insertmacro OptionClicked $OpenBox $OpenToggle $OpenWhenDone
FunctionEnd

Function InstallProgressPage
  ${If} $Comparison == "1"
  ${OrIf} $UpdateMode = 1
    StrCpy $R7 "$(updating)"
  ${Else}
    StrCpy $R7 "$(installing)"
  ${EndIf}
  ${If} $OpenWhenDone = 1
    StrCpy $R8 "$(installingText)"
  ${Else}
    StrCpy $R8 ""
  ${EndIf}
  Call ProgressPage
FunctionEnd

; Uninstaller pages
UninstPage custom un.ConfirmPage
UninstPage instfiles "" un.UninstallProgressPage

Function un.ConfirmPage
  ${IfThen} $PassiveMode = 1 ${|} Abort ${|}
  Call un.HideFrame
  nsDialogs::Create 1018
  Pop $0
  StrCpy $R7 "$(uninstallTitle)"
  StrCpy $R8 "$(uninstallText)"
  Call un.Header
  !insertmacro Option ${MARGIN} 1 ${TEXT_WIDTH} $DeleteAppDataState "$(deleteSettings)" un.ToggleDeleteAppData $DeleteAppDataBox $DeleteAppDataToggle
  StrCpy $R5 "$(cancelButton)"
  !insertmacro Button 196 108 ${TEXT_COLOR} ${CARD} un.Cancel $1
  StrCpy $R5 "$(uninstallButton)"
  !insertmacro Button 316 132 ${BACKGROUND} ${AMBER} un.Continue $1
  nsDialogs::Show
FunctionEnd

Function un.ToggleDeleteAppData
  !insertmacro OptionClicked $DeleteAppDataBox $DeleteAppDataToggle $DeleteAppDataState
FunctionEnd

Function un.Cancel
  Pop $0
  SendMessage $HWNDPARENT ${WM_COMMAND} 2 0
FunctionEnd

Function un.UninstallProgressPage
  StrCpy $R7 "$(uninstalling)"
  StrCpy $R8 ""
  Call un.ProgressPage
FunctionEnd

; Stops with a message: the progress window has no room for errors, nor a button to close it
!macro Fail message
  MessageBox MB_ICONSTOP "${message}" /SD IDOK
  SetErrorLevel 2
  Quit
!macroend

; Closes the app if it runs, without asking: the installer opens it again at the end
!macro CloseApp
  !insertmacro RestartManager_ShutdownFile "$INSTDIR\${MAINBINARYNAME}.exe" $R0
  ${If} $R0 <> 0
    !insertmacro Fail "$(appStillRunning)"
  ${EndIf}
!macroend

Function .onInit
  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/NS" $NoShortcutMode
  ${IfNot} ${Errors}
    StrCpy $NoShortcutMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}

  !insertmacro SetContext

  ${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"
    StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${PRODUCTNAME}"
    Call RestorePreviousInstallLocation
  ${EndIf}

  StrCpy $Comparison ""
  ReadRegStr $InstalledVersion SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${If} $InstalledVersion != ""
    nsis_tauri_utils::SemverCompare "${VERSION}" $InstalledVersion
    Pop $Comparison
  ${EndIf}

  ; A new installation offers everything, /NS no shortcuts; over an installed version things stay as they are
  StrCpy $OpenWhenDone 1
  ${If} $InstalledVersion == ""
    StrCpy $StartAtLogin 1
    IntOp $DesktopShortcut $NoShortcutMode ^ 1
    StrCpy $StartMenuShortcut $DesktopShortcut
  ${Else}
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
    ${IfThen} $0 != "" ${|} StrCpy $StartAtLogin 1 ${|}
    ${IfThen} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk" ${|} StrCpy $DesktopShortcut 1 ${|}
    ${IfThen} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk" ${|} StrCpy $StartMenuShortcut 1 ${|}
  ${EndIf}
FunctionEnd

Section WebView2
  ${If} ${RunningX64}
    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${Else}
    ReadRegStr $4 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $4 == ""
    ReadRegStr $4 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}

  ${If} $4 == ""
    ${If} $UpdateMode <> 1
      !if "${INSTALLWEBVIEW2MODE}" == "downloadBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        DetailPrint "$(webview2Downloading)"
        NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Pop $0
        ${If} $0 == "success"
          DetailPrint "$(webview2DownloadSuccess)"
        ${Else}
          DetailPrint "$(webview2DownloadError)"
          !insertmacro Fail "$(webview2AbortError)"
        ${EndIf}
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "offlineInstaller"
        Delete "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe" "${WEBVIEW2INSTALLERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        Goto install_webview2
      !endif

      Goto webview2_done

      install_webview2:
        DetailPrint "$(installingWebview2)"
        ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
        ${If} $1 = 0
          DetailPrint "$(webview2InstallSuccess)"
        ${Else}
          DetailPrint "$(webview2InstallError)"
          !insertmacro Fail "$(webview2AbortError)"
        ${EndIf}
      webview2_done:
    ${EndIf}
  ${Else}
    !if "${MINIMUMWEBVIEW2VERSION}" != ""
      ${VersionCompare} "${MINIMUMWEBVIEW2VERSION}" "$4" $R0
      ${If} $R0 = 1
        update_webview:
          DetailPrint "$(installingWebview2)"
          ${If} ${RunningX64}
            ReadRegStr $R1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate" "path"
          ${Else}
            ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 == ""
            ReadRegStr $R1 HKCU "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 != ""
            ExecWait `"$R1" /install appguid=${WEBVIEW2APPGUID}&needsadmin=true` $1
            ${If} $1 = 0
              DetailPrint "$(webview2InstallSuccess)"
            ${Else}
              MessageBox MB_ICONEXCLAMATION|MB_ABORTRETRYIGNORE "$(webview2InstallError)" IDIGNORE ignore IDRETRY update_webview
              Quit
              ignore:
            ${EndIf}
          ${EndIf}
      ${EndIf}
    !endif
  ${EndIf}
SectionEnd

Section Install
  SetOutPath $INSTDIR

  !ifmacrodef NSIS_HOOK_PREINSTALL
    !insertmacro NSIS_HOOK_PREINSTALL
  !endif

  !insertmacro CloseApp

  File "${MAINBINARYSRCPATH}"

  {{#each resources_dirs}}
    CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
    File /a "/oname={{this.[1]}}" "{{no-escape @key}}"
  {{/each}}

  {{#each binaries}}
    File /a "/oname={{this}}" "{{no-escape @key}}"
  {{/each}}

  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
       !insertmacro APP_ASSOCIATE "{{ext}}" "{{or association.name ext}}" "{{association-description association.description ext}}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0" "Open with ${PRODUCTNAME}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
    {{/each}}
  {{/each}}

  {{#each deep_link_protocols as |protocol| ~}}
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "URL Protocol" ""
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "" "URL:${BUNDLEID} protocol"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  {{/each}}

  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; For the next installations, such as updates
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR

  ; An older main binary under another name goes
  ReadRegStr $OldMainBinaryName SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    Delete "$INSTDIR\$OldMainBinaryName"
  ${EndIf}
  WriteRegStr SHCTX "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"

  ; Windows' list of apps
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr SHCTX "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoRepair" "1"

  ${GetSize} "$INSTDIR" "/M=uninstall.exe /S=0K /G=0" $0 $1 $2
  IntOp $0 $0 + ${ESTIMATEDSIZE}
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD SHCTX "${UNINSTKEY}" "EstimatedSize" "$0"

  !if "${HOMEPAGE}" != ""
    WriteRegStr SHCTX "${UNINSTKEY}" "URLInfoAbout" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "URLUpdateInfo" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "HelpLink" "${HOMEPAGE}"
  !endif

  Call CreateOrUpdateStartMenuShortcut
  Call CreateOrUpdateDesktopShortcut

  !ifmacrodef NSIS_HOOK_POSTINSTALL
    !insertmacro NSIS_HOOK_POSTINSTALL
  !endif
SectionEnd

Function .onInstSuccess
  ; The window's installs open the app; updates and silent installs only with /R
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/R" $R0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $R0
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "$R0"
    ${EndIf}
  ${ElseIf} $OpenWhenDone = 1
    nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
  ${EndIf}
FunctionEnd

Function un.onInit
  !insertmacro SetContext

  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
FunctionEnd

Section Uninstall
  !ifmacrodef NSIS_HOOK_PREUNINSTALL
    !insertmacro NSIS_HOOK_PREUNINSTALL
  !endif

  !insertmacro CloseApp

  Delete "$INSTDIR\${MAINBINARYNAME}.exe"

  {{#each resources}}
    Delete "$INSTDIR\\{{this.[1]}}"
  {{/each}}

  {{#each binaries}}
    Delete "$INSTDIR\\{{this}}"
  {{/each}}

  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
      !insertmacro APP_UNASSOCIATE "{{ext}}" "{{or association.name ext}}"
    {{/each}}
  {{/each}}

  {{#each deep_link_protocols as |protocol| ~}}
    ReadRegStr $R7 SHCTX "Software\Classes\\{{protocol}}\shell\open\command" ""
    ${If} $R7 == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
      DeleteRegKey SHCTX "Software\Classes\\{{protocol}}"
    ${EndIf}
  {{/each}}

  Delete "$INSTDIR\uninstall.exe"

  {{#each resources_ancestors}}
  RMDir /REBOOTOK "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"

  ; Updates keep the shortcuts
  ${If} $UpdateMode <> 1
    !insertmacro DeleteAppUserModelId

    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}

    !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}

  DeleteRegKey HKCU "${UNINSTKEY}"

  ; The login item stays through updates
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}

  ${If} $DeleteAppDataState = 1
  ${AndIf} $UpdateMode <> 1
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"
    SetShellVarContext current
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTUNINSTALL
    !insertmacro NSIS_HOOK_POSTUNINSTALL
  !endif

  SetAutoClose true
SectionEnd

Function RestorePreviousInstallLocation
  ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
  StrCmp $4 "" +2 0
    StrCpy $INSTDIR $4
FunctionEnd

; An option unticked over an installed version: its shortcut goes, if it opens the app under its name or an older one
!macro RemoveShortcut path
  !insertmacro IsShortcutTarget "${path}" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 <> 1
  ${AndIf} $OldMainBinaryName != ""
    !insertmacro IsShortcutTarget "${path}" "$INSTDIR\$OldMainBinaryName"
    Pop $0
  ${EndIf}
  ${If} $0 = 1
    !insertmacro UnpinShortcut "${path}"
    Delete "${path}"
  ${EndIf}
!macroend

Function CreateOrUpdateStartMenuShortcut
  ${If} $StartMenuShortcut <> 1
    !insertmacro RemoveShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    Return
  ${EndIf}

  ; Shortcuts to an older main binary name move to the new one
  !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Return
  ${EndIf}

  CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
FunctionEnd

Function CreateOrUpdateDesktopShortcut
  ${If} $DesktopShortcut <> 1
    !insertmacro RemoveShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
    Return
  ${EndIf}

  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Return
  ${EndIf}

  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
FunctionEnd
