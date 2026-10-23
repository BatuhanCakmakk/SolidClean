import { de } from "./locales/de";
import { en } from "./locales/en";
import { es } from "./locales/es";
import { fr } from "./locales/fr";
import { it } from "./locales/it";
import { ja } from "./locales/ja";
import { pt } from "./locales/pt";
import { ru } from "./locales/ru";
import { tr, type Strings } from "./locales/tr";
import { zh } from "./locales/zh";

/** Uygulama içi diller. `locale` sayı ve tarih biçimi için; arka uca `code` gönderilir. */
export const LANGUAGES = [
  { code: "en", name: "English", locale: "en-US", strings: en },
  { code: "tr", name: "Türkçe", locale: "tr-TR", strings: tr },
  { code: "de", name: "Deutsch", locale: "de-DE", strings: de },
  { code: "fr", name: "Français", locale: "fr-FR", strings: fr },
  { code: "es", name: "Español", locale: "es-ES", strings: es },
  { code: "pt", name: "Português (Brasil)", locale: "pt-BR", strings: pt },
  { code: "it", name: "Italiano", locale: "it-IT", strings: it },
  { code: "ru", name: "Русский", locale: "ru-RU", strings: ru },
  { code: "ja", name: "日本語", locale: "ja-JP", strings: ja },
  { code: "zh", name: "简体中文", locale: "zh-CN", strings: zh },
] as const satisfies readonly { code: string; name: string; locale: string; strings: Strings }[];

export type LanguageCode = (typeof LANGUAGES)[number]["code"];

const STORAGE_KEY = "language";
const find = (tag: string | null | undefined) => LANGUAGES.find((l) => tag?.toLowerCase().split(/[-_]/)[0] === l.code);

function readSaved(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** Seçilen dil kaydedilir; metinler modül yüklenirken seçildiği için pencere yenilenir. */
export function setLanguage(code: LanguageCode) {
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // Kaydedilemezse yine de bu oturumda uygulanır.
  }
  const url = new URL(location.href);
  url.searchParams.delete("lang");
  location.replace(url);
}

// Sıra: geliştirmede adres çubuğu (?lang=en, mağaza görselleri bununla çekilir), kullanıcının
// seçimi, Windows'un dil listesi, İngilizce.
const forced = import.meta.env.DEV ? new URLSearchParams(location.search).get("lang") : null;
const chosen = find(forced) ?? find(readSaved()) ?? navigator.languages.map(find).find(Boolean) ?? LANGUAGES[0];

export const t: Strings = chosen.strings;
/** Arka uca gönderilen dil kodu (kural metinleri ve temizlik mesajları bu dilde gelir). */
export const lang: LanguageCode = chosen.code;
// Bölgesel biçim (ör. en-GB'de gün/ay sırası) Windows'un diliyle aynı dildeyse korunur.
export const locale = !forced && find(navigator.language) === chosen ? navigator.language : chosen.locale;

document.documentElement.lang = chosen.locale;
