use leptos::*;
use web_sys::window;

/// Consistent page size options across all paginated views
pub const PAGE_SIZE_OPTIONS: [usize; 5] = [5, 10, 20, 30, 50];

/// Hook to manage pagination page size with localStorage persistence
///
/// Returns a tuple of:
/// - ReadSignal<usize>: Current page size
/// - WriteSignal<usize>: Setter for page size
/// - ReadSignal<bool>: Readiness flag (true once preference is loaded)
pub fn use_pagination_preference(
    storage_key: &'static str,
    default_size: usize,
) -> (ReadSignal<usize>, WriteSignal<usize>, ReadSignal<bool>) {
    // Try to load synchronously - will work in browser, return None in SSR
    let initial_value = load_page_size(storage_key)
        .and_then(|size| {
            if PAGE_SIZE_OPTIONS.contains(&size) {
                Some(size)
            } else {
                None
            }
        })
        .unwrap_or(default_size);

    web_sys::console::log_1(
        &format!(
            "[PaginationPrefs] {} loaded from storage: {} (default: {})",
            storage_key, initial_value, default_size
        )
        .into(),
    );

    let (page_size, set_page_size_inner) = create_signal(initial_value);

    // Readiness flag - set to true immediately since we load synchronously
    // No need to wait for next frame since the value is already loaded and stable
    let (is_ready, _set_is_ready) = create_signal(true);

    // Create an effect that saves whenever the signal changes
    // But use previous value tracking to avoid saving on initial load
    let storage_key_for_effect = storage_key;
    create_effect(move |prev_value: Option<usize>| {
        let size = page_size.get();
        // Only save if this is not the first run
        if prev_value.is_some() {
            save_page_size(storage_key_for_effect, size);
            web_sys::console::log_1(
                &format!(
                    "[PaginationPrefs] {} saved to localStorage: {}",
                    storage_key_for_effect, size
                )
                .into(),
            );
        }
        size
    });

    (page_size, set_page_size_inner, is_ready)
}

/// Load page size from localStorage
fn load_page_size(key: &str) -> Option<usize> {
    let window = window()?;
    let storage = window.local_storage().ok()??;
    let value_str = storage.get_item(key).ok()??;
    value_str.parse::<usize>().ok()
}

/// Save page size to localStorage
fn save_page_size(key: &str, size: usize) {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(key, &size.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_size_options_are_sorted() {
        let mut sorted = PAGE_SIZE_OPTIONS;
        sorted.sort();
        assert_eq!(
            PAGE_SIZE_OPTIONS, sorted,
            "PAGE_SIZE_OPTIONS should be in ascending order"
        );
    }

    #[test]
    fn test_page_size_options_contain_common_values() {
        assert!(PAGE_SIZE_OPTIONS.contains(&5));
        assert!(PAGE_SIZE_OPTIONS.contains(&10));
        assert!(PAGE_SIZE_OPTIONS.contains(&20));
        assert!(PAGE_SIZE_OPTIONS.contains(&30));
        assert!(PAGE_SIZE_OPTIONS.contains(&50));
    }
}
