use crate::app::Nullamp;
use crate::message::Message;
use crate::theme;

use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

pub fn scan_modal_view(app: &Nullamp) -> Element<'_, Message> {
    let title = text("LIBRARY SCAN").size(14).color(theme::ACCENT_AMBER);

    // ── Phase badge ───────────────────────────────────────
    let (phase_label, phase_color) = match app.scan_progress.as_ref().map(|p| p.phase.as_str()) {
        Some("discovering") => ("DISCOVERING", theme::TEXT_MUTED),
        Some("indexing") => ("INDEXING", theme::ACCENT_AMBER),
        Some("removing") => ("REMOVING", theme::TEXT_MUTED),
        Some("complete") => ("COMPLETE", theme::TEXT_PRIMARY),
        Some("cancelled") => ("CANCELLED", theme::ERROR),
        _ => {
            if app.is_scanning {
                ("STARTING", theme::ACCENT_AMBER)
            } else {
                ("IDLE", theme::TEXT_MUTED)
            }
        }
    };

    let phase_badge = container(text(phase_label).size(8).color(phase_color))
        .padding([3, 7])
        .style(move |_theme| container::Style {
            background: Some(
                iced::Color {
                    a: 0.15,
                    ..phase_color
                }
                .into(),
            ),
            border: iced::Border {
                color: phase_color,
                width: 1.0,
                radius: 3.into(),
            },
            ..Default::default()
        });

    // ── Progress bar ──────────────────────────────────────
    let (files_processed, files_found) = app
        .scan_progress
        .as_ref()
        .map(|p| (p.files_processed, p.files_found))
        .unwrap_or((0, 0));

    let ratio = if files_found > 0 {
        (files_processed as f32 / files_found as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let bar = progress_bar(0.0..=1.0, ratio)
        .width(Length::Fill)
        .height(Length::Fixed(6.0))
        .style(|_theme| iced::widget::progress_bar::Style {
            background: theme::BG_DISPLAY.into(),
            bar: theme::ACCENT_AMBER.into(),
            border: iced::Border::default(),
        });

    let progress_label = text(format!("{files_processed} / {files_found} files"))
        .size(9)
        .color(theme::TEXT_MUTED);

    // ── Rate + ETA ────────────────────────────────────────
    let rate_eta: Element<'_, Message> = if let Some(ref p) = app.scan_progress {
        let s = if p.phase == "complete" {
            format!("Done in {}", fmt_elapsed(p.elapsed_ms))
        } else if p.rate > 0.0 {
            let eta = fmt_eta(p.eta_secs);
            if eta.is_empty() {
                format!("{:.0} files/sec", p.rate)
            } else {
                format!("{:.0} files/sec  ·  {}", p.rate, eta)
            }
        } else {
            fmt_elapsed(p.elapsed_ms)
        };
        text(s).size(9).color(theme::TEXT_MUTED).into()
    } else {
        Space::with_height(0).into()
    };

    // ── Stats grid ────────────────────────────────────────
    let stats: Element<'_, Message> = if let Some(ref p) = app.scan_progress {
        row![
            stat_cell("FOUND", p.files_found),
            stat_cell("NEW", p.files_processed),
            stat_cell("SKIPPED", p.files_skipped),
            stat_cell("FAILED", p.files_failed),
            stat_cell("REMOVED", p.files_removed),
        ]
        .spacing(0)
        .into()
    } else {
        Space::with_height(0).into()
    };

    // ── Current file ──────────────────────────────────────
    let current: Element<'_, Message> = app
        .scan_progress
        .as_ref()
        .filter(|p| !p.current_file.is_empty())
        .map(|p| {
            text(file_name(&p.current_file))
                .size(9)
                .color(theme::TEXT_MUTED)
                .into()
        })
        .unwrap_or_else(|| Space::with_height(0).into());

    // ── Recent files list ─────────────────────────────────
    let recent_section: Element<'_, Message> = if !app.scan_recent_files.is_empty() {
        let items: Vec<Element<'_, Message>> = app
            .scan_recent_files
            .iter()
            .rev()
            .map(|path| {
                text(file_name(path))
                    .size(9)
                    .color(theme::TEXT_MUTED)
                    .into()
            })
            .collect();

        column![
            section_label("RECENT FILES"),
            Space::with_height(4),
            container(scrollable(column(items).spacing(3)).height(Length::Fixed(160.0)))
                .style(|_theme| container::Style {
                    background: Some(theme::BG_DISPLAY.into()),
                    border: iced::Border {
                        color: theme::BORDER_FRAME,
                        width: 1.0,
                        radius: 3.into(),
                    },
                    ..Default::default()
                })
                .padding([6, 8])
                .width(Length::Fill),
        ]
        .spacing(0)
        .into()
    } else {
        Space::with_height(0).into()
    };

    // ── Action button ─────────────────────────────────────
    let action_btn: Element<'_, Message> = if app.is_scanning {
        button(text("CANCEL SCAN").size(10).color(theme::ERROR))
            .on_press(Message::ScanCancel)
            .padding([4, 12])
            .style(|_theme, _| button::Style {
                background: Some(theme::BG_SURFACE.into()),
                border: iced::Border {
                    color: theme::ERROR,
                    width: 1.0,
                    radius: 2.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        button(text("CLOSE").size(10).color(theme::TEXT_MUTED))
            .on_press(Message::CloseScanModal)
            .padding([4, 12])
            .style(|_theme, _| button::Style {
                background: Some(theme::BG_SURFACE.into()),
                border: iced::Border {
                    color: theme::BORDER_FRAME,
                    width: 1.0,
                    radius: 2.into(),
                },
                ..Default::default()
            })
            .into()
    };

    // ── Layout ────────────────────────────────────────────
    let content = column![
        row![title, Space::with_width(Length::Fill), phase_badge].align_y(Alignment::Center),
        Space::with_height(14),
        section_label("PROGRESS"),
        Space::with_height(6),
        bar,
        Space::with_height(4),
        row![progress_label, Space::with_width(Length::Fill), rate_eta].align_y(Alignment::Center),
        Space::with_height(12),
        section_label("STATISTICS"),
        Space::with_height(6),
        stats,
        Space::with_height(10),
        current,
        Space::with_height(8),
        recent_section,
        Space::with_height(14),
        row![Space::with_width(Length::Fill), action_btn],
        Space::with_height(4),
    ]
    .spacing(0)
    .padding(16);

    scrollable(content).height(Length::Shrink).into()
}

fn section_label(s: &str) -> iced::widget::Text<'static> {
    text(s.to_string()).size(9).color(theme::TEXT_MUTED)
}

fn stat_cell(label: &str, value: usize) -> Element<'static, Message> {
    container(
        column![
            text(value.to_string()).size(13).color(theme::TEXT_PRIMARY),
            text(label.to_string()).size(8).color(theme::TEXT_MUTED),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding([4, 0])
    .into()
}

fn fmt_eta(secs: f64) -> String {
    if secs <= 0.0 {
        return String::new();
    }
    if secs < 60.0 {
        format!("{:.0}s left", secs)
    } else {
        let m = (secs / 60.0) as u32;
        let s = (secs % 60.0) as u32;
        format!("{m}m {s:02}s left")
    }
}

fn fmt_elapsed(ms: u64) -> String {
    let s = ms / 1000;
    if s < 60 {
        format!("{s}s")
    } else {
        let m = s / 60;
        let rem = s % 60;
        format!("{m}m {rem:02}s")
    }
}

fn file_name(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}
