; The start at login is a Run value written by Landry itself: remove it with the app.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Landry"
!macroend
