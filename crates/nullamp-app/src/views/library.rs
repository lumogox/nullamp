use crate::app::Nullamp;
use crate::message::{LibraryViewMode, Message, TrackAction, TrackSource};
use crate::theme;

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length, Theme};

/// Library browser — shows all tracks in a scrollable list.
pub fn library_view(app: &Nullamp) -> Element<'_, Message> {
    if app.all_tracks.is_empty() {
        return container(
            column![
                text(app.t("no_library")).size(12).color(theme::TEXT_MUTED),
                text(app.t("no_library_hint"))
                    .size(10)
                    .color(theme::TEXT_MUTED),
            ]
            .spacing(8)
            .padding(16),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(theme::BG_DISPLAY.into()),
            ..Default::default()
        })
        .into();
    }

    let mode = app.library_view_mode;
    let is_all = mode == LibraryViewMode::AllTracks;

    let all_btn = toggle_mode_btn(
        app.t("view_all"),
        is_all,
        Message::SetLibraryViewMode(LibraryViewMode::AllTracks),
    );
    let artists_btn = toggle_mode_btn(
        app.t("view_artists"),
        !is_all,
        Message::SetLibraryViewMode(LibraryViewMode::ArtistTree),
    );

    let header = container(
        row![
            all_btn,
            artists_btn,
            Space::with_width(6),
            text(format!("{} tracks", app.all_tracks.len()))
                .size(10)
                .color(theme::TEXT_MUTED),
            Space::with_width(Length::Fill),
            button(text(app.t("queue_all")).size(9).color(theme::ACCENT_AMBER))
                .on_press(Message::QueueAll)
                .padding([2, 8])
                .style(|_theme, _status| button::Style {
                    background: Some(theme::BG_SURFACE.into()),
                    border: iced::Border {
                        color: theme::BORDER_FRAME,
                        width: 1.0,
                        radius: 2.into(),
                    },
                    ..Default::default()
                }),
        ]
        .align_y(Alignment::Center)
        .spacing(4)
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

    let list = if is_all {
        let tracks: Vec<Element<Message>> = app
            .all_tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                track_row(
                    TrackSource::Library,
                    i,
                    track,
                    app.current_index
                        .and_then(|ci| app.playlist.get(ci).map(|t| t.id == track.id))
                        .unwrap_or(false),
                )
            })
            .collect();
        scrollable(column(tracks).spacing(0)).height(Length::Fill)
    } else {
        // Artist tree mode
        let rows = artist_tree_rows(app);
        scrollable(column(rows).spacing(0)).height(Length::Fill)
    };

    column![header, list]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Search view with text input and results.
pub fn search_view(app: &Nullamp) -> Element<'_, Message> {
    let input = text_input(app.t("search_placeholder"), &app.search_query)
        .on_input(Message::SearchQueryChanged)
        .size(11)
        .padding([6, 8]);

    let results_info: Element<'_, Message> = if !app.search_query.is_empty() {
        let queue_all_btn: Element<'_, Message> =
            button(text(app.t("queue_all")).size(9).color(theme::ACCENT_AMBER))
                .on_press(Message::QueueAllSearchResults)
                .padding([2, 8])
                .style(|_theme: &Theme, _status: button::Status| button::Style {
                    background: Some(theme::BG_SURFACE.into()),
                    border: iced::Border {
                        color: theme::BORDER_FRAME,
                        width: 1.0,
                        radius: 2.into(),
                    },
                    ..Default::default()
                })
                .into();
        row![
            text(format!("{} {}", app.search_results.len(), app.t("results")))
                .size(9)
                .color(theme::TEXT_MUTED),
            Space::with_width(Length::Fill),
            queue_all_btn,
        ]
        .align_y(Alignment::Center)
        .into()
    } else {
        text(app.t("search_type_hint"))
            .size(9)
            .color(theme::TEXT_MUTED)
            .into()
    };

    let tracks: Vec<Element<Message>> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, track)| track_row(TrackSource::Search, i, track, false))
        .collect();

    let list = scrollable(column(tracks).spacing(0)).height(Length::Fill);

    column![
        container(column![input, results_info].spacing(4).padding([8, 8]))
            .style(|_theme| container::Style {
                background: Some(theme::BG_SURFACE.into()),
                ..Default::default()
            })
            .width(Length::Fill),
        list,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Build the expandable artist tree rows.
fn artist_tree_rows(app: &Nullamp) -> Vec<Element<'_, Message>> {
    let mut rows: Vec<Element<Message>> = Vec::new();

    // Group tracks by artist
    for artist_name in &app.artists {
        let artist_tracks: Vec<(usize, &nullamp_core::models::Track)> = app
            .all_tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.artist.as_deref().unwrap_or("Unknown") == artist_name.as_str())
            .collect();

        let count = artist_tracks.len();
        let is_expanded = app.expanded_artists.contains(artist_name.as_str());
        let arrow = if is_expanded { "▼" } else { "▶" };

        let artist_row = button(
            row![
                text(arrow)
                    .size(9)
                    .color(theme::ACCENT_AMBER)
                    .width(Length::Fixed(14.0)),
                text(artist_name).size(10).color(theme::TEXT_PRIMARY),
                Space::with_width(Length::Fill),
                text(format!("{count} tracks"))
                    .size(8)
                    .color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center)
            .padding([5, 8])
            .spacing(4),
        )
        .on_press(Message::ToggleArtistExpanded(artist_name.clone()))
        .width(Length::Fill)
        .padding(0)
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(theme::BG_SURFACE.into()),
                _ => Some(theme::BG_DISPLAY.into()),
            },
            ..Default::default()
        });

        rows.push(artist_row.into());

        if is_expanded {
            // Group by album within this artist
            let mut albums: Vec<String> = artist_tracks
                .iter()
                .map(|(_, t)| t.album.clone().unwrap_or_else(|| "(No Album)".to_string()))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            albums.sort();

            for album_name in &albums {
                let album_tracks: Vec<(usize, &nullamp_core::models::Track)> = artist_tracks
                    .iter()
                    .filter(|(_, t)| {
                        t.album.as_deref().unwrap_or("(No Album)") == album_name.as_str()
                    })
                    .copied()
                    .collect();

                // Album header row (indented)
                let album_row = container(
                    row![
                        Space::with_width(Length::Fixed(16.0)),
                        text("▸")
                            .size(8)
                            .color(theme::TEXT_MUTED)
                            .width(Length::Fixed(12.0)),
                        text(album_name.clone()).size(9).color(theme::TEXT_MUTED),
                        Space::with_width(Length::Fill),
                        text(format!("{}", album_tracks.len()))
                            .size(8)
                            .color(theme::TEXT_MUTED),
                    ]
                    .align_y(Alignment::Center)
                    .padding([3, 8])
                    .spacing(4),
                )
                .style(|_theme| container::Style {
                    background: Some(
                        iced::Color {
                            a: 0.3,
                            ..theme::BG_DISPLAY
                        }
                        .into(),
                    ),
                    ..Default::default()
                })
                .width(Length::Fill);

                rows.push(album_row.into());

                // Track rows (double-indented)
                for (track_num, (lib_idx, track)) in album_tracks.iter().enumerate() {
                    let is_current = app
                        .current_index
                        .and_then(|ci| app.playlist.get(ci))
                        .map(|t| t.id == track.id)
                        .unwrap_or(false);

                    rows.push(indented_track_row(
                        TrackSource::Library,
                        *lib_idx,
                        track,
                        track_num + 1,
                        is_current,
                    ));
                }
            }
        }
    }

    rows
}

/// Track row with right-click context menu support.
pub fn track_row<'a>(
    source: TrackSource,
    index: usize,
    track: &'a nullamp_core::models::Track,
    is_current: bool,
) -> Element<'a, Message> {
    let title = track.title.as_deref().unwrap_or("Unknown");
    let artist = track.artist.as_deref().unwrap_or("Unknown");
    let dur = fmt_duration(track.duration_secs);

    let title_color = if is_current {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_PRIMARY
    };

    let content = row![
        text(format!("{}.", index + 1))
            .size(9)
            .color(theme::TEXT_MUTED)
            .width(Length::Fixed(24.0)),
        column![
            text(title).size(10).color(title_color),
            text(artist).size(8).color(theme::TEXT_MUTED),
        ]
        .spacing(1),
        Space::with_width(Length::Fill),
        text(dur).size(9).color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center)
    .padding([3, 8])
    .spacing(4);

    let play_btn = button(content)
        .on_press(Message::TrackAction(source, index, TrackAction::PlayNow))
        .width(Length::Fill)
        .padding(0)
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Some(theme::BG_SURFACE.into()),
                _ => {
                    if is_current {
                        Some(iced::Background::Color(iced::Color {
                            a: 0.15,
                            ..theme::ACCENT_AMBER
                        }))
                    } else {
                        None
                    }
                }
            };
            button::Style {
                background: bg,
                border: iced::Border::default(),
                ..Default::default()
            }
        });

    mouse_area(play_btn)
        .on_right_press(Message::ContextMenuOpen { source, index })
        .into()
}

/// Track row indented for artist tree view.
fn indented_track_row<'a>(
    source: TrackSource,
    index: usize,
    track: &'a nullamp_core::models::Track,
    track_num: usize,
    is_current: bool,
) -> Element<'a, Message> {
    let title = track.title.as_deref().unwrap_or("Unknown");
    let dur = fmt_duration(track.duration_secs);
    let title_color = if is_current {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_PRIMARY
    };

    let content = row![
        Space::with_width(Length::Fixed(32.0)), // indent
        text(format!("{track_num}."))
            .size(8)
            .color(theme::TEXT_MUTED)
            .width(Length::Fixed(20.0)),
        text(title).size(9).color(title_color),
        Space::with_width(Length::Fill),
        text(dur).size(8).color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center)
    .padding([3, 8])
    .spacing(4);

    let play_btn = button(content)
        .on_press(Message::TrackAction(source, index, TrackAction::PlayNow))
        .width(Length::Fill)
        .padding(0)
        .style(move |_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(theme::BG_SURFACE.into()),
                _ => {
                    if is_current {
                        Some(iced::Background::Color(iced::Color {
                            a: 0.15,
                            ..theme::ACCENT_AMBER
                        }))
                    } else {
                        None
                    }
                }
            },
            border: iced::Border::default(),
            ..Default::default()
        });

    mouse_area(play_btn)
        .on_right_press(Message::ContextMenuOpen { source, index })
        .into()
}

/// Toggle button for All / Artists mode.
fn toggle_mode_btn<'a>(
    label: &'a str,
    active: bool,
    msg: Message,
) -> iced::widget::Button<'a, Message> {
    let color = if active {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_MUTED
    };
    button(text(label).size(8).color(color))
        .on_press(msg)
        .padding([2, 6])
        .style(move |_theme, _status| button::Style {
            background: if active {
                Some(theme::BG_DISPLAY.into())
            } else {
                None
            },
            border: iced::Border {
                color: if active {
                    theme::BORDER_FRAME
                } else {
                    iced::Color::TRANSPARENT
                },
                width: if active { 1.0 } else { 0.0 },
                radius: 2.into(),
            },
            ..Default::default()
        })
}

/// Format duration seconds as M:SS.
pub fn fmt_duration(secs: Option<f64>) -> String {
    secs.map(|d| {
        let m = (d / 60.0) as u32;
        let s = (d % 60.0) as u32;
        format!("{m}:{s:02}")
    })
    .unwrap_or_default()
}
