use crate::app::Nullamp;
use crate::icons::icon_text;
use crate::message::Message;
use crate::theme;

use iced::widget::{button, column, container, row, slider, text, Space};
use iced::{Alignment, Element, Length};
use iced_fonts::bootstrap::Bootstrap;
use nullamp_audio::player::PlayState;

/// Format seconds as M:SS, or "--:--" when no track is loaded.
fn fmt_time(secs: f64, has_track: bool) -> String {
    if !has_track {
        return "--:--".to_string();
    }
    let m = (secs / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    format!("{m}:{s:02}")
}

/// Player bar — display area + seek bar + transport controls.
/// Matches the original app layout: recessed display on top, buttons below.
pub fn player_bar(app: &Nullamp) -> Element<'_, Message> {
    let is_playing = app.player.state() == PlayState::Playing;

    // Current track info
    let has_track = app.current_index.is_some();
    let no_track = app.t("no_track_loaded");
    let (track_title, _track_artist, duration) = if let Some(idx) = app.current_index {
        if let Some(track) = app.playlist.get(idx) {
            (
                track.title.as_deref().unwrap_or("Unknown"),
                track.artist.as_deref().unwrap_or("Unknown"),
                track.duration_secs.unwrap_or(0.0),
            )
        } else {
            (no_track, "", 0.0)
        }
    } else {
        (no_track, "", 0.0)
    };

    let position = app.player.position().as_secs_f64();
    let seek_pct = if duration > 0.0 {
        (position / duration) as f32
    } else {
        0.0
    };

    // ── Display area (recessed dark box) ──
    // Share Tech Mono gives the authentic Winamp LED-display look.
    let display_title = text(track_title)
        .size(12)
        .font(crate::FONT_LED)
        .color(theme::TEXT_PRIMARY);
    let display_time = text(format!(
        "{} / {}",
        fmt_time(position, has_track),
        fmt_time(duration, has_track)
    ))
    .size(10)
    .font(crate::FONT_LED)
    .color(theme::TEXT_PRIMARY);

    let display = container(
        column![display_title, display_time]
            .spacing(2)
            .padding([8, 10]),
    )
    .style(|_theme| container::Style {
        background: Some(theme::BG_DISPLAY.into()),
        border: iced::Border {
            color: theme::BORDER_FRAME,
            width: 1.0,
            radius: 3.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill);

    // ── Seek slider ──
    let seek = slider(0.0..=1.0, seek_pct, Message::Seek)
        .width(Length::Fill)
        .step(0.001);

    // ── Transport buttons (Bootstrap icons) ──
    let prev_btn = transport_button(Bootstrap::SkipBackwardFill, Message::PrevTrack);
    let play_pause_btn = transport_button(
        if is_playing {
            Bootstrap::PauseFill
        } else {
            Bootstrap::PlayFill
        },
        Message::TogglePlayPause,
    );
    let stop_btn = transport_button(Bootstrap::StopFill, Message::Stop);
    let next_btn = transport_button(Bootstrap::SkipForwardFill, Message::NextTrack);

    let controls = row![prev_btn, play_pause_btn, stop_btn, next_btn]
        .spacing(4)
        .align_y(Alignment::Center);

    // ── Shuffle / Repeat ──
    let shuffle_color = if app.shuffle {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_MUTED
    };
    let shuffle_btn = button(icon_text(Bootstrap::Shuffle, 11.0, shuffle_color))
        .on_press(Message::ToggleShuffle)
        .padding([2, 4])
        .style(|_theme, _status| button::Style {
            background: None,
            ..Default::default()
        });

    let repeat_icon = match app.repeat {
        crate::message::RepeatMode::Off => Bootstrap::Repeat,
        crate::message::RepeatMode::All => Bootstrap::Repeat,
        crate::message::RepeatMode::One => Bootstrap::RepeatOne,
    };
    let repeat_color = if app.repeat != crate::message::RepeatMode::Off {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_MUTED
    };
    let repeat_btn = button(icon_text(repeat_icon, 11.0, repeat_color))
        .on_press(Message::CycleRepeat)
        .padding([2, 4])
        .style(|_theme, _status| button::Style {
            background: None,
            ..Default::default()
        });

    // ── Volume ──
    let vol_pct = app.player.volume() * 100.0;
    let volume = row![
        slider(0.0..=100.0, vol_pct, Message::VolumeChanged)
            .width(Length::Fixed(60.0))
            .step(1.0),
        icon_text(Bootstrap::VolumeUpFill, 10.0, theme::TEXT_MUTED),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // ── Assemble: display -> seek -> controls row ──
    let transport_row = row![
        controls,
        Space::with_width(8),
        shuffle_btn,
        repeat_btn,
        Space::with_width(Length::Fill),
        volume,
    ]
    .align_y(Alignment::Center)
    .spacing(4);

    container(
        column![display, seek, transport_row]
            .spacing(4)
            .padding([6, 8]),
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
    .into()
}

fn transport_button(bs: Bootstrap, msg: Message) -> Element<'static, Message> {
    button(icon_text(bs, 12.0, theme::TEXT_PRIMARY))
        .on_press(msg)
        .padding([4, 8])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_DISPLAY.into()),
            text_color: theme::TEXT_PRIMARY,
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
        .into()
}
