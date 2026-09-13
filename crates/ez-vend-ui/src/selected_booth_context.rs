use domain::models::booth::Booth;
use domain::models::shared::BoothId;
use domain::repositories::BoothRepository;
use leptos::*;
use std::sync::Arc;
use uuid::Uuid;
use web_sys::window;

const SELECTED_BOOTH_STORAGE_KEY: &str = "ez-vend-selected-booth-id";
const BOOTH_LIST_VERSION_STORAGE_KEY: &str = "ez-vend-list-version";

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedBoothContext(pub RwSignal<Option<Booth>>);

/// Context for triggering booth list reloads
/// Increment this signal to notify all components that the booth list has changed
#[derive(Clone, Copy, Debug)]
pub struct BoothListVersionContext(pub RwSignal<u32>);

fn clear_selected_booth(
    booth_signal: &RwSignal<Option<Booth>>,
    booth_list_version: Option<&RwSignal<u32>>,
) {
    save_selected_booth_id(None);
    booth_signal.set(None);

    if let Some(version_signal) = booth_list_version {
        version_signal.update(|version| *version += 1);
    }
}

fn refresh_selected_booth(
    booth_repository: Arc<dyn BoothRepository>,
    booth_signal: RwSignal<Option<Booth>>,
    booth_list_version: Option<RwSignal<u32>>,
) {
    spawn_local(async move {
        let Some(selected_booth) = booth_signal.get_untracked() else {
            return;
        };

        match booth_repository.find_by_id(&selected_booth.id).await {
            Ok(Some(booth)) if booth.is_archived() => {
                if let Some(version_signal) = booth_list_version.as_ref() {
                    clear_selected_booth(&booth_signal, Some(version_signal));
                } else {
                    clear_selected_booth(&booth_signal, None);
                }
            }
            Ok(Some(booth)) => {
                booth_signal.set(Some(booth));
            }
            Ok(None) => {
                clear_selected_booth(&booth_signal, None);
            }
            Err(_) => {}
        }
    });
}

/// Get localStorage from the browser window
fn get_local_storage() -> Option<web_sys::Storage> {
    let window = window()?;
    window.local_storage().ok().flatten()
}

/// Load the selected booth ID from localStorage
fn load_selected_booth_id() -> Option<BoothId> {
    let storage = get_local_storage()?;
    let id_str = storage.get_item(SELECTED_BOOTH_STORAGE_KEY).ok()??;
    let uuid = Uuid::parse_str(&id_str).ok()?;
    Some(BoothId::from_uuid(uuid))
}

/// Save the selected booth ID to localStorage
fn save_selected_booth_id(booth_id: Option<&str>) {
    if let Some(storage) = get_local_storage() {
        match booth_id {
            Some(id) => {
                let _ = storage.set_item(SELECTED_BOOTH_STORAGE_KEY, id);
            }
            None => {
                let _ = storage.remove_item(SELECTED_BOOTH_STORAGE_KEY);
            }
        }
    }
}

pub fn provide_selected_booth_context() -> RwSignal<Option<Booth>> {
    let booth_signal = RwSignal::new(None::<Booth>);
    provide_context(SelectedBoothContext(booth_signal));

    // Provide booth list version signal for triggering reloads
    // Load initial version from localStorage
    let initial_version = get_local_storage()
        .and_then(|storage| {
            storage
                .get_item(BOOTH_LIST_VERSION_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let booth_list_version = RwSignal::new(initial_version);
    provide_context(BoothListVersionContext(booth_list_version));

    booth_signal
}

pub fn use_selected_booth() -> RwSignal<Option<Booth>> {
    if let Some(context) = use_context::<SelectedBoothContext>() {
        context.0
    } else {
        web_sys::console::warn_1(
            &"SelectedBoothContext not found. Falling back to empty booth selection.".into(),
        );
        RwSignal::new(None)
    }
}

/// Get the booth list version signal to trigger or react to booth list changes
/// Increment this signal when booths are created, updated, or deleted
pub fn use_booth_list_version() -> RwSignal<u32> {
    if let Some(context) = use_context::<BoothListVersionContext>() {
        context.0
    } else {
        web_sys::console::warn_1(
            &"BoothListVersionContext not found. Falling back to static version signal.".into(),
        );
        RwSignal::new(0)
    }
}

#[component]
pub fn SelectedBoothProvider(children: Children) -> impl IntoView {
    let booth_signal = provide_selected_booth_context();
    let booth_list_version = use_booth_list_version();

    // Restore selected booth from localStorage on mount
    // Track if we've already attempted restoration
    let restored = RwSignal::new(false);

    // Get app_state resource - we need to track it reactively
    // We use a separate effect to wait for AppState context to be available
    Effect::new(move |_| {
        // Only try to restore once
        if restored.get() {
            return;
        }

        // Try to get app_state from context
        let Some(app_state) = use_context::<Resource<(), Result<crate::state::AppState, String>>>()
        else {
            web_sys::console::log_1(&"AppState context not available yet...".into());
            return;
        };

        // Track app_state resource to make effect reactive
        let Some(Ok(state)) = app_state.get() else {
            web_sys::console::log_1(&"AppState not ready yet...".into());
            return;
        };

        web_sys::console::log_1(&"AppState is ready, checking for saved booth...".into());

        let Some(stored_booth_id) = load_selected_booth_id() else {
            web_sys::console::log_1(&"No booth ID in localStorage".into());
            restored.set(true);

            let booth_repository = state.booth_repository.clone();
            spawn_local(async move {
                match booth_repository.find_all().await {
                    Ok(mut booths) if booths.len() == 1 => {
                        let booth = booths.pop().expect("single booth exists");
                        web_sys::console::log_1(
                            &format!(
                                "Auto-selecting only booth on startup: {}",
                                booth.description
                            )
                            .into(),
                        );
                        booth_signal.set(Some(booth));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        web_sys::console::log_1(
                            &format!(
                                "Error checking single-booth startup auto-selection: {:?}",
                                e
                            )
                            .into(),
                        );
                    }
                }
            });
            return;
        };

        web_sys::console::log_1(
            &format!(
                "Attempting to restore booth ID: {}",
                stored_booth_id.as_str()
            )
            .into(),
        );

        // Mark as restored to prevent duplicate attempts
        restored.set(true);

        let booth_repository = state.booth_repository.clone();
        spawn_local(async move {
            // Validate that the booth still exists
            match booth_repository.find_by_id(&stored_booth_id).await {
                Ok(Some(booth)) => {
                    if booth.is_archived() {
                        clear_selected_booth(&booth_signal, Some(&booth_list_version));
                        return;
                    }

                    web_sys::console::log_1(
                        &format!("Booth restored: {}", booth.description).into(),
                    );
                    booth_signal.set(Some(booth));
                }
                Ok(None) => {
                    web_sys::console::log_1(&"Booth not found, clearing storage".into());
                    // Booth was deleted, clear the stored selection
                    save_selected_booth_id(None);
                }
                Err(e) => {
                    web_sys::console::log_1(
                        &format!("Error loading booth: {:?}, clearing storage", e).into(),
                    );
                    // Error loading booth, clear the stored selection
                    save_selected_booth_id(None);
                }
            }
        });
    });

    // Save selected booth to localStorage whenever it changes
    // Track if this is the first run to avoid clearing localStorage before restoration
    let is_first_save = RwSignal::new(true);

    Effect::new(move |_| {
        let booth = booth_signal.get();

        // Skip saving on the very first run to allow restoration to happen first
        if is_first_save.get() {
            web_sys::console::log_1(&"Save effect: Initial run, skipping...".into());
            is_first_save.set(false);
            return;
        }

        let booth_id_str = booth.as_ref().map(|b| b.id.as_str());

        if let Some(id) = booth_id_str.as_deref() {
            web_sys::console::log_1(&format!("Saving booth ID to localStorage: {}", id).into());
        } else {
            web_sys::console::log_1(&"Clearing booth ID from localStorage".into());
        }

        save_selected_booth_id(booth_id_str.as_deref());
    });

    let synced_booth_list_version = RwSignal::new(None::<u32>);

    // Keep the selected booth fresh in the same tab after booth edits.
    Effect::new(move |_| {
        if !restored.get() {
            return;
        }

        let version = booth_list_version.get();

        let Some(app_state) = use_context::<Resource<(), Result<crate::state::AppState, String>>>()
        else {
            return;
        };

        let Some(Ok(state)) = app_state.get() else {
            return;
        };

        match synced_booth_list_version.get() {
            None => {
                synced_booth_list_version.set(Some(version));
                return;
            }
            Some(previous_version) if previous_version == version => return,
            Some(_) => synced_booth_list_version.set(Some(version)),
        }

        if booth_signal.get().is_none() {
            return;
        }

        refresh_selected_booth(
            state.booth_repository.clone(),
            booth_signal,
            Some(booth_list_version),
        );
    });

    // Listen for storage events from other tabs
    // This allows cross-tab synchronization when booth is deleted elsewhere
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let Some(window) = web_sys::window() else {
            return;
        };

        let Some(app_state) = use_context::<Resource<(), Result<crate::state::AppState, String>>>()
        else {
            return;
        };

        let handle_storage = move |event: web_sys::StorageEvent| {
            // Only handle changes to our booth selection key
            if event.key().as_deref() == Some(SELECTED_BOOTH_STORAGE_KEY) {
                web_sys::console::log_1(
                    &"Storage event: booth selection changed in another tab".into(),
                );

                // Check the new value
                match event.new_value() {
                    None => {
                        // Booth was cleared in another tab
                        web_sys::console::log_1(
                            &"Storage event: booth cleared in another tab".into(),
                        );
                        booth_signal.set(None);
                    }
                    Some(new_id_str) => {
                        // Booth was changed in another tab - validate and load it
                        web_sys::console::log_1(
                            &format!(
                                "Storage event: booth changed to {} in another tab",
                                new_id_str
                            )
                            .into(),
                        );

                        if let Ok(uuid) = Uuid::parse_str(&new_id_str) {
                            let booth_id = BoothId::from_uuid(uuid);

                            // Get app state and load the booth
                            if let Some(Ok(state)) = app_state.get() {
                                let booth_repository = state.booth_repository.clone();
                                spawn_local(async move {
                                    match booth_repository.find_by_id(&booth_id).await {
                                        Ok(Some(booth)) => {
                                            if booth.is_archived() {
                                                clear_selected_booth(
                                                    &booth_signal,
                                                    Some(&booth_list_version),
                                                );
                                                return;
                                            }

                                            web_sys::console::log_1(
                                                &format!(
                                                    "Storage event: loaded booth {}",
                                                    booth.description
                                                )
                                                .into(),
                                            );
                                            booth_signal.set(Some(booth));
                                        }
                                        Ok(None) => {
                                            web_sys::console::log_1(
                                                &"Storage event: booth not found".into(),
                                            );
                                            booth_signal.set(None);
                                        }
                                        Err(e) => {
                                            web_sys::console::log_1(
                                                &format!(
                                                    "Storage event: error loading booth: {:?}",
                                                    e
                                                )
                                                .into(),
                                            );
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            } else if event.key().as_deref() == Some(BOOTH_LIST_VERSION_STORAGE_KEY) {
                if let Some(Ok(state)) = app_state.get() {
                    refresh_selected_booth(
                        state.booth_repository.clone(),
                        booth_signal,
                        Some(booth_list_version),
                    );
                }
            }
        };

        let closure = Closure::wrap(Box::new(handle_storage) as Box<dyn Fn(web_sys::StorageEvent)>);

        let _ =
            window.add_event_listener_with_callback("storage", closure.as_ref().unchecked_ref());

        // Keep closure alive for the lifetime of the component
        closure.forget();
    });

    // Sync booth list version to localStorage when it changes
    let booth_list_version = use_booth_list_version();
    Effect::new(move |_| {
        let version = booth_list_version.get();

        if let Some(storage) = get_local_storage() {
            let version_str = version.to_string();
            let _ = storage.set_item(BOOTH_LIST_VERSION_STORAGE_KEY, &version_str);
            web_sys::console::log_1(
                &format!("Booth list version saved to localStorage: {}", version).into(),
            );
        }
    });

    // Listen for booth list version changes from other tabs
    Effect::new(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let Some(window) = web_sys::window() else {
            return;
        };

        let handle_version_change = move |event: web_sys::StorageEvent| {
            // Only handle changes to booth list version key
            if event.key().as_deref() == Some(BOOTH_LIST_VERSION_STORAGE_KEY) {
                if let Some(new_version_str) = event.new_value() {
                    if let Ok(new_version) = new_version_str.parse::<u32>() {
                        web_sys::console::log_1(
                            &format!(
                                "Storage event: booth list version changed to {} in another tab",
                                new_version
                            )
                            .into(),
                        );
                        booth_list_version.set(new_version);
                    }
                }
            }
        };

        let closure =
            Closure::wrap(Box::new(handle_version_change) as Box<dyn Fn(web_sys::StorageEvent)>);

        let _ =
            window.add_event_listener_with_callback("storage", closure.as_ref().unchecked_ref());

        // Keep closure alive for the lifetime of the component
        closure.forget();
    });

    children()
}
