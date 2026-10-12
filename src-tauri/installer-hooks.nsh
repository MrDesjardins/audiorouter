!include "LogicLib.nsh"
Var ar_remove_exit

!macro NSIS_HOOK_PREUNINSTALL
  MessageBox MB_YESNO|MB_DEFBUTTON2 "Also remove AudioRouter virtual cables? Other applications may use these devices. Choose No to keep them." IDNO ar_keep_cables
  IfFileExists "$INSTDIR\resources\audiorouter-driver-helper.exe" 0 ar_helper_missing
  ExecWait '"$INSTDIR\resources\audiorouter-driver-helper.exe" remove' $ar_remove_exit
  ${If} $ar_remove_exit != 0
    MessageBox MB_OK|MB_ICONEXCLAMATION "AudioRouter could not remove the virtual cables (result $ar_remove_exit). The app will still be uninstalled. You can reinstall AudioRouter and remove the cables from Setup later."
  ${EndIf}
  Goto ar_keep_cables
  ar_helper_missing:
    MessageBox MB_OK|MB_ICONEXCLAMATION "The AudioRouter driver helper is missing. The virtual cables were kept."
  ar_keep_cables:
!macroend
