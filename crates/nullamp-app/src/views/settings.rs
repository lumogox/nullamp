use crate::app::Nullamp;
use crate::message::Message;
use crate::theme;
use crate::views::voice::model_grid_section;

use iced::widget::{button, column, pick_list, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

const GEMINI_MODELS: &[&str] = &[
    "gemini-3.1-flash-lite-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
];

/// Settings overlay — music folder, scan, Gemini, language.
pub fn settings_view(app: &Nullamp) -> Element<'_, Message> {
    let title = text(app.t("settings")).size(14).color(theme::ACCENT_AMBER);

    // ── Music Folder ──────────────────────────────────────
    let section_label = |s: &str| text(s.to_uppercase()).size(9).color(theme::TEXT_MUTED);

    let folder_path = text(
        app.settings
            .music_folder
            .as_deref()
            .unwrap_or(app.t("not_set")),
    )
    .size(10)
    .color(theme::TEXT_PRIMARY);

    let pick_btn = small_btn(app.t("browse"), Message::PickMusicFolder);

    let folder_row = row![folder_path, Space::with_width(Length::Fill), pick_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    // Scan button
    let scan_btn = if app.is_scanning {
        button(text(app.t("scan_cancel")).size(10).color(theme::ERROR))
            .on_press(Message::ScanCancel)
            .padding([4, 12])
            .style(|_theme, _status| button::Style {
                background: Some(theme::BG_SURFACE.into()),
                border: iced::Border {
                    color: theme::ERROR,
                    width: 1.0,
                    radius: 2.into(),
                },
                ..Default::default()
            })
    } else {
        button(
            text(app.t("scan_library"))
                .size(10)
                .color(theme::TEXT_PRIMARY),
        )
        .on_press(Message::ScanStart)
        .padding([4, 12])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_SURFACE.into()),
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
    };

    let scan_info: Element<'_, Message> = if let Some(ref progress) = app.scan_progress {
        text(format!(
            "{} — {}/{} files",
            progress.phase, progress.files_processed, progress.files_found
        ))
        .size(9)
        .color(theme::TEXT_MUTED)
        .into()
    } else {
        Space::with_height(0).into()
    };

    // ── Gemini API ────────────────────────────────────────
    let key_placeholder = if app.settings.gemini_api_key.is_empty() {
        app.t("not_set")
    } else {
        app.t("configured")
    };

    let key_input = text_input(key_placeholder, &app.settings.gemini_api_key)
        .on_input(Message::GeminiApiKeyChanged)
        .secure(!app.show_gemini_key)
        .size(10)
        .padding([4, 8]);

    let show_label = if app.show_gemini_key {
        app.t("hide")
    } else {
        app.t("show")
    };
    let show_btn = small_btn(show_label, Message::ShowGeminiKey(!app.show_gemini_key));

    let api_status = if app.settings.gemini_api_key.is_empty() {
        text(app.t("not_set")).size(8).color(theme::TEXT_MUTED)
    } else {
        text(app.t("configured")).size(8).color(theme::TEXT_PRIMARY)
    };

    let key_row = row![key_input, show_btn]
        .spacing(6)
        .align_y(Alignment::Center);

    // Model picker
    let model_names: Vec<String> = GEMINI_MODELS.iter().map(|s| s.to_string()).collect();
    let current_model = app.settings.gemini_model.clone();
    let model_picker = pick_list(
        model_names,
        Some(current_model),
        Message::GeminiModelChanged,
    )
    .text_size(10)
    .padding([3, 6]);

    // ── Language ──────────────────────────────────────────
    let lang = app.settings.language.as_str();

    let lang_en = lang_toggle_btn("EN", lang == "en", Message::LanguageToggled);
    let lang_es = lang_toggle_btn("ES", lang == "es", Message::LanguageToggled);

    // ── Close ─────────────────────────────────────────────
    let close_btn = button(text(app.t("close")).size(10).color(theme::TEXT_MUTED))
        .on_press(Message::CloseSettings)
        .padding([4, 12])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_SURFACE.into()),
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        });

    let content = column![
        title,
        Space::with_height(10),
        // Music folder
        section_label(app.t("music_folder")),
        Space::with_height(4),
        folder_row,
        Space::with_height(4),
        row![scan_btn].spacing(8),
        scan_info,
        Space::with_height(10),
        // Gemini
        section_label(app.t("api_key")),
        Space::with_height(4),
        key_row,
        row![api_status].padding([2, 0]),
        Space::with_height(4),
        section_label(app.t("gemini_model")),
        Space::with_height(2),
        model_picker,
        Space::with_height(10),
        // Language
        section_label(app.t("language")),
        Space::with_height(4),
        row![lang_en, lang_es].spacing(4),
        Space::with_height(10),
        // Whisper models
        model_grid_section(app),
        Space::with_height(8),
        row![Space::with_width(Length::Fill), close_btn],
        Space::with_height(4),
    ]
    .spacing(4)
    .padding(16);

    scrollable(content).height(Length::Shrink).into()
}

fn small_btn(label: &str, msg: Message) -> iced::widget::Button<'_, Message> {
    button(text(label).size(10).color(theme::ACCENT_AMBER))
        .on_press(msg)
        .padding([4, 10])
        .style(|_theme, _status| button::Style {
            background: Some(theme::BG_SURFACE.into()),
            border: iced::Border {
                color: theme::BORDER_FRAME,
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
}

fn lang_toggle_btn(label: &str, active: bool, msg: Message) -> iced::widget::Button<'_, Message> {
    let color = if active {
        theme::ACCENT_AMBER
    } else {
        theme::TEXT_MUTED
    };
    button(text(label).size(9).color(color))
        .on_press(msg)
        .padding([3, 8])
        .style(move |_theme, _status| button::Style {
            background: if active {
                Some(theme::BG_DISPLAY.into())
            } else {
                Some(theme::BG_SURFACE.into())
            },
            border: iced::Border {
                color: if active {
                    theme::ACCENT_AMBER
                } else {
                    theme::BORDER_FRAME
                },
                width: 1.0,
                radius: 2.into(),
            },
            ..Default::default()
        })
}
