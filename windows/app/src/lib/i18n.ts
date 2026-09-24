// Toda string que o usuário lê vem do catálogo (`locales/<idioma>.json`), com as
// MESMAS chaves do app macOS quando o conceito é o mesmo — e o mesmo formato de
// placeholder (`%@`, `%1$@`, `%d`, `%%`), para os textos poderem ser comparados
// lado a lado com os `Localizable.strings`. O `scripts/check-strings.mjs`
// barra chave faltando, tradução órfã, placeholder divergente e texto solto.

import en from "../locales/en.json";
import ptBR from "../locales/pt-BR.json";
import type { Locale } from "./types";

export type Key = keyof typeof en;

const catalogs: Record<Locale, Record<string, string>> = { en, "pt-BR": ptBR };

let current: Record<string, string> = en;
let currentLocale: Locale = "en";

/** Fixado uma vez na subida, pelo idioma que o backend diz ser o do Windows. */
export function setLocale(locale: Locale): void {
  currentLocale = locale;
  current = catalogs[locale];
}

export function locale(): Locale {
  return currentLocale;
}

/**
 * Preenche um modelo no estilo do macOS: `%@` (texto), `%d` (inteiro), com
 * posição opcional (`%2$@`) e `%%` para o sinal de porcento.
 */
export function format(template: string, args: readonly (string | number)[]): string {
  let next = 0;
  return template.replace(/%(?:(\d+)\$)?([@d%])/g, (_match, position: string | undefined, kind: string) => {
    if (kind === "%") return "%";
    const index = position ? Number(position) - 1 : next++;
    const value = args[index];
    if (value === undefined) return "";
    return kind === "d" ? String(Math.trunc(Number(value))) : String(value);
  });
}

/** O texto de uma chave no idioma atual (o inglês é a base). */
export function t(key: Key, ...args: (string | number)[]): string {
  const template = current[key] ?? en[key] ?? key;
  return args.length > 0 || template.includes("%%") ? format(template, args) : template;
}
