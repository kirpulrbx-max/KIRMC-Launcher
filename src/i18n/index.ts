// i18n mínimo (sin i18next): diccionarios tipados, cero dependencias.
import { useCallback } from "react";
import type { Lang } from "../lib/api";
import { useApp } from "../store/app";
import es, { type Dict } from "./es";
import en from "./en";
import ja from "./ja";

export type Key = keyof Dict;
type Vars = Record<string, string | number>;

const dicts: Record<Lang, Dict> = { es, en, ja };

export const LANGS: { id: Lang; label: string }[] = [
  { id: "es", label: "Español" },
  { id: "ja", label: "日本語" },
  { id: "en", label: "English" },
];

export function translate(lang: Lang, key: Key, vars?: Vars): string {
  let s = (dicts[lang] ?? es)[key] ?? es[key];
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, String(v));
  return s;
}

export function useT() {
  const lang = useApp((s) => s.config?.language ?? "es");
  return useCallback((key: Key, vars?: Vars) => translate(lang, key, vars), [lang]);
}
