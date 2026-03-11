use crate::ui::common::{font_bold, info_block, panel_card};
use crate::ui::theme::{primary_button_style, ACCENT_BLUE, ACCENT_RED, TEXT_DIM};
use crate::app::{Language, Message, RepackApp};
use iced::widget::{button, checkbox, column, pick_list, row, text, Space};
use iced::{Color, Element, Length};

const WIN_VERSIONS: [&str; 3] = ["win10", "win7", "winxp"];

pub fn view_settings(app: &RepackApp) -> Element<'_, Message> {
    let queue_label = if app.community_queue_pending {
        app.tr("community_queue_pending")
    } else {
        app.tr("community_queue_empty")
    };
    let queue_color = if app.community_queue_pending {
        ACCENT_RED
    } else {
        ACCENT_BLUE
    };
    let last_retry = app
        .community_last_retry_at
        .as_deref()
        .unwrap_or(app.tr("community_never"));
    let last_error = app
        .community_last_error
        .as_deref()
        .unwrap_or(app.tr("community_none"));
    let repo_root = app
        .community_repo_root
        .as_deref()
        .unwrap_or(app.tr("community_not_found"));

    let content = column![
        row![
            panel_card(
                app.tr("engine_modules"),
                column![
                    checkbox(app.tr("dxvk_trans"), app.opt_dxvk).on_toggle(Message::ToggleDxvk),
                    checkbox(app.tr("vc_run"), app.opt_vcrun2022).on_toggle(Message::ToggleVcrun),
                    checkbox(app.tr("xaudio_fix"), app.opt_xaudio).on_toggle(Message::ToggleXAudio),
                    checkbox(app.tr("win32_mode"), app.opt_win32).on_toggle(Message::ToggleWin32),
                    checkbox(app.tr("ultra_compat"), app.opt_ultra_compat)
                        .on_toggle(Message::ToggleUltra),
                    checkbox(app.tr("no_dxvk"), app.opt_no_dxvk).on_toggle(Message::ToggleNoDxvk),
                    checkbox(app.tr("mangohud_over"), app.opt_mangohud)
                        .on_toggle(Message::ToggleMango),
                    checkbox(app.tr("feral_gamemode"), app.opt_gamemode)
                        .on_toggle(Message::ToggleGamemode),
                ]
                .spacing(10),
            )
            .width(Length::FillPortion(1)),
            panel_card(
                app.tr("legacy_compat"),
                column![
                    checkbox(app.tr("legacy_mode"), app.opt_legacy_mode)
                        .on_toggle(Message::ToggleLegacyMode),
                    pick_list(WIN_VERSIONS, Some(app.opt_windows_version.as_str()), |v| {
                        Message::ToggleWindowsVersion(v.to_string())
                    })
                    .width(Length::Fill),
                    Space::with_height(10),
                    checkbox(app.tr("directx9"), app.opt_d3dx9).on_toggle(Message::ToggleD3dx9),
                    checkbox(app.tr("vcrun2005"), app.opt_vcrun2005)
                        .on_toggle(Message::ToggleVcrun2005),
                    checkbox(app.tr("vcrun2008"), app.opt_vcrun2008)
                        .on_toggle(Message::ToggleVcrun2008),
                    checkbox(app.tr("xact_audio"), app.opt_xact).on_toggle(Message::ToggleXact),
                    checkbox(app.tr("physx"), app.opt_physx).on_toggle(Message::TogglePhysx),
                    checkbox(app.tr("enable_csmt"), app.opt_csmt).on_toggle(Message::ToggleCsmt),
                ]
                .spacing(8),
            )
            .width(Length::FillPortion(1)),
            panel_card(
                app.tr("system_language"),
                column![
                    text(app.tr("lang_select")).size(11).color(TEXT_DIM),
                    pick_list(&Language::ALL[..], Some(app.lang), Message::LanguageChanged)
                        .width(Length::Fill),
                    checkbox(
                        app.tr("welcome_screen_setting"),
                        app.cfg.welcome_screen_enabled
                    )
                    .on_toggle(Message::ToggleWelcomeScreen),
                    checkbox(
                        app.tr("welcome_animation"),
                        app.cfg.welcome_animation_enabled
                    )
                    .on_toggle(Message::ToggleWelcomeAnimation),
                    Space::with_height(14),
                    text(app.tr("settings_shortcut_title"))
                        .size(12)
                        .font(font_bold()),
                    text(app.tr("settings_shortcut_desc"))
                        .size(11)
                        .color(TEXT_DIM),
                    button(text(app.tr("settings_add_shortcut_btn")).size(11))
                        .on_press(Message::InstallAppShortcutPressed)
                        .style(|t, s| primary_button_style(t, s))
                        .width(Length::Fill),
                    Space::with_height(20),
                    text(app.tr("cloud_knowledge")).size(12).font(font_bold()),
                    button(text(app.tr("sync_now")).size(11))
                        .on_press(Message::SyncCloudDatabase)
                        .style(|t, s| primary_button_style(t, s))
                        .width(Length::Fill),
                    Space::with_height(10),
                    text(app.tr("gpu_vendor")).size(11).color(TEXT_DIM),
                    text(&app.gpu_vendor).size(14).color(Color::WHITE),
                ]
                .spacing(10),
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(18),
        row![
            panel_card(
                app.tr("export_options"),
                column![
                    checkbox(app.tr("export_standalone"), app.opt_export_standalone)
                        .on_toggle(Message::ToggleExportStandalone),
                    checkbox(app.tr("export_installer"), app.opt_export_installer)
                        .on_toggle(Message::ToggleExportInstaller),
                    checkbox(app.tr("include_deps"), app.opt_include_deps)
                        .on_toggle(Message::ToggleIncludeDeps),
                    checkbox(app.tr("auto_launch_export"), app.opt_auto_launch)
                        .on_toggle(Message::ToggleAutoLaunch),
                ]
                .spacing(10),
            )
            .width(Length::FillPortion(1)),
            panel_card(
                app.tr("community_sync"),
                column![
                    text(queue_label)
                        .size(12)
                        .font(font_bold())
                        .color(queue_color),
                    row![
                        info_block(
                            app.tr("queue_attempts"),
                            app.community_queue_attempts.to_string()
                        ),
                        info_block(
                            app.tr("remote_token"),
                            if app.community_remote_enabled {
                                app.tr("enabled")
                            } else {
                                app.tr("disabled")
                            }
                        ),
                    ]
                    .spacing(10),
                    info_block(app.tr("last_retry"), last_retry),
                    info_block(app.tr("last_error"), last_error),
                    info_block(app.tr("repo_root"), repo_root),
                    button(text(app.tr("retry_queue_now")).size(11))
                        .on_press(Message::ProcessCommunityQueue)
                        .style(|t, s| primary_button_style(t, s))
                        .width(Length::Fill),
                ]
                .spacing(10),
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(18),
    ]
    .spacing(24);

    crate::ui::theme::glass_container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
