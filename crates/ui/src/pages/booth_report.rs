#![allow(clippy::clone_on_copy)]

use crate::components::*;
use crate::error_translator::translate_domain_error;
use crate::i18n::translate_with_params;
use crate::selected_booth_context::use_selected_booth;
use crate::state::*;
use crate::t;
use domain::models::booth::Booth;
use domain::models::{BoothId, BoothSummary};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};
use std::str::FromStr;
use web_sys::window;

fn confirmation_token_from_booth(booth_id: &BoothId) -> String {
    confirmation_token_from_id_str(&booth_id.as_str())
}

fn confirmation_token_from_id_str(id: &str) -> String {
    let sanitized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    const FALLBACK_TOKEN: &str = "DELETE";

    if sanitized.len() < 4 {
        return FALLBACK_TOKEN.to_string();
    }

    let len = sanitized.len();
    let start = len.saturating_sub(4);
    sanitized[start..].to_uppercase()
}

fn focus_and_select_input(input_ref: &NodeRef<html::Input>) {
    if let Some(input) = input_ref.get() {
        let _ = input.focus();
        input.select();
    }
}

#[component]
pub fn BoothReportPage() -> impl IntoView {
    let app_state = use_app_state();
    let selected_booth = use_selected_booth();
    let booth_list_version = crate::selected_booth_context::use_booth_list_version();
    let navigate = StoredValue::new_local(use_navigate());
    let go_to =
        move |path: String| navigate.with_value(|navigate| navigate(&path, Default::default()));
    let toast = use_toast();
    let params = use_params_map();

    let booth_id = Signal::derive(move || {
        params
            .get()
            .get("id")
            .and_then(|id| uuid::Uuid::from_str(&id).ok())
            .map(BoothId::from_uuid)
    });

    let (booth, set_booth) = signal(None::<Booth>);
    let (is_loading, set_is_loading) = signal(true);
    let (summary, set_summary) = signal(None::<BoothSummary>);
    let (is_loading_report, set_is_loading_report) = signal(false);

    let (show_archive_modal, set_show_archive_modal) = signal(false);
    let (show_copy_modal, set_show_copy_modal) = signal(false);
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);
    let (is_checking_delete_requirements, set_is_checking_delete_requirements) = signal(false);
    let (delete_confirmation_token, set_delete_confirmation_token) = signal(String::new());
    let (delete_confirmation_input, set_delete_confirmation_input) = signal(String::new());
    let delete_confirmation_ref: NodeRef<html::Input> = NodeRef::new();

    let deletion_token_matches = Memo::new(move |_| {
        let required = delete_confirmation_token.get().trim().to_uppercase();
        if required.is_empty() {
            return false;
        }
        let entered = delete_confirmation_input.get().trim().to_uppercase();
        entered == required
    });

    // Load booth by id on mount / when id or list version changes.
    Effect::new(move |_| {
        let state_result = app_state.get();
        let Some(id) = booth_id.get() else {
            go_to("/booths".to_string());
            return;
        };
        let _ = booth_list_version.get();

        if let Some(Ok(state)) = state_result {
            set_is_loading.set(true);
            spawn_local(async move {
                match state.booth_repository.find_by_id(&id).await {
                    Ok(Some(found)) => {
                        set_booth.set(Some(found));
                        set_is_loading.set(false);
                    }
                    Ok(None) | Err(_) => {
                        go_to("/booths".to_string());
                    }
                }
            });
        }
    });

    // Generate report for active booths.
    Effect::new(move |_| {
        let state_result = app_state.get();
        let current_booth = booth.get();

        let Some(current_booth) = current_booth else {
            return;
        };

        if current_booth.is_archived() {
            set_summary.set(None);
            set_is_loading_report.set(false);
            return;
        }

        if let Some(Ok(state)) = state_result {
            set_is_loading_report.set(true);
            let id = current_booth.id;
            spawn_local(async move {
                match state.report_service.generate_booth_summary(&id, None).await {
                    Ok(report) => {
                        if booth.get_untracked().map(|b| b.id) == Some(id) {
                            set_summary.set(Some(report));
                        }
                    }
                    Err(e) => {
                        let error_msg = translate_with_params(
                            "report.errors.generate_failed",
                            std::collections::HashMap::from([("error", format_error_message(&e))]),
                        );
                        toast.error(error_msg);
                    }
                }
                if booth.get_untracked().map(|b| b.id) == Some(id) {
                    set_is_loading_report.set(false);
                }
            });
        }
    });

    let go_back_to_list = move || go_to("/booths".to_string());

    let go_to_edit = move || {
        if let Some(b) = booth.get_untracked() {
            go_to(format!("/booths/{}/edit", b.id));
        }
    };

    let close_copy_modal = move || {
        set_show_copy_modal.set(false);
    };

    let handle_copy_booth = move |data: CopyBoothFormData| {
        let state_result = app_state.get();
        let source_booth = booth.get();

        spawn_local(async move {
            if let (Some(Ok(state)), Some(source_booth)) = (state_result, source_booth) {
                let Some(new_date) = data.parse_date() else {
                    toast.error(t!("validation.date_invalid")());
                    return;
                };

                match state
                    .booth_service
                    .copy_booth(source_booth.id, data.description.clone(), new_date)
                    .await
                {
                    Ok(copied) => {
                        if source_booth.booth_type == domain::models::BoothType::DirectSale {
                            let mut group_id_map = std::collections::HashMap::new();
                            if let Ok(groups) = state
                                .product_group_repository
                                .find_by_booth(&source_booth.id)
                                .await
                            {
                                for group in &groups {
                                    let new_id = domain::models::ProductGroupId::new();
                                    group_id_map.insert(group.id, new_id);
                                    let _ = state
                                        .product_group_repository
                                        .save(&domain::models::ProductGroup {
                                            id: new_id,
                                            booth_id: copied.id,
                                            name: group.name.clone(),
                                            color: group.color,
                                            emoji: group.emoji.clone(),
                                            sort_order: group.sort_order,
                                        })
                                        .await;
                                }
                            }
                            if let Ok(products) = state
                                .product_repository
                                .find_by_booth(&source_booth.id)
                                .await
                            {
                                for product in &products {
                                    if let Some(&new_group_id) =
                                        group_id_map.get(&product.product_group_id)
                                    {
                                        let _ = state
                                            .product_repository
                                            .save(&domain::models::Product {
                                                id: domain::models::ProductId::new(),
                                                booth_id: copied.id,
                                                product_group_id: new_group_id,
                                                name: product.name.clone(),
                                                price: product.price,
                                                sort_order: product.sort_order,
                                                initial_stock: product.initial_stock,
                                            })
                                            .await;
                                    }
                                }
                            }
                        }

                        if let Ok(mut loaded_booths) = state.booth_repository.find_all().await {
                            if loaded_booths.len() == 1 {
                                if !copied.is_archived() {
                                    selected_booth.set(Some(copied.clone()));
                                }
                                toast.success(
                                    t!("booth.success.copied_and_selected")()
                                        .replace("{description}", &copied.description),
                                );
                            } else {
                                toast.success(
                                    t!("booth.success.copied")()
                                        .replace("{description}", &copied.description),
                                );
                            }
                            crate::booth_ordering::sort_booths(&mut loaded_booths);
                        }

                        set_show_copy_modal.set(false);
                        booth_list_version.update(|v| *v += 1);
                    }
                    Err(err) => {
                        toast.error(translate_domain_error(&err));
                    }
                }
            }
        });
    };

    let close_delete_modals = move || {
        set_show_delete_confirm.set(false);
        set_delete_confirmation_token.set(String::new());
        set_delete_confirmation_input.set(String::new());
        set_is_checking_delete_requirements.set(false);
    };

    let prompt_delete_booth = move || {
        let state_result = app_state.get();
        let Some(current_booth) = booth.get() else {
            return;
        };
        set_is_checking_delete_requirements.set(true);
        set_show_delete_confirm.set(false);
        set_delete_confirmation_token.set(String::new());
        set_delete_confirmation_input.set(String::new());

        spawn_local(async move {
            let Some(Ok(state)) = state_result else {
                set_is_checking_delete_requirements.set(false);
                return;
            };

            match state
                .purchase_repository
                .find_by_booth(&current_booth.id)
                .await
            {
                Ok(purchases) => {
                    set_is_checking_delete_requirements.set(false);
                    if purchases.is_empty() {
                        set_show_delete_confirm.set(true);
                    } else {
                        set_delete_confirmation_token
                            .set(confirmation_token_from_booth(&current_booth.id));
                        set_delete_confirmation_input.set(String::new());
                        set_timeout(
                            move || focus_and_select_input(&delete_confirmation_ref),
                            std::time::Duration::from_millis(0),
                        );
                    }
                }
                Err(_) => {
                    set_is_checking_delete_requirements.set(false);
                    toast.error(t!("booth.errors_detail.delete_failed")());
                }
            }
        });
    };

    let handle_delete_booth = move || {
        let state_result = app_state.get();
        let Some(current_booth) = booth.get() else {
            return;
        };

        spawn_local(async move {
            if let Some(Ok(state)) = state_result {
                match state
                    .archive_service
                    .delete_booth_with_cascade(&current_booth.id)
                    .await
                {
                    Ok(_) => {
                        crate::pages::checkout::clear_checkout_draft(&current_booth.id.as_str());

                        if let Some(current) = selected_booth.get() {
                            if current.id == current_booth.id {
                                selected_booth.set(None);
                            }
                        }

                        toast.success(
                            t!("booth.success.deleted")()
                                .replace("{description}", &current_booth.description),
                        );
                        booth_list_version.update(|v| *v += 1);
                        go_to("/booths".to_string());
                    }
                    Err(_) => {
                        toast.error(t!("booth.errors_detail.delete_failed")());
                    }
                }
            }
        });
    };

    let close_archive_modal = move || {
        set_show_archive_modal.set(false);
    };

    let handle_archive_completed = move || {
        if let Some(b) = booth.get() {
            if let Some(selected) = selected_booth.get() {
                if selected.id == b.id {
                    selected_booth.set(None);
                }
            }
        }
        close_archive_modal();
        booth_list_version.update(|v| *v += 1);
        go_to("/booths".to_string());
    };

    let delete_message = move || {
        booth
            .get()
            .map(|b| t!("booth.delete_confirm_message")().replace("{description}", &b.description))
            .unwrap_or_else(|| t!("booth.delete_confirm")())
    };

    let handle_print = move |_| {
        set_timeout(
            move || {
                if let Some(window) = window() {
                    let _ = window.print();
                }
            },
            std::time::Duration::from_millis(100),
        );
    };

    let page_title = move || {
        booth.get().map(|b| {
            if b.is_archived() {
                t!("archive.summary_title")()
            } else {
                t!("report.booth_summary_report")()
            }
        })
    };
    let page_subtitle = move || booth.get().map(|b| b.description);

    view! {
        <>
            <div class="print:hidden">
                <Container>
                    <div class="space-y-6 pb-32 pt-6">
                        <Show
                            when=move || is_loading.get()
                            fallback=move || {
                                let current_booth = booth.get();
                                let Some(current_booth) = current_booth else {
                                    return ().into_any();
                                };
                                let is_archived = current_booth.is_archived();
                                let booth_for_copy = current_booth.clone();
                                let booth_for_archive = current_booth.clone();
                                let booth_id_for_export = current_booth.id;

                                view! {
                                    <div class="flex items-center justify-between gap-3">
                                        <div>
                                            <h1 class="text-xl font-semibold text-gray-900">
                                                {page_title().unwrap_or_default()}
                                            </h1>
                                            <h2 class="text-sm text-gray-600">
                                                {page_subtitle().unwrap_or_default()}
                                            </h2>
                                        </div>
                                        <Button variant=ButtonVariant::Secondary on_click=Box::new(go_back_to_list)>
                                            {t!("common.back")()}
                                        </Button>
                                    </div>

                                    <div class="flex items-center gap-1">
                                        <Show when=move || !is_archived>
                                            <button
                                                type="button"
                                                on:click=move |_| go_to_edit()
                                                class="inline-flex h-10 w-10 items-center justify-center rounded-lg text-gray-600 transition-colors hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                                                title={t!("booth.edit_button")()}
                                                aria-label={t!("booth.edit_button")()}
                                            >
                                                <Icon icon=LuPenSquare class="h-5 w-5" />
                                            </button>
                                        </Show>
                                        <button
                                            type="button"
                                            on:click=move |_| set_show_copy_modal.set(true)
                                            class="inline-flex h-10 w-10 items-center justify-center rounded-lg text-gray-600 transition-colors hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                                            title={t!("booth.copy_button")()}
                                            aria-label={t!("booth.copy_button")()}
                                        >
                                            <Icon icon=LuCopy class="h-5 w-5" />
                                        </button>
                                        <ExportButton scope=ExportScope::Booth(booth_id_for_export) />
                                        <button
                                            type="button"
                                            on:click=handle_print
                                            class="inline-flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                                            title={t!("report.print_report")()}
                                            aria-label={t!("report.print_report")()}
                                        >
                                            <Icon icon=LuPrinter class="h-4 w-4" />
                                            <span>{t!("report.print_report")}</span>
                                        </button>
                                    </div>

                                    <div>
                                        <Show
                                            when=move || is_loading_report.get()
                                            fallback=move || {
                                                if is_archived {
                                                    view! {
                                                        <ArchivedBoothSummaryDisplay booth=current_booth.clone() />
                                                    }
                                                    .into_any()
                                                } else {
                                                    view! {
                                                        <Show when=move || summary.get().is_some()>
                                                            {move || summary.get().map(|s| view! { <BoothSummaryDisplay summary=s /> })}
                                                        </Show>
                                                    }
                                                    .into_any()
                                                }
                                            }
                                        >
                                            <div class="flex items-center justify-center py-12">
                                                <div class="text-center">
                                                    <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                                                    <p class="mt-4 text-gray-600">{t!("report.loading")}</p>
                                                </div>
                                            </div>
                                        </Show>
                                    </div>

                                    <Show when=move || !is_archived>
                                        <div class="flex justify-end gap-2 border-t border-gray-200 pt-4">
                                            <Button
                                                variant=ButtonVariant::Secondary
                                                class="text-amber-700 hover:bg-amber-50 hover:text-amber-800".to_string()
                                                on_click=Box::new(move || {
                                                    set_show_archive_modal.set(true);
                                                })
                                            >
                                                {t!("archive.archive_button")()}
                                            </Button>
                                            <Button
                                                variant=ButtonVariant::Danger
                                                on_click=Box::new(prompt_delete_booth)
                                            >
                                                {t!("booth.delete_button")()}
                                            </Button>
                                        </div>
                                    </Show>

                                    {move || booth.get().filter(|_| show_archive_modal.get()).map(|_| {
                                        view! {
                                            <ArchiveWizard
                                                booth=booth_for_archive.clone()
                                                show=Signal::derive(move || show_archive_modal.get())
                                                on_close=close_archive_modal
                                                on_archived=handle_archive_completed.clone()
                                            />
                                        }
                                    })}

                                    <Modal
                                        show=show_copy_modal
                                        on_close=close_copy_modal
                                        title=Signal::derive(move || t!("booth.copy_title")())
                                        size=ModalSize::Medium
                                        action_bar=
                                            view! {
                                                <div class="contents">
                                                    <Button
                                                        on_click=Box::new(move || set_show_copy_modal.set(false))
                                                        variant=ButtonVariant::Secondary
                                                    >
                                                        {t!("common.cancel")()}
                                                    </Button>
                                                    <Button
                                                        variant=ButtonVariant::Primary
                                                        button_type="submit".to_string()
                                                        form="copy-booth-form".to_string()
                                                    >
                                                        {t!("booth.copy_confirm")()}
                                                    </Button>
                                                </div>
                                            }
                                            .into_any()
                                    >
                                        {move || show_copy_modal.get().then(|| {
                                            view! {
                                                <CopyBoothDialog
                                                    source_booth=booth_for_copy.clone()
                                                    form_id="copy-booth-form".to_string()
                                                    autofocus_description=true
                                                    on_submit=handle_copy_booth
                                                />
                                            }
                                        })}
                                    </Modal>

                                    <ConfirmModal
                                        show=show_delete_confirm
                                        on_close=close_delete_modals
                                        on_confirm=handle_delete_booth.clone()
                                        title=Signal::derive(move || t!("booth.delete")())
                                        message=Signal::derive(delete_message)
                                        confirm_text=Signal::derive(move || t!("common.delete")())
                                        cancel_text=Signal::derive(move || t!("common.cancel")())
                                        is_destructive=true
                                    />

                                    <Modal
                                        show=Signal::derive(move || {
                                            is_checking_delete_requirements.get()
                                                || !delete_confirmation_token.get().is_empty()
                                        })
                                        on_close=close_delete_modals
                                        title=Signal::derive(move || {
                                            if is_checking_delete_requirements.get() {
                                                t!("booth.delete")()
                                            } else {
                                                t!("booth.delete_modal.title")()
                                            }
                                        })
                                        size=ModalSize::Medium
                                        action_bar=
                                            view! {
                                                <div class="contents">
                                                    <Show when=move || !is_checking_delete_requirements.get()>
                                                        <Button
                                                            variant=ButtonVariant::Secondary
                                                            on_click=Box::new(close_delete_modals)
                                                        >
                                                            {t!("common.cancel")}
                                                        </Button>
                                                        <Button
                                                            variant=ButtonVariant::Danger
                                                            disabled=!deletion_token_matches.get()
                                                            on_click=Box::new(handle_delete_booth.clone())
                                                        >
                                                            {t!("booth.delete_modal.confirm")}
                                                        </Button>
                                                    </Show>
                                                </div>
                                            }
                                            .into_any()
                                    >
                                        <Show
                                            when=move || is_checking_delete_requirements.get()
                                            fallback=move || {
                                                view! {
                                                    <div class="space-y-4">
                                                        <p class="text-gray-700">
                                                            {move || {
                                                                translate_with_params(
                                                                    "booth.delete_modal.instructions",
                                                                    std::collections::HashMap::from([(
                                                                        "token",
                                                                        delete_confirmation_token.get(),
                                                                    )]),
                                                                )
                                                            }}
                                                        </p>
                                                        <input
                                                            class="w-full rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-red-500"
                                                            placeholder=t!("booth.delete_modal.placeholder")()
                                                            value=move || delete_confirmation_input.get()
                                                            node_ref=delete_confirmation_ref
                                                            on:input=move |ev| {
                                                                set_delete_confirmation_input.set(event_target_value(&ev));
                                                            }
                                                            on:keydown={
                                                                let handle_delete_booth = handle_delete_booth.clone();
                                                                move |ev: web_sys::KeyboardEvent| {
                                                                    if ev.key() == "Enter" && deletion_token_matches.get() {
                                                                        ev.prevent_default();
                                                                        handle_delete_booth();
                                                                    }
                                                                }
                                                            }
                                                        />
                                                    </div>
                                                }
                                            }
                                        >
                                            <div class="flex items-center justify-center py-12">
                                                <div class="text-center">
                                                    <div class="inline-block h-12 w-12 animate-spin rounded-full border-b-2 border-blue-600"></div>
                                                    <p class="mt-4 text-gray-600">{t!("common.loading")}</p>
                                                </div>
                                            </div>
                                        </Show>
                                    </Modal>
                                }
                                .into_any()
                            }
                        >
                            <Card>
                                <div class="text-center py-12">
                                    <p class="text-gray-600">{t!("booth.loading_message")}</p>
                                </div>
                            </Card>
                        </Show>
                    </div>
                </Container>
            </div>

            <div class="hidden print:block">
                {move || booth.get().map(|b| {
                    if b.is_archived() {
                        view! { <PrintArchivedBoothSummary booth=b /> }.into_any()
                    } else if let Some(s) = summary.get() {
                        view! {
                            <PrintBoothSummary summary=s booth_name=b.description.clone() booth_date=b.date />
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                })}
            </div>
        </>
    }
}

#[cfg(test)]
mod tests {
    use super::confirmation_token_from_id_str;

    #[test]
    fn confirmation_token_uses_last_four_alphanumeric_characters() {
        assert_eq!(
            confirmation_token_from_id_str("12345678-1234-1234-1234-1234567890ab"),
            "90AB"
        );
    }

    #[test]
    fn confirmation_token_falls_back_when_id_has_too_few_alphanumeric_characters() {
        assert_eq!(confirmation_token_from_id_str("--"), "DELETE");
        assert_eq!(confirmation_token_from_id_str("a-1"), "DELETE");
    }
}
