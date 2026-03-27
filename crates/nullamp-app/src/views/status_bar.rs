use crate::app::Nullamp;
use crate::message::Message;
use crate::theme;

use iced::widget::{container, row, text, Space};
use iced::{Alignment, Element, Length};

/// Status bar at the bottom — shows track count or scan progress.
pub fn status_bar(app: &Nullamp) -> Element<'_, Message> {
    let content: Element<Message> = if app.is_scanning {
        if let Some(ref progress) = app.scan_progress {
            let phase = &progress.phase;
            let info = format!(
                "{} — {}/{} files",
                phase, progress.files_processed, progress.files_found
            );
            row![
                text(info).size(9).color(theme::TEXT_PRIMARY),
                Space::with_width(Length::Fill),
            ]
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                text(app.t("scanning")).size(9).color(theme::TEXT_PRIMARY),
                Space::with_width(Length::Fill),
            ]
            .align_y(Alignment::Center)
            .into()
        }
    } else {
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
        .align_y(Alignment::Center)
        .into()
    };

    container(container(content).padding([0, 8]))
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
