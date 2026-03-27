mod app;
mod icons;
mod message;
mod theme;
mod views;

use app::Nullamp;

const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const SHARE_TECH_MONO: &[u8] = include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf");

pub const FONT_LED: iced::Font = iced::Font::with_name("Share Tech Mono");

fn main() -> iced::Result {
    env_logger::init();

    iced::application(Nullamp::title, Nullamp::update, Nullamp::view)
        .theme(Nullamp::theme)
        .subscription(Nullamp::subscription)
        .window_size(iced::Size::new(480.0, 720.0))
        .decorations(false)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .font(JETBRAINS_MONO)
        .font(SHARE_TECH_MONO)
        .default_font(iced::Font::with_name("JetBrains Mono"))
        .run_with(Nullamp::new)
}
