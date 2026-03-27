use crate::app::Nullamp;
use crate::message::Message;
use crate::theme;

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length};

/// Status bar at the bottom — shows track count or clickable scan progress.
pub fn status_bar(app: &Nullamp) -> Element<'_, Message> {
    let content: Element<Message> = if app.is_scanning {
        let info = if let Some(ref p) = app.scan_progress {
            let rate_part = if p.rate > 0.0 {
                format!("  ·  {:.0}/s", p.rate)
            } else {
                String::new()
            };
            format!(
                "{}  —  {} / {} files{}",
                p.phase.to_uppercase(),
                p.files_processed,
                p.files_found,
                rate_part
            )
        } else {
            app.t("scanning").to_string()
        };

        button(
            row![
                text("●").size(8).color(theme::ACCENT_AMBER),
                Space::with_width(6),
                text(info).size(9).color(theme::TEXT_PRIMARY),
                Space::with_width(Length::Fill),
                text("details →").size(8).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenScanModal)
        .padding([0, 8])
        .width(Length::Fill)
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered | button::Status::Pressed => Some(
                    iced::Color {
                        a: 0.08,
                        ..theme::ACCENT_AMBER
                    }
                    .into(),
                ),
                _ => None,
            },
            border: iced::Border::default(),
            ..Default::default()
        })
        .into()
    } else {
        container(
            row![
                text(format!(
                    "{} {}",
                    app.all_tracks.len(),
                    app.t("tracks_in_library")
                ))
                .size(9)
                .color(theme::TEXT_MUTED),
                Space::with_width(Length::Fill),
                text("Nullamp v0.1").size(9).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        )
        .padding([0, 8])
        .into()
    };

    container(content)
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
        .height(Length::Fixed(24.0))
        .center_y(Length::Fixed(24.0))
        .into()
}
