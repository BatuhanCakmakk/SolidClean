//! Arka ucun kullanıcıya gösterdiği metinler: temizlik sonuçları ve kural açıklamaları.
//! Her dil `locales/<kod>.toml` dosyasındadır. Kuralların Türkçe ve İngilizce metinleri
//! rules.toml'da kuralın yanında durur; diğer diller onları `[rules.<id>]` altında çevirir.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
    Tr,
    En,
    De,
    Fr,
    Es,
    Pt,
    It,
    Ru,
    Ja,
    Zh,
}

const SOURCES: [(Lang, &str, &str); 10] = [
    (Lang::Tr, "tr", include_str!("../locales/tr.toml")),
    (Lang::En, "en", include_str!("../locales/en.toml")),
    (Lang::De, "de", include_str!("../locales/de.toml")),
    (Lang::Fr, "fr", include_str!("../locales/fr.toml")),
    (Lang::Es, "es", include_str!("../locales/es.toml")),
    (Lang::Pt, "pt", include_str!("../locales/pt.toml")),
    (Lang::It, "it", include_str!("../locales/it.toml")),
    (Lang::Ru, "ru", include_str!("../locales/ru.toml")),
    (Lang::Ja, "ja", include_str!("../locales/ja.toml")),
    (Lang::Zh, "zh", include_str!("../locales/zh.toml")),
];

impl Lang {
    /// "pt-BR", "zh_CN" gibi kodları da kabul eder; bilinmeyen dil İngilizce olur.
    pub fn parse(code: &str) -> Self {
        let base = code.split(['-', '_']).next().unwrap_or_default().to_lowercase();
        SOURCES.iter().find(|(_, c, _)| *c == base).map_or(Lang::En, |(lang, _, _)| *lang)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Messages {
    pub close_apps: String,
    pub command_failed: String,
    pub nothing_removed: String,
    /// `{count}` atlanan öğe sayısıyla değiştirilir.
    pub in_use: String,
    pub restart_pending: String,
    pub all_drives: String,
    pub elevation_declined: String,
    /// `{error}` hata metniyle değiştirilir.
    pub elevation_failed: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuleText {
    pub name: Option<String>,
    pub about: Option<String>,
    pub effect: Option<String>,
    pub manual: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Locale {
    messages: Messages,
    #[serde(default)]
    rules: HashMap<String, RuleText>,
}

fn locales() -> &'static HashMap<Lang, Locale> {
    static LOCALES: OnceLock<HashMap<Lang, Locale>> = OnceLock::new();
    LOCALES.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(lang, code, source)| {
                let locale = toml::from_str(source).unwrap_or_else(|e| panic!("locales/{code}.toml geçersiz: {e}"));
                (*lang, locale)
            })
            .collect()
    })
}

pub fn messages(lang: Lang) -> &'static Messages {
    &locales()[&lang].messages
}

/// Kuralın bu dildeki çevirisi; Türkçe ve İngilizce için yoktur (rules.toml'dadır).
pub fn rule_text(lang: Lang, id: &str) -> Option<&'static RuleText> {
    locales()[&lang].rules.get(id)
}

#[cfg(test)]
pub(crate) fn all() -> impl Iterator<Item = (Lang, &'static str)> {
    SOURCES.iter().map(|(lang, code, _)| (*lang, *code))
}

#[cfg(test)]
pub(crate) fn rule_ids(lang: Lang) -> Vec<&'static str> {
    locales()[&lang].rules.keys().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_parse_from_windows_tags() {
        assert_eq!(Lang::parse("tr-TR"), Lang::Tr);
        assert_eq!(Lang::parse("pt-BR"), Lang::Pt);
        assert_eq!(Lang::parse("zh_CN"), Lang::Zh);
        assert_eq!(Lang::parse("JA"), Lang::Ja);
        assert_eq!(Lang::parse("ko-KR"), Lang::En);
        assert_eq!(Lang::parse(""), Lang::En);
    }

    #[test]
    fn every_locale_has_its_messages() {
        for (lang, code) in all() {
            let m = messages(lang);
            assert!(m.in_use.contains("{count}"), "{code}: in_use");
            assert!(m.elevation_failed.contains("{error}"), "{code}: elevation_failed");
            for text in [&m.close_apps, &m.command_failed, &m.nothing_removed, &m.restart_pending, &m.all_drives, &m.elevation_declined] {
                assert!(!text.trim().is_empty(), "{code}: boş mesaj");
            }
        }
    }
}
