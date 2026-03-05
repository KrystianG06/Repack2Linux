use crate::ui::common::{font_mono, panel_card};
use crate::ui::theme::{
    brand_button_style, danger_button_style, ACCENT_BLUE, ACCENT_CYAN, ACCENT_GRAY, ACCENT_RED,
    TEXT_DIM,
};
use crate::{dependencies::DependencyManager, installer::Installer, Message, RepackApp};
use iced::widget::{button, column, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length};

pub fn view_tools(app: &RepackApp) -> Element<'_, Message> {
    let missing_libs = DependencyManager::check_system_libs();

    let gamemode_available = Installer::check_tool("gamemoderun");
    let mangohud_available = Installer::check_tool("mangohud");
    let gamemode_ok = !app.opt_gamemode || gamemode_available;
    let mangohud_ok = !app.opt_mangohud || mangohud_available;

    let log_entries = app
        .logs
        .iter()
        .rev()
        .take(80)
        .map(|line| {
            let color = if line.contains("[ERROR]") || line.contains("[CRITICAL]") {
                ACCENT_RED
            } else if line.contains("[OK]") || line.contains("[SUCCESS]") {
                ACCENT_CYAN
            } else {
                ACCENT_GRAY
            };
            text(line).size(12).font(font_mono()).color(color).into()
        })
        .collect::<Vec<Element<'_, Message>>>();

    let dependency_body: Element<'_, Message> = if missing_libs.is_empty() {
        text(app.tr("tools_drivers_ok"))
            .size(12)
            .color(TEXT_DIM)
            .into()
    } else {
        column(
            missing_libs
                .iter()
                .map(|lib| {
                    text(format!("- {}", lib))
                        .size(12)
                        .color(ACCENT_CYAN)
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(4)
        .into()
    };

    column![
        row![
            panel_card(
                app.tr("tools_system_health"),
                column![
                    status_row(app, app.tr("tools_wine_proton"), true),
                    status_row(
                        app,
                        app.tr("tools_winetricks"),
                        Installer::check_tool("winetricks")
                    ),
                    status_row(
                        app,
                        app.tr("tools_ntlm"),
                        Installer::check_tool("ntlm_auth")
                    ),
                    status_row(app, app.tr("tools_drivers32"), missing_libs.is_empty()),
                    status_row(
                        app,
                        app.tr("tools_vulkan_runtime"),
                        DependencyManager::check_vulkan_functional()
                    ),
                    status_row(app, app.tr("tools_gamemode"), gamemode_ok),
                    status_row(app, app.tr("tools_mangohud"), mangohud_ok),
                ]
                .spacing(8),
            )
            .width(Length::FillPortion(1)),
            panel_card(
                app.tr("tools_utility_actions"),
                column![
                    button(text(app.tr("tools_fix_system")))
                        .on_press(Message::InstallMissingPressed)
                        .style(|t, s| brand_button_style(t, s, true))
                        .width(Length::Fill),
                    button(text(app.tr("open_debug")))
                        .on_press(Message::OpenDebugShellPressed)
                        .style(|t, s| brand_button_style(t, s, false))
                        .width(Length::Fill),
                    row![
                        button(text(app.tr("unmount_iso")))
                            .on_press(Message::UnmountISO)
                            .style(|t, s| danger_button_style(t, s))
                            .width(Length::FillPortion(1)),
                        button(text(app.tr("tools_kill_wine")))
                            .on_press(Message::KillWinePressed)
                            .style(|t, s| danger_button_style(t, s))
                            .width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                    row![
                        button(text(app.tr("tools_copy_logs")))
                            .on_press(Message::CopyLogsToClipboard)
                            .style(|t, s| brand_button_style(t, s, false))
                            .width(Length::FillPortion(1)),
                        button(text(app.tr("tools_save_logs")))
                            .on_press(Message::SaveLogsPressed)
                            .style(|t, s| brand_button_style(t, s, false))
                            .width(Length::FillPortion(1)),
                        button(text(app.tr("tools_analyze")))
                            .on_press(Message::AnalyzeLogsPressed)
                            .style(|t, s| brand_button_style(t, s, true))
                            .width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                ]
                .spacing(12),
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(20),
        row![
            panel_card(
                app.tr("tools_live_logs"),
                scrollable(column(log_entries).spacing(6)).height(Length::FillPortion(1)),
            )
            .width(Length::FillPortion(2)),
            panel_card(
                app.tr("tools_dependencies"),
                column![
                    text(app.tr("tools_missing_packages")).size(12).color(
                        if missing_libs.is_empty() {
                            ACCENT_CYAN
                        } else {
                            Color::from_rgb(1.0, 0.5, 0.3)
                        }
                    ),
                    dependency_body,
                ]
                .spacing(10),
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(20),
    ]
    .spacing(24)
    .padding(32)
    .into()
}

fn status_row<'a>(app: &'a RepackApp, label: &'a str, ok: bool) -> Element<'a, Message> {
    let accent = if ok { ACCENT_BLUE } else { ACCENT_RED };
    row![
        text(if ok {
            app.tr("tools_ok")
        } else {
            app.tr("tools_fail")
        })
        .size(11)
        .color(accent),
        Space::with_width(8),
        text(label).size(13).color(Color::WHITE),
        Space::with_width(Length::Fill),
        text(if ok {
            app.tr("tools_ready")
        } else {
            app.tr("tools_missing")
        })
        .size(10)
        .color(TEXT_DIM),
    ]
    .align_y(Alignment::Center)
    .into()
}
