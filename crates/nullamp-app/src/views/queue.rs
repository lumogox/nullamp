use crate::app::Nullamp;
use crate::icons::icon_text;
use crate::message::{Message, TrackSource};
use crate::theme;
use crate::views::library::fmt_duration;

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use iced_fonts::bootstrap::Bootstrap;

/// Queue panel — always visible at the bottom, shows current playlist.
pub fn queue_panel(app: &Nullamp) -> Element<'_, Message> {
    // Identify the currently-playing track by its playlist index.
    let current_path: Option<&str> = app
        .current_index
        .and_then(|i| app.playlist.get(i))
        .map(|t| t.file_path.as_str());
    let total_dur: f64 = app.playlist.iter().filter_map(|t| t.duration_secs).sum();
    let dur_m = (total_dur / 60.0) as u32;
    let dur_s = (total_dur % 60.0) as u32;

    // Clear queue button (only when queue has tracks)
    let clear_btn: Element<Message> = if !app.playlist.is_empty() {
        button(icon_text(Bootstrap::Trash, 10.0, theme::TEXT_MUTED))
            .on_press(Message::ClearQueue)
            .padding([2, 4])
            .style(|_theme, status| button::Style {
                background: match status {
                    button::Status::Hovered => Some(theme::BG_DISPLAY.into()),
                    _ => None,
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::with_width(0).into()
    };

    // Header
    let header = container(
        row![
            text(app.t("queue").to_uppercase())
                .size(10)
                .color(theme::ACCENT_AMBER),
            Space::with_width(Length::Fill),
            text(format!(
                "{} {} · {}:{:02}",
                app.playlist.len(),
                app.t("tracks"),
                dur_m,
                dur_s
            ))
            .size(9)
            .color(theme::TEXT_MUTED),
            clear_btn,
        ]
        .align_y(Alignment::Center)
        .spacing(6)
        .padding([4, 8]),
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
    .width(Length::Fill);

    // Queue content
    let content: Element<'_, Message> = if app.playlist.is_empty() {
        container(
            column![
                text(app.t("queue_empty")).size(11).color(theme::TEXT_MUTED),
                text(app.t("queue_hint")).size(9).color(theme::TEXT_MUTED),
            ]
            .spacing(4)
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .padding(16)
        .center_x(Length::Fill)
        .into()
    } else {
        let rows: Vec<Element<Message>> = app
            .playlist
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let is_current = current_path == Some(track.file_path.as_str());
                let title = track.title.as_deref().unwrap_or("Unknown");
                let artist = track.artist.as_deref().unwrap_or("Unknown");
                let dur = fmt_duration(track.duration_secs);

                let title_color = if is_current {
                    theme::ACCENT_AMBER
                } else {
                    theme::TEXT_PRIMARY
                };

                let bg = if is_current {
                    Some(iced::Background::Color(iced::Color {
                        a: 0.12,
                        ..theme::ACCENT_AMBER
                    }))
                } else {
                    None
                };

                let row_content = row![
                    text(format!("{}.", i + 1))
                        .size(9)
                        .color(theme::TEXT_MUTED)
                        .width(Length::Fixed(20.0)),
                    column![
                        text(title).size(9).color(title_color),
                        text(artist).size(7).color(theme::TEXT_MUTED),
                    ]
                    .spacing(1),
                    Space::with_width(Length::Fill),
                    text(dur).size(8).color(theme::TEXT_MUTED),
                ]
                .align_y(Alignment::Center)
                .padding([3, 8])
                .spacing(4);

                let jump_btn = button(row_content)
                    .on_press(Message::JumpToQueueTrack(i))
                    .width(Length::Fill)
                    .padding(0)
                    .style(move |_theme, status| button::Style {
                        background: match status {
                            button::Status::Hovered => Some(theme::BG_SURFACE.into()),
                            _ => bg,
                        },
                        border: iced::Border::default(),
                        ..Default::default()
                    });

                mouse_area(jump_btn)
                    .on_right_press(Message::ContextMenuOpen {
                        source: TrackSource::Queue,
                        index: i,
                    })
                    .into()
            })
            .collect();

        scrollable(column(rows).spacing(0))
            .height(Length::Fill)
            .into()
    };

    column![header, content]
        .width(Length::Fill)
        .height(Length::Fixed(150.0))
        .into()
}
