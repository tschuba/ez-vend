use crate::components::*;
use crate::state::use_app_state;
use crate::t;
use leptos::*;
use leptos_router::use_navigate;

#[component]
pub fn HomePage() -> impl IntoView {
    // Smart redirect based on booth availability
    let (is_redirecting, set_is_redirecting) = create_signal(true);

    {
        let app_state = use_app_state();
        create_effect(move |_| {
            // Wait for app state to be ready
            let Some(Ok(state)) = app_state.get() else {
                return;
            };

            let booth_repository = state.booth_repository.clone();
            let navigate = use_navigate();

            spawn_local(async move {
                // Load all booths and check if any exist
                match booth_repository.find_all().await {
                    Ok(booths) => {
                        let has_booths = !booths.is_empty();

                        if has_booths {
                            navigate("/checkout", Default::default());
                        } else {
                            navigate("/booths", Default::default());
                        }
                    }
                    Err(_) => {
                        // Error loading booths - show home page
                        set_is_redirecting.set(false);
                    }
                }
            });
        });
    }

    view! {
        <Container class="mt-6">
            <Show
                when=move || !is_redirecting.get()
                fallback=move || view! {
                    <div class="pb-12 text-center">
                        <p class="text-gray-600">{t!("common.loading")}</p>
                    </div>
                }
            >
                <div class="pb-12 text-center">
                    <Card>
                        <h2 class="text-xl font-semibold mb-4">{t!("app.title")}</h2>
                        <p class="text-gray-600 mb-6">
                            {t!("home.error_loading_booths")}
                        </p>
                        <div class="flex gap-4 justify-center flex-wrap">
                            <a href=format!("{}/booths", crate::base_path())>
                                <Button>{t!("booth.list_title")}</Button>
                            </a>
                            <a href=format!("{}/checkout", crate::base_path())>
                                <Button>{t!("checkout.title")}</Button>
                            </a>
                        </div>
                    </Card>
                </div>
            </Show>
        </Container>
    }
}
