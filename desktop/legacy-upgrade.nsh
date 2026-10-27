; Compatibility with the updater in the shipped Tauri build. Electron's own update helper does
; not pass /UPDATE. Only this explicit handoff may adopt a legacy install path/registry entry.
!macro customHeader
  !ifndef BUILD_UNINSTALLER
    Var CodexLegacyUpdate
    Var CodexLegacyPath
  !endif
!macroend

!macro customInit
  StrCpy $CodexLegacyUpdate "0"
  ClearErrors
  ${GetOptions} $CMDLINE "/UPDATE" $0
  ${IfNot} ${Errors}
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex Minus" "MainBinaryName"
    ReadRegStr $1 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex Minus" "DisplayName"
    ${If} $0 != "codex-minus.exe"
    ${OrIf} $1 != "Codex Minus"
      SetErrorLevel 2
      Quit
    ${EndIf}
    ReadRegStr $CodexLegacyPath HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex Minus" "InstallLocation"
    StrCpy $0 $CodexLegacyPath 1
    ${If} $0 == '"'
      StrCpy $CodexLegacyPath $CodexLegacyPath "" 1
      StrCpy $CodexLegacyPath $CodexLegacyPath -1
    ${EndIf}
    ${IfNot} ${FileExists} "$CodexLegacyPath\codex-minus.exe"
      SetErrorLevel 2
      Quit
    ${EndIf}
    StrCpy $INSTDIR $CodexLegacyPath
    StrCpy $CodexLegacyUpdate "1"
    ; The old updater's passive /P path already has explicit user authorization. Skip the
    ; assisted install wizard, use the existing location, and close on completion.
    ClearErrors
    ${GetOptions} $CMDLINE "/P" $0
    ${IfNot} ${Errors}
      SetSilent silent
    ${EndIf}
  ${EndIf}
!macroend

!macro customInstall
  ${If} $CodexLegacyUpdate == "1"
    ; Executed after Electron files and its own uninstaller registration are written.
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex Minus"
    Delete "$INSTDIR\uninstall.exe"
    ClearErrors
    ${GetOptions} $CMDLINE "/R" $0
    ${IfNot} ${Errors}
      Exec '"$INSTDIR\codex-minus.exe"'
    ${EndIf}
    SetAutoClose true
  ${EndIf}
!macroend
