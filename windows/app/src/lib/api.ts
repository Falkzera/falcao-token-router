// A ponte com o backend (Rust). Toda chamada do front passa por aqui, e só por
// aqui: fora do Tauri (a página aberta no navegador pelo `npm run dev`) quem
// responde é o backend simulado de `mock.ts` — é assim que cada estado da tela
// é conferido no Chrome sem conta, sem disco e sem processo.

import { invoke } from "@tauri-apps/api/core";
import { mockInvoke } from "./mock";
import type { AppInfo } from "./types";

/** Dentro do WebView do Tauri? (O Tauri injeta este objeto antes da página.) */
export const insideTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return insideTauri ? invoke<T>(command, args) : mockInvoke<T>(command, args);
}

export function appInfo(): Promise<AppInfo> {
  return call<AppInfo>("app_info");
}
