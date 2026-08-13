//! Windows-native context menu for the pet overlay.

use deskhud_ui::{MessageKey, UiPreferences};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreatePopupMenu, DestroyMenu, EnableMenuItem, GetCursorPos,
    GetForegroundWindow, MF_CHECKED, MF_DISABLED, MF_ENABLED, MF_SEPARATOR, MF_STRING,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx,
};

use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};

const ID_SETTINGS: u32 = 1;
const ID_TOPMOST: u32 = 2;
const ID_MASTER: u32 = 3;
const ID_LAYOUT: u32 = 4;
const ID_QUIT: u32 = 5;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn popup_position(cursor: POINT, pet_left: i32, pet_top: i32, pet_size: i32) -> POINT {
    let right = pet_left.saturating_add(pet_size);
    let bottom = pet_top.saturating_add(pet_size);
    POINT {
        x: if cursor.x <= pet_left + pet_size / 2 {
            pet_left
        } else {
            right
        },
        y: if cursor.y <= pet_top + pet_size / 2 {
            pet_top
        } else {
            bottom
        },
    }
}

/// Shows the native menu and sends product-level actions to the host.
pub(crate) fn show(bus: OverlayControlBus, pet_left: i32, pet_top: i32, pet_size: i32) {
    let prefs: UiPreferences = deskhud_ui::persist::load().unwrap_or_default();
    let master = prefs.hud.is_master_enabled();
    let topmost = prefs.shell.topmost;
    let labels = [
        wide(&prefs.t(MessageKey::MenuSettings)),
        wide(&prefs.t(MessageKey::SettingsTopmost)),
        wide(&prefs.t(MessageKey::SettingsNavHud)),
        wide(&prefs.t(MessageKey::MenuHudLayout)),
        wide(&prefs.t(MessageKey::MenuQuit)),
    ];
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    unsafe {
        AppendMenuW(menu, MF_STRING, ID_SETTINGS as usize, labels[0].as_ptr());
        AppendMenuW(menu, MF_STRING, ID_TOPMOST as usize, labels[1].as_ptr());
        CheckMenuItem(menu, ID_TOPMOST, if topmost { MF_CHECKED } else { 0 });
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, ID_MASTER as usize, labels[2].as_ptr());
        CheckMenuItem(menu, ID_MASTER, if master { MF_CHECKED } else { 0 });
        AppendMenuW(menu, MF_STRING, ID_LAYOUT as usize, labels[3].as_ptr());
        EnableMenuItem(
            menu,
            ID_LAYOUT,
            if master { MF_ENABLED } else { MF_DISABLED },
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, ID_QUIT as usize, labels[4].as_ptr());
        let mut cursor = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut cursor);
        let anchor = popup_position(cursor, pet_left, pet_top, pet_size);
        let command = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
            anchor.x,
            anchor.y,
            GetForegroundWindow(),
            std::ptr::null(),
        );
        DestroyMenu(menu);
        match command as u32 {
            ID_SETTINGS => bus.request(OverlayControlCommand::OpenSettings),
            ID_TOPMOST => bus.request(OverlayControlCommand::SetTopmost(!topmost)),
            ID_MASTER => bus.request(OverlayControlCommand::SetHudMaster(!master)),
            ID_LAYOUT if master => bus.request(OverlayControlCommand::OpenHudLayout),
            ID_QUIT => bus.request(OverlayControlCommand::Quit),
            _ => {}
        }
    }
}
