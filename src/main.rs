use iced::window;
use iced::{Task, Theme};

mod command_runner;
mod community_sync;
mod config;
mod database;
mod dependencies;
mod detector;
mod engine;
mod export;
mod installer;
mod mounter;
mod presets;
mod proton;
mod shortcuts;
mod ui;
mod app;

use app::{RepackApp, Message};

const APP_VERSION: &str = "1.3.0";

pub fn main() -> iced::Result {
    iced::application(
        |_state: &RepackApp| format!("Repack2Linux v{}", APP_VERSION),
        RepackApp::update,
        RepackApp::view,
    )
        .window(app_window_settings())
        .theme(|_| Theme::Dark)
        .subscription(RepackApp::subscription)
        .run_with(|| {
            (
                RepackApp::default(),
                Task::batch(vec![
                    Task::done(Message::SyncCloudDatabase),
                    Task::done(Message::ProcessCommunityQueue),
                    Task::done(Message::CheckForUpdates),
                ]),
            )
        })
}

fn app_window_settings() -> window::Settings {
    window::Settings {
        icon: app_window_icon(),
        #[cfg(target_os = "linux")]
        platform_specific: window::settings::PlatformSpecific {
            application_id: "repack2linux".to_string(),
            ..Default::default()
        },
        ..window::Settings::default()
    }
}

fn app_window_icon() -> Option<window::Icon> {
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
