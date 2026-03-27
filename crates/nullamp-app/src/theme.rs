use iced::color;
use iced::{Color, Theme};

// ─── Color constants matching the Winamp/NULLAMP CSS tokens ───

pub const BG_PRIMARY: Color = color!(0x1a, 0x1a, 0x2e); // #1a1a2e
pub const BG_SURFACE: Color = color!(0x23, 0x23, 0x42); // #232342
pub const BG_DISPLAY: Color = color!(0x0a, 0x0a, 0x0a); // #0a0a0a
pub const TEXT_PRIMARY: Color = color!(0x00, 0xff, 0x00); // #00ff00
pub const TEXT_MUTED: Color = color!(0x61, 0x61, 0x7a); // #61617a
pub const ACCENT_AMBER: Color = color!(0xff, 0x8c, 0x00); // #ff8c00
pub const ERROR: Color = color!(0xff, 0x41, 0x36); // #ff4136
pub const BORDER_FRAME: Color = color!(0x3a, 0x3a, 0x6a); // #3a3a6a

/// Build the custom NULLAMP dark theme.
pub fn nullamp_theme() -> Theme {
    Theme::custom(
        "Nullamp".to_string(),
        iced::theme::Palette {
            background: BG_PRIMARY,
            text: TEXT_PRIMARY,
            primary: ACCENT_AMBER,
            success: TEXT_PRIMARY,
            danger: ERROR,
        },
    )
}
