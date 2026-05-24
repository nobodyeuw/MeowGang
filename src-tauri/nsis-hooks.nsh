!macro NSIS_HOOK_POSTUNINSTALL
  ; Tauri deletes its identifier-based app data when the user selects the
  ; uninstall data checkbox. LOA Tracker stores userlogs.db and app resources
  ; in a separate product-named LocalAppData folder, so remove that too.
  StrCmp $DeleteAppDataCheckboxState 1 0 cleanup_done
  SetShellVarContext current
  RMDir /r "$LOCALAPPDATA\LOA Tracker"
  cleanup_done:
!macroend
