import { es } from "./es";
import { en } from "./en";

export type LocaleDictionary = typeof es;
export type Language = "es" | "en";

export { es, en };

/**
 * Resolves a nested dot-notated key from a dictionary and substitutes {parameters}.
 *
 * @param dict The dictionary to look up the translation in.
 * @param path Dot-separated path to the translation key (e.g. "roles.buyer.title").
 * @param params Optional dictionary of parameters to substitute into placeholders like "{param}".
 * @returns The resolved string with replacements, or the fallback path if not found.
 */
export function getTranslation(
  dict: LocaleDictionary,
  path: string,
  params?: Record<string, string | number>
): string {
  if (!path) return "";

  const segments = path.split(".");
  let current: unknown = dict;

  for (const segment of segments) {
    if (current && typeof current === "object" && segment in current) {
      current = (current as Record<string, unknown>)[segment];
    } else {
      current = undefined;
      break;
    }
  }

  if (typeof current !== "string") {
    return path;
  }

  let result = current;
  if (params) {
    for (const [key, val] of Object.entries(params)) {
      result = result.replaceAll(`{${key}}`, String(val));
    }
  }

  return result;
}
