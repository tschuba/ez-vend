#![allow(clippy::redundant_closure)]
#![allow(clippy::while_let_on_iterator)]
#![allow(clippy::map_identity)]

use leptos::*;
use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Supported locales with regional variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    De,   // German (fallback) - EUR
    DeDE, // Germany - EUR
    DeAT, // Austria - EUR
    DeCH, // Switzerland - CHF
    En,   // English (fallback) - EUR (default)
    EnUS, // United States - USD
    EnGB, // United Kingdom - GBP
    EnEU, // English speakers in Europe - EUR
}

impl Locale {
    /// Get language code for translation file loading (backward compatible)
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "de",
            Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "en",
        }
    }

    /// Parse locale string with regional variant support
    pub fn from_str(s: &str) -> Self {
        match s {
            // English variants
            "en-US" => Locale::EnUS,
            "en-GB" => Locale::EnGB,
            "en-IE" | "en-MT" | "en-EU" => Locale::EnEU,

            // German variants
            "de-DE" => Locale::DeDE,
            "de-AT" => Locale::DeAT,
            "de-CH" | "de-LI" => Locale::DeCH,

            // Fallbacks (for generic "en" or "de")
            s if s.starts_with("en") => Locale::En,
            s if s.starts_with("de") => Locale::De,

            // Global fallback
            _ => Locale::De,
        }
    }

    /// Get full locale code for persistence (e.g., "de-DE", "en-US")
    pub fn full_code(&self) -> &'static str {
        match self {
            Locale::De => "de",
            Locale::DeDE => "de-DE",
            Locale::DeAT => "de-AT",
            Locale::DeCH => "de-CH",
            Locale::En => "en",
            Locale::EnUS => "en-US",
            Locale::EnGB => "en-GB",
            Locale::EnEU => "en-EU",
        }
    }
}

/// Translation data structure
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Translations {
    #[serde(flatten)]
    data: HashMap<String, serde_json::Value>,
}

impl Translations {
    /// Get a translation by key (supports nested keys with dots)
    pub fn get(&self, key: &str) -> String {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &serde_json::Value::Object(
            self.data.clone().into_iter().map(|(k, v)| (k, v)).collect(),
        );

        for part in parts {
            match current {
                serde_json::Value::Object(map) => {
                    if let Some(value) = map.get(part) {
                        current = value;
                    } else {
                        return format!("[missing: {}]", key);
                    }
                }
                _ => return format!("[missing: {}]", key),
            }
        }

        match current {
            serde_json::Value::String(s) => s.clone(),
            _ => format!("[invalid: {}]", key),
        }
    }

    pub fn format(&self, key: &str, params: &HashMap<&str, String>) -> String {
        let template = self.get(key);
        let mut result = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some('}') = chars.peek().cloned() {
                    chars.next();
                    result.push_str("{}");
                    continue;
                }

                let mut key_buf = String::new();
                while let Some(next_ch) = chars.next() {
                    if next_ch == '}' {
                        break;
                    }
                    key_buf.push(next_ch);
                }

                if key_buf.is_empty() {
                    result.push_str("{}");
                } else if let Some(value) = params.get(key_buf.as_str()) {
                    result.push_str(value);
                } else {
                    result.push('{');
                    result.push_str(&key_buf);
                    result.push('}');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}

/// Detect browser locale
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["navigator"], js_name = language, thread_local_v2)]
    static LANGUAGE: String;
}

/// Detect the user's browser locale
pub fn detect_locale() -> Locale {
    LANGUAGE.with(|lang| Locale::from_str(lang))
}

/// Load translations for a given locale
pub fn load_translations(locale: Locale) -> Translations {
    let json_str = match locale.as_str() {
        "de" => include_str!("../locales/de.json"),
        "en" => include_str!("../locales/en.json"),
        _ => include_str!("../locales/de.json"), // Fallback
    };

    serde_json::from_str(json_str).expect("Failed to parse translations")
}

/// Translate helper
#[allow(dead_code)]
pub fn translate(key: &str) -> String {
    let translations = use_translations();
    translations.with(|t| t.get(key))
}

pub fn translate_with_params(key: &str, params: HashMap<&str, String>) -> String {
    let translations = use_translations();
    translations.with(|t| t.format(key, &params))
}

const LOCALE_STORAGE_KEY: &str = "ez_vend_locale";

/// Save locale to localStorage
pub fn persist_locale(locale: Locale) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(LOCALE_STORAGE_KEY, locale.full_code());
        }
    }
}

/// Load locale from localStorage
pub fn load_persisted_locale() -> Option<Locale> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let locale_str = storage.get_item(LOCALE_STORAGE_KEY).ok()??;
    Some(Locale::from_str(&locale_str))
}

/// Initialize i18n context with locale persistence
pub fn provide_i18n() {
    // Try persisted locale first, fall back to browser detection
    let initial_locale = load_persisted_locale().unwrap_or_else(|| detect_locale());

    let locale = RwSignal::new(initial_locale);
    let translations = Memo::new(move |_| load_translations(locale.get()));

    // Persist locale whenever it changes
    Effect::new(move |_| {
        persist_locale(locale.get());
    });

    provide_context(locale);
    provide_context(translations);
}

/// Get current locale from context
pub fn use_locale() -> RwSignal<Locale> {
    if let Some(locale) = use_context::<RwSignal<Locale>>() {
        locale
    } else {
        web_sys::console::warn_1(
            &"Locale context not found. Falling back to German locale.".into(),
        );
        RwSignal::new(Locale::De)
    }
}

/// Get translations from context
pub fn use_translations() -> Memo<Translations> {
    if let Some(translations) = use_context::<Memo<Translations>>() {
        translations
    } else {
        web_sys::console::warn_1(
            &"Translations context not found. Falling back to German translations.".into(),
        );
        Memo::new(|_| load_translations(Locale::De))
    }
}

/// Macro for easy translation access
#[macro_export]
macro_rules! t {
    ($key:expr) => {{
        let translations = $crate::i18n::use_translations();
        move || translations.with(|t| t.get($key))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_from_str() {
        // English variants
        assert_eq!(Locale::from_str("en-US"), Locale::EnUS);
        assert_eq!(Locale::from_str("en-GB"), Locale::EnGB);
        assert_eq!(Locale::from_str("en-EU"), Locale::EnEU);
        assert_eq!(Locale::from_str("en-IE"), Locale::EnEU);
        assert_eq!(Locale::from_str("en"), Locale::En);

        // German variants
        assert_eq!(Locale::from_str("de-DE"), Locale::DeDE);
        assert_eq!(Locale::from_str("de-AT"), Locale::DeAT);
        assert_eq!(Locale::from_str("de-CH"), Locale::DeCH);
        assert_eq!(Locale::from_str("de-LI"), Locale::DeCH);
        assert_eq!(Locale::from_str("de"), Locale::De);

        // Fallback
        assert_eq!(Locale::from_str("fr"), Locale::De);
        assert_eq!(Locale::from_str("es"), Locale::De);
    }

    #[test]
    fn test_locale_as_str() {
        // All German variants return "de"
        assert_eq!(Locale::De.as_str(), "de");
        assert_eq!(Locale::DeDE.as_str(), "de");
        assert_eq!(Locale::DeAT.as_str(), "de");
        assert_eq!(Locale::DeCH.as_str(), "de");

        // All English variants return "en"
        assert_eq!(Locale::En.as_str(), "en");
        assert_eq!(Locale::EnUS.as_str(), "en");
        assert_eq!(Locale::EnGB.as_str(), "en");
        assert_eq!(Locale::EnEU.as_str(), "en");
    }

    #[test]
    fn test_locale_full_code() {
        assert_eq!(Locale::De.full_code(), "de");
        assert_eq!(Locale::DeDE.full_code(), "de-DE");
        assert_eq!(Locale::DeAT.full_code(), "de-AT");
        assert_eq!(Locale::DeCH.full_code(), "de-CH");
        assert_eq!(Locale::En.full_code(), "en");
        assert_eq!(Locale::EnUS.full_code(), "en-US");
        assert_eq!(Locale::EnGB.full_code(), "en-GB");
        assert_eq!(Locale::EnEU.full_code(), "en-EU");
    }

    #[test]
    fn test_load_translations() {
        let de = load_translations(Locale::De);
        assert_eq!(de.get("common.save"), "Speichern");
        assert_eq!(de.get("booth.title"), "Veranstaltung");

        let en = load_translations(Locale::En);
        assert_eq!(en.get("common.save"), "Save");
        assert_eq!(en.get("booth.title"), "Event");
    }

    #[test]
    fn test_missing_key() {
        let de = load_translations(Locale::De);
        assert!(de.get("nonexistent.key").contains("[missing:"));
    }
}
