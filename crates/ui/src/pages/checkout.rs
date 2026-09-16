#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::useless_format)]
#![allow(clippy::manual_div_ceil)]

use crate::audio::play_error_sound;
use crate::components::*;
use crate::error_logging::{current_route, stack_trace, use_error_logger, ErrorLogDraft};
use crate::error_translator::translate_domain_error;
use crate::formatting::{
    decimal_separator, format_currency, format_decimal_for_input, is_allowed_amount_key,
    parse_decimal_input, sanitize_amount_input, DecimalInputParseError,
};
use crate::i18n::{translate_with_params, use_locale, Locale};
use crate::selected_booth_context;
use crate::state::use_app_state;
use crate::t;
use chrono::{DateTime, Local, Utc};
use domain::error::DomainError;
use domain::error_code::ValidationError;
use domain::models::booth::{VendorIdOmissionRules, VendorIdValidation};
use domain::models::product::{Product, ProductGroup, TailwindColor};
use domain::models::purchase::{Purchase, PurchaseItem};
use domain::models::shared::{ProductId, PurchaseId, VendorId};
use domain::models::BoothType;
use domain::validation::{validate_amount_matches_step, validate_vendor_id};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use log::{error, info, warn};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::js_sys::Date;
use web_sys::window;

#[derive(Clone, Debug, PartialEq)]
struct CheckoutItem {
    amount: Decimal,
    vendor_id: String,
    product_id: Option<ProductId>,
    added_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CheckoutFormData {
    vendor_id: String,
    current_amount: String,
    items: Vec<CheckoutItem>,
    vendor_error: Option<String>,
    amount_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct PendingDeletion {
    purchase_id: Option<PurchaseId>,
    token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredCheckoutItem {
    amount: String,
    vendor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    product_id: Option<String>,
    added_at_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredCheckoutForm {
    booth_id: Option<String>,
    vendor_id: String,
    current_amount: String,
    items: Vec<StoredCheckoutItem>,
}

const CHECKOUT_DRAFT_STORAGE_KEY: &str = "ez-vend-checkout-draft";
const CHECKOUT_KEYBOARD_VISIBLE_STORAGE_KEY: &str = "ez-vend-checkout-keyboard-visible";
const CHECKOUT_AMOUNT_INPUT_MODE_STORAGE_KEY: &str = "ez-vend-checkout-amount-input-mode";
const CHECKOUT_ERROR_SOUND_ENABLED_STORAGE_KEY: &str = "ez-vend-checkout-error-sound-enabled";
const MAX_ITEM_AMOUNT: Decimal = Decimal::from_parts(1_000_000, 0, 0, false, 0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveInput {
    VendorId,
    Amount,
}

#[derive(Clone, Debug, PartialEq)]
enum DraftLoadOutcome {
    Empty,
    Restored {
        booth_id: Option<String>,
        form_data: CheckoutFormData,
    },
    CorruptedCleared,
}

#[derive(Clone, Debug, PartialEq)]
enum DraftNotice {
    Restored,
    CorruptedCleared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum CheckoutMode {
    PriceInput,
    ProductButtons,
}

impl CheckoutFormData {
    fn total(&self) -> Decimal {
        self.items.iter().map(|item| item.amount).sum()
    }
}

fn format_item_timestamp(added_at: DateTime<Utc>, locale: Locale) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(added_at);

    if duration.num_seconds() < 60 {
        let secs = duration.num_seconds().max(1);
        let key = if secs == 1 {
            "checkout.time.seconds_one"
        } else {
            "checkout.time.seconds"
        };
        translate_with_params(key, HashMap::from([("count", secs.to_string())]))
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes().max(1);
        let key = if mins == 1 {
            "checkout.time.minutes_one"
        } else {
            "checkout.time.minutes"
        };
        translate_with_params(key, HashMap::from([("count", mins.to_string())]))
    } else {
        let format_str = match locale {
            Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "%H:%M",
            Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "%I:%M %p",
        };
        let local_time = added_at
            .with_timezone(&Local)
            .format(format_str)
            .to_string();
        translate_with_params(
            "checkout.time.time_of_day",
            HashMap::from([("time", local_time)]),
        )
    }
}

fn format_item_tooltip(added_at: DateTime<Utc>, locale: Locale) -> String {
    let format_str = match locale {
        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "%d.%m.%Y %H:%M",
        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "%Y-%m-%d %I:%M %p",
    };
    let local_time = added_at
        .with_timezone(&Local)
        .format(format_str)
        .to_string();
    translate_with_params(
        "checkout.time.tooltip",
        HashMap::from([("datetime", local_time)]),
    )
}

fn format_purchase_timestamp(timestamp: DateTime<Utc>, locale: Locale) -> String {
    let format_str = match locale {
        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "%d.%m.%Y %H:%M",
        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "%Y-%m-%d %I:%M %p",
    };
    timestamp
        .with_timezone(&Local)
        .format(format_str)
        .to_string()
}

fn confirmation_token_from_purchase(purchase_id: &PurchaseId) -> String {
    let id = purchase_id.as_str();
    let sanitized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    if sanitized.is_empty() {
        return String::new();
    }

    let len = sanitized.len();
    let start = len.saturating_sub(4);
    sanitized[start..].to_uppercase()
}

fn focus_and_select_input(input_ref: &NodeRef<html::Input>) {
    if let Some(input) = input_ref.get() {
        let _ = input.focus();
        let _ = input.select();
    }
}

fn is_repeat_focus_active(
    vendor_input_ref: &NodeRef<html::Input>,
    amount_input_ref: &NodeRef<html::Input>,
) -> bool {
    vendor_input_ref
        .get()
        .is_some_and(|input| input.matches(":focus").ok().unwrap_or(false))
        || amount_input_ref
            .get()
            .is_some_and(|input| input.matches(":focus").ok().unwrap_or(false))
}

fn utf16_index_to_byte_index(value: &str, utf16_index: u32) -> usize {
    let mut utf16_count = 0;

    for (byte_index, ch) in value.char_indices() {
        let next_utf16_count = utf16_count + ch.len_utf16() as u32;
        if utf16_index <= utf16_count {
            return byte_index;
        }
        if utf16_index < next_utf16_count {
            return byte_index;
        }
        utf16_count = next_utf16_count;
    }

    value.len()
}

fn replace_input_range(value: &str, start: u32, end: u32, replacement: &str) -> String {
    let start_byte = utf16_index_to_byte_index(value, start);
    let end_byte = utf16_index_to_byte_index(value, end);

    format!(
        "{}{}{}",
        &value[..start_byte],
        replacement,
        &value[end_byte..]
    )
}

fn backspace_input_range(value: &str, start: u32, end: u32) -> String {
    if start != end {
        return replace_input_range(value, start, end, "");
    }

    if start == 0 {
        return value.to_string();
    }

    let previous_utf16 = start.saturating_sub(1);
    replace_input_range(value, previous_utf16, start, "")
}

fn input_selection_range(input_ref: &NodeRef<html::Input>, fallback: &str) -> (String, u32, u32) {
    if let Some(input) = input_ref.get() {
        let value = input.value();
        let start = input
            .selection_start()
            .ok()
            .flatten()
            .unwrap_or_else(|| value.encode_utf16().count() as u32);
        let end = input.selection_end().ok().flatten().unwrap_or(start);
        (value, start, end)
    } else {
        let value = fallback.to_string();
        let cursor = value.encode_utf16().count() as u32;
        (value, cursor, cursor)
    }
}

fn focus_input_with_cursor(input_ref: &NodeRef<html::Input>, cursor: u32) {
    if let Some(input) = input_ref.get() {
        let _ = input.focus();
        let _ = input.set_selection_range(cursor, cursor);
    }
}

fn default_amount_for_mode(mode: AmountInputMode, locale: Locale) -> String {
    match mode {
        AmountInputMode::RightToLeft => format_decimal_for_input(Decimal::ZERO, locale, 2),
        AmountInputMode::Regular => String::new(),
    }
}

fn amount_is_effectively_empty(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || parse_decimal_input(trimmed)
            .map(|amount| amount == Decimal::ZERO)
            .unwrap_or(false)
}

fn parse_amount_to_cents(input: &str) -> i64 {
    parse_decimal_input(input)
        .ok()
        .and_then(|amount| (amount * Decimal::new(100, 0)).round_dp(0).to_i64())
        .unwrap_or(0)
}

fn format_cents_to_amount(cents: i64, locale: Locale) -> String {
    format_decimal_for_input(Decimal::new(cents, 2), locale, 2)
}

fn rtl_add_digit(current: &str, digit: u8, locale: Locale) -> String {
    let next_cents = parse_amount_to_cents(current)
        .checked_mul(10)
        .and_then(|value| value.checked_add(i64::from(digit)));

    match next_cents {
        Some(next_cents) => {
            let next_amount = Decimal::new(next_cents, 2);
            if next_amount > MAX_ITEM_AMOUNT {
                current.to_string()
            } else {
                format_decimal_for_input(next_amount, locale, 2)
            }
        }
        None => current.to_string(),
    }
}

fn rtl_backspace(current: &str, locale: Locale) -> String {
    let next_cents = parse_amount_to_cents(current) / 10;
    format_cents_to_amount(next_cents, locale)
}

fn normalize_amount_for_mode(value: &str, mode: AmountInputMode, locale: Locale) -> String {
    if amount_is_effectively_empty(value) {
        return default_amount_for_mode(mode, locale);
    }

    match mode {
        AmountInputMode::RightToLeft => parse_decimal_input(value)
            .map(|amount| format_decimal_for_input(amount, locale, 2))
            .unwrap_or_else(|_| default_amount_for_mode(mode, locale)),
        AmountInputMode::Regular => value.to_string(),
    }
}

fn normalize_form_data_for_mode(
    mut form_data: CheckoutFormData,
    mode: AmountInputMode,
    locale: Locale,
) -> CheckoutFormData {
    form_data.current_amount = normalize_amount_for_mode(&form_data.current_amount, mode, locale);
    form_data
}

fn load_keyboard_visible_preference() -> bool {
    get_local_storage()
        .and_then(|storage| {
            storage
                .get_item(CHECKOUT_KEYBOARD_VISIBLE_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(false)
}

fn persist_keyboard_visible_preference(is_visible: bool) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(
            CHECKOUT_KEYBOARD_VISIBLE_STORAGE_KEY,
            if is_visible { "true" } else { "false" },
        );
    }
}

fn load_amount_input_mode_preference() -> AmountInputMode {
    get_local_storage()
        .and_then(|storage| {
            storage
                .get_item(CHECKOUT_AMOUNT_INPUT_MODE_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .map(|value| match value.as_str() {
            "regular" => AmountInputMode::Regular,
            _ => AmountInputMode::Regular,
        })
        .unwrap_or_default()
}

fn load_error_sound_enabled_preference() -> bool {
    get_local_storage()
        .and_then(|storage| {
            storage
                .get_item(CHECKOUT_ERROR_SOUND_ENABLED_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(true)
}

fn persist_amount_input_mode_preference(mode: AmountInputMode) {
    if let Some(storage) = get_local_storage() {
        let value = match mode {
            AmountInputMode::RightToLeft => "rtl",
            AmountInputMode::Regular => "regular",
        };
        let _ = storage.set_item(CHECKOUT_AMOUNT_INPUT_MODE_STORAGE_KEY, value);
    }
}

fn persist_error_sound_enabled_preference(is_enabled: bool) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(
            CHECKOUT_ERROR_SOUND_ENABLED_STORAGE_KEY,
            if is_enabled { "true" } else { "false" },
        );
    }
}

fn play_checkout_error_sound_if_enabled(
    is_enabled: ReadSignal<bool>,
    last_played_at: RwSignal<u128>,
) {
    if !is_enabled.get_untracked() {
        return;
    }

    let now = Date::now() as u128;
    let last_played = last_played_at.get_untracked();

    if last_played > now {
        last_played_at.set(0);
    } else if now.saturating_sub(last_played) < 700 {
        return;
    }

    last_played_at.set(now);

    play_error_sound();
}

fn should_play_inline_error(previous_error: &Option<String>, next_error: &Option<String>) -> bool {
    next_error.is_some() && previous_error.as_ref() != next_error.as_ref()
}

fn validate_inline_amount(
    value: &str,
    amount_stepping: Option<Decimal>,
    locale: Locale,
) -> Option<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    match classify_inline_amount(trimmed, amount_stepping) {
        InlineAmountValidation::Valid => None,
        InlineAmountValidation::TooLarge => Some(t!("checkout.errors.amount_too_large")()),
        InlineAmountValidation::TooManyDecimals => {
            Some(t!("booth_form.errors.item_amount_too_many_decimals")())
        }
        InlineAmountValidation::StepMismatch(step) => Some(translate_with_params(
            "checkout.errors.amount_stepping",
            HashMap::from([("step", format_currency(step, locale))]),
        )),
        InlineAmountValidation::Invalid => Some(t!("checkout.errors.amount_invalid")()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InlineAmountValidation {
    Valid,
    TooLarge,
    TooManyDecimals,
    StepMismatch(Decimal),
    Invalid,
}

fn classify_inline_amount(value: &str, amount_stepping: Option<Decimal>) -> InlineAmountValidation {
    match parse_decimal_input(value) {
        Ok(amount) => {
            if amount > MAX_ITEM_AMOUNT {
                InlineAmountValidation::TooLarge
            } else if let Some(step) = amount_stepping {
                if validate_amount_matches_step(amount, step).is_err() {
                    InlineAmountValidation::StepMismatch(step)
                } else {
                    InlineAmountValidation::Valid
                }
            } else {
                InlineAmountValidation::Valid
            }
        }
        Err(DecimalInputParseError::TooManyDecimalPlaces) => {
            InlineAmountValidation::TooManyDecimals
        }
        Err(_) => InlineAmountValidation::Invalid,
    }
}

fn update_vendor_input(
    set_form_data: WriteSignal<CheckoutFormData>,
    value: String,
    vendor_validation_rule: Option<VendorIdValidation>,
    vendor_omission_rules: VendorIdOmissionRules,
    error_sound_enabled: ReadSignal<bool>,
    last_error_sound_at: RwSignal<u128>,
) {
    let trimmed = value.trim().to_string();
    let vendor_error = if trimmed.is_empty() {
        None
    } else if let Err(err) = vendor_omission_rules.validate() {
        Some(translate_domain_error(&err))
    } else if let Some(rule) = vendor_validation_rule {
        match validate_vendor_id(&trimmed, &rule) {
            Err(err) => Some(translate_domain_error(&err)),
            Ok(()) => match vendor_omission_rules.is_omitted(&trimmed) {
                Ok(true) => Some(translate_domain_error(&DomainError::Validation(
                    ValidationError::VendorIdOmitted { value: trimmed },
                ))),
                Ok(false) => None,
                Err(err) => Some(translate_domain_error(&err)),
            },
        }
    } else if let Ok(true) = vendor_omission_rules.is_omitted(&trimmed) {
        Some(translate_domain_error(&DomainError::Validation(
            ValidationError::VendorIdOmitted { value: trimmed },
        )))
    } else {
        None
    };

    let mut should_play_sound = false;

    set_form_data.update(|data| {
        should_play_sound = should_play_inline_error(&data.vendor_error, &vendor_error);
        data.vendor_id = value.clone();
        data.vendor_error = vendor_error.clone();
    });

    if should_play_sound {
        play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
    }
}

fn update_amount_input(
    set_form_data: WriteSignal<CheckoutFormData>,
    value: String,
    amount_stepping: Option<Decimal>,
    locale: Locale,
    error_sound_enabled: ReadSignal<bool>,
    last_error_sound_at: RwSignal<u128>,
) {
    let amount_error = validate_inline_amount(&value, amount_stepping, locale);
    let mut should_play_sound = false;

    set_form_data.update(|data| {
        should_play_sound = should_play_inline_error(&data.amount_error, &amount_error);
        data.current_amount = value;
        data.amount_error = amount_error.clone();
    });

    if should_play_sound {
        play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
    }
}

fn get_local_storage() -> Option<web_sys::Storage> {
    let window = window()?;
    window.local_storage().ok().flatten()
}

fn checkout_mode_key(booth_id: &str) -> String {
    format!("ez-vend-checkout-mode-{booth_id}")
}

fn load_checkout_mode(booth_id: &str, default: CheckoutMode) -> CheckoutMode {
    let Some(storage) = get_local_storage() else {
        return default;
    };
    match storage.get_item(&checkout_mode_key(booth_id)) {
        Ok(Some(val)) => serde_json::from_str::<CheckoutMode>(&val).unwrap_or(default),
        _ => default,
    }
}

fn save_checkout_mode(booth_id: &str, mode: CheckoutMode) {
    if let Some(storage) = get_local_storage() {
        if let Ok(val) = serde_json::to_string(&mode) {
            let _ = storage.set_item(&checkout_mode_key(booth_id), &val);
        }
    }
}

fn color_button_class(c: TailwindColor) -> &'static str {
    match c {
        TailwindColor::Red => "bg-red-500 text-white",
        TailwindColor::Orange => "bg-orange-500 text-white",
        TailwindColor::Amber => "bg-amber-500 text-white",
        TailwindColor::Green => "bg-green-500 text-white",
        TailwindColor::Teal => "bg-teal-500 text-white",
        TailwindColor::Blue => "bg-blue-500 text-white",
        TailwindColor::Violet => "bg-violet-500 text-white",
        TailwindColor::Pink => "bg-pink-500 text-white",
    }
}

fn parse_stored_form_data(raw: &str) -> Result<(Option<String>, CheckoutFormData), String> {
    let parsed: StoredCheckoutForm =
        serde_json::from_str(raw).map_err(|err| format!("failed to deserialize draft: {err}"))?;

    let mut items = Vec::with_capacity(parsed.items.len());
    for stored in parsed.items {
        let amount = match Decimal::from_str(&stored.amount) {
            Ok(amount) => amount,
            Err(err) => {
                return Err(format!(
                    "failed to parse stored checkout amount '{}': {err}",
                    stored.amount
                ));
            }
        };
        let added_at = DateTime::<Utc>::from_timestamp_millis(stored.added_at_ms)
            .unwrap_or_else(|| Utc::now());
        items.push(CheckoutItem {
            amount,
            vendor_id: stored.vendor_id,
            product_id: stored
                .product_id
                .and_then(|s| uuid::Uuid::parse_str(&s).ok().map(ProductId::from_uuid)),
            added_at,
        });
    }

    Ok((
        parsed.booth_id,
        CheckoutFormData {
            vendor_id: parsed.vendor_id,
            current_amount: parsed.current_amount,
            items,
            ..Default::default()
        },
    ))
}

fn load_saved_form_data() -> DraftLoadOutcome {
    let Some(storage) = get_local_storage() else {
        return DraftLoadOutcome::Empty;
    };

    let raw = match storage.get_item(CHECKOUT_DRAFT_STORAGE_KEY) {
        Ok(Some(raw)) => raw,
        Ok(None) | Err(_) => return DraftLoadOutcome::Empty,
    };

    match parse_stored_form_data(&raw) {
        Ok((booth_id, form_data)) => DraftLoadOutcome::Restored {
            booth_id,
            form_data,
        },
        Err(err) => {
            error!("Failed to recover checkout draft: {}", err);
            let _ = storage.remove_item(CHECKOUT_DRAFT_STORAGE_KEY);
            DraftLoadOutcome::CorruptedCleared
        }
    }
}

fn persist_form_data(booth_id: Option<String>, data: &CheckoutFormData) -> Result<(), String> {
    let is_empty = data.vendor_id.trim().is_empty()
        && data.current_amount.trim().is_empty()
        && data.items.is_empty();

    if let Some(storage) = get_local_storage() {
        if is_empty {
            storage
                .remove_item(CHECKOUT_DRAFT_STORAGE_KEY)
                .map_err(|err| format!("failed to clear draft: {:?}", err))?;
            return Ok(());
        }

        let stored = StoredCheckoutForm {
            booth_id,
            vendor_id: data.vendor_id.clone(),
            current_amount: data.current_amount.clone(),
            items: data
                .items
                .iter()
                .map(|item| StoredCheckoutItem {
                    amount: item.amount.to_string(),
                    vendor_id: item.vendor_id.clone(),
                    product_id: item.product_id.map(|id| id.as_str()),
                    added_at_ms: item.added_at.timestamp_millis(),
                })
                .collect(),
        };

        let serialized = serde_json::to_string(&stored)
            .map_err(|err| format!("failed to serialize draft: {err}"))?;
        storage
            .set_item(CHECKOUT_DRAFT_STORAGE_KEY, &serialized)
            .map_err(|err| format!("failed to persist draft: {:?}", err))?;
    }

    Ok(())
}

#[component]
pub fn CheckoutPage() -> impl IntoView {
    let app_state = use_app_state();
    let toast = use_toast();
    let log_error = use_error_logger();

    // Use global selected booth context
    let selected_booth = selected_booth_context::use_selected_booth();
    let booth_list_version = selected_booth_context::use_booth_list_version();

    // Purchases for current booth (paginated)
    let (purchases, set_purchases) = signal(Vec::<Purchase>::new());
    let (total_purchase_count, set_total_purchase_count) = signal(0_usize);

    // Pagination state with persistence and readiness flag
    let (page_size, set_page_size, page_size_ready) =
        use_pagination_preference("checkout_page_size", 5);
    let (current_page, set_current_page) = signal(0_usize);

    // Reload toggle - flipped to force re-fetch of purchase list
    let (reload_toggle, set_reload_toggle) = signal(false);

    // Running totals (separate from paginated data)
    let (running_totals, set_running_totals) = signal((Decimal::ZERO, 0_usize, 0_usize));
    let (last_partial_recovery_warning, set_last_partial_recovery_warning) =
        signal::<Option<(String, usize)>>(None);
    let (partial_recovery_count, set_partial_recovery_count) = signal(0_usize);

    // Checkout form data
    let draft_load_outcome = {
        let outcome = load_saved_form_data();
        if let DraftLoadOutcome::Restored {
            booth_id: Some(ref draft_bid),
            ..
        } = outcome
        {
            let current_bid = selected_booth
                .get_untracked()
                .map(|b| b.id.as_str().to_string());
            if current_bid.as_deref() != Some(draft_bid.as_str()) {
                DraftLoadOutcome::Empty
            } else {
                outcome
            }
        } else {
            outcome
        }
    };
    let initial_draft_notice = match &draft_load_outcome {
        DraftLoadOutcome::Restored { .. } => Some(DraftNotice::Restored),
        DraftLoadOutcome::CorruptedCleared => Some(DraftNotice::CorruptedCleared),
        DraftLoadOutcome::Empty => None,
    };
    let locale = use_locale();
    let initial_amount_input_mode = load_amount_input_mode_preference();
    let initial_form_data = match draft_load_outcome {
        DraftLoadOutcome::Restored { form_data, .. } => normalize_form_data_for_mode(
            form_data,
            initial_amount_input_mode,
            locale.get_untracked(),
        ),
        DraftLoadOutcome::Empty | DraftLoadOutcome::CorruptedCleared => CheckoutFormData {
            current_amount: default_amount_for_mode(
                initial_amount_input_mode,
                locale.get_untracked(),
            ),
            ..CheckoutFormData::default()
        },
    };
    let (form_data, set_form_data) = signal(initial_form_data);
    let (keyboard_visible, set_keyboard_visible) = signal(load_keyboard_visible_preference());
    let (amount_input_mode, set_amount_input_mode) = signal(initial_amount_input_mode);
    let (error_sound_enabled, set_error_sound_enabled) =
        signal(load_error_sound_enabled_preference());
    let last_error_sound_at = RwSignal::new(0_u128);
    let is_submitting = RwSignal::new(false);
    let (active_input, set_active_input) = signal(ActiveInput::VendorId);

    let checkout_mode = RwSignal::new(CheckoutMode::PriceInput);
    let product_groups_signal: RwSignal<Vec<ProductGroup>> = RwSignal::new(vec![]);
    let products_signal: RwSignal<Vec<Product>> = RwSignal::new(vec![]);

    let is_direct_sale = Memo::new(move |_| {
        selected_booth
            .get()
            .map(|b| b.booth_type == BoothType::DirectSale)
            .unwrap_or(false)
    });

    let direct_sale_vendor_id_str = Memo::new(move |_| {
        selected_booth
            .get()
            .and_then(|b| b.direct_sale_vendor_id)
            .map(|v| v.as_str().to_string())
            .unwrap_or_default()
    });

    if let Some(notice) = initial_draft_notice {
        match notice {
            DraftNotice::Restored => toast.info(&t!("checkout.draft_restored")()),
            DraftNotice::CorruptedCleared => toast.warning(&t!("checkout.draft_corrupted")()),
        }
    }

    // Cancel confirmation modal
    let (show_cancel_modal, set_show_cancel_modal) = signal(false);

    // Delete confirmation modal state
    let (pending_deletion, set_pending_deletion) = signal(PendingDeletion::default());
    let (delete_confirmation_input, set_delete_confirmation_input) = signal(String::new());
    let delete_confirmation_ref: NodeRef<html::Input> = NodeRef::new();

    // Item deletion state - tracks which item (by index) is armed for deletion
    let item_delete = use_two_step_delete::<usize>();
    let item_delete_signal = item_delete.signal();

    // Purchase deletion state - tracks which purchase (by ID) is armed for deletion
    let (purchase_to_delete, set_purchase_to_delete) = signal::<Option<PurchaseId>>(None);

    // Transaction detail expansion state - tracks which purchase is expanded
    let (expanded_purchase_id, set_expanded_purchase_id) = signal::<Option<PurchaseId>>(None);

    let deletion_token_matches = Memo::new(move |_| {
        let required = pending_deletion.get().token.trim().to_uppercase();

        if required.is_empty() {
            return false;
        }

        let entered = delete_confirmation_input.get().trim().to_uppercase();
        !entered.is_empty() && entered == required
    });

    // Persist form data anytime it changes
    {
        let form_data = form_data.clone();
        let selected_booth = selected_booth.clone();
        let log_error = log_error.clone();
        Effect::new(move |_| {
            let data = form_data.get();
            let booth_id = selected_booth.get().map(|b| b.id.as_str());
            if let Err(err) = persist_form_data(booth_id, &data) {
                error!("Checkout draft persistence failed: {}", err);
                log_error(ErrorLogDraft {
                    error_type: "checkout_draft_save_failed".to_string(),
                    error_message: err.clone(),
                    stack_trace: stack_trace(),
                    user_action: Some("persist checkout draft".to_string()),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: Vec::new(),
                });
                toast.error(&t!("checkout.draft_save_failed")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
            }
        });
    }

    // Input references for focus management
    let vendor_input_ref: NodeRef<html::Input> = NodeRef::new();
    let amount_input_ref: NodeRef<html::Input> = NodeRef::new();
    let vendor_input_ref_for_repeat = vendor_input_ref.clone();
    let amount_input_ref_for_repeat = amount_input_ref.clone();
    let vendor_input_ref_for_repeat_button = vendor_input_ref.clone();
    let amount_input_ref_for_repeat_button = amount_input_ref.clone();

    let can_repeat = Memo::new(move |_| {
        let data = form_data.get();
        let Some(last_item) = data.items.first() else {
            return false;
        };

        data.vendor_id.trim() == last_item.vendor_id
            && is_repeat_focus_active(
                &vendor_input_ref_for_repeat_button,
                &amount_input_ref_for_repeat_button,
            )
    });

    // Each async effect keeps its own cloned error logger so the callback can
    // be moved into spawned tasks without coupling unrelated effect lifetimes.
    // The descriptive names document which flow owns each clone.
    let log_error_for_load = log_error.clone();
    Effect::new(move |_| {
        persist_keyboard_visible_preference(keyboard_visible.get());
    });

    Effect::new(move |_| {
        persist_amount_input_mode_preference(amount_input_mode.get());
    });

    Effect::new(move |_| {
        persist_error_sound_enabled_preference(error_sound_enabled.get());
    });

    // Load checkout mode per booth (DirectSale defaults to ProductButtons)
    Effect::new(move |_| {
        if let Some(booth) = selected_booth.get() {
            let default = if booth.booth_type == BoothType::DirectSale {
                CheckoutMode::ProductButtons
            } else {
                CheckoutMode::PriceInput
            };
            let id = booth.id.as_str();
            checkout_mode.set(load_checkout_mode(&id, default));
        }
    });

    // Persist checkout mode per booth
    Effect::new(move |_| {
        let mode = checkout_mode.get();
        if let Some(booth) = selected_booth.get() {
            let id = booth.id.as_str();
            save_checkout_mode(&id, mode);
        }
    });

    // Load products + groups for DirectSale booths
    Effect::new(move |_| {
        let _ = app_state.get();
        let booth = selected_booth.get();
        if let (Some(Ok(state)), Some(booth)) = (app_state.get(), booth) {
            if booth.booth_type == BoothType::DirectSale {
                let booth_id = booth.id.clone();
                spawn_local(async move {
                    if let Ok(mut gs) = state
                        .product_group_repository
                        .find_by_booth(&booth_id)
                        .await
                    {
                        gs.sort_by_key(|g| g.sort_order);
                        product_groups_signal.set(gs);
                    }
                    if let Ok(mut ps) = state.product_repository.find_by_booth(&booth_id).await {
                        ps.sort_by_key(|p| p.sort_order);
                        products_signal.set(ps);
                    }
                });
            } else {
                product_groups_signal.set(vec![]);
                products_signal.set(vec![]);
            }
        }
    });

    Effect::new(move |_| {
        let locale = locale.get();
        let mode = amount_input_mode.get();

        set_form_data.update(|data| {
            if data.items.is_empty() {
                data.current_amount = normalize_amount_for_mode(&data.current_amount, mode, locale);
            }
        });
    });

    // Loading state
    let (is_loading, set_is_loading) = signal(true);

    // Vendor validation rule for current booth (changes when booth changes)
    let vendor_validation_rule = Memo::new(move |_| {
        selected_booth
            .get()
            .map(|booth| booth.vendor_id_validation.clone())
    });

    let vendor_omission_rules = Memo::new(move |_| {
        selected_booth
            .get()
            .map(|booth| booth.vendor_id_omission_rules.clone())
            .unwrap_or_else(VendorIdOmissionRules::empty)
    });

    let amount_stepping =
        Memo::new(move |_| selected_booth.get().and_then(|booth| booth.amount_stepping));

    Effect::new(move |_| {
        if let Some(booth) = selected_booth.get() {
            if booth.is_archived() {
                toast.error(&t!("archive.cannot_select")());
                selected_booth.set(None);
                booth_list_version.update(|version| *version += 1);
            }
        }
    });

    let show_rules_modal = RwSignal::new(false);

    // Focus vendor input when view is ready and data is loaded
    {
        let vendor_input_ref = vendor_input_ref.clone();
        let is_loading = is_loading.clone();
        let selected_booth = selected_booth.clone();

        Effect::new(move |_| {
            if !is_loading.get() && selected_booth.get().is_some() {
                let vendor_input_ref = vendor_input_ref.clone();
                set_timeout(
                    move || {
                        if let Some(input) = vendor_input_ref.get() {
                            let _ = input.focus();
                            let _ = input.select();
                        }
                    },
                    std::time::Duration::from_millis(0),
                );
            }
        });
    }

    // Load paginated purchases for selected booth
    Effect::new(move |_| {
        let state_result = app_state.get();
        let booth = selected_booth.get();
        let page = current_page.get();
        let page_size_val = page_size.get();
        let is_ready = page_size_ready.get();
        let _ = reload_toggle.get(); // Track reload toggle to force refresh

        // Wait for page size preference to be ready before fetching
        if !is_ready {
            return;
        }

        if let (Some(Ok(state)), Some(booth)) = (state_result, booth) {
            info!(
                "Loading purchases for booth: {}, page: {}, page_size: {}",
                booth.id, page, page_size_val
            );
            set_is_loading.set(true);
            let set_purchases = set_purchases.clone();
            let set_total_count = set_total_purchase_count.clone();
            let set_running_totals = set_running_totals.clone();
            let toast = toast.clone();
            let booth_id = booth.id.clone();
            let warning_booth_id = booth.id.as_str();
            let set_last_partial_recovery_warning = set_last_partial_recovery_warning.clone();
            let log_error = log_error_for_load.clone();

            spawn_local(async move {
                match state
                    .indexed_purchase_repository
                    .find_by_booth_with_diagnostics(&booth_id)
                    .await
                {
                    Ok((mut recovered_purchases, diagnostics)) => {
                        recovered_purchases.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

                        let total_count = recovered_purchases.len();
                        let offset = page * page_size_val;
                        let paginated_items = recovered_purchases
                            .iter()
                            .skip(offset)
                            .take(page_size_val)
                            .cloned()
                            .collect::<Vec<_>>();

                        let total_sales: Decimal = recovered_purchases
                            .iter()
                            .map(|purchase| purchase.total_amount())
                            .sum();
                        let total_items: usize = recovered_purchases
                            .iter()
                            .map(|purchase| purchase.items.len())
                            .sum();

                        set_purchases.set(paginated_items);
                        set_total_count.set(total_count);
                        set_running_totals.set((total_sales, total_items, total_count));

                        if !diagnostics.is_empty() {
                            set_partial_recovery_count.set(diagnostics.len());
                            let warning_key = (warning_booth_id.clone(), diagnostics.len());
                            let already_shown = last_partial_recovery_warning.get_untracked();
                            if already_shown != Some(warning_key.clone()) {
                                let message = translate_with_params(
                                    "checkout.recovery.partial_data_warning",
                                    HashMap::from([("count", diagnostics.len().to_string())]),
                                );
                                toast.warning(&message);
                                set_last_partial_recovery_warning.set(Some(warning_key));
                            }
                        } else {
                            set_partial_recovery_count.set(0);
                            set_last_partial_recovery_warning.set(None);
                        }
                    }
                    Err(e) => {
                        log_error(ErrorLogDraft {
                            error_type: "checkout_load_purchases_failed".to_string(),
                            error_message: e.to_string(),
                            stack_trace: stack_trace(),
                            user_action: Some("load purchases".to_string()),
                            route: current_route(),
                            vendor_id: None,
                            purchase_id: None,
                            details: vec![format!("booth_id={booth_id}")],
                        });
                        let error_msg = translate_with_params(
                            "checkout.errors.load_purchases_failed",
                            HashMap::from([("error", format_error_message(&e))]),
                        );
                        toast.error(&error_msg);
                        play_checkout_error_sound_if_enabled(
                            error_sound_enabled,
                            last_error_sound_at,
                        );
                        set_purchases.set(Vec::new());
                        set_total_count.set(0);
                        set_running_totals.set((Decimal::ZERO, 0, 0));
                        set_partial_recovery_count.set(0);
                    }
                }

                set_is_loading.set(false);
            });
        }
    });

    // Form actions
    let vendor_input_ref_for_add = vendor_input_ref.clone();
    let amount_input_ref_for_add = amount_input_ref.clone();

    let add_item = move || {
        let mut data = form_data.get();

        let vendor_id_for_item = if is_direct_sale.get() {
            direct_sale_vendor_id_str.get()
        } else {
            let trimmed = data.vendor_id.trim().to_string();
            if trimmed != data.vendor_id {
                let message = t!("checkout.info.vendor_trimmed")();
                toast.info(&message);
                set_form_data.update(|form| form.vendor_id = trimmed.clone());
                data.vendor_id = trimmed.clone();
                if let Some(vendor_input) = vendor_input_ref_for_add.get() {
                    vendor_input.set_value(&trimmed);
                }
            }
            if trimmed.is_empty() {
                let message = t!("checkout.errors.vendor_required")();
                toast.warning(&message);
                set_form_data.update(|form| form.vendor_error = Some(message));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                focus_and_select_input(&vendor_input_ref_for_add);
                return;
            }
            // Validate vendor ID against booth rules
            if let Some(rule) = vendor_validation_rule.get() {
                if let Err(e) = validate_vendor_id(&trimmed, &rule) {
                    let error_msg = translate_domain_error(&e);
                    set_form_data.update(|form| form.vendor_error = Some(error_msg));
                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                    focus_and_select_input(&vendor_input_ref_for_add);
                    return;
                }
            }
            let omission_rules = vendor_omission_rules.get();
            if let Err(err) = omission_rules.validate() {
                let error_msg = translate_domain_error(&err);
                set_form_data.update(|form| form.vendor_error = Some(error_msg));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                focus_and_select_input(&vendor_input_ref_for_add);
                return;
            }
            match omission_rules.is_omitted(&trimmed) {
                Ok(true) => {
                    let error_msg = translate_domain_error(&DomainError::Validation(
                        ValidationError::VendorIdOmitted {
                            value: trimmed.clone(),
                        },
                    ));
                    set_form_data.update(|form| form.vendor_error = Some(error_msg));
                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                    focus_and_select_input(&vendor_input_ref_for_add);
                    return;
                }
                Ok(false) => {}
                Err(err) => {
                    let error_msg = translate_domain_error(&err);
                    set_form_data.update(|form| form.vendor_error = Some(error_msg));
                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                    focus_and_select_input(&vendor_input_ref_for_add);
                    return;
                }
            }
            trimmed
        };

        if data.current_amount.trim().is_empty() {
            let message = t!("checkout.errors.amount_required")();
            toast.warning(&message);
            set_form_data.update(|form| form.amount_error = Some(message));
            play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
            focus_and_select_input(&amount_input_ref_for_add);
            return;
        }

        let normalized_amount = data.current_amount.trim().to_string();

        match parse_decimal_input(&normalized_amount) {
            Ok(amount) => {
                if amount <= Decimal::ZERO {
                    let message = t!("checkout.errors.amount_positive")();
                    toast.warning(&message);
                    set_form_data.update(|form| {
                        form.amount_error = Some(message.clone());
                    });
                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                    focus_and_select_input(&amount_input_ref_for_add);
                    return;
                }

                if amount > MAX_ITEM_AMOUNT {
                    let message = t!("checkout.errors.amount_too_large")();
                    toast.warning(&message);
                    set_form_data.update(|form| {
                        form.amount_error = Some(message.clone());
                    });
                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                    focus_and_select_input(&amount_input_ref_for_add);
                    return;
                }

                if let Some(step) = amount_stepping.get_untracked() {
                    if let Err(DomainError::Validation(validation)) =
                        validate_amount_matches_step(amount, step)
                    {
                        let message = match validation {
                            ValidationError::AmountSteppingMismatch { .. } => {
                                translate_with_params(
                                    "checkout.errors.amount_stepping",
                                    HashMap::from([(
                                        "step",
                                        format_currency(step, locale.get_untracked()),
                                    )]),
                                )
                            }
                            _ => translate_domain_error(&DomainError::Validation(validation)),
                        };
                        toast.warning(&message);
                        set_form_data.update(|form| {
                            form.amount_error = Some(message.clone());
                        });
                        play_checkout_error_sound_if_enabled(
                            error_sound_enabled,
                            last_error_sound_at,
                        );
                        focus_and_select_input(&amount_input_ref_for_add);
                        return;
                    }
                }

                let mut new_items = data.items.clone();
                new_items.insert(
                    0,
                    CheckoutItem {
                        amount,
                        vendor_id: vendor_id_for_item.clone(),
                        product_id: None,
                        added_at: Utc::now(),
                    },
                );

                let locale = locale.get_untracked();
                let mode = amount_input_mode.get_untracked();
                set_form_data.set(CheckoutFormData {
                    current_amount: default_amount_for_mode(mode, locale),
                    items: new_items,
                    amount_error: None,
                    vendor_error: None,
                    ..data
                });

                if let Some(vendor_input) = vendor_input_ref_for_add.get() {
                    let _ = vendor_input.select();
                    let _ = vendor_input.focus();
                }

                if let Some(amount_input) = amount_input_ref_for_add.get() {
                    let _ = amount_input.set_value(&default_amount_for_mode(mode, locale));
                }

                toast.info(&t!("checkout.add_item_success")());
            }
            Err(err) => {
                let message = match err {
                    DecimalInputParseError::TooManyDecimalPlaces => {
                        t!("booth_form.errors.item_amount_too_many_decimals")()
                    }
                    _ => t!("checkout.errors.amount_invalid")(),
                };
                toast.error(&message);
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                set_form_data.update(|form| form.amount_error = Some(message));
                focus_and_select_input(&amount_input_ref_for_add);
            }
        }
    };

    let repeat_last_item = move || {
        let data = form_data.get();

        let Some(last_item) = data.items.first().cloned() else {
            toast.warning(&t!("checkout.repeat_error_no_items")());
            return;
        };

        if !is_repeat_focus_active(&vendor_input_ref_for_repeat, &amount_input_ref_for_repeat) {
            return;
        }

        if data.vendor_id.trim() != last_item.vendor_id {
            toast.warning(&t!("checkout.repeat_error_vendor_mismatch")());
            return;
        }

        item_delete_signal.set(None);
        set_purchase_to_delete.set(None);
        set_form_data.update(|form| {
            form.items.insert(
                0,
                CheckoutItem {
                    amount: last_item.amount,
                    vendor_id: last_item.vendor_id,
                    product_id: last_item.product_id,
                    added_at: Utc::now(),
                },
            );
        });

        toast.info(&t!("checkout.repeat_item_success")());
    };

    let add_product_item = move |product: Product| {
        let vendor_id = direct_sale_vendor_id_str.get();
        set_form_data.update(|data| {
            data.items.insert(
                0,
                CheckoutItem {
                    amount: product.price,
                    vendor_id,
                    product_id: Some(product.id),
                    added_at: Utc::now(),
                },
            );
        });
        toast.info(&t!("checkout.add_item_success")());
    };

    let handle_keyboard_key = {
        let locale = locale.clone();
        let set_form_data = set_form_data.clone();
        move |key: KeyboardKey| match active_input.get_untracked() {
            ActiveInput::VendorId => {
                let current = form_data.get_untracked().vendor_id;
                let (input_value, selection_start, selection_end) =
                    input_selection_range(&vendor_input_ref, &current);
                let next = match key {
                    KeyboardKey::Digit(digit) => replace_input_range(
                        &input_value,
                        selection_start,
                        selection_end,
                        &digit.to_string(),
                    ),
                    KeyboardKey::Backspace => {
                        backspace_input_range(&input_value, selection_start, selection_end)
                    }
                    KeyboardKey::Clear => String::new(),
                    _ => input_value.clone(),
                };
                let next_cursor = match key {
                    KeyboardKey::Digit(digit) => selection_start + digit.to_string().len() as u32,
                    KeyboardKey::Backspace => {
                        if selection_start != selection_end {
                            selection_start
                        } else {
                            selection_start.saturating_sub(1)
                        }
                    }
                    KeyboardKey::Clear => 0,
                    _ => selection_end,
                };
                update_vendor_input(
                    set_form_data,
                    next.clone(),
                    vendor_validation_rule.get_untracked(),
                    vendor_omission_rules.get_untracked(),
                    error_sound_enabled,
                    last_error_sound_at,
                );
                if let Some(input) = vendor_input_ref.get() {
                    input.set_value(&next);
                }
                focus_input_with_cursor(&vendor_input_ref, next_cursor);
            }
            ActiveInput::Amount => {
                let locale_value = locale.get_untracked();
                let current = form_data.get_untracked().current_amount;
                let next = match amount_input_mode.get_untracked() {
                    AmountInputMode::RightToLeft => match key {
                        KeyboardKey::Digit(digit) => rtl_add_digit(&current, digit, locale_value),
                        KeyboardKey::Backspace => rtl_backspace(&current, locale_value),
                        KeyboardKey::Clear => {
                            default_amount_for_mode(AmountInputMode::RightToLeft, locale_value)
                        }
                        KeyboardKey::Decimal => current,
                    },
                    AmountInputMode::Regular => match key {
                        KeyboardKey::Digit(digit) => format!("{}{}", current, digit),
                        KeyboardKey::Backspace => {
                            let mut chars = current.chars().collect::<Vec<_>>();
                            chars.pop();
                            chars.into_iter().collect::<String>()
                        }
                        KeyboardKey::Clear => String::new(),
                        KeyboardKey::Decimal => {
                            if current.contains(decimal_separator(locale_value)) {
                                current
                            } else {
                                format!("{}{}", current, decimal_separator(locale_value))
                            }
                        }
                    },
                };
                update_amount_input(
                    set_form_data,
                    next.clone(),
                    amount_stepping.get_untracked(),
                    locale.get_untracked(),
                    error_sound_enabled,
                    last_error_sound_at,
                );
                if let Some(input) = amount_input_ref.get() {
                    input.set_value(&next);
                }
                focus_and_select_input(&amount_input_ref);
            }
        }
    };

    let confirm_clear_form = move || {
        let items_present = !form_data.get().items.is_empty();
        if items_present {
            set_show_cancel_modal.set(true);
        } else {
            let locale = locale.get_untracked();
            let mode = amount_input_mode.get_untracked();
            let empty_form = CheckoutFormData {
                current_amount: default_amount_for_mode(mode, locale),
                ..CheckoutFormData::default()
            };
            set_form_data.set(empty_form.clone());
            let booth_id = selected_booth.get().map(|b| b.id.as_str());
            if let Err(err) = persist_form_data(booth_id, &empty_form) {
                error!("Failed to clear checkout draft: {}", err);
                toast.error(&t!("checkout.draft_save_failed")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
            }
        }
    };

    {
        let repeat_last_item = repeat_last_item.clone();
        let vendor_input_ref = vendor_input_ref.clone();
        let amount_input_ref = amount_input_ref.clone();

        if let Some(document) = window().and_then(|win| win.document()) {
            let handler = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == "+"
                    && is_repeat_focus_active(&vendor_input_ref, &amount_input_ref)
                {
                    event.prevent_default();
                    repeat_last_item();
                }
            }) as Box<dyn Fn(_)>);

            let _ = document
                .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());

            handler.forget();
        }
    }

    let handle_cancel_confirm = {
        let set_form_data = set_form_data.clone();
        let vendor_input_ref = vendor_input_ref.clone();
        let amount_input_ref = amount_input_ref.clone();
        move || {
            // Read current vendor_id from input field before clearing
            let current_vendor_id = if let Some(input) = vendor_input_ref.get() {
                input.value()
            } else {
                String::new()
            };

            let locale = locale.get_untracked();
            let mode = amount_input_mode.get_untracked();
            let mut new_form = CheckoutFormData {
                current_amount: default_amount_for_mode(mode, locale),
                ..CheckoutFormData::default()
            };
            new_form.vendor_id = current_vendor_id.clone();
            set_form_data.set(new_form);
            let booth_id = selected_booth.get().map(|b| b.id.as_str());
            if let Err(err) = persist_form_data(
                booth_id,
                &CheckoutFormData {
                    vendor_id: current_vendor_id.clone(),
                    current_amount: default_amount_for_mode(mode, locale),
                    ..Default::default()
                },
            ) {
                error!("Failed to persist checkout draft after cancel: {}", err);
                toast.error(&t!("checkout.draft_save_failed")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
            }

            // Set focus appropriately
            if current_vendor_id.trim().is_empty() {
                focus_and_select_input(&vendor_input_ref);
            } else {
                focus_and_select_input(&amount_input_ref);
            }
        }
    };

    let log_error_for_submit = log_error.clone();
    let submit_purchase = move || {
        let state_result = app_state.get();
        let booth = selected_booth.get();
        let mut data = form_data.get();

        if booth.is_none() {
            toast.warning(&t!("checkout.errors.booth_required")());
            return;
        }

        let Some(booth) = booth else {
            toast.error(&t!("checkout.errors.no_booth_selected")());
            play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
            return;
        };

        // For ThirdPartySale: validate and trim the form-level vendor ID
        if !is_direct_sale.get() {
            let trimmed_vendor_id = data.vendor_id.trim().to_string();
            if trimmed_vendor_id != data.vendor_id {
                let message = t!("checkout.info.vendor_trimmed")();
                toast.info(&message);
                let trimmed_clone = trimmed_vendor_id.clone();
                set_form_data.update(|form| form.vendor_id = trimmed_clone);
                data.vendor_id = trimmed_vendor_id;
            }

            let omission_rules = vendor_omission_rules.get();

            if let Err(err) = omission_rules.validate() {
                let message = translate_domain_error(&err);
                toast.warning(&message);
                set_form_data.update(|form| form.vendor_error = Some(message));
                focus_and_select_input(&vendor_input_ref_for_add);
                return;
            }

            match omission_rules.is_omitted(&data.vendor_id) {
                Ok(true) => {
                    let message = translate_domain_error(&DomainError::Validation(
                        ValidationError::VendorIdOmitted {
                            value: data.vendor_id.clone(),
                        },
                    ));
                    toast.warning(&message);
                    set_form_data.update(|form| form.vendor_error = Some(message));
                    focus_and_select_input(&vendor_input_ref_for_add);
                    return;
                }
                Ok(false) => {}
                Err(err) => {
                    let message = translate_domain_error(&err);
                    toast.warning(&message);
                    set_form_data.update(|form| form.vendor_error = Some(message));
                    focus_and_select_input(&vendor_input_ref_for_add);
                    return;
                }
            }
        }

        if data.items.is_empty() {
            toast.warning(&t!("checkout.errors.items_required")());
            return;
        }

        // Create purchase items with vendor_id from each item
        let purchase_items: Vec<PurchaseItem> = match data
            .items
            .into_iter()
            .map(|item| {
                let pi = PurchaseItem::new(item.amount, VendorId::new(item.vendor_id))?;
                Ok::<_, DomainError>(match item.product_id {
                    Some(pid) => pi.with_product(pid),
                    None => pi,
                })
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(items) => items,
            Err(err @ DomainError::Validation(_)) => {
                log_error_for_submit(ErrorLogDraft {
                    error_type: "checkout_validation_error".to_string(),
                    error_message: err.to_string(),
                    stack_trace: stack_trace(),
                    user_action: Some("build purchase items".to_string()),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: vec!["phase=purchase_items".to_string()],
                });
                toast.error(&translate_domain_error(&err));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            }
            Err(err) => {
                log_error_for_submit(ErrorLogDraft {
                    error_type: "checkout_build_purchase_items_failed".to_string(),
                    error_message: err.to_string(),
                    stack_trace: stack_trace(),
                    user_action: Some("build purchase items".to_string()),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: Vec::new(),
                });
                toast.error(&translate_with_params(
                    "checkout.errors.validation_failed",
                    HashMap::from([("error", err.to_string())]),
                ));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            }
        };

        let purchase = match Purchase::new(booth.id.clone(), purchase_items) {
            Ok(purchase) => purchase,
            Err(err @ DomainError::Validation(_)) => {
                log_error_for_submit(ErrorLogDraft {
                    error_type: "checkout_validation_error".to_string(),
                    error_message: err.to_string(),
                    stack_trace: stack_trace(),
                    user_action: Some("create purchase".to_string()),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: vec!["phase=purchase_create".to_string()],
                });
                toast.error(&translate_domain_error(&err));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            }
            Err(err) => {
                log_error_for_submit(ErrorLogDraft {
                    error_type: "checkout_create_purchase_failed".to_string(),
                    error_message: err.to_string(),
                    stack_trace: stack_trace(),
                    user_action: Some("create purchase".to_string()),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: Vec::new(),
                });
                toast.error(&translate_with_params(
                    "checkout.errors.validation_failed",
                    HashMap::from([("error", err.to_string())]),
                ));
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            }
        };

        if let Some(Ok(state)) = state_result {
            let booth_id_clone = booth.id.clone();
            let purchase_clone = purchase.clone();
            let vendor_input_ref_clone = vendor_input_ref_for_add.clone();
            let amount_input_ref_clone = amount_input_ref_for_add.clone();
            let log_error = log_error_for_submit.clone();
            is_submitting.set(true);
            spawn_local(async move {
                // Collect unique vendor IDs from all items
                let unique_vendor_ids: Vec<VendorId> = {
                    use std::collections::HashSet;
                    let mut ids: Vec<VendorId> = purchase_clone
                        .items
                        .iter()
                        .map(|item| item.vendor_id.clone())
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect();
                    ids.sort();
                    ids
                };

                // Get or create all vendors involved in this purchase
                for vendor_id in unique_vendor_ids {
                    let vendor_id_str = vendor_id.as_str().to_string();
                    let vendor_id_for_lookup = vendor_id_str.clone();
                    match state
                        .vendor_service
                        .get_or_create(booth_id_clone.clone(), vendor_id_for_lookup)
                        .await
                    {
                        Ok(_vendor) => {
                            // Vendor successfully retrieved or created
                        }
                        Err(e) => {
                            log_error(ErrorLogDraft {
                                error_type: "checkout_vendor_create_failed".to_string(),
                                error_message: e.to_string(),
                                stack_trace: stack_trace(),
                                user_action: Some("ensure vendor exists".to_string()),
                                route: current_route(),
                                vendor_id: Some(vendor_id_str.clone()),
                                purchase_id: None,
                                details: vec![format!("booth_id={}", booth_id_clone)],
                            });
                            let error_msg = translate_with_params(
                                "checkout.errors.vendor_create_failed",
                                HashMap::from([("error", format_error_message(&e))]),
                            );
                            toast.error(&error_msg);
                            play_checkout_error_sound_if_enabled(
                                error_sound_enabled,
                                last_error_sound_at,
                            );
                            is_submitting.set(false);
                            return;
                        }
                    }
                }

                match state.purchase_repository.save(&purchase_clone).await {
                    Ok(_) => {
                        // Reload first page to show new purchase
                        set_current_page.set(0);

                        // Clear input fields explicitly and focus vendor input for next checkout
                        if let Some(vendor_input) = vendor_input_ref_clone.get() {
                            let _ = vendor_input.set_value("");
                            let _ = vendor_input.focus();
                            let _ = vendor_input.select();
                        }
                        if let Some(amount_input) = amount_input_ref_clone.get() {
                            let _ = amount_input.set_value("");
                        }

                        toast.success(&t!("checkout.success")());
                        let locale = locale.get_untracked();
                        let mode = amount_input_mode.get_untracked();
                        let empty_form = CheckoutFormData {
                            current_amount: default_amount_for_mode(mode, locale),
                            ..CheckoutFormData::default()
                        };
                        set_form_data.set(empty_form.clone());
                        let booth_id = selected_booth.get().map(|b| b.id.as_str());
                        if let Err(err) = persist_form_data(booth_id, &empty_form) {
                            error!(
                                "Failed to clear checkout draft after purchase save: {}",
                                err
                            );
                            toast.error(&t!("checkout.draft_save_failed")());
                            play_checkout_error_sound_if_enabled(
                                error_sound_enabled,
                                last_error_sound_at,
                            );
                        }
                    }
                    Err(e) => {
                        log_error(ErrorLogDraft {
                            error_type: "checkout_save_purchase_failed".to_string(),
                            error_message: e.to_string(),
                            stack_trace: stack_trace(),
                            user_action: Some("save purchase".to_string()),
                            route: current_route(),
                            vendor_id: None,
                            purchase_id: Some(purchase_clone.id.as_str()),
                            details: vec![format!("booth_id={}", purchase_clone.booth_id)],
                        });
                        let error_msg = translate_with_params(
                            "checkout.errors.save_purchase_failed",
                            HashMap::from([("error", format_error_message(&e))]),
                        );
                        toast.error(&error_msg);
                        play_checkout_error_sound_if_enabled(
                            error_sound_enabled,
                            last_error_sound_at,
                        );
                    }
                }
                is_submitting.set(false);
            });
        }
    };

    let delete_purchase = move |purchase_id| {
        info!("delete_purchase called for purchase_id: {:?}", purchase_id);
        let token = confirmation_token_from_purchase(&purchase_id);
        set_pending_deletion.set(PendingDeletion {
            purchase_id: Some(purchase_id),
            token: token.clone(),
        });
        set_delete_confirmation_input.set(String::new());
        info!("Opening delete confirmation modal with token: {}", token);
        spawn_local(async move {
            if let Some(input) = delete_confirmation_ref.get_untracked() {
                let _ = input.set_value("");
                let _ = input.focus();
                let _ = input.select();
                info!("Delete confirmation input focused");
            } else {
                warn!("Delete confirmation input ref not available");
            }
        });
    };

    // Handle clicking on a purchase or its overlay
    // First click: arm the purchase for deletion (show red overlay)
    // Second click (on overlay): trigger deletion modal with token confirmation
    let handle_purchase_click = move |purchase_id: PurchaseId| {
        if purchase_to_delete.get() == Some(purchase_id) {
            // Clicking armed overlay - trigger deletion modal
            delete_purchase(purchase_id);
        } else {
            // Clicking normal purchase - arm it for deletion
            set_purchase_to_delete.set(Some(purchase_id));
        }
    };

    // Handle clicking on transaction card to expand/collapse details
    let handle_transaction_detail_click = move |purchase_id: PurchaseId| {
        // Priority 1: Don't interfere with armed deletion state
        if purchase_to_delete.get() == Some(purchase_id) {
            return; // Let deletion overlay handle the click
        }

        // Priority 2: Toggle expansion (accordion behavior)
        let current = expanded_purchase_id.get();
        if current == Some(purchase_id) {
            set_expanded_purchase_id.set(None); // Collapse if already expanded
        } else {
            set_expanded_purchase_id.set(Some(purchase_id)); // Expand, auto-collapsing any other
        }
    };

    let perform_delete_purchase = {
        let app_state = app_state.clone();
        let selected_booth = selected_booth.clone();
        let set_reload_toggle = set_reload_toggle.clone();
        let set_pending_deletion = set_pending_deletion.clone();
        let set_delete_confirmation_input = set_delete_confirmation_input.clone();
        let set_purchase_to_delete = set_purchase_to_delete.clone();
        let toast = toast.clone();
        let log_error = log_error.clone();
        move || {
            let pending = pending_deletion.get();
            if pending.purchase_id.is_none() {
                warn!("perform_delete_purchase called but no purchase_id in pending_deletion");
                return;
            }

            let Some(purchase_id) = pending.purchase_id else {
                warn!("perform_delete_purchase called but pending purchase id missing");
                toast.error(&t!("checkout.errors.invalid_delete_state")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            };
            let booth_id_opt = selected_booth.get().map(|b| b.id.clone());

            if booth_id_opt.is_none() {
                warn!("No booth selected, cannot delete purchase");
                toast.error(&t!("checkout.errors.no_booth_selected")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            }

            let Some(booth_id) = booth_id_opt else {
                toast.error(&t!("checkout.errors.no_booth_selected")());
                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                return;
            };
            info!(
                "perform_delete_purchase: deleting purchase_id: {:?} from booth: {:?}",
                purchase_id, booth_id
            );
            let state_result = app_state.get();

            if let Some(Ok(state)) = state_result {
                let log_error = log_error.clone();
                spawn_local(async move {
                    info!("Calling purchase_repository.delete_from_booth for purchase_id: {:?}, booth_id: {:?}", purchase_id, booth_id);
                    match state
                        .purchase_repository
                        .delete_from_booth(&booth_id, &purchase_id)
                        .await
                    {
                        Ok(_) => {
                            info!("Successfully deleted purchase_id: {:?}", purchase_id);

                            // Reset deletion state AFTER successful deletion
                            info!("Resetting deletion state signals after successful deletion");
                            set_pending_deletion.set(PendingDeletion::default());
                            set_delete_confirmation_input.set(String::new());
                            set_purchase_to_delete.set(None);

                            // Toggle reload signal to force purchase list refresh
                            set_reload_toggle.update(|v| *v = !*v);
                            info!("Toggled reload signal to refresh purchase list");
                            toast.success(&t!("checkout.delete_modal.success")());
                        }
                        Err(e) => {
                            error!("Failed to delete purchase_id {:?}: {:?}", purchase_id, e);
                            log_error(ErrorLogDraft {
                                error_type: "checkout_delete_purchase_failed".to_string(),
                                error_message: e.to_string(),
                                stack_trace: stack_trace(),
                                user_action: Some("delete purchase".to_string()),
                                route: current_route(),
                                vendor_id: None,
                                purchase_id: Some(purchase_id.as_str()),
                                details: vec![format!("booth_id={}", booth_id)],
                            });

                            // Reset deletion state even on error so user can retry
                            set_pending_deletion.set(PendingDeletion::default());
                            set_delete_confirmation_input.set(String::new());
                            set_purchase_to_delete.set(None);

                            let error_msg = translate_with_params(
                                "checkout.errors.delete_purchase_failed",
                                HashMap::from([("error", format_error_message(&e))]),
                            );
                            toast.error(&error_msg);
                            play_checkout_error_sound_if_enabled(
                                error_sound_enabled,
                                last_error_sound_at,
                            );
                        }
                    }
                });
            } else {
                warn!("App state not available for deletion");
                // Reset state if app state not available
                set_pending_deletion.set(PendingDeletion::default());
                set_delete_confirmation_input.set(String::new());
                set_purchase_to_delete.set(None);
            }
        }
    };
    let submit_purchase_action = StoredValue::new_local(submit_purchase);
    let perform_delete_purchase_action = StoredValue::new_local(perform_delete_purchase.clone());
    let armed_product_id: RwSignal<Option<ProductId>> = RwSignal::new(None);
    let delete_armed_pid: RwSignal<Option<ProductId>> = RwSignal::new(None);

    let cancel_delete_purchase = {
        let set_pending_deletion = set_pending_deletion.clone();
        let set_delete_confirmation_input = set_delete_confirmation_input.clone();
        let set_purchase_to_delete = set_purchase_to_delete.clone();
        move || {
            set_pending_deletion.set(PendingDeletion::default());
            set_delete_confirmation_input.set(String::new());
            set_purchase_to_delete.set(None);
        }
    };

    view! {
        <Container class="mt-6">
            <div
                class="space-y-6"
                on:click=move |_| {
                    item_delete_signal.set(None);
                    set_purchase_to_delete.set(None);
                }
            >
                <div class="flex flex-col gap-6 lg:flex-row">
                    <div class="flex-1 space-y-6">
                        <Card>
                            <div class="mb-4 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                                <h2 class="text-xl font-semibold">{t!("checkout.title")}</h2>
                                <div class="flex flex-wrap items-center gap-2 sm:justify-end">
                                    <button
                                        type="button"
                                        class="inline-flex items-center rounded-full border border-slate-200 bg-white/80 px-4 py-2 text-slate-700 shadow-sm backdrop-blur transition-colors hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                                        aria-label=move || {
                                            if keyboard_visible.get() {
                                                t!("checkout.keyboard_toggle_hide")()
                                            } else {
                                                t!("checkout.keyboard_toggle_show")()
                                            }
                                        }
                                        title=move || {
                                            if keyboard_visible.get() {
                                                t!("checkout.keyboard_toggle_hide")()
                                            } else {
                                                t!("checkout.keyboard_toggle_show")()
                                            }
                                        }
                                        aria-pressed=move || if keyboard_visible.get() { "true" } else { "false" }
                                        on:click=move |_| {
                                            set_keyboard_visible.update(|value| *value = !*value);
                                        }
                                    >
                                        <Icon icon=LuKeyboard class="h-5 w-5" />
                                    </button>
                                    <SoundToggle
                                        enabled=Signal::derive(move || error_sound_enabled.get())
                                        on_toggle=Callback::new(move |_| {
                                            set_error_sound_enabled.update(|value| *value = !*value);
                                        })
                                    />
                                    <Show when=move || !is_direct_sale.get()>
                                        <button
                                            type="button"
                                            class="inline-flex items-center rounded-full border border-slate-200 bg-white/80 px-4 py-2 text-slate-700 shadow-sm backdrop-blur transition-colors hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                                            aria-label=t!("checkout.rules_button_aria")()
                                            title=t!("checkout.rules_button_title")()
                                            on:click=move |_| {
                                                show_rules_modal.set(true);
                                            }
                                        >
                                            <Icon icon=LuInfo class="h-5 w-5" />
                                        </button>
                                    </Show>
                                </div>
                            </div>
                            <Show
                                when=move || selected_booth.get().is_none()
                            fallback=move || {
                                view! {
                                    <Show
                                        when=move || is_loading.get()
                                        fallback=move || {
                                            view! { <div class="space-y-6">

                                                // ── Mode toggle (DirectSale only) ───────────────
                                                <Show when=move || is_direct_sale.get()>
                                                    <div class="flex overflow-hidden rounded-lg border border-gray-300">
                                                        <button
                                                            type="button"
                                                            class=move || if checkout_mode.get() == CheckoutMode::PriceInput {
                                                                "flex-1 bg-blue-600 px-4 py-2 text-sm font-medium text-white"
                                                            } else {
                                                                "flex-1 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                                                            }
                                                            on:click=move |_| checkout_mode.set(CheckoutMode::PriceInput)
                                                        >
                                                            {t!("checkout.mode_price_input")}
                                                        </button>
                                                        <button
                                                            type="button"
                                                            class=move || if checkout_mode.get() == CheckoutMode::ProductButtons {
                                                                "flex-1 bg-blue-600 px-4 py-2 text-sm font-medium text-white"
                                                            } else {
                                                                "flex-1 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                                                            }
                                                            on:click=move |_| checkout_mode.set(CheckoutMode::ProductButtons)
                                                        >
                                                            {t!("checkout.mode_product_buttons")}
                                                        </button>
                                                    </div>
                                                </Show>

                                                // ── Product buttons (DirectSale + ProductButtons) ─
                                                <Show when=move || is_direct_sale.get() && checkout_mode.get() == CheckoutMode::ProductButtons>
                                                    <Show
                                                        when=move || !products_signal.get().is_empty()
                                                        fallback=move || view! {
                                                            <div class="rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-800">
                                                                {t!("checkout.no_products_warning")}
                                                            </div>
                                                        }
                                                    >
                                                        <div class="space-y-4">
                                                            {move || {
                                                                let all_products = products_signal.get();
                                                                product_groups_signal.get().into_iter().filter_map(|group| {
                                                                    let group_products: Vec<Product> = all_products.iter()
                                                                        .filter(|p| p.product_group_id == group.id)
                                                                        .cloned()
                                                                        .collect();
                                                                    if group_products.is_empty() { return None; }
                                                                    let btn_class = color_button_class(group.color);
                                                                    let header = format!("{} {}",
                                                                        group.emoji.as_deref().unwrap_or(""),
                                                                        group.name.clone()
                                                                    );
                                                                    Some(view! {
                                                                        <div>
                                                                            <p class="mb-2 text-sm font-medium text-gray-600">{header}</p>
                                                                            <div class="grid grid-cols-3 gap-2">
                                                                                {group_products.into_iter().map(|product| {
                                                                                    let p = product.clone();
                                                                                    let locale_val = locale.get();
                                                                                    let name = p.name.clone();
                                                                                    let price = format_currency(p.price, locale_val);
                                                                                    view! {
                                                                                        <button
                                                                                            type="button"
                                                                                            class=format!("{btn_class} rounded-lg px-3 py-2 min-h-[44px] font-medium text-sm hover:opacity-90 active:scale-95 active:opacity-75 transition-all duration-75 flex flex-col items-center justify-center")
                                                                                            on:click=move |_| {
                                                                                armed_product_id.set(None);
                                                                                delete_armed_pid.set(None);
                                                                                add_product_item(product.clone());
                                                                            }
                                                                                        >
                                                                                            <span class="font-semibold leading-tight">{name}</span>
                                                                                            <span class="text-xs opacity-70 leading-tight">{price}</span>
                                                                                        </button>
                                                                                    }
                                                                                }).collect_view()}
                                                                            </div>
                                                                        </div>
                                                                    })
                                                                }).collect_view()
                                                            }}
                                                        </div>
                                                    </Show>
                                                </Show>

                                                // ── Input section (ThirdPartySale; DirectSale + PriceInput) ─
                                                <Show when=move || !is_direct_sale.get() || checkout_mode.get() == CheckoutMode::PriceInput>
                                                <div class=move || if !is_direct_sale.get() { "grid gap-6 md:grid-cols-2 md:items-start" } else { "" }>
                                                    <Show when=move || !is_direct_sale.get()>
                                                    <div>
                                                    <label class="block text-sm font-medium text-gray-700 mb-1">
                                                        {t!("checkout.vendor_id")}
                                                    </label>
                                                    <input
                                                        class={move || format!(
                                                            "w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 {}",
                                                            if form_data.get().vendor_error.is_some() {
                                                                "border-red-500 focus:ring-red-500"
                                                            } else {
                                                                "border-gray-300"
                                                            }
                                                        )}
                                                        placeholder=move || t!("checkout.vendor_placeholder")()
                                                        value=move || form_data.get().vendor_id
                                                        node_ref=vendor_input_ref
                                                        on:focus=move |_| {
                                                            set_active_input.set(ActiveInput::VendorId);
                                                        }
                                                        on:input=move |ev| {
                                                            item_delete_signal.set(None);
                                                            set_purchase_to_delete.set(None);
                                                            let value = event_target_value(&ev);
                                                            update_vendor_input(
                                                                set_form_data,
                                                                value,
                                                                vendor_validation_rule.get(),
                                                                vendor_omission_rules.get(),
                                                                error_sound_enabled,
                                                                last_error_sound_at,
                                                            );
                                                        }
                                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                            if ev.key() == "Enter" {
                                                                ev.prevent_default();

                                                                let current_value = form_data.get().vendor_id;
                                                                let trimmed = current_value.trim().to_string();

                                                                if trimmed != current_value {
                                                                    let message = t!("checkout.info.vendor_trimmed")();
                                                                    toast.info(&message);
                                                                    set_form_data.update(|data| data.vendor_id = trimmed.clone());
                                                                    if let Some(input) = vendor_input_ref.get() {
                                                                        input.set_value(&trimmed);
                                                                    }
                                                                }

                                                                if trimmed.is_empty() {
                                                                    let message = t!("checkout.errors.vendor_required")();
                                                                    toast.warning(&message);
                                                                    set_form_data.update(|data| data.vendor_error = Some(message));
                                                                    play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                                                                    if let Some(input) = vendor_input_ref.get() {
                                                                        let _ = input.focus();
                                                                        let _ = input.select();
                                                                    }
                                                                } else {
                                                                    // Validate against booth rules before advancing
                                                                    let omission_check = vendor_omission_rules.get().is_omitted(&trimmed);
                                                                    let is_valid = if let Some(rule) = vendor_validation_rule.get() {
                                                                        validate_vendor_id(&trimmed, &rule).is_ok()
                                                                            && omission_check
                                                                                .as_ref()
                                                                                .map(|is_omitted| !*is_omitted)
                                                                                .unwrap_or(false)
                                                                    } else {
                                                                        // No booth selected - treat as invalid
                                                                        false
                                                                    };

                                                                    if is_valid {
                                                                        // Valid: clear error and advance to amount field
                                                                        set_form_data.update(|data| data.vendor_error = None);
                                                                        if let Some(amount_input) = amount_input_ref.get() {
                                                                            let _ = amount_input.focus();
                                                                            let _ = amount_input.select();
                                                                        }
                                                                    } else {
                                                                        // Invalid: keep focus on vendor, select text, ensure error is set
                                                                        if let Some(rule) = vendor_validation_rule.get() {
                                                                            if let Err(e) = validate_vendor_id(&trimmed, &rule) {
                                                                                let error_msg = translate_domain_error(&e);
                                                                                set_form_data.update(|data| {
                                                                                    data.vendor_error = Some(error_msg);
                                                                                });
                                                                                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                                                                            } else if let Ok(true) = omission_check {
                                                                                let error_msg = translate_domain_error(&DomainError::Validation(
                                                                                    ValidationError::VendorIdOmitted {
                                                                                        value: trimmed.clone(),
                                                                                    },
                                                                                ));
                                                                                set_form_data.update(|data| {
                                                                                    data.vendor_error = Some(error_msg);
                                                                                });
                                                                                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                                                                            } else if let Err(err) = omission_check {
                                                                                let error_msg = translate_domain_error(&err);
                                                                                set_form_data.update(|data| {
                                                                                    data.vendor_error = Some(error_msg);
                                                                                });
                                                                                play_checkout_error_sound_if_enabled(error_sound_enabled, last_error_sound_at);
                                                                            }
                                                                        }
                                                                        if let Some(input) = vendor_input_ref.get() {
                                                                            let _ = input.focus();
                                                                            let _ = input.select();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        on:blur=move |_| {
                                                            // Auto-trim and re-validate on blur
                                                            let current_value = form_data.get().vendor_id;
                                                            let trimmed = current_value.trim().to_string();

                                                            if trimmed != current_value {
                                                                set_form_data.update(|data| data.vendor_id = trimmed.clone());
                                                                if let Some(input) = vendor_input_ref.get() {
                                                                    input.set_value(&trimmed);
                                                                }
                                                            }

                                                            // Re-validate after trimming
                                                            if trimmed.is_empty() {
                                                                set_form_data.update(|data| data.vendor_error = None);
                                                            } else if let Some(rule) = vendor_validation_rule.get() {
                                                                match validate_vendor_id(&trimmed, &rule) {
                                                                    Ok(()) => {
                                                                        match vendor_omission_rules.get().is_omitted(&trimmed) {
                                                                            Ok(true) => {
                                                                                let error_msg = translate_domain_error(&DomainError::Validation(
                                                                                    ValidationError::VendorIdOmitted {
                                                                                        value: trimmed.clone(),
                                                                                    },
                                                                                ));
                                                                                set_form_data.update(|data| {
                                                                                    data.vendor_error = Some(error_msg);
                                                                                });
                                                                            }
                                                                            Ok(false) => {
                                                                                set_form_data.update(|data| data.vendor_error = None);
                                                                            }
                                                                            Err(err) => {
                                                                                let error_msg = translate_domain_error(&err);
                                                                                set_form_data.update(|data| {
                                                                                    data.vendor_error = Some(error_msg);
                                                                                });
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        let error_msg = translate_domain_error(&e);
                                                                        set_form_data.update(|data| {
                                                                            data.vendor_error = Some(error_msg);
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    />
                                                    <Show when=move || form_data.get().vendor_error.is_some()>
                                                        <p class="mt-1 text-sm text-red-600">{move || form_data.get().vendor_error.clone().unwrap_or_default()}</p>
                                                    </Show>
                                                    </div>
                                                    </Show>

                                                    <div>
                                                    <label class="mb-1 block text-sm font-medium text-gray-700">
                                                        {t!("checkout.amount")}
                                                    </label>
                                                    <div class="flex flex-col gap-2">
                                                        <input
                                                            class={move || format!(
                                                                "flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 {}",
                                                                if form_data.get().amount_error.is_some() {
                                                                    "border-red-500 focus:ring-red-500"
                                                                } else {
                                                                    "border-gray-300"
                                                                }
                                                            )}
                                                            placeholder=move || t!("checkout.amount_placeholder")()
                                                            inputmode="decimal"
                                                            value=move || form_data.get().current_amount
                                                            node_ref=amount_input_ref
                                                            on:focus=move |_| {
                                                                set_active_input.set(ActiveInput::Amount);
                                                            }
                                                            on:input=move |ev| {
                                                                item_delete_signal.set(None);
                                                                set_purchase_to_delete.set(None);
                                                                let value = event_target_value(&ev);
                                                                let locale_value = locale.get();
                                                                let sanitized_value =
                                                                    sanitize_amount_input(
                                                                        &value,
                                                                        decimal_separator(
                                                                            locale_value,
                                                                        ),
                                                                    );
                                                                let previous = form_data.get().current_amount;
                                                                let next = match amount_input_mode.get() {
                                                                    AmountInputMode::Regular => sanitized_value.clone(),
                                                                    AmountInputMode::RightToLeft => {
                                                                        if sanitized_value.len() < previous.len() {
                                                                            rtl_backspace(&previous, locale_value)
                                                                        } else {
                                                                            let added_digit = sanitized_value
                                                                                .chars()
                                                                                .rev()
                                                                                .find(|ch| ch.is_ascii_digit())
                                                                                .and_then(|ch| ch.to_digit(10))
                                                                                .map(|digit| digit as u8);

                                                                            if let Some(digit) = added_digit {
                                                                                rtl_add_digit(&previous, digit, locale_value)
                                                                            } else {
                                                                                previous
                                                                            }
                                                                        }
                                                                    }
                                                                };
                                                                update_amount_input(
                                                                    set_form_data,
                                                                    next,
                                                                    amount_stepping.get_untracked(),
                                                                    locale.get_untracked(),
                                                                    error_sound_enabled,
                                                                    last_error_sound_at,
                                                                );
                                                            }
                                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                                let key = ev.key();

                                                                if key == "Enter" {
                                                                    ev.prevent_default();
                                                                    add_item();
                                                                    return;
                                                                }

                                                                if amount_input_mode.get() == AmountInputMode::RightToLeft {
                                                                    if key == "Backspace" {
                                                                        ev.prevent_default();
                                                                        handle_keyboard_key(KeyboardKey::Backspace);
                                                                    } else if key == "Delete" {
                                                                        ev.prevent_default();
                                                                        handle_keyboard_key(KeyboardKey::Clear);
                                                                    } else if key.len() == 1 {
                                                                        if let Some(digit) = key.chars().next().and_then(|ch| ch.to_digit(10)) {
                                                                            ev.prevent_default();
                                                                            handle_keyboard_key(KeyboardKey::Digit(digit as u8));
                                                                        } else {
                                                                            ev.prevent_default();
                                                                        }
                                                                    }
                                                                } else {
                                                                    let current_value = form_data.get().current_amount;
                                                                    let decimal_sep = decimal_separator(locale.get());

                                                                    if !is_allowed_amount_key(&key, &current_value, decimal_sep)
                                                                    {
                                                                        ev.prevent_default();
                                                                    }
                                                                }
                                                            }
                                                        />
                                                        <Show when=move || form_data.get().amount_error.is_some()>
                                                            <p class="text-sm text-red-600">{move || form_data.get().amount_error.clone().unwrap_or_default()}</p>
                                                        </Show>
                                                    </div>
                                                    </div>
                                                </div>

                                                <Show when=move || keyboard_visible.get()>
                                                    <OnScreenKeyboard
                                                        is_visible=Signal::derive(move || keyboard_visible.get())
                                                        on_key=Callback::new(handle_keyboard_key)
                                                        on_mode_change=Callback::new({
                                                            let amount_input_mode = amount_input_mode;
                                                            let set_amount_input_mode = set_amount_input_mode;
                                                            let set_form_data = set_form_data;
                                                            let form_data = form_data;
                                                            let amount_input_ref = amount_input_ref;
                                                            let locale = locale.clone();
                                                            move |_| {
                                                                let next_mode = match amount_input_mode.get() {
                                                                    AmountInputMode::RightToLeft => AmountInputMode::Regular,
                                                                    AmountInputMode::Regular => AmountInputMode::RightToLeft,
                                                                };
                                                                let locale_value = locale.get();
                                                                let current_amount = form_data.get().current_amount;
                                                                let normalized_amount = normalize_amount_for_mode(
                                                                    &current_amount,
                                                                    next_mode,
                                                                    locale_value,
                                                                );
                                                                set_amount_input_mode.set(next_mode);
                                                                set_form_data.update(|data| {
                                                                    data.current_amount = normalized_amount.clone();
                                                                    data.amount_error = validate_inline_amount(
                                                                        &normalized_amount,
                                                                        amount_stepping.get_untracked(),
                                                                        locale.get_untracked(),
                                                                    );
                                                                });
                                                                if let Some(amount_input) = amount_input_ref.get() {
                                                                    amount_input.set_value(&normalized_amount);
                                                                    let _ = amount_input.focus();
                                                                }
                                                            }
                                                        })
                                                        current_mode=Signal::derive(move || amount_input_mode.get())
                                                        locale=locale.get()
                                                    />
                                                </Show>
                                                </Show>

                                                {/* Action buttons - side by side on desktop, stacked on mobile */}
                                                <div class="flex flex-col sm:flex-row gap-4">
                                                    <Show when=move || !is_direct_sale.get() || checkout_mode.get() == CheckoutMode::PriceInput>
                                                        <button
                                                            type="button"
                                                            on:click=move |_| {
                                                                item_delete_signal.set(None);
                                                                set_purchase_to_delete.set(None);
                                                                add_item();
                                                            }
                                                            title=move || t!("checkout.add_item")()
                                                            aria-label=move || t!("checkout.add_item")()
                                                            class="inline-flex items-center justify-center rounded-lg bg-gray-200 px-4 py-2 text-base font-medium text-gray-900 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 hover:bg-gray-300 sm:flex-[1]"
                                                        >
                                                            <Icon icon=LuPlus class="w-8 h-8" />
                                                        </button>

                                                        <button
                                                            type="button"
                                                            disabled=move || !can_repeat.get()
                                                            on:mousedown=move |ev| ev.prevent_default()
                                                            on:click=move |_| repeat_last_item()
                                                            title=move || t!("checkout.repeat_item")()
                                                            aria-label=move || t!("checkout.repeat_item")()
                                                            class="inline-flex items-center justify-center gap-2 rounded-lg bg-gray-200 px-4 py-2 text-base font-medium text-gray-900 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 hover:bg-gray-300 disabled:cursor-not-allowed disabled:opacity-50 sm:flex-[1]"
                                                        >
                                                            <Icon icon=LuCopy class="w-8 h-8" />
                                                        </button>
                                                    </Show>

                                                    <Button
                                                        variant=ButtonVariant::Success
                                                        class="sm:flex-[3] shadow-lg ring-2 ring-green-300/50".to_string()
                                                        disabled=Signal::derive(move || is_submitting.get())
                                                        on_click=Box::new(move || {
                                                            item_delete_signal.set(None);
                                                            set_purchase_to_delete.set(None);
                                                            submit_purchase_action.with_value(|submit| submit());
                                                        })
                                                    >
                                                        <span class="inline-flex items-center justify-center gap-4">
                                                        <Icon icon=LuWallet class="w-8 h-8" />
                                                            <span class="text-2xl font-semibold">{move || {
                                                                let locale = use_locale().get();
                                                                format_currency(form_data.get().total(), locale)
                                                            }}</span>
                                                        </span>
                                                    </Button>
                                                </div>
                                            </div>
                                        }
                                        }
                                        >
                                            <p class="text-gray-600">{t!("checkout.loading_message")}</p>
                                        </Show>
                                }
                            }
                            >
                                <p class="text-gray-600">{t!("checkout.prompt_select_booth")}</p>
                            </Show>
                        </Card>

                        <Card>
                            {/* Custom header with title and Clear Items button */}
                            <div class="flex items-center justify-between mb-4">
                                <h2 class="text-xl font-semibold">{t!("checkout.current_items")}</h2>
                                <Show when=move || !form_data.get().items.is_empty()>
                                    <Button
                                        variant=ButtonVariant::Danger
                                        on_click=Box::new(move || confirm_clear_form())
                                        class="px-3 py-2 !bg-red-100 hover:!bg-red-200 !text-red-700".to_string()
                                        aria_label={t!("checkout.confirm_cancel_confirm")()}
                                    >
                                        {/* Trash icon */}
                                        <Icon icon=LuTrash2 class="w-6 h-6" />
                                    </Button>
                                </Show>
                            </div>
                            <div class="space-y-2"
                                on:click=move |_| {
                                    armed_product_id.set(None);
                                    delete_armed_pid.set(None);
                                }
                            >
                                <Show
                                    when=move || form_data.get().items.is_empty()
                                    fallback=move || {
                                        let data = form_data.get();
                                        let items = data.items;

                                        // Build product groups + collect manual items
                                        let prod_map: std::collections::HashMap<ProductId, String> = {
                                            products_signal.get().into_iter().map(|p| (p.id, p.name)).collect()
                                        };
                                        // (product_id, name, unit_price, count)
                                        let mut product_groups: Vec<(ProductId, String, Decimal, usize)> = Vec::new();
                                        let mut manual_items: Vec<(usize, CheckoutItem)> = Vec::new();
                                        for (idx, item) in items.iter().enumerate() {
                                            if let Some(pid) = item.product_id {
                                                if let Some(g) = product_groups.iter_mut().find(|g| g.0 == pid) {
                                                    g.3 += 1;
                                                } else {
                                                    let name = prod_map.get(&pid).cloned().unwrap_or_else(|| pid.as_str());
                                                    product_groups.push((pid, name, item.amount, 1));
                                                }
                                            } else {
                                                manual_items.push((idx, item.clone()));
                                            }
                                        }
                                        let total_manual = manual_items.len();

                                        view! {
                                            <p class="text-xs text-gray-500 mb-3 px-1">
                                                {t!("checkout.items_list_hint")}
                                            </p>
                                            <ul class="space-y-2">
                                                // ── Grouped product items ──────────────────
                                                {product_groups.into_iter().map(|(pid, name, unit_price, count)| {
                                                    let total = unit_price * rust_decimal::Decimal::from(count as u64);
                                                    view! {
                                                        <li
                                                            class="relative text-sm p-2 border rounded-lg bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors select-none"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                item_delete_signal.set(None);
                                                                if armed_product_id.get() == Some(pid) {
                                                                    armed_product_id.set(None);
                                                                } else {
                                                                    armed_product_id.set(Some(pid));
                                                                }
                                                            }
                                                        >
                                                            <div class="flex items-center justify-between pointer-events-none">
                                                                <div>
                                                                    <p class="font-medium">{name.clone()}</p>
                                                                    <p class="text-xs text-gray-500">{
                                                                        let locale = use_locale().get();
                                                                        format!("{count}× {}", format_currency(unit_price, locale))
                                                                    }</p>
                                                                </div>
                                                                <span class="font-semibold">{
                                                                    let locale = use_locale().get();
                                                                    format_currency(total, locale)
                                                                }</span>
                                                            </div>
                                                            <Show when=move || armed_product_id.get() == Some(pid)>
                                                                <div
                                                                    class="absolute inset-0 rounded-lg z-10 flex items-center justify-center gap-3 px-3"
                                                                    style="background: rgba(0,0,0,0.65); backdrop-filter: blur(2px);"
                                                                    on:click=move |e| e.stop_propagation()
                                                                >
                                                                    <button
                                                                        type="button"
                                                                        class="text-white rounded-full w-9 h-9 flex items-center justify-center bg-white/20 hover:bg-white/30 active:scale-95 transition-all pointer-events-auto"
                                                                        on:click=move |e| {
                                                                            e.stop_propagation();
                                                                            set_form_data.update(|data| {
                                                                                let remaining = data.items.iter().filter(|i| i.product_id == Some(pid)).count();
                                                                                if remaining <= 1 {
                                                                                    data.items.retain(|i| i.product_id != Some(pid));
                                                                                    armed_product_id.set(None);
                                                                                } else if let Some(idx) = data.items.iter().position(|i| i.product_id == Some(pid)) {
                                                                                    data.items.remove(idx);
                                                                                }
                                                                            });
                                                                        }
                                                                    >
                                                                        <Icon icon=LuMinus class="w-5 h-5" />
                                                                    </button>
                                                                    <span class="text-white font-bold text-base min-w-[1.5rem] text-center">{count}</span>
                                                                    <button
                                                                        type="button"
                                                                        class="text-white rounded-full w-9 h-9 flex items-center justify-center bg-white/20 hover:bg-white/30 active:scale-95 transition-all pointer-events-auto"
                                                                        on:click=move |e| {
                                                                            e.stop_propagation();
                                                                            let vendor_id = direct_sale_vendor_id_str.get();
                                                                            set_form_data.update(|data| {
                                                                                data.items.insert(0, CheckoutItem {
                                                                                    amount: unit_price,
                                                                                    vendor_id,
                                                                                    product_id: Some(pid),
                                                                                    added_at: Utc::now(),
                                                                                });
                                                                            });
                                                                        }
                                                                    >
                                                                        <Icon icon=LuPlus class="w-5 h-5" />
                                                                    </button>
                                                                    <div class="flex-1" />
                                                                    <button
                                                                        type="button"
                                                                        class=move || format!(
                                                                            "text-white rounded-full w-9 h-9 flex items-center justify-center active:scale-95 transition-all pointer-events-auto {}",
                                                                            if delete_armed_pid.get() == Some(pid) { "bg-red-600 ring-2 ring-white" } else { "bg-red-500/70 hover:bg-red-500/90" }
                                                                        )
                                                                        on:click=move |e| {
                                                                            e.stop_propagation();
                                                                            if delete_armed_pid.get() == Some(pid) {
                                                                                set_form_data.update(|data| {
                                                                                    data.items.retain(|i| i.product_id != Some(pid));
                                                                                });
                                                                                armed_product_id.set(None);
                                                                                delete_armed_pid.set(None);
                                                                            } else {
                                                                                delete_armed_pid.set(Some(pid));
                                                                            }
                                                                        }
                                                                    >
                                                                        <Icon icon=LuTrash2 class="w-5 h-5" />
                                                                    </button>
                                                                </div>
                                                            </Show>
                                                        </li>
                                                    }
                                                }).collect_view()}

                                                // ── Manual items (unchanged behaviour) ────────
                                                {manual_items.into_iter().enumerate().map(move |(manual_idx, (global_idx, item))| {
                                                    let display_number = total_manual - manual_idx;
                                                    let vendor_label = if item.vendor_id.trim().is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        item.vendor_id.clone()
                                                    };
                                                    view! {
                                                        <li
                                                            class="relative text-sm p-2 border rounded-lg bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors select-none"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                armed_product_id.set(None);
                                                                if item_delete_signal.get() == Some(global_idx) {
                                                                    set_form_data.update(|data| {
                                                                        if global_idx < data.items.len() {
                                                                            data.items.remove(global_idx);
                                                                        }
                                                                    });
                                                                    item_delete_signal.set(None);
                                                                } else {
                                                                    item_delete_signal.set(Some(global_idx));
                                                                }
                                                            }
                                                        >
                                                            <div class="flex items-start justify-between pointer-events-none">
                                                                <div>
                                                                    <p class="font-medium">{format!("{} {}", t!("checkout.vendor_label")(), vendor_label)}</p>
                                                                    <p class="text-xs text-gray-500">{format!("{} {}", t!("checkout.item_label")(), display_number)}</p>
                                                                </div>
                                                                <div class="text-right">
                                                                    <span class="block font-semibold">{
                                                                        let locale = use_locale().get();
                                                                        format_currency(item.amount, locale)
                                                                    }</span>
                                                                    <p class="text-xs text-gray-400" title={
                                                                        let locale = use_locale().get();
                                                                        format_item_tooltip(item.added_at, locale)
                                                                    }>{
                                                                        let locale = use_locale().get();
                                                                        format!("{}", format_item_timestamp(item.added_at, locale))
                                                                    }</p>
                                                                </div>
                                                            </div>
                                                            <Show when=move || item_delete_signal.get() == Some(global_idx)>
                                                                <DeleteOverlay
                                                                    prompt={t!("checkout.remove_item_confirm")()}
                                                                    aria_label={t!("checkout.remove_item_confirm")()}
                                                                    on_click={move |_| {
                                                                        if item_delete_signal.get() == Some(global_idx) {
                                                                            set_form_data.update(|data| {
                                                                                if global_idx < data.items.len() {
                                                                                    data.items.remove(global_idx);
                                                                                }
                                                                            });
                                                                            item_delete_signal.set(None);
                                                                        } else {
                                                                            item_delete_signal.set(Some(global_idx));
                                                                        }
                                                                    }}
                                                                />
                                                            </Show>
                                                        </li>
                                                    }
                                                }).collect_view()}
                                            </ul>
                                        }
                                    }
                                >
                                    <div class="flex flex-col items-center justify-center py-8 text-center">
                                        <Icon icon=LuInbox class="mb-3 h-12 w-12 text-gray-300" />
                                        <p class="text-sm font-medium text-gray-700">{t!("checkout.no_items")}</p>
                                        <p class="mt-1 text-xs text-gray-500">{t!("checkout.empty_state_hint")}</p>
                                    </div>
                                </Show>
                            </div>
                        </Card>
                    </div>

                    <div class="flex-1 space-y-6">
                        <Show when=move || { partial_recovery_count.get() > 0 }>
                            <Card>
                                <div class="rounded-xl border border-amber-300 bg-amber-50 px-4 py-4 text-amber-950">
                                    <div class="flex items-start gap-3">
                                        <div class="mt-0.5 shrink-0 text-amber-700">
                                            <Icon icon=LuAlertTriangle class="h-5 w-5" />
                                        </div>
                                        <div class="space-y-2">
                                            <p class="text-sm font-semibold">{t!("checkout.recovery.partial_data_title")()}</p>
                                            <p class="text-sm leading-6">
                                                {move || {
                                                    translate_with_params(
                                                        "checkout.recovery.partial_data_warning",
                                                        HashMap::from([(
                                                            "count",
                                                            partial_recovery_count.get().to_string(),
                                                        )]),
                                                    )
                                                }}
                                            </p>
                                            <ul class="list-disc space-y-1 pl-5 text-sm text-amber-900">
                                                <li>{t!("checkout.recovery.partial_data_step_review")()}</li>
                                                <li>{t!("checkout.recovery.partial_data_step_refresh")()}</li>
                                                <li>{t!("checkout.recovery.partial_data_step_stop")()}</li>
                                            </ul>
                                        </div>
                                    </div>
                                </div>
                            </Card>
                        </Show>

                        <Card title_view={t!("checkout.running_totals_title").into_any()}>
                            <div class="space-y-4">
                                <div class="flex justify-between">
                                    <span class="text-gray-600">{t!("checkout.running_totals.sales")}</span>
                                    <span class="text-lg font-semibold">{move || {
                                        let locale = use_locale().get();
                                        format_currency(running_totals.get().0, locale)
                                    }}</span>
                                </div>
                                <div class="flex justify-between">
                                    <span class="text-gray-600">{t!("checkout.running_totals.items")}</span>
                                    <span class="text-lg font-semibold">{move || running_totals.get().1.to_string()}</span>
                                </div>
                                <div class="flex justify-between">
                                    <span class="text-gray-600">{t!("checkout.running_totals.checkouts")}</span>
                                    <span class="text-lg font-semibold">{move || running_totals.get().2.to_string()}</span>
                                </div>
                            </div>
                        </Card>

                        <Card title_view={t!("checkout.recent_transactions_title").into_any()}>
                            <Show
                                when=move || purchases.get().is_empty() && !is_loading.get()
                                fallback=move || {
                                    view! {
                                        <div class="space-y-2">
                                        {/* Explanatory hint text */}
                                            <p class="text-xs text-gray-500 mb-3 px-1">
                                                {t!("checkout.transactions_hint")}
                                            </p>
                                            <div class="mb-3 rounded-lg border border-gray-200 bg-gray-50 px-3 py-3 text-xs text-gray-600">
                                                <p class="font-medium text-gray-700">{t!("checkout.delete_guidance.title")()}</p>
                                                <p class="mt-1">{t!("checkout.delete_guidance.body")()}</p>
                                            </div>

                                            {/* Top pagination controls */}
                                            <Show when=move || {
                                                let total_count = total_purchase_count.get();
                                                let page_size_val = page_size.get();
                                                let total_pages = if page_size_val > 0 {
                                                    (total_count + page_size_val - 1) / page_size_val
                                                } else {
                                                    0
                                                };
                                                total_pages > 1
                                            }>
                                                <div class="mb-4">
                                                    <Pagination
                                                        current_page=current_page
                                                        total_items=Signal::derive(move || total_purchase_count.get())
                                                        page_size=page_size
                                                        on_page_change=move |page| set_current_page.set(page)
                                                        on_page_size_change=move |size| {
                                                            set_page_size.set(size);
                                                            set_current_page.set(0); // Reset to first page
                                                        }
                                                        translation_prefix="checkout.pagination"
                                                        show_page_size_selector=true
                                                    />
                                                </div>
                                            </Show>

                                            {move || purchases.get().into_iter().map(|purchase| {
                                                let amount = purchase.total_amount();
                                                let purchase_id = purchase.id;
                                                let purchase_id_label = purchase_id.as_str().to_string();
                                                let item_count = purchase.items.len();
                                                let items_label = if item_count == 1 {
                                                    t!("checkout.recent.items_label_one")()
                                                } else {
                                                    translate_with_params(
                                                        "checkout.recent.items_label",
                                                        HashMap::from([(
                                                            "count",
                                                            item_count.to_string(),
                                                        )]),
                                                    )
                                                };
                                                view! {
                                                    <div
                                                        class=move || format!(
                                                            "relative group text-sm border rounded-lg bg-gray-50 transition-all duration-300 select-none {}",
                                                            if expanded_purchase_id.get() == Some(purchase_id) {
                                                                "border-blue-500 shadow-sm" // Active state: blue border
                                                            } else {
                                                                "border-gray-200 hover:border-blue-300" // Default/hover state
                                                            }
                                                        )
                                                    >
                                                        {/* Card header - clickable area */}
                                                        <div
                                                            class=move || format!(
                                                                "p-3 transition-colors duration-300 cursor-pointer {}",
                                                                if expanded_purchase_id.get() == Some(purchase_id) {
                                                                    "bg-blue-50" // Active state: blue background
                                                                } else {
                                                                    ""
                                                                }
                                                            )
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                handle_transaction_detail_click(purchase_id);
                                                            }
                                                        >

                                                            {/* Purchase content */}
                                                            <div class="flex items-center justify-between pr-12">
                                                                <div class="space-y-1">
                                                                    <p class="text-sm font-semibold">{items_label.clone()}</p>
                                                                     <p class="text-xs text-gray-500">
                                                                        {let locale = use_locale().get();
                                                                        translate_with_params(
                                                                            "checkout.recent.timestamp",
                                                                            HashMap::from([(
                                                                                "datetime",
                                                                                format_purchase_timestamp(purchase.timestamp, locale)
                                                                            )])
                                                                        )
                                                                        }
                                                                     </p>
                                                                    <p class="text-xs text-gray-500 font-mono break-all">
                                                                        {translate_with_params(
                                                                            "checkout.recent.purchase_id",
                                                                            HashMap::from([(
                                                                                "id",
                                                                                purchase_id_label.clone()
                                                                            )])
                                                                        )}
                                                                    </p>
                                                                </div>
                                                                <div>
                                                                    <span class="text-lg font-bold">{
                                                                        let locale = use_locale().get();
                                                                        format_currency(amount, locale)
                                                                    }</span>
                                                                </div>
                                                            </div>
                                                        </div>

                                                        {/* Expanded detail section */}
                                                        <Show when=move || expanded_purchase_id.get() == Some(purchase_id)>
                                                            <div
                                                                class="pl-6 pr-14 pb-4 pt-3 space-y-3 animate-in slide-in-from-top-2 duration-300"
                                                                style="border-top: 1px dashed #e5e7eb;" // Subtle dashed separator
                                                            >
                                                                {/* Items list with vendor per item */}
                                                                <div class="space-y-2">
                                                                    {
                                                                        let vendor_label = format!("{}: ", t!("checkout.vendor_label")());
                                                                        purchase.items.iter().enumerate().map(|(idx, item)| {
                                                                            let position_num = idx + 1;
                                                                            let locale = use_locale().get();
                                                                            let vendor_text = format!("{}{}", vendor_label, item.vendor_id.as_str());
                                                                            view! {
                                                                                <div class="py-2 border-b border-gray-100 last:border-0">
                                                                                    {/* First line: Vendor and amount */}
                                                                                    <div class="flex justify-between text-sm">
                                                                                        <span class="font-medium text-gray-900">
                                                                                            {vendor_text}
                                                                                        </span>
                                                                                        <span class="font-medium text-gray-900">
                                                                                            {format_currency(item.amount, locale)}
                                                                                        </span>
                                                                                    </div>

                                                                                    {/* Second line: Item number */}
                                                                                    <div class="text-xs text-gray-500 mt-0.5">
                                                                                        {translate_with_params(
                                                                                            "checkout.transaction_detail.item_number",
                                                                                            HashMap::from([("number", position_num.to_string())])
                                                                                        )}
                                                                                    </div>
                                                                                </div>
                                                                            }
                                                                        }).collect_view()
                                                                    }
                                                                </div>
                                                            </div>
                                                        </Show>

                                                        {/* Separator + Delete Icon (hover-visible on desktop, always visible on mobile) */}
                                                        <div
                                                            class="absolute right-0 top-0 h-full flex items-center
                                                                   opacity-0 group-hover:opacity-100 sm:opacity-100 
                                                                   transition-opacity cursor-pointer"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                handle_purchase_click(purchase_id);
                                                            }
                                                            aria-label=move || t!("checkout.confirm_cancel_confirm")()
                                                        >
                                                            {/* Vertical separator */}
                                                            <div class="h-full w-px bg-gray-300"></div>

                                                            {/* Delete icon area */}
                                                            <div class="px-3 py-2 hover:bg-red-50 transition-colors h-full flex items-center">
                                                                <Icon icon=LuTrash2 class="w-5 h-5 text-gray-400 hover:text-red-600 transition-colors" />
                                                            </div>
                                                        </div>

                                                        {/* RED OVERLAY - shown when purchase is armed for deletion */}
                                                        <Show when=move || purchase_to_delete.get() == Some(purchase_id)>
                                                            <DeleteOverlay
                                                                prompt={t!("checkout.remove_transaction_confirm")()}
                                                                aria_label={t!("checkout.remove_transaction_confirm")()}
                                                                on_click={move |_| handle_purchase_click(purchase_id)}
                                                            />
                                                        </Show>
                                                    </div>
                                                }
                                            }).collect_view()}

                                            {/* Pagination controls */}
                                            <Show when=move || {
                                                let total_count = total_purchase_count.get();
                                                let page_size_val = page_size.get();
                                                let total_pages = if page_size_val > 0 {
                                                    (total_count + page_size_val - 1) / page_size_val
                                                } else {
                                                    0
                                                };
                                                total_pages > 1
                                            }>
                                                <Pagination
                                                    current_page=current_page
                                                    total_items=Signal::derive(move || total_purchase_count.get())
                                                    page_size=page_size
                                                    on_page_change=move |page| set_current_page.set(page)
                                                    on_page_size_change=move |size| {
                                                        set_page_size.set(size);
                                                        set_current_page.set(0); // Reset to first page
                                                    }
                                                    translation_prefix="checkout.pagination"
                                                    show_page_size_selector=true
                                                />
                                            </Show>
                                        </div>
                                    }
                                }
                            >
                                <p class="text-gray-500">{t!("checkout.no_transactions_message")}</p>
                            </Show>
                        </Card>
                    </div>
                </div>

            </div>
        </Container>

        <ConfirmModal
            show=show_cancel_modal
            on_close=move || set_show_cancel_modal.set(false)
            on_confirm=handle_cancel_confirm.clone()
            title=Signal::derive(move || t!("checkout.confirm_cancel_title")())
            message=Signal::derive(|| t!("checkout.confirm_cancel")())
            confirm_text=Signal::derive(move || t!("checkout.confirm_cancel_confirm")())
            cancel_text=Signal::derive(move || t!("common.cancel")())
            is_destructive=true
        />

        <Modal
            show=Signal::derive(move || pending_deletion.get().purchase_id.is_some())
            on_close=cancel_delete_purchase.clone()
            title=Signal::derive(move || t!("checkout.delete_modal.title")())
            size=ModalSize::Medium
            action_bar=
                view! {
                    <div class="contents">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Box::new(cancel_delete_purchase.clone())
                        >
                            {t!("common.cancel")}
                        </Button>
                        <Button
                            variant=ButtonVariant::Danger
                            disabled=Signal::derive(move || !deletion_token_matches.get())
                            on_click=Box::new(perform_delete_purchase.clone())
                        >
                            {t!("checkout.delete_modal.confirm")}
                        </Button>
                    </div>
                }
                .into_any()
        >
            <Show when=move || pending_deletion.get().purchase_id.is_some()>
                <div class="space-y-4">
                    <p class="text-gray-700">
                        {move || {
                            let token = pending_deletion.get().token;
                            translate_with_params(
                                "checkout.delete_modal.instructions",
                                HashMap::from([("token", token)]),
                            )
                        }}
                    </p>
                    <input
                        class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-red-500 border-gray-300"
                        placeholder=t!("checkout.delete_modal.placeholder")()
                        value=move || delete_confirmation_input.get()
                        node_ref=delete_confirmation_ref
                        on:input=move |ev| {
                            set_delete_confirmation_input.set(event_target_value(&ev));
                        }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" && deletion_token_matches.get() {
                                ev.prevent_default();
                                perform_delete_purchase_action.with_value(|handler| handler());
                            }
                        }
                    />
            </div>
        </Show>
        </Modal>

        <RulesInfoModal
            show=Signal::derive(move || show_rules_modal.get())
            on_close=move || show_rules_modal.set(false)
            vendor_validation_rule=Signal::derive(move || vendor_validation_rule.get())
            vendor_omission_rules=Signal::derive(move || vendor_omission_rules.get())
            amount_stepping=Signal::derive(move || amount_stepping.get())
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stored_form_data_restores_valid_draft() {
        let raw = r#"{"booth_id":"booth-1","vendor_id":"12","current_amount":"5.50","items":[{"amount":"5.50","vendor_id":"12","added_at_ms":1711576800000}]}"#;

        let (booth_id, form_data) = parse_stored_form_data(raw).unwrap();
        assert_eq!(booth_id, Some("booth-1".to_string()));
        assert_eq!(form_data.vendor_id, "12");
        assert_eq!(form_data.current_amount, "5.50");
        assert_eq!(form_data.items.len(), 1);
        assert_eq!(form_data.items[0].vendor_id, "12");
        assert_eq!(
            form_data.items[0].amount,
            Decimal::from_str("5.50").unwrap()
        );
    }

    #[test]
    fn parse_stored_form_data_rejects_invalid_json() {
        let raw = r#"{"vendor_id":"12","current_amount":"5.50""#;
        assert!(parse_stored_form_data(raw).is_err());
    }

    #[test]
    fn parse_stored_form_data_rejects_invalid_amount() {
        let raw = r#"{"booth_id":null,"vendor_id":"12","current_amount":"5.50","items":[{"amount":"oops","vendor_id":"12","added_at_ms":1711576800000}]}"#;
        assert!(parse_stored_form_data(raw).is_err());
    }

    #[test]
    fn rtl_digit_entry_shifts_amount_from_right_to_left() {
        assert_eq!(rtl_add_digit("0.00", 5, Locale::En), "0.05");
        assert_eq!(rtl_add_digit("0.05", 5, Locale::En), "0.55");
        assert_eq!(rtl_add_digit("0.55", 5, Locale::En), "5.55");
        assert_eq!(rtl_add_digit("5.55", 0, Locale::En), "55.50");
    }

    #[test]
    fn rtl_backspace_shifts_amount_back_toward_zero() {
        assert_eq!(rtl_backspace("55.50", Locale::En), "5.55");
        assert_eq!(rtl_backspace("5.55", Locale::En), "0.55");
        assert_eq!(rtl_backspace("0.55", Locale::En), "0.05");
        assert_eq!(rtl_backspace("0.05", Locale::En), "0.00");
    }

    #[test]
    fn default_amount_uses_locale_and_mode() {
        assert_eq!(
            default_amount_for_mode(AmountInputMode::RightToLeft, Locale::En),
            "0.00"
        );
        assert_eq!(
            default_amount_for_mode(AmountInputMode::RightToLeft, Locale::De),
            "0,00"
        );
        assert_eq!(
            default_amount_for_mode(AmountInputMode::Regular, Locale::En),
            ""
        );
        assert_eq!(
            default_amount_for_mode(AmountInputMode::Regular, Locale::De),
            ""
        );
    }

    #[test]
    fn amount_input_mode_defaults_to_regular() {
        assert_eq!(AmountInputMode::default(), AmountInputMode::Regular);
    }

    #[test]
    fn normalize_form_data_preserves_regular_mode_text() {
        let form_data = CheckoutFormData {
            vendor_id: "12".to_string(),
            current_amount: "12,34".to_string(),
            ..Default::default()
        };

        let normalized =
            normalize_form_data_for_mode(form_data, AmountInputMode::Regular, Locale::De);

        assert_eq!(normalized.vendor_id, "12");
        assert_eq!(normalized.current_amount, "12,34");
    }

    #[test]
    fn utf16_index_maps_surrogate_pairs_to_character_boundaries() {
        let value = "A😀B";
        assert_eq!(utf16_index_to_byte_index(value, 0), 0);
        assert_eq!(utf16_index_to_byte_index(value, 1), 1);
        assert_eq!(utf16_index_to_byte_index(value, 2), 1);
        assert_eq!(utf16_index_to_byte_index(value, 3), 5);
        assert_eq!(utf16_index_to_byte_index(value, 4), 6);
    }

    #[test]
    fn backspace_removes_entire_surrogate_pair_character() {
        assert_eq!(backspace_input_range("A😀B", 3, 3), "AB");
    }

    #[test]
    fn normalize_form_data_restores_amount_for_selected_mode() {
        let form_data = CheckoutFormData {
            vendor_id: "12".to_string(),
            current_amount: "5.50".to_string(),
            ..Default::default()
        };

        let normalized =
            normalize_form_data_for_mode(form_data, AmountInputMode::RightToLeft, Locale::De);

        assert_eq!(normalized.vendor_id, "12");
        assert_eq!(normalized.current_amount, "5,50");
    }

    #[test]
    fn rtl_digit_entry_stops_at_domain_maximum_amount() {
        assert_eq!(rtl_add_digit("1000000.00", 1, Locale::En), "1000000.00");
    }

    #[test]
    fn regular_mode_decimal_button_respects_locale_specific_separator() {
        let current = "12.34";
        let locale = Locale::De;

        let next = if current.contains(decimal_separator(locale)) {
            current.to_string()
        } else {
            format!("{}{}", current, decimal_separator(locale))
        };

        assert_eq!(next, "12.34,");
    }

    #[test]
    fn classify_inline_amount_rejects_more_than_two_decimals() {
        assert_eq!(
            classify_inline_amount("12.345", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("12,345", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("12.34", None),
            InlineAmountValidation::Valid
        );
        assert_eq!(
            classify_inline_amount(".213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount(",213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("0.213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("0,213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("1.213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount("1,213", None),
            InlineAmountValidation::TooManyDecimals
        );
        assert_eq!(
            classify_inline_amount(".21", None),
            InlineAmountValidation::Valid
        );
        assert_eq!(
            classify_inline_amount(",21", None),
            InlineAmountValidation::Valid
        );
        assert_eq!(
            classify_inline_amount("0.21", None),
            InlineAmountValidation::Valid
        );
        assert_eq!(
            classify_inline_amount("0,21", None),
            InlineAmountValidation::Valid
        );
    }

    #[test]
    fn classify_inline_amount_rejects_values_that_do_not_match_step() {
        assert_eq!(
            classify_inline_amount("3.5", Some(Decimal::new(5, 1))),
            InlineAmountValidation::Valid
        );
        assert_eq!(
            classify_inline_amount("1.1", Some(Decimal::new(5, 1))),
            InlineAmountValidation::StepMismatch(Decimal::new(5, 1))
        );
        assert_eq!(
            classify_inline_amount("8.7", Some(Decimal::ONE)),
            InlineAmountValidation::StepMismatch(Decimal::ONE)
        );
    }
}
