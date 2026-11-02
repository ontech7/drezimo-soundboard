; The app starts at login, in the tray. Turned off in Windows' startup apps it stays off; the uninstaller removes it.
!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --autostart'
!macroend
