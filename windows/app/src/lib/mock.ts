// O backend simulado: responde aos mesmos comandos do Rust quando a página roda
// no navegador (`npm run dev`). O cenário e o idioma vêm da URL —
// `?view=home&lang=pt-BR&tab=settings` — para cada estado ser aberto de propósito.

import type { AppInfo, HomeTab, Locale } from "./types";

function param(name: string): string | null {
  return new URLSearchParams(window.location.search).get(name);
}

function mockLocale(): Locale {
  return param("lang") === "pt-BR" ? "pt-BR" : "en";
}

function mockTab(): HomeTab {
  return param("tab") === "settings" ? "settings" : "groups";
}

const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0-mock", locale: mockLocale(), initialTab: mockTab() }),
};

export async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[command];
  if (!handler) throw new Error(`mock: comando desconhecido ${command}`);
  return handler(args) as T;
}
