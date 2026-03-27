use crate::app::Nullamp;
use crate::icons::icon_text;
use crate::message::{Message, WhisperModelInfo};
use crate::theme;

use iced::widget::canvas::{self, Frame, Program};
use iced::widget::{button, column, container, progress_bar, row, text, text_input, Canvas, Space};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use iced_fonts::bootstrap::Bootstrap;

/// Voice panel — mic button centered, pills + command input pinned to bottom.
pub fn voice_view(app: &Nullamp) -> Element<'_, Message> {
    // Mic area fills all available height and is centered within it
    let upper = container(mic_button_area(app))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    // Pills + input always sit at the bottom
    let bottom = column![
        status_pills_row(app),
        Space::with_height(8),
        command_input(app),
        Space::with_height(10),
    ]
    .padding([0, 12])
    .spacing(0);

    let layout = column![upper, bottom]
        .width(Length::Fill)
        .height(Length::Fill);

    container(layout)
        .style(|_theme| container::Style {
            background: Some(theme::BG_DISPLAY.into()),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Mic button area ────────────────────────────────────────────────────────────

fn mic_button_area(app: &Nullamp) -> Element<'_, Message> {
    // Derive state
    // Amber "busy" state: transcribing audio, processing intent, or waiting for Gemini
    let is_transcribing = !app.is_recording
        && !app.voice_status.is_empty()
        && (app.voice_status.contains("Transcribing")
            || app.voice_status.contains("Processing")
            || app.voice_status.contains("Listening")
            || app.voice_status.contains("Searching"));

    let (mic_bg, mic_border, mic_icon_color, mic_msg): (
        iced::Color,
        iced::Color,
        iced::Color,
        Option<Message>,
    ) = if app.voice_model_missing {
        (
            theme::BG_SURFACE,
            theme::BORDER_FRAME,
            theme::TEXT_MUTED,
            None,
        )
    } else if app.is_recording {
        (
            iced::Color {
                r: 0.7,
                g: 0.1,
                b: 0.1,
                a: 0.3,
            },
            theme::ERROR,
            theme::ERROR,
            Some(Message::VoiceStopRecording),
        )
    } else if is_transcribing {
        (
            iced::Color {
                a: 0.2,
                ..theme::ACCENT_AMBER
            },
            theme::ACCENT_AMBER,
            theme::ACCENT_AMBER,
            None,
        )
    } else {
        (
            theme::BG_SURFACE,
            theme::BORDER_FRAME,
            theme::TEXT_MUTED,
            Some(Message::VoiceStartRecording),
        )
    };

    // The icon (spinner substitute uses rotating mic based on waveform_phase)
    let mic_icon = if is_transcribing {
        icon_text(Bootstrap::Cpu, 28.0, mic_icon_color)
    } else if app.is_recording {
        icon_text(Bootstrap::MicFill, 28.0, mic_icon_color)
    } else {
        icon_text(Bootstrap::MicFill, 28.0, mic_icon_color)
    };

    let mut mic_btn = button(
        container(mic_icon)
            .width(Length::Fixed(72.0))
            .height(Length::Fixed(72.0))
            .center_x(Length::Fixed(72.0))
            .center_y(Length::Fixed(72.0)),
    )
    .padding(0)
    .style(move |_theme, status| button::Style {
        background: Some(match status {
            button::Status::Hovered => iced::Color {
                a: mic_bg.a + 0.1,
                ..mic_bg
            }
            .into(),
            _ => mic_bg.into(),
        }),
        border: iced::Border {
            color: mic_border,
            width: 2.0,
            radius: 36.0.into(),
        },
        ..Default::default()
    });

    if let Some(msg) = mic_msg {
        mic_btn = mic_btn.on_press(msg);
    }

    // Waveform during recording, otherwise status / hint text
    let below: Element<'_, Message> = if app.is_recording {
        Canvas::new(Waveform {
            phase: app.waveform_phase,
        })
        .width(Length::Fill)
        .height(Length::Fixed(36.0))
        .into()
    } else if !app.voice_status.is_empty() {
        text(&app.voice_status)
            .size(9)
            .color(theme::TEXT_MUTED)
            .into()
    } else if app.voice_model_missing {
        text(app.t("voice_model_desc"))
            .size(9)
            .color(theme::TEXT_MUTED)
            .into()
    } else {
        column![
            text(app.t("voice_click_to_speak"))
                .size(9)
                .color(theme::TEXT_MUTED),
            text(app.t("voice_example_1"))
                .size(8)
                .color(theme::BORDER_FRAME),
            text(app.t("voice_example_2"))
                .size(8)
                .color(theme::BORDER_FRAME),
            text(app.t("voice_example_3"))
                .size(8)
                .color(theme::BORDER_FRAME),
        ]
        .spacing(1)
        .align_x(Alignment::Center)
        .into()
    };

    column![
        container(mic_btn).center_x(Length::Fill),
        Space::with_height(8),
        container(below).center_x(Length::Fill),
    ]
    .align_x(Alignment::Center)
    .into()
}

// ── Model grid (pub — reused in Settings) ─────────────────────────────────────

pub fn model_grid_section(app: &Nullamp) -> Element<'_, Message> {
    let header = text(app.t("whisper_models").to_uppercase())
        .size(8)
        .color(theme::TEXT_MUTED);

    // Build 2-column grid
    let mut grid_rows: Vec<Element<'_, Message>> = Vec::new();
    let infos: Vec<&WhisperModelInfo> = app.model_infos.iter().collect();

    for chunk in infos.chunks(2) {
        let left = model_card(chunk[0], &app.model_download_progress, app);
        let right = if let Some(info) = chunk.get(1) {
            model_card(info, &app.model_download_progress, app)
        } else {
            Space::with_width(Length::Fill).into()
        };
        grid_rows.push(
            row![left, Space::with_width(4), right]
                .width(Length::Fill)
                .into(),
        );
    }

    column(
        std::iter::once(header.into())
            .chain(std::iter::once(Space::with_height(4).into()))
            .chain(grid_rows)
            .collect::<Vec<_>>(),
    )
    .spacing(4)
    .into()
}

pub fn model_card<'a>(
    info: &'a WhisperModelInfo,
    progress: &'a std::collections::HashMap<String, (f32, u64, u64)>,
    app: &'a Nullamp,
) -> Element<'a, Message> {
    let label_color = if info.is_active {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_PRIMARY
    };

    let size_label = if info.size_mb >= 1000 {
        format!("{:.1}GB", info.size_mb as f32 / 1024.0)
    } else {
        format!("{}MB", info.size_mb)
    };

    let name_row = row![
        text(info.id.to_uppercase()).size(10).color(label_color),
        Space::with_width(Length::Fill),
        text(size_label).size(8).color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    let action: Element<'_, Message> = if let Some(&(pct, dl, total)) = progress.get(info.id) {
        // Downloading
        let mb_done = dl / (1024 * 1024);
        let mb_total = total / (1024 * 1024);
        column![
            progress_bar(0.0..=1.0, pct).height(Length::Fixed(4.0)),
            text(format!("{}MB / {}MB", mb_done, mb_total))
                .size(7)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(2)
        .into()
    } else if info.is_active {
        container(
            text(app.t("voice_model_active").to_uppercase())
                .size(7)
                .color(theme::ACCENT_AMBER),
        )
        .padding([2, 6])
        .style(|_theme| container::Style {
            border: iced::Border {
                color: theme::ACCENT_AMBER,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
        .into()
    } else if info.downloaded {
        let id_str = info.id.to_string();
        button(
            text(app.t("voice_model_use").to_uppercase())
                .size(7)
                .color(theme::TEXT_PRIMARY),
        )
        .on_press(Message::WhisperModelSwitch(id_str))
        .padding([2, 6])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_PRIMARY.into()),
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        let id_str = info.id.to_string();
        button(
            text(format!(
                "\u{2193} {}",
                app.t("voice_download").to_uppercase()
            ))
            .size(7)
            .color(theme::ACCENT_AMBER),
        )
        .on_press(Message::WhisperModelDownload(id_str))
        .padding([2, 6])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_PRIMARY.into()),
            border: iced::Border {
                color: theme::ACCENT_AMBER,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
        .into()
    };

    container(column![name_row, action].spacing(4).padding([6, 6]))
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(theme::BG_SURFACE.into()),
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 3.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── Status pills ──────────────────────────────────────────────────────────────

fn status_pills_row(app: &Nullamp) -> Element<'_, Message> {
    let lang_label = app.settings.language.to_uppercase();
    let model_label = app
        .active_model_id
        .as_deref()
        .map(|id| id.to_uppercase())
        .unwrap_or_else(|| "NO MODEL".to_string());
    let (ai_label, ai_color) = if app.settings.gemini_api_key.is_empty() {
        ("TEXT ONLY".to_string(), theme::TEXT_MUTED)
    } else {
        ("AI ON".to_string(), theme::ACCENT_AMBER)
    };

    row![
        pill_static(lang_label, theme::TEXT_MUTED),
        Space::with_width(4),
        pill_static(model_label, theme::ACCENT_AMBER),
        Space::with_width(4),
        pill_static(ai_label, ai_color),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn pill_static(label: String, color: iced::Color) -> Element<'static, Message> {
    container(text(label).size(7).color(color))
        .padding([2, 6])
        .style(move |_theme| container::Style {
            border: iced::Border {
                color,
                width: 1.0,
                radius: 8.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── Text command input ─────────────────────────────────────────────────────────

fn command_input(app: &Nullamp) -> Element<'_, Message> {
    let input = text_input(app.t("voice_type_command"), &app.voice_transcript)
        .on_input(Message::VoiceTranscription)
        .on_submit(Message::VoiceTextCommand(app.voice_transcript.clone()))
        .size(10)
        .padding([5, 8]);

    let submit_btn = button(icon_text(
        Bootstrap::ArrowReturnLeft,
        10.0,
        theme::TEXT_MUTED,
    ))
    .on_press(Message::VoiceTextCommand(app.voice_transcript.clone()))
    .padding([4, 6])
    .style(|_theme, _status| button::Style {
        background: Some(theme::BG_SURFACE.into()),
        border: iced::Border {
            color: theme::BORDER_FRAME,
            width: 1.0,
            radius: 2.into(),
        },
        ..Default::default()
    });

    row![input, submit_btn]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

// ── Waveform animation canvas ─────────────────────────────────────────────────

struct Waveform {
    phase: f32,
}

impl Program<Message> for Waveform {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let w = bounds.width;
        let h = bounds.height;
        let mut frame = Frame::new(renderer, Size::new(w, h));

        let num_bars: usize = 12;
        let bar_w = (w / num_bars as f32) * 0.55;
        let gap = w / num_bars as f32;

        for i in 0..num_bars {
            let x = gap * (i as f32 + 0.5);
            // Animated bar height: sin wave with offset per bar
            let amplitude = (self.phase + i as f32 * 0.7).sin().abs();
            let bar_h = 4.0 + amplitude * (h - 8.0);
            let bar_y = (h - bar_h) / 2.0;

            let intensity = 0.5 + amplitude * 0.5;
            let color = Color {
                r: theme::TEXT_PRIMARY.r * intensity,
                g: theme::TEXT_PRIMARY.g * intensity,
                b: theme::TEXT_PRIMARY.b * intensity,
                a: 0.7 + amplitude * 0.3,
            };

            frame.fill_rectangle(
                Point {
                    x: x - bar_w / 2.0,
                    y: bar_y,
                },
                Size::new(bar_w, bar_h),
                color,
            );
        }

        vec![frame.into_geometry()]
    }
}
