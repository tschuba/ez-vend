use domain::models::TailwindColor;

/// Product tile grid shared by the checkout and the config tab. Tiles wrap
/// natively; the last row may be incomplete.
pub const PRODUCT_GRID_CLASS: &str = "grid grid-cols-3 gap-2";

/// Left accent border color for a product tile, taken from its group color.
pub fn color_accent_class(c: TailwindColor) -> &'static str {
    match c {
        TailwindColor::Red => "border-l-red-400",
        TailwindColor::Orange => "border-l-orange-400",
        TailwindColor::Amber => "border-l-amber-400",
        TailwindColor::Green => "border-l-green-400",
        TailwindColor::Teal => "border-l-teal-400",
        TailwindColor::Blue => "border-l-blue-400",
        TailwindColor::Violet => "border-l-violet-400",
        TailwindColor::Pink => "border-l-pink-400",
    }
}
