use crate::app::{ContextMenuState, Nullamp};
use crate::message::{Message, TrackAction, TrackSource};
use crate::theme;

use iced::widget::{button, column, container, text};
use iced::{Border, Element, Length};

/// Floating context menu rendered over the track list.
/// Appears at the cursor position (positioned by parent via Space offsets).
pub fn context_menu_widget<'a>(
    ctx: &'a ContextMenuState,
    app: &'a Nullamp,
) -> Element<'a, Message> {
    let src = ctx.source;
    let idx = ctx.track_index;

    // Determine which track we're acting on (for display)
    let track_name = match src {
        TrackSource::Library => app
            .all_tracks
            .get(idx)
            .and_then(|t| t.title.as_deref())
            .unwrap_or("Track"),
        TrackSource::Search => app
            .search_results
            .get(idx)
            .and_then(|t| t.title.as_deref())
            .unwrap_or("Track"),
        TrackSource::Queue => app
            .playlist
            .get(idx)
            .and_then(|t| t.title.as_deref())
            .unwrap_or("Track"),
    };

    // Truncate long names
    let display_name = if track_name.len() > 22 {
        format!("{}…", &track_name[..21])
    } else {
        track_name.to_string()
    };

    let label = text(display_name).size(8).color(theme::TEXT_MUTED);

    let menu_btn = |label_str: &'static str, msg: Message| {
        button(text(label_str).size(10).color(theme::TEXT_PRIMARY))
            .on_press(msg)
            .width(Length::Fill)
            .padding([5, 12])
            .style(|_theme, status| button::Style {
                background: match status {
                    button::Status::Hovered | button::Status::Pressed => {
                        Some(theme::BG_PRIMARY.into())
                    }
                    _ => None,
                },
                text_color: theme::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            })
    };

    let mut items: Vec<Element<Message>> = vec![
        container(label).padding([6, 12]).width(Length::Fill).into(),
        // Separator
        container(iced::widget::horizontal_rule(1))
            .padding([0, 8])
            .width(Length::Fill)
            .into(),
    ];

    match src {
        TrackSource::Library | TrackSource::Search => {
            items.push(
                menu_btn(
                    "▶  Play Now",
                    Message::TrackAction(src, idx, TrackAction::PlayNow),
                )
                .into(),
            );
            items.push(
                menu_btn(
                    "↪  Play Next",
                    Message::TrackAction(src, idx, TrackAction::PlayNext),
                )
                .into(),
            );
            items.push(
                menu_btn(
                    "+  Add to Queue",
                    Message::TrackAction(src, idx, TrackAction::AddToQueue),
                )
                .into(),
            );
        }
        TrackSource::Queue => {
            items.push(menu_btn("▶  Play Now", Message::JumpToQueueTrack(idx)).into());
            items.push(
                menu_btn(
                    "✕  Remove",
                    Message::TrackAction(src, idx, TrackAction::RemoveFromQueue),
                )
                .into(),
            );
        }
    }

    container(column(items).spacing(0))
        .style(|_theme| container::Style {
            background: Some(theme::BG_SURFACE.into()),
            border: Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 3.into(),
            },
            shadow: iced::Shadow {
                color: iced::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
                offset: iced::Vector { x: 2.0, y: 2.0 },
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .width(Length::Fixed(160.0))
        .into()
}
