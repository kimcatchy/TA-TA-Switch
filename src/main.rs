#![windows_subsystem = "windows"]

use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use nwg::NativeUi;
use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem, MenuEvent},
    TrayIconBuilder, TrayIcon,
};
use std::cell::RefCell;
use std::rc::Rc;
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
                        nwg::modal_error_message(&self.window.handle, "Error", &format!("Failed to launch: {}", e));
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
                        let mut dialog = nwg::FileDialog::default();
                        nwg::FileDialog::builder()
                            .title("Select Session Directory")
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
                        let mut dialog = nwg::FileDialog::default();
                        nwg::FileDialog::builder()
                            .title("Select Game Executable")
                            .filters("Executable (*.exe)")
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
                            let msg = format!("Paths detected and saved successfully:\n\nSession: {}\nGame: {}", settings.session_path, settings.game_path);
                            nwg::modal_info_message(&nwg::ControlHandle::NoHandle, "Auto Detect", &msg);
                        } else {
                            nwg::modal_error_message(&nwg::ControlHandle::NoHandle, "Auto Detect", "Could not auto-detect paths.");
                        }
                    });
                }
                "check_paths" => {
                    let settings = settings_manager::load_settings();
                    let msg = format!("Current Path Settings:\n\nSession: {}\nGame: {}", settings.session_path, settings.game_path);
                    nwg::modal_info_message(&self.window.handle, "Current Paths", &msg);
                }
                id if id.starts_with("lang:") => {
                    let lang = &id[5..];
                    let mut settings = settings_manager::load_settings();
                    settings.language = lang.to_string();
                    if let Err(e) = settings_manager::save_settings(&settings) {
                        nwg::modal_error_message(&self.window.handle, "Error", &e);
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
                    let confirm = nwg::modal_message(&self.window.handle, &nwg::MessageParams {
                        title: "Confirm Delete",
                        content: &format!("Are you sure you want to delete profile '{}'?", acc_name),
                        buttons: nwg::MessageButtons::YesNo,
                        icons: nwg::MessageIcons::Question,
                    });
                    if confirm == nwg::MessageChoice::Yes {
                        if let Err(e) = account_manager::delete_account(acc_name) {
                            nwg::modal_error_message(&self.window.handle, "Error", &e);
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
    #[nwg_resource(title: "Add Profile", size: (300, 150), position: (300, 300))]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 10)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "Profile Name:")]
    #[nwg_layout_item(layout: layout, row: 0, col: 0)]
    label: nwg::Label,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, row: 1, col: 0, col_span: 2)]
    input: nwg::TextInput,

    #[nwg_control(text: "Save")]
    #[nwg_layout_item(layout: layout, row: 2, col: 0)]
    save_btn: nwg::Button,

    #[nwg_control(text: "Cancel")]
    #[nwg_layout_item(layout: layout, row: 2, col: 1)]
    cancel_btn: nwg::Button,
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
        let app = AddProfileApp::build_ui(Default::default()).expect("Failed to build Add Profile UI");
        
        let app_rc = Rc::new(app);
        let app_rc_clone = app_rc.clone();
        
        let handler = nwg::full_bind_event_handler(&app_rc.window.handle, move |evt, _evt_data, handle| {
            use nwg::Event as E;
            match evt {
                E::OnButtonClick => {
                    if handle == app_rc_clone.save_btn {
                        let text = app_rc_clone.input.text();
                        if !text.is_empty() {
                            let settings = crate::settings_manager::load_settings();
                            match crate::account_manager::save_account_session(&settings.session_path, &text) {
                                Ok(_) => {
                                    nwg::modal_info_message(&app_rc_clone.window.handle, "Success", "Profile saved successfully.");
                                    let _ = tx.send(());
                                }
                                Err(e) => {
                                    nwg::modal_error_message(&app_rc_clone.window.handle, "Error", &e);
                                }
                            }
                            app_rc_clone.window.close();
                        }
                    } else if handle == app_rc_clone.cancel_btn {
                        app_rc_clone.window.close();
                    }
                }
                E::OnWindowClose => {
                    if handle == app_rc_clone.window {
                        nwg::stop_thread_dispatch();
                    }
                }
                _ => {}
            }
        });

        nwg::dispatch_thread_events();
        nwg::unbind_event_handler(&handler);
    });
}
