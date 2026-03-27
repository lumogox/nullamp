use iced::widget::text;
use iced::Color;
use iced_fonts::bootstrap::{icon_to_char, Bootstrap};
use iced_fonts::BOOTSTRAP_FONT;

/// Create a Bootstrap icon Text widget (for embedding in row!/column! macros).
pub fn icon_text(bs: Bootstrap, size: f32, color: Color) -> iced::widget::Text<'static> {
    text(icon_to_char(bs).to_string())
        .font(BOOTSTRAP_FONT)
        .size(size)
        .color(color)
}
