pub mod context_menu;
pub mod eq_canvas;
mod equalizer;
mod library;
mod player_bar;
mod queue;
mod scan_modal;
mod settings;
mod status_bar;
mod voice;

use crate::app::Nullamp;
use crate::icons::icon_text;
use crate::message::{Message, Tab};
use crate::theme;

use iced::widget::{button, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};
use iced_fonts::bootstrap::{icon_to_char, Bootstrap};
use iced_fonts::BOOTSTRAP_FONT;

pub use player_bar::player_bar;
pub use queue::queue_panel;
pub use scan_modal::scan_modal_view;
pub use settings::settings_view;
pub use status_bar::status_bar;

/// Title bar with app name, window controls, language toggle, and settings button.
/// Uses a custom draggable bar since decorations are disabled.
pub fn title_bar(app: &Nullamp) -> Element<'_, Message> {
    let title = text("NULLAMP").size(11).color(theme::ACCENT_AMBER);

    // Language toggle — borderless text button, muted → amber on hover.
    let lang = app.settings.language.as_str();
    let lang_label = if lang == "en" { "EN" } else { "ES" };
    let lang_btn = button(text(lang_label).size(8))
        .on_press(Message::LanguageToggled)
        .padding([0, 8])
        .style(|_theme, status| button::Style {
            background: None,
            text_color: match status {
                button::Status::Hovered | button::Status::Pressed => theme::ACCENT_AMBER,
                _ => theme::TEXT_MUTED,
            },
            border: iced::Border::default(),
            ..Default::default()
        });

    // Settings gear — borderless, muted → primary on hover.
    let settings_btn = icon_ctrl_btn(
        Bootstrap::GearFill,
        theme::BG_PRIMARY,
        theme::TEXT_PRIMARY,
        Message::OpenSettings,
    );

    // Window controls — square ghost buttons matching React app style.
    let minimize_btn = icon_ctrl_btn(
        Bootstrap::DashLg,
        theme::BG_PRIMARY,
        theme::TEXT_PRIMARY,
        Message::MinimizeWindow,
    );
    let close_btn = icon_ctrl_btn(
        Bootstrap::XLg,
        iced::Color {
            a: 0.2,
            ..theme::ERROR
        },
        theme::ERROR,
        Message::CloseWindow,
    );

    let inner = row![
        title,
        Space::with_width(Length::Fill),
        lang_btn,
        settings_btn,
        minimize_btn,
        close_btn,
    ]
    .spacing(0)
    .align_y(Alignment::Center)
    .padding([0, 4]);

    // Wrap in mouse_area so clicking the empty title bar area drags the window.
    // Buttons inside capture their own clicks and won't trigger the drag.
    let draggable = mouse_area(
        container(inner)
            .style(|_theme| container::Style {
                background: Some(theme::BG_SURFACE.into()),
                border: iced::Border {
                    color: theme::BORDER_FRAME,
                    width: 1.0,
                    radius: 0.into(),
                },
                ..Default::default()
            })
            .width(Length::Fill),
    )
    .on_press(Message::DragWindow);

    draggable.into()
}

/// Square ghost icon button matching the React app's title bar controls.
/// Normal: transparent bg, muted icon. Hover: colored bg fill + bright icon.
fn icon_ctrl_btn(
    bs: Bootstrap,
    hover_bg: iced::Color,
    hover_color: iced::Color,
    msg: Message,
) -> iced::widget::Button<'static, Message> {
    // Text widget WITHOUT .color() so button::Style.text_color drives the color.
    let icon = text(icon_to_char(bs).to_string())
        .font(BOOTSTRAP_FONT)
        .size(11.0);

    button(
        container(icon)
            .width(Length::Fixed(30.0))
            .height(Length::Fixed(30.0))
            .center_x(Length::Fixed(30.0))
            .center_y(Length::Fixed(30.0)),
    )
    .on_press(msg)
    .padding(0)
    .style(move |_theme, status| button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(hover_bg.into()),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => hover_color,
            _ => theme::TEXT_MUTED,
        },
        border: iced::Border::default(),
        ..Default::default()
    })
}

/// Tab bar for switching between Library, Search, Voice, EQ.
pub fn tab_bar(app: &Nullamp) -> Element<'_, Message> {
    let tabs: [(Tab, &'static str, Bootstrap); 4] = [
        (Tab::Library, "tab_library", Bootstrap::MusicNoteList),
        (Tab::Search, "tab_search", Bootstrap::Search),
        (Tab::Voice, "tab_voice", Bootstrap::MicFill),
        (Tab::Equalizer, "tab_eq", Bootstrap::Sliders),
    ];

    let tab_buttons: Vec<Element<Message>> = tabs
        .iter()
        .map(|(tab, key, icon_bs)| {
            let label = app.t(key);
            let is_active = app.active_tab == *tab;
            let text_color = if is_active {
                theme::ACCENT_AMBER
            } else {
                theme::TEXT_MUTED
            };
            let _hover_bg = theme::BG_SURFACE;

            button(
                row![
                    icon_text(*icon_bs, 11.0, text_color),
                    text(label).size(11).color(text_color),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .on_press(Message::TabSelected(*tab))
            .padding([6, 14])
            .style(move |_theme, status| {
                let bg = if is_active {
                    theme::BG_DISPLAY
                } else {
                    match status {
                        button::Status::Hovered | button::Status::Pressed => theme::BG_PRIMARY,
                        _ => theme::BG_SURFACE,
                    }
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border::default(),
                    ..Default::default()
                }
            })
            .into()
        })
        .collect();

    container(
        row(tab_buttons)
            .spacing(2)
            .padding([4, 8])
            .align_y(Alignment::Center),
    )
    .style(|_theme| container::Style {
        background: Some(theme::BG_SURFACE.into()),
        border: iced::Border {
            color: theme::BORDER_FRAME,
            width: 1.0,
            radius: 0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fixed(36.0))
    .into()
}

/// Route to the active tab's content view.
pub fn tab_content(app: &Nullamp) -> Element<'_, Message> {
    match app.active_tab {
        Tab::Library => library::library_view(app),
        Tab::Search => library::search_view(app),
        Tab::Voice => voice::voice_view(app),
        Tab::Equalizer => equalizer::equalizer_view(app),
    }
}
