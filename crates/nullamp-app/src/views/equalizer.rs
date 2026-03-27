use crate::app::Nullamp;
use crate::message::Message;
use crate::theme;
use crate::views::eq_canvas::EqCurve;

use iced::widget::canvas::Canvas;
use iced::widget::{column, container, pick_list, row, slider, text, toggler, Space};
use iced::{Alignment, Element, Length};

use nullamp_audio::eq::EQ_PRESETS;

/// Equalizer panel — SVG curve canvas + preamp slider + presets.
pub fn equalizer_view(app: &Nullamp) -> Element<'_, Message> {
    let eq = app.player.eq_params();
    let enabled = eq.is_enabled();
    let preamp = eq.get_preamp();

    // Header: ON/OFF toggle + preset picker
    let toggle = toggler(enabled).on_toggle(Message::EqToggled).size(14.0);

    let preset_names: Vec<String> = EQ_PRESETS
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    let preset_picker = pick_list(preset_names, None::<String>, Message::EqPresetSelected)
        .placeholder("Preset…")
        .text_size(10.0)
        .padding([3, 6]);

    let on_label = text(if enabled { "ON" } else { "OFF" })
        .size(9)
        .color(if enabled {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        });

    let header = container(
        row![
            on_label,
            toggle,
            Space::with_width(Length::Fill),
            preset_picker
        ]
        .spacing(6)
        .align_y(Alignment::Center)
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

    // Preamp strip above canvas
    let preamp_color = if preamp.abs() < 0.5 {
        theme::TEXT_MUTED
    } else if preamp > 0.0 {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_PRIMARY
    };
    let preamp_strip = container(
        row![
            text("PRE")
                .size(8)
                .color(theme::TEXT_MUTED)
                .width(Length::Fixed(28.0)),
            slider(-12.0..=12.0, preamp, Message::EqPreampChanged)
                .step(0.5)
                .width(Length::Fill),
            text(format!("{:+.0}", preamp))
                .size(8)
                .color(preamp_color)
                .width(Length::Fixed(28.0)),
        ]
        .align_y(Alignment::Center)
        .spacing(6)
        .padding([4, 8]),
    )
    .style(|_theme| container::Style {
        background: Some(theme::BG_DISPLAY.into()),
        ..Default::default()
    })
    .width(Length::Fill);

    // EQ curve canvas
    let curve = EqCurve::new(eq.clone(), enabled);
    let canvas_widget = Canvas::new(curve)
        .width(Length::Fill)
        .height(Length::Fixed(160.0));

    // dB labels on left + canvas side by side
    let db_labels = column![
        text("+12").size(7).color(theme::TEXT_MUTED),
        Space::with_height(Length::Fill),
        text("+6").size(7).color(theme::TEXT_MUTED),
        Space::with_height(Length::Fill),
        text("0").size(7).color(theme::TEXT_MUTED),
        Space::with_height(Length::Fill),
        text("-6").size(7).color(theme::TEXT_MUTED),
        Space::with_height(Length::Fill),
        text("-12").size(7).color(theme::TEXT_MUTED),
    ]
    .align_x(Alignment::End)
    .width(Length::Fixed(22.0))
    .height(Length::Fixed(160.0));

    let canvas_row = row![db_labels, canvas_widget]
        .spacing(4)
        .align_y(Alignment::Center);

    // Frequency labels below the canvas
    let freq_labels: Vec<Element<Message>> = nullamp_audio::eq::EQ_FREQUENCIES
        .iter()
        .map(|&f| {
            let label = if f >= 1000.0 {
                format!("{}K", f as u32 / 1000)
            } else {
                format!("{}", f as u32)
            };
            text(label)
                .size(7)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .into()
        })
        .collect();

    let freq_row = container(row![Space::with_width(22), row(freq_labels)].spacing(4)).padding(
        iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 4.0,
            left: 4.0,
        },
    );

    let canvas_area = container(column![canvas_row, freq_row])
        .style(|_theme| container::Style {
            background: Some(theme::BG_DISPLAY.into()),
            ..Default::default()
        })
        .padding([8, 4])
        .width(Length::Fill)
        .height(Length::Fill);

    column![header, preamp_strip, canvas_area]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
