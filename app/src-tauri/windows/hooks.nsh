; The installer's own files (installer.nsi): its texts and logo
!define DREZIMO_WINDOWS "${__FILEDIR__}"

; The app starts at login, in the tray, when the install page's option is ticked (installer.nsi): ticked means on, even if
; it was turned off in Windows' startup apps. An update leaves that as it is. The uninstaller removes it.
!macro NSIS_HOOK_POSTINSTALL
  ${If} $StartAtLogin = 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --autostart'
    ${If} $UpdateMode <> 1
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}"
    ${EndIf}
  ${Else}
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}
!macroend
