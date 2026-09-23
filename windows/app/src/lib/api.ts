// A ponte com o backend (Rust). Toda chamada do front passa por aqui, e só por
// aqui: fora do Tauri (a página aberta no navegador pelo `npm run dev`) quem
// responde é o backend simulado de `mock.ts` — é assim que cada estado da tela
// é conferido no Chrome sem conta, sem disco e sem processo.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { mockInvoke } from "./mock";
import type { AppInfo, HomeTab, Snapshot, View } from "./types";

/** Dentro do WebView do Tauri? (O Tauri injeta este objeto antes da página.) */
export const insideTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return insideTauri ? invoke<T>(command, args) : mockInvoke<T>(command, args);
}

/** Ouve um evento do backend; no navegador não há backend para emitir. */
async function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!insideTauri) return () => {};
  return listen<T>(event, (e) => handler(e.payload));
}

/** A superfície desta janela: o rótulo dela no Tauri; `?view=` no navegador. */
export function currentView(): View {
  const label = insideTauri
    ? getCurrentWebviewWindow().label
    : new URLSearchParams(window.location.search).get("view");
  return label === "flyout" ? "flyout" : "home";
}

export function appInfo(): Promise<AppInfo> {
  return call<AppInfo>("app_info");
}

/** A conta já aberta no cartão de detalhe — só no navegador (`?select=`), para
 *  conferir o cartão sem clicar; no app, o flyout abre sem seleção. */
export function initialSelection(): string | null {
  return insideTauri ? null : new URLSearchParams(window.location.search).get("select");
}

export function getSnapshot(): Promise<Snapshot> {
  return call<Snapshot>("get_snapshot");
}

/** O flyout acompanha a altura do conteúdo (px lógicos). */
export function fitFlyout(height: number): Promise<void> {
  return call<void>("fit_flyout", { height });
}

/** Uma porta do rodapé do flyout: abre a janela na aba. */
export function openHome(tab: HomeTab): Promise<void> {
  return call<void>("open_home", { tab });
}

export function quitApp(): Promise<void> {
  return call<void>("quit_app");
}

/** A bandeja pediu outra aba com a janela já aberta. */
export function onNavigate(handler: (tab: HomeTab) => void): Promise<UnlistenFn> {
  return on<HomeTab>("navigate", handler);
}

/** O backend releu o quadro (laço de 30 s, ou uma ação mudou algo). */
export function onSnapshotChanged(handler: () => void): Promise<UnlistenFn> {
  return on<null>("snapshot-changed", () => handler());
}
