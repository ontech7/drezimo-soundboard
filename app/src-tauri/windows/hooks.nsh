; The installer's own files (installer.nsi): its texts and logo
!define DREZIMO_WINDOWS "${__FILEDIR__}"

; The app starts at login, in the tray, when the install page's option is ticked (installer.nsi). Turned off in Windows'
; startup apps it stays off; the uninstaller removes it.
!macro NSIS_HOOK_POSTINSTALL
  ${If} $StartAtLogin = 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --autostart'
  ${Else}
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}
!macroend
