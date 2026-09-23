; Ganchos do instalador NSIS do Tauri (bundle > windows > nsis > installerHooks).
; UTF-8 com BOM, como os .nsh do próprio Tauri: sem BOM o NSIS lê como ANSI.
;
; O router.exe fica EM USO enquanto dura cada sessão aberta por `claude <grupo>`:
; o `router launch` espera o claude sair. O Windows não deixa sobrescrever nem
; apagar um executável em execução, mas deixa RENOMEÁ-LO — o processo segue com
; a imagem que já carregou. Sem estes ganchos (conferido em 23/09/2026):
; - a atualização silenciosa pulava o router.exe e saía com sucesso, deixando o
;   app novo com o router velho; a interativa parava em "Error opening file for
;   writing";
; - a desinstalação deixava o router.exe, e com ele a pasta, para trás.

!include LogicLib.nsh

; Tira o router.exe do caminho. Livre, ele só é apagado; em uso, vai para o
; %TEMP% (mesmo volume do perfil) com nome único e, se nem isso der, fica na
; pasta com outro nome. As sobras de vezes anteriores, já sem processo, saem.
!macro FALCAO_ROUTER_OUT_OF_THE_WAY
  Push $0
  Delete "$TEMP\falcao-router-*.exe.old"
  Delete "$INSTDIR\router-*.exe.old"
  Delete "$INSTDIR\router.exe"
  ${If} ${FileExists} "$INSTDIR\router.exe"
    System::Call "kernel32::GetTickCount() i .r0"
    Rename "$INSTDIR\router.exe" "$TEMP\falcao-router-$0.exe.old"
    ${If} ${FileExists} "$INSTDIR\router.exe"
      Rename "$INSTDIR\router.exe" "$INSTDIR\router-$0.exe.old"
    ${EndIf}
  ${EndIf}
  Pop $0
  ClearErrors
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro FALCAO_ROUTER_OUT_OF_THE_WAY
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro FALCAO_ROUTER_OUT_OF_THE_WAY
!macroend

; O modelo do Tauri tenta o `RMDir $INSTDIR` antes deste gancho: um router
; renomeado na própria pasta (quando o %TEMP% não deu) o teria impedido.
!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\router-*.exe.old"
  RMDir "$INSTDIR"
!macroend
