use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    window, Blob, BlobPropertyBag, Document, File, HtmlAnchorElement, HtmlTextAreaElement, Url,
};

use ez_vend_storage::export::{sanitize_filename_component, DeviceInfo};

const DEVICE_IDENTIFIER_MIN_LENGTH: usize = 3;
const DEVICE_IDENTIFIER_MAX_LENGTH: usize = 50;

/// Format an error message for user display, truncating if too long
pub fn format_error_message<E: std::fmt::Debug>(error: &E) -> String {
    const MAX_LEN: usize = 140;
    let mut formatted = format!("{:?}", error).replace(['\n', '\r'], " ");

    if formatted.len() > MAX_LEN {
        formatted.truncate(MAX_LEN - 3);
        formatted.push_str("...");
    }

    formatted
}

const DEVICE_IDENTIFIER_STORAGE_KEY: &str = "ez-vend-device-identifier";

pub fn current_device_info() -> DeviceInfo {
    let identifier = load_or_create_device_identifier();
    let (platform, browser) = current_platform_and_browser();

    DeviceInfo {
        identifier,
        platform,
        browser,
    }
}

pub fn create_json_file(file_name: &str, contents: &str) -> Result<File, String> {
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&JsValue::from_str(contents));

    let options = BlobPropertyBag::new();
    options.set_type("application/json;charset=utf-8");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &options)
        .map_err(|err| format!("failed to create blob: {err:?}"))?;

    let file_options = web_sys::FilePropertyBag::new();
    file_options.set_type("application/json;charset=utf-8");

    File::new_with_blob_sequence_and_options(
        &js_sys::Array::of1(&blob.into()),
        file_name,
        &file_options,
    )
    .map_err(|err| format!("failed to create file: {err:?}"))
}

pub fn download_text_file(file_name: &str, contents: &str, mime_type: &str) -> Result<(), String> {
    let window = window().ok_or_else(|| "window not available".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "document not available".to_string())?;

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&JsValue::from_str(contents));

    let options = BlobPropertyBag::new();
    options.set_type(mime_type);

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &options)
        .map_err(|err| format!("failed to create blob: {err:?}"))?;
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|err| format!("failed to create object URL: {err:?}"))?;

    let anchor = document
        .create_element("a")
        .map_err(|err| format!("failed to create anchor: {err:?}"))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| "failed to cast anchor element".to_string())?;

    anchor.set_href(&url);
    anchor.set_download(file_name);

    let body = document
        .body()
        .ok_or_else(|| "document body not available".to_string())?;

    body.append_child(&anchor)
        .map_err(|err| format!("failed to append anchor: {err:?}"))?;
    anchor.click();
    let _ = body.remove_child(&anchor);
    Url::revoke_object_url(&url).map_err(|err| format!("failed to revoke object URL: {err:?}"))?;

    Ok(())
}

pub async fn copy_text_to_clipboard(contents: &str) -> Result<(), String> {
    let window = window().ok_or_else(|| "window not available".to_string())?;
    let clipboard = window.navigator().clipboard();

    if !clipboard.is_undefined() {
        return wasm_bindgen_futures::JsFuture::from(clipboard.write_text(contents))
            .await
            .map(|_| ())
            .map_err(|err| format!("failed to write to clipboard: {err:?}"));
    }

    copy_text_with_exec_command(
        &window
            .document()
            .ok_or_else(|| "document not available".to_string())?,
        contents,
    )
}

fn copy_text_with_exec_command(document: &Document, contents: &str) -> Result<(), String> {
    let textarea = document
        .create_element("textarea")
        .map_err(|err| format!("failed to create textarea: {err:?}"))?
        .dyn_into::<HtmlTextAreaElement>()
        .map_err(|_| "failed to cast textarea element".to_string())?;

    textarea.set_value(contents);
    textarea
        .set_attribute("readonly", "readonly")
        .map_err(|err| format!("failed to set textarea readonly: {err:?}"))?;
    textarea
        .set_attribute(
            "style",
            "position:fixed;top:-1000px;left:-1000px;opacity:0;pointer-events:none;",
        )
        .map_err(|err| format!("failed to set textarea style: {err:?}"))?;

    let body = document
        .body()
        .ok_or_else(|| "document body not available".to_string())?;
    body.append_child(&textarea)
        .map_err(|err| format!("failed to append textarea: {err:?}"))?;

    textarea
        .focus()
        .map_err(|err| format!("failed to focus textarea: {err:?}"))?;
    textarea.select();

    let exec_command = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("execCommand"))
        .map_err(|err| format!("failed to access execCommand: {err:?}"))?;
    let exec_command = exec_command
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "execCommand not available on document".to_string())?;
    let copied = exec_command
        .call1(document.as_ref(), &JsValue::from_str("copy"))
        .map_err(|err| format!("failed to execute copy command: {err:?}"))?
        .as_bool()
        .unwrap_or(false);

    let _ = body.remove_child(&textarea);

    if copied {
        Ok(())
    } else {
        Err("clipboard not available in this browser context".to_string())
    }
}

pub fn open_print_window_html(html: &str) -> Result<(), String> {
    let window = window().ok_or_else(|| "window not available".to_string())?;
    let print_window = window
        .open_with_url_and_target("about:blank", "_blank")
        .map_err(|err| format!("failed to open print window: {err:?}"))?
        .ok_or_else(|| "failed to open print window".to_string())?;

    let document = print_window
        .document()
        .ok_or_else(|| "print window document not available".to_string())?;
    let document_element = document
        .document_element()
        .ok_or_else(|| "print document element not available".to_string())?;
    document_element.set_inner_html(html);

    let _ = print_window.focus();
    print_window
        .print()
        .map_err(|err| format!("failed to trigger print dialog: {err:?}"))
}

pub fn supports_native_share_with_files() -> bool {
    let Some(window) = window() else {
        return false;
    };

    let navigator = window.navigator();
    let navigator_value: JsValue = navigator.into();

    let has_share =
        js_sys::Reflect::has(&navigator_value, &JsValue::from_str("share")).unwrap_or(false);
    if !has_share {
        return false;
    }

    let Ok(can_share_value) =
        js_sys::Reflect::get(&navigator_value, &JsValue::from_str("canShare"))
    else {
        return false;
    };

    if !can_share_value.is_function() {
        return false;
    }

    let Ok(test_file) = create_json_file("ez-vend-share-test.json", "{}") else {
        return false;
    };

    let share_data = js_sys::Object::new();
    let files = js_sys::Array::new();
    files.push(&test_file);
    let _ = js_sys::Reflect::set(&share_data, &JsValue::from_str("files"), &files);

    let Ok(can_share_fn) = can_share_value.dyn_into::<js_sys::Function>() else {
        return false;
    };

    can_share_fn
        .call1(&navigator_value, &share_data)
        .ok()
        .and_then(|result| result.as_bool())
        .unwrap_or(false)
}

pub async fn share_json_file(file_name: &str, contents: &str, title: &str) -> Result<(), String> {
    let Some(window) = window() else {
        return Err("window not available".to_string());
    };

    let navigator = window.navigator();
    let navigator_value: JsValue = navigator.into();
    let share_value = js_sys::Reflect::get(&navigator_value, &JsValue::from_str("share"))
        .map_err(|_| "native share is not available".to_string())?;
    let share_fn = share_value
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "native share is not available".to_string())?;

    let file = create_json_file(file_name, contents)?;
    let files = js_sys::Array::new();
    files.push(&file);

    let share_data = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &share_data,
        &JsValue::from_str("title"),
        &JsValue::from_str(title),
    );
    let _ = js_sys::Reflect::set(
        &share_data,
        &JsValue::from_str("text"),
        &JsValue::from_str(title),
    );
    let _ = js_sys::Reflect::set(&share_data, &JsValue::from_str("files"), &files);

    let promise = share_fn
        .call1(&navigator_value, &share_data)
        .map_err(|err| format!("native share failed to start: {err:?}"))?
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "native share did not return a promise".to_string())?;

    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|err| format!("native share failed: {err:?}"))
}

pub fn load_device_identifier() -> Option<String> {
    let window = window()?;
    let storage = window.local_storage().ok()??;
    let value = storage.get_item(DEVICE_IDENTIFIER_STORAGE_KEY).ok()??;
    normalize_device_identifier(&value).ok()
}

pub fn save_device_identifier(value: String) -> Result<String, String> {
    let normalized = normalize_device_identifier(&value)?;
    persist_device_identifier(&normalized)?;
    Ok(normalized)
}

pub fn reset_device_identifier() -> Result<String, String> {
    let generated = generate_default_device_identifier();
    persist_device_identifier(&generated)?;
    Ok(generated)
}

pub fn validate_device_identifier(value: &str) -> Result<(), String> {
    let trimmed = value.trim();

    if trimmed.len() < DEVICE_IDENTIFIER_MIN_LENGTH || trimmed.len() > DEVICE_IDENTIFIER_MAX_LENGTH
    {
        return Err(format!(
            "device identifier must be {}-{} characters",
            DEVICE_IDENTIFIER_MIN_LENGTH, DEVICE_IDENTIFIER_MAX_LENGTH
        ));
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(
            "device identifier may only contain letters, numbers, hyphens, and underscores"
                .to_string(),
        );
    }

    Ok(())
}

fn load_or_create_device_identifier() -> String {
    if let Some(existing) = load_device_identifier() {
        return existing;
    }

    let generated = generate_default_device_identifier();
    let _ = persist_device_identifier(&generated);
    generated
}

fn normalize_device_identifier(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    validate_device_identifier(trimmed)?;
    Ok(trimmed.to_string())
}

fn persist_device_identifier(value: &str) -> Result<(), String> {
    let window = window().ok_or_else(|| "window not available".to_string())?;
    let storage = window
        .local_storage()
        .map_err(|err| format!("localStorage unavailable: {err:?}"))?
        .ok_or_else(|| "localStorage unavailable".to_string())?;

    storage
        .set_item(DEVICE_IDENTIFIER_STORAGE_KEY, value)
        .map_err(|err| format!("failed to store device identifier: {err:?}"))
}

fn generate_default_device_identifier() -> String {
    let (platform, browser) = current_platform_and_browser();
    let basis = format!("{platform}-{browser}");
    let sanitized = sanitize_filename_component(&basis);
    if sanitized.is_empty() {
        "this-device".to_string()
    } else {
        sanitized
    }
}

fn current_platform_and_browser() -> (String, String) {
    let user_agent = window()
        .map(|win| win.navigator().user_agent().unwrap_or_default())
        .unwrap_or_default();

    let platform = detect_platform(&user_agent);
    let browser = detect_browser(&user_agent);
    (platform.to_string(), browser.to_string())
}

fn detect_platform(user_agent: &str) -> &'static str {
    let lower = user_agent.to_ascii_lowercase();
    if lower.contains("windows") {
        "Windows"
    } else if lower.contains("android") {
        "Android"
    } else if lower.contains("iphone") || lower.contains("ipad") || lower.contains("ios") {
        "iOS"
    } else if lower.contains("mac os") || lower.contains("macintosh") {
        "macOS"
    } else if lower.contains("linux") {
        "Linux"
    } else {
        "Unknown"
    }
}

pub(crate) fn detect_browser(user_agent: &str) -> &'static str {
    let lower = user_agent.to_ascii_lowercase();
    if lower.contains("edg/") {
        "Edge"
    } else if lower.contains("firefox/") {
        "Firefox"
    } else if lower.contains("chrome/") && !lower.contains("edg/") {
        "Chrome"
    } else if lower.contains("safari/") && !lower.contains("chrome/") {
        "Safari"
    } else {
        "Browser"
    }
}

#[cfg(test)]
mod tests {
    use super::validate_device_identifier;

    #[test]
    fn accepts_valid_device_identifiers() {
        assert!(validate_device_identifier("marias-laptop").is_ok());
        assert!(validate_device_identifier("AWO_Desk_01").is_ok());
        assert!(validate_device_identifier("abc").is_ok());
    }

    #[test]
    fn rejects_invalid_device_identifiers() {
        assert!(validate_device_identifier("ab").is_err());
        assert!(validate_device_identifier("name with spaces").is_err());
        assert!(validate_device_identifier("device!").is_err());
        assert!(validate_device_identifier(&"a".repeat(51)).is_err());
    }
}
