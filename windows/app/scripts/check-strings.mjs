#!/usr/bin/env node
// Confere as strings da interface (≙ Scripts/check-strings.sh do macOS):
//
// 1. toda chave usada no código (front em TS/Svelte e a bandeja em Rust) existe
//    nos dois catálogos;
// 2. nenhum catálogo carrega tradução órfã (chave que código nenhum usa);
// 3. os placeholders (%@, %d, %1$@…) batem entre os idiomas — uma tradução que
//    perde um %@ engole um argumento em silêncio;
// 4. nenhum texto solto na marcação Svelte (texto entre tags ou em
//    title/placeholder/aria-label/alt fora de `{…}`).
//
// Existe porque é o tipo de defeito que não produz erro nenhum: a chave crua ou
// o texto em um idioma só aparecem para o usuário, e o compilador não tem como
// saber.
//
// Uso: node scripts/check-strings.mjs   (a partir de windows/app)

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
// Os prefixos são o contrato: chave de usuário começa por um destes. Sem
// `alerts` (o medidor não entra no porte); `tray` é superfície nova do Windows.
const PREFIXES = ["panel", "settings", "format", "groups", "home", "tray"];
const LOCALES = ["en", "pt-BR"];
const SKIP = new Set(["node_modules", "dist", "target", "gen"]);

function walk(dir, extensions, out = []) {
  for (const name of readdirSync(dir)) {
    if (SKIP.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path, extensions, out);
    else if (extensions.includes(extname(name))) out.push(path);
  }
  return out;
}

const keyBody = `((?:${PREFIXES.join("|")})\\.[A-Za-z0-9.]+)`;
// No front a chave pode vir entre aspas, apóstrofos ou crases (template
// literal). Em Rust só aspas duplas são string — crase ali é comentário
// (`` `tray.rs` `` num doc comment não é chave).
const frontKey = new RegExp(`["'\`]${keyBody}["'\`]`, "g");
const rustKey = new RegExp(`"${keyBody}"`, "g");
const sources = [
  ...walk(join(ROOT, "src"), [".ts", ".svelte"]),
  ...walk(join(ROOT, "src-tauri", "src"), [".rs"]),
];
// Nome de arquivo não é chave: `"settings.json"` casa com o prefixo `settings.`.
const fileName = /\.(json|ps1|sh|exe|md|toml|rs|ts|svelte|png|ico|log)$/;
const used = new Set();
for (const file of sources) {
  const pattern = file.endsWith(".rs") ? rustKey : frontKey;
  for (const match of readFileSync(file, "utf8").matchAll(pattern)) {
    if (!fileName.test(match[1])) used.add(match[1]);
  }
}

const errors = [];
const catalogs = {};
for (const locale of LOCALES) {
  const path = join(ROOT, "src", "locales", `${locale}.json`);
  try {
    catalogs[locale] = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    errors.push(`catálogo ilegível ${relative(ROOT, path)}: ${error.message}`);
    catalogs[locale] = {};
  }
}

for (const locale of LOCALES) {
  const defined = new Set(Object.keys(catalogs[locale]));
  const missing = [...used].filter((k) => !defined.has(k)).sort();
  if (missing.length) errors.push(`chaves usadas no código e ausentes em ${locale}:\n    ${missing.join("\n    ")}`);
  const orphan = [...defined].filter((k) => !used.has(k)).sort();
  if (orphan.length) errors.push(`traduções órfãs em ${locale} (nenhum código as usa):\n    ${orphan.join("\n    ")}`);
  const wrongPrefix = [...defined].filter((k) => !PREFIXES.some((p) => k.startsWith(`${p}.`))).sort();
  if (wrongPrefix.length) errors.push(`chaves fora dos prefixos (${PREFIXES.join("|")}) em ${locale}:\n    ${wrongPrefix.join("\n    ")}`);
}

/** Os placeholders de um modelo, com posição explícita: `%@ %d` → ["1@", "2d"]. */
function placeholders(template) {
  let next = 0;
  const found = [];
  for (const match of template.matchAll(/%(?:(\d+)\$)?([@d%])/g)) {
    if (match[2] === "%") continue;
    const position = match[1] ? Number(match[1]) : ++next;
    found.push(`${position}${match[2]}`);
  }
  return found.sort().join(",");
}

const base = catalogs[LOCALES[0]];
for (const locale of LOCALES.slice(1)) {
  const divergent = Object.keys(base)
    .filter((k) => k in catalogs[locale] && placeholders(base[k]) !== placeholders(catalogs[locale][k]))
    .sort();
  if (divergent.length) {
    errors.push(`placeholders diferentes entre ${LOCALES[0]} e ${locale}:\n    ${divergent.join("\n    ")}`);
  }
}

/** Tira da marcação os blocos `{…}` (expressões, `{#if}`, `{/each}`), com aninhamento. */
function withoutExpressions(markup) {
  let out = "";
  let depth = 0;
  for (const ch of markup) {
    if (ch === "{") depth++;
    else if (ch === "}" && depth > 0) depth--;
    else if (depth === 0) out += ch;
  }
  return out;
}

const letter = /\p{L}/u;
for (const file of sources.filter((f) => f.endsWith(".svelte"))) {
  const markup = withoutExpressions(
    readFileSync(file, "utf8")
      .replace(/<script[\s\S]*?<\/script>/g, "")
      .replace(/<style[\s\S]*?<\/style>/g, "")
      .replace(/<!--[\s\S]*?-->/g, ""),
  );
  const loose = [];
  for (const match of markup.matchAll(/>([^<]*)</g)) {
    const text = match[1].trim();
    if (letter.test(text)) loose.push(text);
  }
  for (const match of markup.matchAll(/\b(title|placeholder|aria-label|alt|label)="([^"]*)"/g)) {
    if (letter.test(match[2])) loose.push(`${match[1]}="${match[2]}"`);
  }
  if (loose.length) {
    errors.push(`texto solto em ${relative(ROOT, file)} (use t("…") ou um dado):\n    ${loose.join("\n    ")}`);
  }
}

if (errors.length) {
  for (const error of errors) console.error(`erro: ${error}`);
  process.exit(1);
}
console.log(`==> strings ok: ${used.size} chaves em ${LOCALES.length} idiomas`);
