use leptos::*;

/// Card component for consistent layout
#[component]
pub fn Card(
    /// Card content
    children: Children,
    /// Card title (static string)
    #[prop(optional)]
    title: Option<&'static str>,
    /// Card title (dynamic view) - takes precedence over static title
    #[prop(optional)]
    title_view: Option<View>,
    /// Additional CSS classes
    #[prop(optional)]
    class: Option<&'static str>,
    /// ARIA label for accessibility
    #[prop(optional)]
    aria_label: Option<String>,
) -> impl IntoView {
    let additional_classes = class.unwrap_or("");
    let card_classes = format!("bg-white rounded-lg shadow-md p-6 {}", additional_classes);

    view! {
        <article class=card_classes aria-label=aria_label role="region">
            {if let Some(tv) = title_view {
                view! {
                    <h2 class="text-xl font-semibold mb-4">{tv}</h2>
                }.into_view()
            } else if let Some(t) = title {
                view! {
                    <h2 class="text-xl font-semibold mb-4">{t}</h2>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
            {children()}
        </article>
    }
}

/// Container component for centering content
#[component]
pub fn Container(
    /// Container content
    children: Children,
    /// Maximum width (default: "max-w-7xl")
    #[prop(optional)]
    max_width: Option<&'static str>,
    /// Additional CSS classes
    #[prop(optional)]
    class: Option<&'static str>,
    /// Use as a landmark region
    #[prop(optional)]
    as_landmark: Option<bool>,
    /// ARIA label for accessibility (when used as landmark)
    #[prop(optional)]
    aria_label: Option<String>,
) -> impl IntoView {
    let max_width = max_width.unwrap_or("max-w-7xl");
    let additional_classes = class.unwrap_or("");
    let container_classes = format!(
        "mx-auto px-4 sm:px-6 lg:px-8 {} {}",
        max_width, additional_classes
    );

    let as_landmark = as_landmark.unwrap_or(false);

    view! {
        <div
            class=container_classes
            role=if as_landmark { Some("region") } else { None }
            aria-label=aria_label
        >
            {children()}
        </div>
    }
}
