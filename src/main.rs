#![windows_subsystem = "windows"]

use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use nwg::NativeUi;
use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem, MenuEvent},
    TrayIconBuilder, TrayIcon,
};
use std::cell::RefCell;
use std::time::Duration;
use std::sync::mpsc;
use image::io::Reader as ImageReader;
use std::io::Cursor;

mod settings_manager;
mod path_manager;
mod account_manager;
mod game_launcher;
mod i18n_manager;

#[derive(Default, NwgUi)]
pub struct TrayApp {
    #[nwg_resource]
    font: nwg::Font,

    #[nwg_control]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::MessageWindow,

    #[nwg_control(parent: window, interval: Duration::from_millis(100))]
    #[nwg_events( OnTimerTick: [TrayApp::tick] )]
    timer: nwg::AnimationTimer,

    tray_icon: RefCell<Option<TrayIcon>>,
    
    // Communication channels for background dialogs to trigger UI updates.
    update_tx: RefCell<Option<mpsc::Sender<()>>>,
    update_rx: RefCell<Option<mpsc::Receiver<()>>>,
}

impl TrayApp {
    /// Main tick function checking for menu events and background updates.
    fn tick(&self) {
        if let Some(rx) = self.update_rx.borrow().as_ref() {
            if rx.try_recv().is_ok() {
                self.update_tray_menu();
            }
        }

        let menu_channel = MenuEvent::receiver();
        if let Ok(event) = menu_channel.try_recv() {
            let id = event.id.0.as_str();
            match id {
                "quit" => nwg::stop_thread_dispatch(),
                "launch" => {
                    let settings = settings_manager::load_settings();
                    if let Err(e) = game_launcher::launch_endfield(&settings.game_path) {
                        let title = i18n_manager::get_message("common", "error");
                        let msg = i18n_manager::get_message("notifications", "launch_failed").replace("{}", &e);
                        nwg::modal_error_message(&self.window.handle, &title, &msg);
                    }
                }
                "add_profile" => {
                    if let Some(tx) = self.update_tx.borrow().as_ref() {
                        run_add_profile_dialog(tx.clone());
                    }
                }
                "set_session_path" => {
                    std::thread::spawn(|| {
                        nwg::init().unwrap_or_default();
                        let title = i18n_manager::get_message("dialogs", "select_session_dir");
                        let mut dialog = nwg::FileDialog::default();
                        nwg::FileDialog::builder()
                            .title(&title)
                            .action(nwg::FileDialogAction::OpenDirectory)
                            .build(&mut dialog).unwrap();
                        if dialog.run::<&nwg::ControlHandle>(None) {
                            if let Ok(path) = dialog.get_selected_item() {
                                let mut settings = settings_manager::load_settings();
                                settings.session_path = path.to_string_lossy().into_owned();
                                let _ = settings_manager::save_settings(&settings);
                            }
                        }
                    });
                }
                "set_game_path" => {
                    std::thread::spawn(|| {
                        nwg::init().unwrap_or_default();
                        let title = i18n_manager::get_message("dialogs", "select_game_exe");
                        let filter = i18n_manager::get_message("dialogs", "exe_filter");
                        let mut dialog = nwg::FileDialog::default();
                        nwg::FileDialog::builder()
                            .title(&title)
                            .filters(&filter)
                            .build(&mut dialog).unwrap();
                        if dialog.run::<&nwg::ControlHandle>(None) {
                            if let Ok(path) = dialog.get_selected_item() {
                                let mut settings = settings_manager::load_settings();
                                settings.game_path = path.to_string_lossy().into_owned();
                                let _ = settings_manager::save_settings(&settings);
                            }
                        }
                    });
                }
                "auto_detect_paths" => {
                    std::thread::spawn(|| {
                        nwg::init().unwrap_or_default();
                        let mut settings = settings_manager::load_settings();
                        let mut changed = false;
                        if let Ok(p) = path_manager::auto_detect_session_path() {
                            settings.session_path = p;
                            changed = true;
                        }
                        if let Ok(p) = path_manager::auto_detect_game_path() {
                            settings.game_path = p;
                            changed = true;
                        }
                        if changed {
                            let _ = settings_manager::save_settings(&settings);
                            let title = i18n_manager::get_message("tray", "auto_detect");
                            let msg = i18n_manager::get_message("dialogs", "auto_detect_success")
                                .replace("{}", &settings.session_path)
                                .replace("{}", &settings.game_path);
                            nwg::modal_info_message(&nwg::ControlHandle::NoHandle, &title, &msg);
                        } else {
                            let title = i18n_manager::get_message("tray", "auto_detect");
                            let msg = i18n_manager::get_message("dialogs", "auto_detect_failed");
                            nwg::modal_error_message(&nwg::ControlHandle::NoHandle, &title, &msg);
                        }
                    });
                }
                "check_paths" => {
                    let settings = settings_manager::load_settings();
                    let title = i18n_manager::get_message("tray", "check_paths");
                    let msg = i18n_manager::get_message("dialogs", "current_paths_msg")
                        .replace("{}", &settings.session_path)
                        .replace("{}", &settings.game_path);
                    nwg::modal_info_message(&self.window.handle, &title, &msg);
                }
                id if id.starts_with("lang:") => {
                    let lang = &id[5..];
                    let mut settings = settings_manager::load_settings();
                    settings.language = lang.to_string();
                    if let Err(e) = settings_manager::save_settings(&settings) {
                        let title = i18n_manager::get_message("common", "error");
                        nwg::modal_error_message(&self.window.handle, &title, &e);
                    }
                    self.update_tray_menu();
                }
                id if id.starts_with("switch:") => {
                    let acc_name = &id[7..];
                    let settings = settings_manager::load_settings();
                    match account_manager::load_account_session(&settings.session_path, acc_name) {
                        Ok(_) => {
                            let title = i18n_manager::get_message("notifications", "switch_success_title");
                            let msg = i18n_manager::get_message("notifications", "switch_success_msg").replace("{}", acc_name);
                            nwg::modal_info_message(&self.window.handle, &title, &msg);
                        }
                        Err(e) => {
                            let title = i18n_manager::get_message("notifications", "switch_failed_title");
                            let msg = i18n_manager::get_message("notifications", "switch_failed_msg").replace("{}", &e);
                            nwg::modal_error_message(&self.window.handle, &title, &msg);
                        }
                    }
                    self.update_tray_menu();
                }
                id if id.starts_with("delete:") => {
                    let acc_name = &id[7..];
                    let title = i18n_manager::get_message("dialogs", "delete_confirm_title");
                    let msg = i18n_manager::get_message("dialogs", "delete_confirm_msg").replace("{}", acc_name);
                    let confirm = nwg::modal_message(&self.window.handle, &nwg::MessageParams {
                        title: &title,
                        content: &msg,
                        buttons: nwg::MessageButtons::YesNo,
                        icons: nwg::MessageIcons::Question,
                    });
                    if confirm == nwg::MessageChoice::Yes {
                        if let Err(e) = account_manager::delete_account(acc_name) {
                            let err_title = i18n_manager::get_message("common", "error");
                            nwg::modal_error_message(&self.window.handle, &err_title, &e);
                        }
                        self.update_tray_menu();
                    }
                }
                _ => {}
            }
        }
    }

    /// Regenerates the tray menu to reflect state changes.
    fn update_tray_menu(&self) {
        let menu = build_menu();
        if let Some(tray) = self.tray_icon.borrow_mut().as_mut() {
            tray.set_menu(Some(Box::new(menu)));
        }
    }
}

/// Dialog for adding a new profile based on current session files.
#[derive(Default, NwgUi)]
pub struct AddProfileApp {
    #[nwg_resource(family: "Segoe UI", size: 15)]
    small_font: nwg::Font,

    #[nwg_control(size: (300, 130), flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 10)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "", font: Some(&data.small_font))]
    #[nwg_layout_item(layout: layout, row: 0, col: 0, col_span: 2)]
    label: nwg::Label,

    #[nwg_control(text: "", font: Some(&data.small_font))]
    #[nwg_layout_item(layout: layout, row: 1, col: 0, col_span: 2)]
    input: nwg::TextInput,

    #[nwg_control(text: "", font: Some(&data.small_font))]
    #[nwg_layout_item(layout: layout, row: 2, col: 0)]
    #[nwg_events( OnButtonClick: [AddProfileApp::save] )]
    save_btn: nwg::Button,

    #[nwg_control(text: "", font: Some(&data.small_font))]
    #[nwg_layout_item(layout: layout, row: 2, col: 1)]
    #[nwg_events( OnButtonClick: [AddProfileApp::cancel] )]
    cancel_btn: nwg::Button,

    update_tx: Option<mpsc::Sender<()>>,
}

impl AddProfileApp {
    fn save(&self) {
        let text = self.input.text();
        if text.trim().is_empty() {
            let title = i18n_manager::get_message("common", "error");
            let msg = i18n_manager::get_message("dialogs", "enter_name_error");
            nwg::modal_error_message(&self.window, &title, &msg);
            return;
        }

        let settings = settings_manager::load_settings();
        match account_manager::save_account_session(&settings.session_path, &text) {
            Ok(_) => {
                let title = i18n_manager::get_message("common", "success");
                let msg = i18n_manager::get_message("dialogs", "save_success_msg");
                nwg::modal_info_message(&self.window, &title, &msg);
                if let Some(ref tx) = self.update_tx {
                    let _ = tx.send(());
                }
                self.window.close();
            }
            Err(e) => {
                let title = i18n_manager::get_message("common", "error");
                nwg::modal_error_message(&self.window, &title, &e);
            }
        }
    }

    fn cancel(&self) {
        self.window.close();
    }
}

/// Constructs the hierarchical tray menu.
fn build_menu() -> Menu {
    let accounts = account_manager::get_saved_accounts().unwrap_or_default();
    let active_account = account_manager::get_active_account();
    let settings = settings_manager::load_settings();
    
    let menu = Menu::new();
    
    // Section 1: Launch Game
    let launch_label = i18n_manager::get_message("tray", "launch");
    let launch_item = MenuItem::with_id("launch", launch_label, true, None);
    menu.append(&launch_item).unwrap();
    
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Section 2: Profile Management
    let profiles_label = i18n_manager::get_message("tray", "profiles");
    let remove_label = i18n_manager::get_message("tray", "remove_profile");
    let save_session_label = i18n_manager::get_message("tray", "save_session");
    
    let profile_menu = Submenu::new(profiles_label, true);
    let delete_menu = Submenu::new(remove_label, true);

    for acc in accounts {
        let is_active = Some(acc.clone()) == active_account;
        let switch_item = CheckMenuItem::with_id(format!("switch:{}", acc), acc.clone(), true, is_active, None);
        profile_menu.append(&switch_item).unwrap();

        let delete_item = MenuItem::with_id(format!("delete:{}", acc), acc.clone(), true, None);
        delete_menu.append(&delete_item).unwrap();
    }
    
    menu.append(&profile_menu).unwrap();
    menu.append(&delete_menu).unwrap();
    menu.append(&MenuItem::with_id("add_profile", save_session_label, true, None)).unwrap();
    
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Section 3: Path Detection and Status
    let auto_detect_label = i18n_manager::get_message("tray", "auto_detect");
    let check_paths_label = i18n_manager::get_message("tray", "check_paths");
    menu.append(&MenuItem::with_id("auto_detect_paths", auto_detect_label, true, None)).unwrap();
    menu.append(&MenuItem::with_id("check_paths", check_paths_label, true, None)).unwrap();
    
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Section 4: Configuration and Language
    let set_session_label = i18n_manager::get_message("tray", "set_session");
    let set_game_label = i18n_manager::get_message("tray", "set_game");
    let set_language_label = i18n_manager::get_message("tray", "set_language");
    
    menu.append(&MenuItem::with_id("set_session_path", set_session_label, true, None)).unwrap();
    menu.append(&MenuItem::with_id("set_game_path", set_game_label, true, None)).unwrap();
    
    let lang_menu = Submenu::new(set_language_label, true);
    lang_menu.append(&CheckMenuItem::with_id("lang:en", "English", true, settings.language == "en", None)).unwrap();
    lang_menu.append(&CheckMenuItem::with_id("lang:ko", "한국어", true, settings.language == "ko", None)).unwrap();
    menu.append(&lang_menu).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();
    
    // Section 5: Quit
    let quit_label = i18n_manager::get_message("tray", "quit");
    menu.append(&MenuItem::with_id("quit", quit_label, true, None)).unwrap();

    menu
}

/// Decodes the embedded icon file into a format compatible with tray-icon.
fn load_icon() -> tray_icon::Icon {
    let icon_data = include_bytes!("icon.ico");
    match ImageReader::new(Cursor::new(icon_data)).with_guessed_format().expect("Failed to guess icon format").decode() {
        Ok(image) => {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).unwrap()
        }
        Err(_) => tray_icon::Icon::from_rgba(vec![128; 32 * 32 * 4], 32, 32).unwrap()
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let app = TrayApp::build_ui(Default::default()).expect("Failed to build TrayApp UI");
    
    let (tx, rx) = mpsc::channel();
    *app.update_tx.borrow_mut() = Some(tx);
    *app.update_rx.borrow_mut() = Some(rx);
    
    let icon = load_icon();

    let menu = build_menu();
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("TA-TA Switch")
        .with_icon(icon)
        .build()
        .unwrap();

    *app.tray_icon.borrow_mut() = Some(tray_icon);
    app.timer.start();

    nwg::dispatch_thread_events();
}

/// Spawns an isolated thread for the "Add Profile" dialog to prevent blocking the main loop.
fn run_add_profile_dialog(tx: mpsc::Sender<()>) {
    std::thread::spawn(move || {
        nwg::init().unwrap_or_default();
        nwg::Font::set_global_family("Segoe UI").unwrap_or_default();

        let mut app_data = AddProfileApp::default();
        app_data.update_tx = Some(tx);

        let app = AddProfileApp::build_ui(app_data).expect("Failed to build Add Profile UI");
        
        // Center the window
        let sx = nwg::Monitor::width();
        let sy = nwg::Monitor::height();
        let (wx, wy) = app.window.size();
        app.window.set_position(sx as i32 / 2 - wx as i32 / 2, sy as i32 / 2 - wy as i32 / 2);

        // Set localized texts
        app.window.set_text(&i18n_manager::get_message("dialogs", "add_profile_title"));
        app.label.set_text(&i18n_manager::get_message("dialogs", "profile_name_label"));
        app.save_btn.set_text(&i18n_manager::get_message("common", "save"));
        app.cancel_btn.set_text(&i18n_manager::get_message("common", "cancel"));

        nwg::dispatch_thread_events();
    });
}
