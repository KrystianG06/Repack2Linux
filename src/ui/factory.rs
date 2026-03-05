use crate::config::UiMode;
use crate::export::{ExportAudit, ExportScope};
use crate::ui::common::{font_bold, font_mono, info_block, panel_card};
use crate::ui::theme::{
    brand_button_style, glass_container, ACCENT_BLUE, ACCENT_CYAN, ACCENT_GRAY, ACCENT_RED,
    TEXT_DIM,
};
use crate::{ExportStatus, Message, RepackApp};
use iced::widget::{
    button, column, container, pick_list, progress_bar, row, scrollable, text, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Vector};

pub fn view_factory(app: &RepackApp) -> Element<'_, Message> {
    match app.ui_mode {
        UiMode::Simple => view_simple(app),
        UiMode::Advanced => view_advanced(app),
    }
}

fn status_tag<'a>(label: &'a str, color: Color) -> Element<'a, Message> {
    container(text(label).size(10).font(font_bold()).color(color))
        .padding([2, 8])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(
                color.r, color.g, color.b, 0.1,
            ))),
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color::from_rgba(color.r, color.g, color.b, 0.3),
            },
            ..Default::default()
        })
        .into()
}

fn view_simple(app: &RepackApp) -> Element<'_, Message> {
    let (status_label, status_name, status_color) =
        if let Some((_, _, _, db_name, _)) = app.db.find_cloud_preset(&app.game_name) {
            (app.tr("factory_preset_applied"), db_name, ACCENT_CYAN)
        } else if app.game_name.is_empty() {
            (
                app.tr("factory_ready"),
                app.tr("factory_select_game").to_string(),
                TEXT_DIM,
            )
        } else {
            (
                app.tr("factory_auto_detect"),
                app.game_name.clone(),
                ACCENT_GRAY,
            )
        };

    let header = column![
        row![
            text(app.tr("factory_title"))
                .size(22)
                .font(font_bold())
                .color(ACCENT_CYAN),
            Space::with_width(Length::Fill),
            text("v1.01").size(11).color(TEXT_DIM),
        ],
        row![
            status_tag(status_label, status_color),
            Space::with_width(10),
            text(status_name)
                .size(14)
                .font(font_bold())
                .color(Color::WHITE),
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(10);

    let env_label = app
        .selected_proton
        .as_deref()
        .unwrap_or("Default")
        .to_string();
    let mode_label = app.factory_mode.to_string();
    let info_row = row![
        info_block(app.tr("factory_hardware"), &app.gpu_vendor),
        info_block(app.tr("factory_environment"), &env_label,),
        info_block(app.tr("factory_mode_label"), &mode_label),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let action_buttons = if app.repack_path.is_none() {
        Element::from(
            button(
                container(
                    text(app.tr("factory_open_source"))
                        .size(16)
                        .font(font_bold()),
                )
                .padding(20)
                .center_x(Length::Fill),
            )
            .on_press(Message::SelectRepackPressed)
            .style(|t, s| brand_button_style(t, s, false))
            .width(Length::Fill),
        )
    } else if app.is_producing {
        Element::from(
            button(
                container(
                    text(app.tr("factory_processing"))
                        .size(24)
                        .font(font_bold()),
                )
                .center_x(Length::Fill),
            )
            .width(Length::Fill)
            .padding(30)
            .style(|t, s| brand_button_style(t, s, true)),
        )
    } else {
        Element::from(
            column![
                button(
                    container(
                        text(app.tr("factory_start_production"))
                            .size(24)
                            .font(font_bold())
                    )
                    .center_x(Length::Fill)
                )
                .on_press(Message::StartProductionPressed)
                .width(Length::Fill)
                .padding(30)
                .style(|t, s| brand_button_style(t, s, true)),
                button(
                    container(
                        text(app.tr("factory_change_source"))
                            .size(12)
                            .color(TEXT_DIM)
                    )
                    .center_x(Length::Fill)
                )
                .on_press(Message::SelectRepackPressed)
                .width(Length::Fill)
                .style(|_, _| button::Style {
                    background: None,
                    text_color: TEXT_DIM,
                    ..Default::default()
                })
            ]
            .spacing(8),
        )
    };

    let progress_row = row![
        progress_bar(0.0..=1.0, app.progress)
            .height(8)
            .style(|_| progress_bar::Style {
                background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)),
                bar: Background::Color(ACCENT_BLUE),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                }
            }),
        text(format!("{}%", (app.progress * 100.0) as i32))
            .size(12)
            .font(font_bold())
            .color(ACCENT_BLUE),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let log_list = scrollable(
        column(
            app.logs
                .iter()
                .rev()
                .take(12)
                .map(|l| {
                    let color = if l.contains("[ERROR]") {
                        ACCENT_RED
                    } else if l.contains("[OK]") || l.contains("[SUCCESS]") {
                        ACCENT_BLUE
                    } else {
                        ACCENT_GRAY
                    };
                    text(format!("> {}", l))
                        .size(11)
                        .font(font_mono())
                        .color(color)
                        .into()
                })
                .collect::<Vec<Element<'_, Message>>>(),
        )
        .spacing(4),
    )
    .height(Length::FillPortion(1));

    let log_panel = panel_card(app.tr("factory_activity_feed"), log_list)
        .width(Length::Fill)
        .height(Length::FillPortion(1));

    let inspector_panel = panel_card(
        app.tr("factory_preset_inspector"),
        column![
            row![
                info_block(
                    app.tr("factory_source"),
                    app.preset_inspector_source.clone()
                ),
                info_block(
                    app.tr("factory_confidence"),
                    format!("{}%", app.preset_inspector_confidence),
                ),
                info_block(app.tr("factory_match"), app.preset_inspector_match.clone()),
            ]
            .spacing(10),
            info_block(
                app.tr("factory_reason"),
                app.preset_inspector_reason.clone()
            ),
            button(text(app.tr("factory_rollback_learned")))
                .on_press(Message::RollbackLearnedPressed)
                .style(|t, s| brand_button_style(t, s, false))
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .width(Length::Fill);

    let main_panel = glass_container(
        column![
            header,
            info_row,
            inspector_panel,
            action_buttons,
            progress_row,
            log_panel,
        ]
        .spacing(18),
    )
    .height(Length::Fill);

    column![main_panel].spacing(16).into()
}

fn view_advanced(app: &RepackApp) -> Element<'_, Message> {
    let (status_label, status_color) = if app.db.find_cloud_preset(&app.game_name).is_some() {
        (app.tr("factory_preset_applied"), ACCENT_CYAN)
    } else if app.game_name.is_empty() {
        (app.tr("factory_idle"), TEXT_DIM)
    } else {
        (app.tr("factory_heuristic_analysis"), ACCENT_GRAY)
    };

    let header = row![
        column![
            text(app.tr("factory_advanced_pipeline"))
                .size(22)
                .font(font_bold())
                .color(ACCENT_CYAN),
            row![
                status_tag(status_label, status_color),
                Space::with_width(10),
                text(&app.engine_insight)
                    .size(11)
                    .font(font_mono())
                    .color(TEXT_DIM),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(6),
        Space::with_width(Length::Fill),
        button(text(app.tr("factory_simple_mode")))
            .on_press(Message::ToggleUiMode(UiMode::Simple))
            .style(|t, s| brand_button_style(t, s, false)),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let environment_card = panel_card(
        app.tr("factory_base_environment"),
        column![
            column![
                text(app.tr("factory_gpu")).size(9).color(TEXT_DIM),
                text(&app.gpu_vendor)
                    .size(14)
                    .font(font_bold())
                    .color(Color::WHITE),
            ]
            .spacing(4),
            pick_list(
                app.ge_protons.as_slice(),
                app.selected_proton.clone(),
                Message::ProtonSelected,
            )
            .width(Length::Fill),
            row![
                button(text(app.tr("factory_change_exe")))
                    .on_press(Message::SelectGameExePressed)
                    .style(|t, s| brand_button_style(t, s, false))
                    .width(Length::FillPortion(1)),
                button(text(app.tr("factory_copy_logs")))
                    .on_press(Message::CopyLogsToClipboard)
                    .style(|t, s| brand_button_style(t, s, false))
                    .width(Length::FillPortion(1)),
            ]
            .spacing(8),
            panel_card(
                app.tr("factory_preset_inspector"),
                column![
                    row![
                        info_block(
                            app.tr("factory_source"),
                            app.preset_inspector_source.clone()
                        ),
                        info_block(
                            app.tr("factory_confidence"),
                            format!("{}%", app.preset_inspector_confidence),
                        ),
                    ]
                    .spacing(8),
                    info_block(app.tr("factory_match"), app.preset_inspector_match.clone()),
                    info_block(
                        app.tr("factory_reason"),
                        app.preset_inspector_reason.clone()
                    ),
                    button(text(app.tr("factory_rollback_learned")))
                        .on_press(Message::RollbackLearnedPressed)
                        .style(|t, s| brand_button_style(t, s, false))
                        .width(Length::Fill),
                ]
                .spacing(8),
            ),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(1));

    let engine_card = panel_card(
        app.tr("factory_engine_parameters"),
        scrollable(
            column![
                iced::widget::checkbox(app.tr("factory_dxvk"), app.opt_dxvk)
                    .on_toggle(Message::ToggleDxvk),
                iced::widget::checkbox(app.tr("factory_xaudio"), app.opt_xaudio)
                    .on_toggle(Message::ToggleXAudio),
                iced::widget::checkbox(app.tr("factory_vcrun2022"), app.opt_vcrun2022)
                    .on_toggle(Message::ToggleVcrun),
                iced::widget::checkbox(app.tr("factory_d3dx9"), app.opt_d3dx9)
                    .on_toggle(Message::ToggleD3dx9),
                iced::widget::checkbox(app.tr("factory_vcrun2005"), app.opt_vcrun2005)
                    .on_toggle(Message::ToggleVcrun2005),
                iced::widget::checkbox(app.tr("factory_vcrun2008"), app.opt_vcrun2008)
                    .on_toggle(Message::ToggleVcrun2008),
                iced::widget::checkbox(app.tr("factory_physx_support"), app.opt_physx)
                    .on_toggle(Message::TogglePhysx),
                iced::widget::checkbox(app.tr("factory_xact_legacy"), app.opt_xact)
                    .on_toggle(Message::ToggleXact),
                iced::widget::checkbox(app.tr("factory_prefix_32"), app.opt_win32)
                    .on_toggle(Message::ToggleWin32),
            ]
            .spacing(10),
        )
        .height(Length::FillPortion(1)),
    )
    .width(Length::FillPortion(1));

    let production_controls = if app.repack_path.is_none() {
        column![
            button(
                container(text(app.tr("factory_browse_source")))
                    .padding(16)
                    .center_x(Length::Fill)
            )
            .on_press(Message::SelectRepackPressed)
            .style(|t, s| brand_button_style(t, s, false))
            .width(Length::Fill),
            button(
                container(text(app.tr("factory_mount_iso")))
                    .padding(16)
                    .center_x(Length::Fill)
            )
            .on_press(Message::SelectFilePressed)
            .style(|t, s| brand_button_style(t, s, false))
            .width(Length::Fill),
        ]
        .spacing(12)
    } else if app.is_producing {
        column![button(
            container(text(app.tr("factory_producing")))
                .padding(30)
                .center_x(Length::Fill),
        )
        .width(Length::Fill)
        .style(|t, s| brand_button_style(t, s, true)),]
    } else {
        column![button(
            container(text(app.tr("factory_start_production")))
                .padding(30)
                .center_x(Length::Fill),
        )
        .on_press(Message::StartProductionPressed)
        .width(Length::Fill)
        .style(|t, s| brand_button_style(t, s, true)),]
    };

    let production_card = panel_card(
        app.tr("factory_production"),
        column![
            production_controls,
            row![
                progress_bar(0.0..=1.0, app.progress)
                    .height(8)
                    .width(Length::Fill)
                    .style(|_| progress_bar::Style {
                        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)),
                        bar: Background::Color(ACCENT_BLUE),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        }
                    }),
                text(format!("{}%", (app.progress * 100.0) as i32))
                    .size(12)
                    .font(font_bold())
                    .color(ACCENT_BLUE),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(16),
    )
    .width(Length::FillPortion(2));

    let log_card = panel_card(
        app.tr("factory_live_feed"),
        scrollable(
            column(
                app.logs
                    .iter()
                    .rev()
                    .take(30)
                    .map(|l| {
                        text(format!("> {}", l))
                            .size(11)
                            .font(font_mono())
                            .color(Color::from_rgb(0.5, 0.7, 1.0))
                            .into()
                    })
                    .collect::<Vec<Element<'_, Message>>>(),
            )
            .spacing(4),
        )
        .height(Length::FillPortion(1)),
    )
    .height(Length::FillPortion(1));

    column![
        glass_container(column![header].spacing(4),),
        column![
            row![environment_card, engine_card]
                .spacing(16)
                .width(Length::Fill)
                .align_y(Alignment::Start),
            production_card,
            log_card,
        ]
        .spacing(18)
        .padding(0),
    ]
    .spacing(20)
    .padding(24)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn view_export_dialog(app: &RepackApp) -> Element<'_, Message> {
    match &app.export_status {
        ExportStatus::Idle => view_export_setup(app),
        ExportStatus::Running(msg) => {
            view_export_progress(app, msg, app.progress, app.export_scope, app.export_dry_run)
        }
        ExportStatus::Success {
            path,
            audits,
            scope,
            dry_run,
        } => view_export_success(app, path, audits, *scope, *dry_run),
        ExportStatus::Error(err) => view_export_error(app, err),
    }
}

fn modal_container<'a>(
    width: u16,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content)
        .width(width)
        .padding(40)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.05, 0.05, 0.08))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color::from_rgba(0.0, 0.9, 1.0, 0.2),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 30.0,
            },
            ..Default::default()
        })
        .into()
}

fn view_export_setup(app: &RepackApp) -> Element<'_, Message> {
    modal_container(
        600,
        column![
            column![
                text(app.tr("export_config_title"))
                    .size(24)
                    .font(font_bold())
                    .color(ACCENT_CYAN),
                text(app.tr("export_config_desc")).size(12).color(TEXT_DIM),
            ]
            .spacing(4),
            Space::with_height(30),
            container(
                column![
                    text(app.tr("export_pack_options"))
                        .size(14)
                        .font(font_bold())
                        .color(ACCENT_CYAN),
                    Space::with_height(15),
                    iced::widget::checkbox(app.tr("export_standalone"), app.opt_export_standalone)
                        .on_toggle(Message::ToggleExportStandalone),
                    iced::widget::checkbox(app.tr("export_installer"), app.opt_export_installer)
                        .on_toggle(Message::ToggleExportInstaller),
                    Space::with_height(10),
                    iced::widget::checkbox(app.tr("auto_launch_export"), app.opt_auto_launch)
                        .on_toggle(Message::ToggleAutoLaunch),
                    iced::widget::checkbox(app.tr("export_keep_workdir"), app.opt_skip_cleanup,)
                        .on_toggle(Message::ToggleSkipCleanup),
                ]
                .spacing(12)
            )
            .padding(20)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.02))),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            Space::with_height(20),
            container(
                column![
                    text(app.tr("export_scope_title"))
                        .size(14)
                        .font(font_bold())
                        .color(ACCENT_CYAN),
                    pick_list(
                        ExportScope::ALL,
                        Some(app.export_scope),
                        Message::ExportScopeChanged
                    )
                    .width(Length::Fill),
                    Space::with_height(10),
                    iced::widget::checkbox(app.tr("export_dry_run_label"), app.export_dry_run)
                        .on_toggle(Message::ToggleDryRun),
                ]
                .spacing(10),
            ),
            Space::with_height(25),
            column![
                text(app.tr("export_target"))
                    .size(14)
                    .font(font_bold())
                    .color(ACCENT_CYAN),
                Space::with_height(10),
                row![
                    button(text(app.tr("export_change_folder")))
                        .on_press(Message::SelectExportPathPressed)
                        .style(|t, s| brand_button_style(t, s, false)),
                    Space::with_width(15),
                    text(
                        app.export_dest_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| app.tr("export_no_folder").to_string())
                    )
                    .size(11)
                    .color(Color::WHITE),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(5),
            Space::with_height(40),
            row![
                button(
                    container(text(app.tr("export_start")).font(font_bold()))
                        .padding(15)
                        .center_x(Length::Fill)
                )
                .on_press(Message::RunExportPressed)
                .style(|t, s| brand_button_style(t, s, true))
                .width(Length::Fill),
                button(
                    container(text(app.tr("export_cancel")).font(font_bold()))
                        .padding(15)
                        .center_x(Length::Fill)
                )
                .on_press(Message::CloseModalPressed)
                .style(|t, s| brand_button_style(t, s, false))
                .width(Length::Fill),
            ]
            .spacing(20)
        ]
        .padding(10),
    )
}

fn view_export_progress<'a>(
    app: &'a RepackApp,
    msg: &'a str,
    progress: f32,
    scope: ExportScope,
    dry_run: bool,
) -> Element<'a, Message> {
    modal_container(
        600,
        column![
            text(app.tr("export_running"))
                .size(24)
                .font(font_bold())
                .color(ACCENT_CYAN),
            Space::with_height(10),
            text(msg).size(14).color(Color::WHITE),
            Space::with_height(40),
            container(column![
                row![
                    text(app.tr("export_progress")).size(12).color(TEXT_DIM),
                    Space::with_width(Length::Fill),
                    text(format!("{}%", (progress * 100.0) as i32))
                        .size(12)
                        .font(font_bold())
                        .color(ACCENT_CYAN)
                ],
                text(format!("{}: {}", app.tr("export_scope_label"), scope))
                    .size(11)
                    .color(TEXT_DIM),
                text(if dry_run {
                    app.tr("export_running_dry")
                } else {
                    app.tr("export_running_full")
                })
                .size(11)
                .color(if dry_run { ACCENT_RED } else { ACCENT_BLUE }),
                Space::with_height(10),
                progress_bar(0.0..=1.0, progress)
                    .height(12)
                    .style(|_| progress_bar::Style {
                        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)),
                        bar: Background::Color(ACCENT_CYAN),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        }
                    })
            ])
            .padding(20)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25))),
                border: Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            Space::with_height(30),
            text(app.tr("export_running_note")).size(11).color(TEXT_DIM),
        ]
        .padding(20),
    )
}

fn view_export_success<'a>(
    app: &'a RepackApp,
    path: &'a str,
    audits: &'a [ExportAudit],
    scope: ExportScope,
    dry_run: bool,
) -> Element<'a, Message> {
    let p = path.to_string();
    modal_container(
        600,
        column![
            text(app.tr("export_done_title"))
                .size(28)
                .font(font_bold())
                .color(ACCENT_CYAN),
            Space::with_height(10),
            text(app.tr("export_done_desc"))
                .size(14)
                .color(Color::WHITE),
            Space::with_height(20),
            column![
                text(format!("{}: {}", app.tr("export_scope_short"), scope))
                    .size(11)
                    .color(TEXT_DIM),
                text(if dry_run {
                    app.tr("export_done_dry")
                } else {
                    app.tr("export_done_full")
                })
                .size(11)
                .color(if dry_run { ACCENT_RED } else { ACCENT_CYAN }),
            ]
            .spacing(4),
            Space::with_height(10),
            container(text(path).size(11).font(font_mono()))
                .padding(15)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3))),
                    border: Border {
                        radius: 5.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            Space::with_height(30),
            if audits.is_empty() {
                Element::from(text(app.tr("export_no_integrity")).size(10).color(TEXT_DIM))
            } else {
                Element::from(
                    column![
                        text(app.tr("export_integrity")).size(11).color(TEXT_DIM),
                        column(
                            audits
                                .iter()
                                .map(|audit| {
                                    row![
                                        text(&audit.label).size(11).color(Color::WHITE),
                                        Space::with_width(Length::Fill),
                                        text(short_hash(&audit.sha256)).size(10).color(ACCENT_GRAY)
                                    ]
                                    .align_y(Alignment::Center)
                                    .into()
                                })
                                .collect::<Vec<Element<'_, Message>>>(),
                        )
                        .spacing(6),
                    ]
                    .spacing(4),
                )
            },
            Space::with_height(30),
            row![
                button(
                    container(text(app.tr("export_open_folder")).font(font_bold()))
                        .padding(15)
                        .center_x(Length::Fill)
                )
                .on_press(Message::LogAppended(format!("[OPEN] Opening: {}", p)))
                .style(|t, s| brand_button_style(t, s, true))
                .width(Length::Fill),
                button(
                    container(text(app.tr("export_close")).font(font_bold()))
                        .padding(15)
                        .center_x(Length::Fill)
                )
                .on_press(Message::CloseModalPressed)
                .style(|t, s| brand_button_style(t, s, false))
                .width(Length::Fill),
            ]
            .spacing(20)
        ]
        .padding(20)
        .align_x(Alignment::Center),
    )
}

fn view_export_error<'a>(app: &'a RepackApp, err: &'a str) -> Element<'a, Message> {
    modal_container(
        600,
        column![
            text(app.tr("export_error"))
                .size(28)
                .font(font_bold())
                .color(Color::from_rgb(1.0, 0.3, 0.3)),
            Space::with_height(20),
            text(err).size(14).color(Color::WHITE),
            Space::with_height(40),
            button(
                container(text(app.tr("export_back_to_config")).font(font_bold()))
                    .padding(15)
                    .center_x(Length::Fill)
            )
            .on_press(Message::CloseModalPressed)
            .style(|t, s| brand_button_style(t, s, true))
            .width(Length::Fill),
        ]
        .padding(20)
        .align_x(Alignment::Center),
    )
}

fn short_hash(hash: &str) -> String {
    if hash.len() > 32 {
        format!("{}…", &hash[..32])
    } else {
        hash.to_string()
    }
}
