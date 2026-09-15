use leptos::prelude::*;
use leptos::svg;

pub use icondata::{
    LuCalendar, LuCheck, LuChevronDown, LuChevronLeft, LuChevronRight, LuChevronsLeft,
    LuChevronsRight, LuCopy, LuDownload, LuInbox, LuInfo, LuKeyboard, LuListX, LuLoaderCircle,
    LuMinus, LuPlus, LuPrinter, LuSettings, LuShare2, LuStore, LuTrash2, LuUpload, LuUsers,
    LuVolume2, LuVolumeX, LuWallet, LuX,
};
// Aliases for call sites that still use old Lucide names
pub use icondata::LuEllipsisVertical as LuMoreVertical;
pub use icondata::LuSquarePen as LuPenSquare;
pub use icondata::LuTriangleAlert as LuAlertTriangle;

// ponytail: own Icon component — leptos_icons 0.7 dropped class prop, icondata_core is all we need
#[component]
pub fn Icon(
    #[prop(into)] icon: Signal<icondata_core::Icon>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    move || {
        let icon = icon.get();
        let mut data = String::with_capacity(icon.data.len() + 7);
        data.push_str("<g>");
        data.push_str(icon.data);
        data.push_str("</g>");

        svg::svg()
            .class(class.get().unwrap_or_default())
            .attr("viewBox", icon.view_box)
            .attr("fill", icon.fill.unwrap_or("currentColor"))
            .attr("stroke", icon.stroke)
            .attr("stroke-width", icon.stroke_width)
            .attr("stroke-linecap", icon.stroke_linecap)
            .attr("stroke-linejoin", icon.stroke_linejoin)
            .attr("role", "graphics-symbol")
            .attr("aria-hidden", "true")
            .child(svg::InertElement::new(data))
    }
}

#[component]
pub fn SpinnerIcon(#[prop(optional)] class: Option<String>) -> impl IntoView {
    let class = class.unwrap_or_else(|| "h-5 w-5 animate-spin".to_string());
    view! { <Icon icon=LuLoaderCircle class=class /> }
}
