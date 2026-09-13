use leptos::*;
pub use leptos_icons::Icon;

pub use icondata::{
    LuAlertTriangle, LuCalendar, LuCheck, LuChevronDown, LuChevronLeft, LuChevronRight,
    LuChevronsLeft, LuChevronsRight, LuCopy, LuDownload, LuInbox, LuInfo, LuKeyboard, LuListX,
    LuLoader2, LuMoreVertical, LuPenSquare, LuPlus, LuPrinter, LuSettings, LuShare2, LuStore,
    LuTrash2, LuUpload, LuUsers, LuVolume2, LuVolumeX, LuWallet, LuX,
};

#[component]
pub fn SpinnerIcon(#[prop(optional)] class: Option<String>) -> impl IntoView {
    let class = class.unwrap_or_else(|| "h-5 w-5 animate-spin".to_string());
    view! { <Icon icon=LuLoader2 class=class /> }
}
