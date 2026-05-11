use crate::{show_main_window, AppState};
use std::sync::atomic::Ordering;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, Theme};

const TRAY_ICON_LIGHT: &[u8] = include_bytes!("../icons/tray-light.png");
const TRAY_ICON_DARK: &[u8] = include_bytes!("../icons/tray-dark.png");

fn tray_icon_for(theme: Theme) -> Result<Image<'static>, tauri::Error> {
    let bytes = if matches!(theme, Theme::Dark) {
        TRAY_ICON_DARK
    } else {
        TRAY_ICON_LIGHT
    };
    Image::from_bytes(bytes)
}

pub(crate) fn apply_tray_theme(tray: &TrayIcon, theme: Theme) {
    if let Ok(icon) = tray_icon_for(theme) {
        let _ = tray.set_icon(Some(icon));
    }
}

pub(crate) fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    // A right-click "Quit" entry is the only way to exit the app
    // when the window is hidden, since Cmd+Q on a hidden window
    // doesn't reach the menu bar.
    let quit_item = MenuItem::with_id(app, "quit", "Quit notd", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let initial_theme = app
        .get_webview_window("main")
        .and_then(|w| w.theme().ok())
        .unwrap_or(Theme::Light);

    let tray = TrayIconBuilder::with_id("notd-tray")
        .icon(tray_icon_for(initial_theme)?)
        .icon_as_template(false)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                let state = app.state::<AppState>();
                state.is_quitting.store(true, Ordering::Relaxed);
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    // Theme swap is wired via WindowEvent::ThemeChanged on the
    // builder-level handler above, which finds the tray by id.
    let _ = tray;

    Ok(())
}
