use iced::widget::{
    button, checkbox, column, container, progress_bar, row, text, text_input, Space,
};
use iced::{
    window, Alignment, Background, Border, Color, Element, Gradient, Length, Shadow, Task, Vector,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;

// --- KOLORY ULTIMATE ---
const ACCENT_CYAN: Color = Color::from_rgb(0.36, 0.72, 0.98);
const ACCENT_PRIMARY: Color = Color::from_rgb(0.16, 0.56, 0.93);
const TEXT_WHITE: Color = Color::from_rgb(0.94, 0.94, 0.96);
const TEXT_DIM: Color = Color::from_rgb(0.56, 0.58, 0.62);
const DEEP_DARK: Color = Color::from_rgb(0.02, 0.02, 0.04);
const GLASS_BG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.04);
const BUTTON_RADIUS: f32 = 14.0;

fn clamp_channel(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

fn adjust_color(color: Color, delta: f32) -> Color {
    Color::from_rgb(
        clamp_channel(color.r + delta),
        clamp_channel(color.g + delta),
        clamp_channel(color.b + delta),
    )
}

const R2P_ICON_SVG: &str = r###"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#0e5fae"/>
      <stop offset="60%" stop-color="#2c6dff"/>
      <stop offset="100%" stop-color="#ff4f58"/>
    </linearGradient>
  </defs>
  <rect width="256" height="256" rx="48" ry="48" fill="#05060f"/>
  <circle cx="128" cy="128" r="78" fill="url(#g)"/>
</svg>"###;

#[derive(Debug, Clone)]
pub enum Message {
    NextStep,
    PrevStep,
    StartInstallation,
    ProgressUpdated(f32, String),
    Finished(Result<String, String>),
    ToggleShortcut(bool),
    ToggleMenuEntry(bool),
    PathChanged(String),
    BrowsePath,
    FixSystem,
    Exit,
}

pub struct InstallerGui {
    game_name: String,
    source_script: PathBuf,
    offset: u64,
    total_files: u64,
    is_64bit: bool,
    req_space_mb: u64,
    available_space_mb: u64,
    install_path: String,
    progress: f32,
    current_file: String,
    step: Step,
    create_shortcut: bool,
    create_menu_entry: bool,
    system_ready: bool,
    missing_libs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Welcome,
    PathSelection,
    Summary,
    Installing,
    Success(String),
    Error(String),
}

impl InstallerGui {
    fn new(
        game_name: String,
        script: PathBuf,
        offset: u64,
        total: u64,
        is_64bit: bool,
        req_mb: u64,
    ) -> (Self, Task<Message>) {
        let missing = Self::check_system_deps(is_64bit);
        let ready = missing.is_empty();
        let default_path = std::env::current_dir()
            .unwrap_or_default()
            .join(&game_name)
            .to_string_lossy()
            .to_string();
        let avail = Self::get_available_space_mb(&default_path);

        (
            Self {
                game_name,
                source_script: script,
                offset,
                total_files: total,
                is_64bit,
                req_space_mb: req_mb,
                available_space_mb: avail,
                install_path: default_path,
                progress: 0.0,
                current_file: String::new(),
                step: Step::Welcome,
                create_shortcut: true,
                create_menu_entry: true,
                system_ready: ready,
                missing_libs: missing,
            },
            Task::none(),
        )
    }

    fn check_system_deps(is_64bit: bool) -> Vec<String> {
        let mut missing = Vec::new();
        if !is_64bit {
            let output = std::process::Command::new("ldconfig").arg("-p").output();
            if let Ok(out) = output {
                let s = String::from_utf8_lossy(&out.stdout);
                if !s.contains("libvulkan.so.1") || !s.contains("x86") {
                    missing.push("libvulkan1:i386".into());
                }
                if !s.contains("libGL.so.1") || !s.contains("x86") {
                    missing.push("libgl1:i386".into());
                }
            }
        }
        missing
    }

    fn get_available_space_mb(path_str: &str) -> u64 {
        let path = Path::new(path_str);
        let check_dir = if path.exists() {
            path
        } else {
            path.parent().unwrap_or(Path::new("/"))
        };
        let output = std::process::Command::new("df")
            .arg("-m")
            .arg("--output=avail")
            .arg(check_dir)
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            s.lines()
                .nth(1)
                .and_then(|l| l.trim().parse().ok())
                .unwrap_or(0)
        } else {
            0
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NextStep => {
                match self.step {
                    Step::Welcome => self.step = Step::PathSelection,
                    Step::PathSelection => self.step = Step::Summary,
                    _ => {}
                }
                Task::none()
            }
            Message::PrevStep => {
                match self.step {
                    Step::PathSelection => self.step = Step::Welcome,
                    Step::Summary => self.step = Step::PathSelection,
                    _ => {}
                }
                Task::none()
            }
            Message::PathChanged(p) => {
                self.install_path = p;
                self.available_space_mb = Self::get_available_space_mb(&self.install_path);
                Task::none()
            }
            Message::BrowsePath => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.install_path = path.join(&self.game_name).to_string_lossy().to_string();
                    self.available_space_mb = Self::get_available_space_mb(&self.install_path);
                }
                Task::none()
            }
            Message::StartInstallation => {
                self.step = Step::Installing;
                let offset = self.offset;
                let source = self.source_script.clone();
                let target = PathBuf::from(&self.install_path);
                let total = self.total_files;
                let create_shortcut = self.create_shortcut;
                let create_menu = self.create_menu_entry;
                let game_name = self.game_name.clone();

                Task::stream(async_stream::stream! {
                    let _ = tokio::fs::create_dir_all(&target).await;
                    let mut cmd = tokio::process::Command::new("bash");
                    let cmd_str = format!("tail -c +{} \"{}\" | zstd -dcq | tar -xv -C \"{}\" -f - && sync", offset, source.display(), target.display());
                    cmd.arg("-c").arg(&cmd_str).stdout(Stdio::piped()).stderr(Stdio::piped());
                    if let Ok(mut child) = cmd.spawn() {
                        use tokio::io::{BufReader, AsyncBufReadExt};
                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();
                        let mut reader = BufReader::new(stdout).lines();
                        let mut err_reader = BufReader::new(stderr).lines();
                        let mut count = 0;
                        let mut stdout_done = false;
                        let mut stderr_done = false;
                        while !stdout_done || !stderr_done {
                            tokio::select! {
                                line = reader.next_line(), if !stdout_done => {
                                    match line {
                                        Ok(Some(l)) => {
                                            count += 1;
                                            if count % 5 == 0 { let p = if total > 0 { (count as f32 / total as f32).min(0.99) } else { 0.5 }; yield Message::ProgressUpdated(p, l); }
                                        }
                                        _ => { stdout_done = true; }
                                    }
                                }
                                err_line = err_reader.next_line(), if !stderr_done => {
                                    match err_line { Ok(Some(el)) => { eprintln!("[TAR ERR] {}", el); } _ => { stderr_done = true; } }
                                }
                            }
                        }
                        let _ = child.wait().await;
                        if !prefix_hives_valid(&target) {
                            yield Message::Finished(Err(
                                "Prefix registry files were truncated. Please recreate the export."
                                    .into(),
                            ));
                            return;
                        }
                    }
                    let play_sh = target.join("play.sh");
                    let play_auto_sh = target.join("play_auto.sh");
                    if play_sh.exists() {
                        #[cfg(unix)] {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = tokio::fs::metadata(&play_sh).await {
                                let mut perms = meta.permissions();
                                perms.set_mode(0o755);
                                let _ = tokio::fs::set_permissions(&play_sh, perms).await;
                            }
                        }
                    }
                    if play_auto_sh.exists() {
                        #[cfg(unix)] {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = tokio::fs::metadata(&play_auto_sh).await {
                                let mut perms = meta.permissions();
                                perms.set_mode(0o755);
                                let _ = tokio::fs::set_permissions(&play_auto_sh, perms).await;
                            }
                        }
                    }
                    let exec_sh = if play_auto_sh.exists() {
                        play_auto_sh.clone()
                    } else {
                        play_sh.clone()
                    };
                    let icon_path = target.join("r2p-icon.svg");
                    let _ = tokio::fs::write(&icon_path, R2P_ICON_SVG).await;
                    let game_icon = target.join("icon.png");
                    let icon_for_desktop = if game_icon.exists() {
                        game_icon
                    } else {
                        icon_path.clone()
                    };
                    let desktop_content = format!(
                        "[Desktop Entry]\nVersion=1.0\nType=Application\nName={}\nExec=\"{}\"\nPath=\"{}\"\nIcon={}\nTerminal=false\nCategories=Game;\nStartupNotify=true\n",
                        game_name,
                        exec_sh.display(),
                        target.display(),
                        icon_for_desktop.display()
                    );
                    let shortcut_name = desktop_file_name(&game_name);
                    let helper_script = target.join("adddesktopicon.sh");
                    let mut helper_done = false;
                    if helper_script.exists() && (play_sh.exists() || play_auto_sh.exists()) && (create_shortcut || create_menu) {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = tokio::fs::metadata(&helper_script).await {
                                let mut perms = meta.permissions();
                                perms.set_mode(0o755);
                                let _ = tokio::fs::set_permissions(&helper_script, perms).await;
                            }
                        }
                        let mut cmd = tokio::process::Command::new("bash");
                        cmd.arg(&helper_script);
                        if create_shortcut && !create_menu {
                            cmd.arg("--desktop-only");
                        } else if !create_shortcut && create_menu {
                            cmd.arg("--menu-only");
                        }
                        if let Ok(status) = cmd.status().await {
                            helper_done = status.success();
                        }
                    }
                    if create_shortcut && (play_sh.exists() || play_auto_sh.exists()) {
                        if !helper_done {
                            for desk in desktop_shortcut_paths(&shortcut_name) {
                                if let Some(parent) = desk.parent() {
                                    let _ = tokio::fs::create_dir_all(parent).await;
                                }
                                let _ = tokio::fs::write(&desk, &desktop_content).await;
                                set_desktop_file_executable(&desk).await;
                            }
                        }
                    }
                    if create_menu && (play_sh.exists() || play_auto_sh.exists()) {
                        if !helper_done {
                            let home = std::env::var("HOME").unwrap_or_default();
                            let menu_path = std::path::PathBuf::from(home)
                                .join(".local/share/applications")
                                .join(format!("{}.desktop", shortcut_name));
                            let _ = tokio::fs::create_dir_all(menu_path.parent().unwrap()).await;
                            let _ = tokio::fs::write(&menu_path, &desktop_content).await;
                            set_desktop_file_executable(&menu_path).await;
                        }
                    }
                    yield Message::Finished(Ok(target.to_string_lossy().to_string()));
                })
            }
            Message::ProgressUpdated(p, file) => {
                self.progress = p;
                self.current_file = file;
                Task::none()
            }
            Message::Finished(res) => {
                match res {
                    Ok(p) => self.step = Step::Success(p),
                    Err(e) => self.step = Step::Error(e),
                };
                Task::none()
            }
            Message::ToggleShortcut(v) => {
                self.create_shortcut = v;
                Task::none()
            }
            Message::ToggleMenuEntry(v) => {
                self.create_menu_entry = v;
                Task::none()
            }
            Message::Exit => {
                std::process::exit(0);
            }
            Message::FixSystem => {
                if !self.missing_libs.is_empty() {
                    let cmd = format!("sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install -y {}", self.missing_libs.join(" "));
                    let _ = std::process::Command::new("gnome-terminal").arg("--").arg("bash").arg("-c").arg(format!("{}; echo 'System gotowy! Zamknij to okno i uruchom instalator ponownie.'; read", cmd)).spawn();
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content: Element<_> = match &self.step {
            Step::Welcome => self.view_welcome(),
            Step::PathSelection => self.view_path_selection(),
            Step::Summary => self.view_summary(),
            Step::Installing => self.view_installing(),
            Step::Success(p) => self.view_success(p.clone()),
            Step::Error(e) => self.view_error(e.clone()),
        };
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(DEEP_DARK.into()),
                ..Default::default()
            })
            .into()
    }

    fn view_welcome(&self) -> Element<'_, Message> {
        self.glass_box(
            600,
            column![
                text("R2L UNIFIED INSTALLER")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(ACCENT_CYAN),
                Space::with_height(10),
                text(&self.game_name)
                    .size(48)
                    .font(font_bold())
                    .color(Color::WHITE),
                text("Version 1.6.8 - Professional Production")
                    .size(12)
                    .color(TEXT_DIM),
                Space::with_height(30),
                if !self.system_ready {
                    Element::from(
                        container(
                            column![
                                text("SYSTEM REQUIRES ATTENTION")
                                    .color(Color::from_rgb(1.0, 0.3, 0.3))
                                    .font(font_bold()),
                                text(format!("Missing: {}", self.missing_libs.join(", ")))
                                    .size(12)
                                    .color(TEXT_WHITE),
                                Space::with_height(10),
                                button(text("AUTO-FIX SYSTEM"))
                                    .on_press(Message::FixSystem)
                                    .padding(10)
                                    .style(|_, status| {
                                        accent_button_style(status, Color::from_rgb(0.8, 0.2, 0.2))
                                    }),
                            ]
                            .spacing(5)
                            .align_x(Alignment::Center),
                        )
                        .padding(15)
                        .style(|_| container::Style {
                            background: Some(Color::from_rgba(1.0, 0.0, 0.0, 0.1).into()),
                            border: Border {
                                radius: 8.0.into(),
                                width: 1.0,
                                color: Color::from_rgba(1.0, 0.0, 0.0, 0.3),
                            },
                            ..Default::default()
                        }),
                    )
                } else {
                    Element::from(
                        row![text("System is ready for this game")
                            .color(TEXT_DIM)
                            .size(14),]
                        .align_y(Alignment::Center),
                    )
                },
                Space::with_height(40),
                row![
                    button(
                        container(text("INSTALL"))
                            .padding(15)
                            .center_x(Length::Fill)
                    )
                    .on_press(Message::NextStep)
                    .width(180)
                    .style(|_, status| accent_button_style(status, ACCENT_PRIMARY)),
                    Space::with_width(20),
                    button(container(text("EXIT")).padding(15).center_x(Length::Fill))
                        .on_press(Message::Exit)
                        .width(120)
                        .style(|_, status| ghost_button_style(status)),
                ]
                .align_y(Alignment::Center)
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
    }

    fn view_path_selection(&self) -> Element<'_, Message> {
        let is_enough_space = self.available_space_mb > self.req_space_mb;
        let space_color = if is_enough_space {
            ACCENT_PRIMARY
        } else {
            Color::from_rgb(1.0, 0.3, 0.3)
        };
        let continue_btn = button(
            container(text("CONTINUE"))
                .padding(15)
                .center_x(Length::Fill),
        )
        .width(200)
        .style(|_, status| accent_button_style(status, ACCENT_CYAN));

        self.glass_box(
            650,
            column![
                text("INSTALLATION PATH")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(ACCENT_CYAN),
                Space::with_height(20),
                row![
                    text_input("Installation Path", &self.install_path)
                        .on_input(Message::PathChanged)
                        .padding(12)
                        .size(14),
                    Space::with_width(10),
                    button(text("Browse..."))
                        .on_press(Message::BrowsePath)
                        .padding(12),
                ]
                .align_y(Alignment::Center),
                Space::with_height(30),
                container(
                    column![
                        row![
                            text("Required Space:").size(13).color(TEXT_DIM),
                            Space::with_width(Length::Fill),
                            text(format!("{} MB", self.req_space_mb))
                                .size(13)
                                .color(Color::WHITE)
                        ],
                        row![
                            text("Available Space:").size(13).color(TEXT_DIM),
                            Space::with_width(Length::Fill),
                            text(format!("{} MB", self.available_space_mb))
                                .size(13)
                                .color(space_color)
                        ],
                    ]
                    .spacing(10)
                )
                .padding(20)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.2))),
                    border: Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                Space::with_height(40),
                row![
                    button(text("Back"))
                        .on_press(Message::PrevStep)
                        .padding(12)
                        .width(100)
                        .style(|_, status| ghost_button_style(status)),
                    Space::with_width(Length::Fill),
                    if is_enough_space {
                        continue_btn.on_press(Message::NextStep)
                    } else {
                        continue_btn
                    }
                ]
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
    }

    fn view_summary(&self) -> Element<'_, Message> {
        self.glass_box(
            600,
            column![
                text("INSTALLATION SUMMARY")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(ACCENT_CYAN),
                Space::with_height(20),
                container(
                    column![
                        self.summary_row("Game:", &self.game_name),
                        self.summary_row("Engine:", "Proton Experimental (R2L Bundle)"),
                        self.summary_row(
                            "Prefix Architecture:",
                            if self.is_64bit { "Win64" } else { "Win32" }
                        ),
                        self.summary_row("DirectX Translation:", "DXVK Enabled"),
                    ]
                    .spacing(12)
                )
                .padding(25)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.02))),
                    border: Border {
                        radius: 10.0.into(),
                        width: 1.0,
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                    },
                    ..Default::default()
                }),
                Space::with_height(30),
                column![
                    checkbox("Create Desktop Shortcut", self.create_shortcut)
                        .on_toggle(Message::ToggleShortcut),
                    checkbox("Create Menu Entry", self.create_menu_entry)
                        .on_toggle(Message::ToggleMenuEntry),
                ]
                .spacing(15)
                .align_x(Alignment::Start),
                Space::with_height(40),
                row![
                    button(text("Back"))
                        .on_press(Message::PrevStep)
                        .padding(12)
                        .width(100),
                    Space::with_width(Length::Fill),
                    button(
                        container(text("START INSTALLATION"))
                            .padding(15)
                            .center_x(Length::Fill)
                    )
                    .on_press(Message::StartInstallation)
                    .width(250)
                    .style(|_, status| accent_button_style(status, ACCENT_CYAN)),
                ]
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
    }

    fn view_installing(&self) -> Element<'_, Message> {
        self.glass_box(
            600,
            column![
                text("INSTALLING ASSETS")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(ACCENT_CYAN),
                Space::with_height(40),
                progress_bar(0.0..=1.0, self.progress)
                    .height(12)
                    .style(|_| progress_bar::Style {
                        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)),
                        bar: Background::Color(ACCENT_PRIMARY),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        }
                    }),
                text(format!("{}%", (self.progress * 100.0) as i32))
                    .size(16)
                    .font(font_bold())
                    .color(ACCENT_PRIMARY),
                Space::with_height(30),
                text(&self.current_file).size(11).color(TEXT_DIM),
            ]
            .padding(20)
            .align_x(Alignment::Center),
        )
    }

    fn view_success(&self, path: String) -> Element<'_, Message> {
        self.glass_box(
            600,
            column![
                text("SUCCESS")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(ACCENT_PRIMARY),
                text("INSTALLATION COMPLETE")
                    .size(28)
                    .font(font_bold())
                    .color(Color::WHITE),
                Space::with_height(20),
                text("Target directory:").size(12).color(TEXT_DIM),
                text(path).size(12).color(ACCENT_CYAN),
                Space::with_height(40),
                button(
                    container(text("CLOSE INSTALLER").font(font_bold()))
                        .padding(15)
                        .center_x(Length::Fill)
                )
                .on_press(Message::Exit)
                .width(200)
                .style(|_, status| accent_button_style(status, ACCENT_PRIMARY))
            ]
            .align_x(Alignment::Center),
        )
    }

    fn view_error(&self, err: String) -> Element<'_, Message> {
        self.glass_box(
            600,
            column![
                text("CRITICAL ERROR")
                    .size(14)
                    .font(iced::Font::MONOSPACE)
                    .color(Color::from_rgb(1.0, 0.2, 0.2)),
                text("INSTALLATION FAILED")
                    .size(28)
                    .font(font_bold())
                    .color(Color::WHITE),
                Space::with_height(20),
                text(err).color(Color::WHITE).size(14),
                Space::with_height(40),
                button(text("EXIT"))
                    .on_press(Message::Exit)
                    .padding(15)
                    .style(|_, status| ghost_button_style(status))
            ]
            .spacing(20)
            .align_x(Alignment::Center),
        )
    }

    fn glass_box<'a>(
        &self,
        width: u16,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        container(content)
            .width(width)
            .padding(50)
            .style(|_| container::Style {
                background: Some(GLASS_BG.into()),
                border: Border {
                    radius: 20.0.into(),
                    width: 1.0,
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                },
                ..Default::default()
            })
            .into()
    }

    fn summary_row<'a>(&self, label: &'a str, value: &'a str) -> Element<'a, Message> {
        row![
            text(label).size(13).color(TEXT_DIM).width(150),
            text(value).size(13).color(Color::WHITE).font(font_bold())
        ]
        .into()
    }
}

fn accent_button_style(status: button::Status, accent: Color) -> button::Style {
    let mut style = button::Style::default();
    style.border = Border {
        radius: BUTTON_RADIUS.into(),
        width: 1.0,
        color: accent,
    };
    style.text_color = Color::WHITE;
    match status {
        button::Status::Pressed => {
            style.background = Some(gradient_background(accent, -0.2, -0.05));
        }
        button::Status::Hovered => {
            style.background = Some(gradient_background(accent, 0.05, 0.2));
            style.shadow = Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.32),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            };
        }
        _ => {
            style.background = Some(gradient_background(accent, 0.0, 0.1));
        }
    }
    style
}

fn ghost_button_style(status: button::Status) -> button::Style {
    let mut style = button::Style::default();
    style.border = Border {
        radius: BUTTON_RADIUS.into(),
        width: 1.0,
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
    };
    style.text_color = Color::WHITE;
    match status {
        button::Status::Hovered => {
            style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.15)));
        }
        _ => {
            style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03)));
        }
    }
    style
}

fn gradient_background(color: Color, start_shift: f32, end_shift: f32) -> Background {
    Background::Gradient(Gradient::Linear(
        iced::gradient::Linear::new(std::f32::consts::FRAC_PI_4)
            .add_stop(0.0, adjust_color(color, start_shift))
            .add_stop(1.0, adjust_color(color, end_shift)),
    ))
}

fn font_bold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..iced::Font::DEFAULT
    }
}

fn prefix_hives_valid(target: &Path) -> bool {
    let prefix = target.join("pfx");
    for hive in ["system.reg", "user.reg", "userdef.reg"] {
        let candidate = prefix.join(hive);
        if let Ok(meta) = std::fs::metadata(&candidate) {
            if meta.len() == 0 {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn desktop_file_name(game_name: &str) -> String {
    let mut name = game_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while name.contains("__") {
        name = name.replace("__", "_");
    }
    name.trim_matches('_').to_string()
}

fn desktop_shortcut_paths(shortcut_name: &str) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();

    if let Some(xdg_dir) = resolve_desktop_dir() {
        dirs.push(xdg_dir);
    }
    if !home.is_empty() {
        dirs.push(PathBuf::from(&home).join("Desktop"));
        dirs.push(PathBuf::from(&home).join("Pulpit"));
    }

    let mut out = Vec::new();
    for dir in dirs {
        if !out.iter().any(|p: &PathBuf| p == &dir) {
            out.push(dir.join(format!("{}.desktop", shortcut_name)));
        }
    }
    out
}

fn resolve_desktop_dir() -> Option<PathBuf> {
    let output = std::process::Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(raw))
    }
}

async fn set_desktop_file_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = tokio::fs::metadata(path).await {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = tokio::fs::set_permissions(path, perms).await;
        }
    }
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        return;
    }
    let game_name = args[1].clone();
    let script = PathBuf::from(&args[2]);
    let offset: u64 = args[3].parse().unwrap_or(0);
    let total: u64 = args[4].parse().unwrap_or(0);
    let is_64: bool = args[5].parse().unwrap_or(true);
    let req_mb: u64 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
    let _ = iced::application(
        |state: &InstallerGui| format!("R2L Installer - {}", state.game_name),
        InstallerGui::update,
        InstallerGui::view,
    )
    .window(installer_window_settings())
    .run_with(move || {
        InstallerGui::new(
            game_name.clone(),
            script.clone(),
            offset,
            total,
            is_64,
            req_mb,
        )
    });
}

fn installer_window_settings() -> window::Settings {
    window::Settings {
        icon: installer_window_icon(),
        ..window::Settings::default()
    }
}

fn installer_window_icon() -> Option<window::Icon> {
    const SIZE: u32 = 64;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];
    let cx = (SIZE as f32) / 2.0;
    let cy = (SIZE as f32) / 2.0;
    let radius = 19.5_f32;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let mut r = 5_u8;
            let mut g = 6_u8;
            let mut b = 15_u8;
            let a = 255_u8;

            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= radius {
                let t = (x as f32 + y as f32) / ((SIZE - 1) as f32 * 2.0);
                r = (14.0 + (255.0 - 14.0) * t) as u8;
                g = (95.0 + (79.0 - 95.0) * t) as u8;
                b = (174.0 + (88.0 - 174.0) * t) as u8;
            }

            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = a;
        }
    }

    window::icon::from_rgba(rgba, SIZE, SIZE).ok()
}
