//! Windows-native context menu for the pet overlay.

use deskhud_ui::{MessageKey, UiPreferences};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Foundation::SIZE;
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, GetStockObject, GetTextExtentPoint32W, SYSTEM_FONT, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreatePopupMenu, DestroyMenu, EnableMenuItem, GetCursorPos,
    GetForegroundWindow, GetMenuItemCount, GetSystemMetrics, MF_CHECKED, MF_DISABLED, MF_ENABLED,
    MF_SEPARATOR, MF_STRING, SM_CXMENUCHECK, SM_CXSCREEN, SM_CYMENU, SM_CYSCREEN, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_RIGHTBUTTON, TPM_TOPALIGN,
    TrackPopupMenuEx,
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

/// 主显示器工作区尺寸（逻辑像素），用于锚点避让。
fn work_area_size() -> (i32, i32) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN).max(1);
        let h = GetSystemMetrics(SM_CYSCREEN).max(1);
        (w, h)
    }
}

/// 基于系统菜单字体与度量估算弹出菜单尺寸（计入勾选/边距）。
fn measure_menu(menu: *mut core::ffi::c_void, labels: &[Vec<u16>]) -> (i32, i32) {
    unsafe {
        let count = GetMenuItemCount(menu).max(0);
        let hdc = CreateCompatibleDC(std::ptr::null_mut());
        let _old = SelectObject(hdc, GetStockObject(SYSTEM_FONT));
        let mut max_w = 0i32;
        let mut item_h = GetSystemMetrics(SM_CYMENU);
        if item_h <= 0 {
            item_h = 20;
        }
        for text in labels {
            let mut sz = SIZE { cx: 0, cy: 0 };
            if GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut sz) != 0 {
                max_w = max_w.max(sz.cx);
            }
        }
        let _ = DeleteDC(hdc);
        let check = GetSystemMetrics(SM_CXMENUCHECK).max(0);
        let width = max_w + check + 32;
        let height = count * item_h;
        (width.max(1), height.max(1))
    }
}

/// 默认让**左上角对齐右击点**；靠右→右边缘对齐、靠下→下边缘对齐，并始终限制在主工作区内。
fn popup_position(
    cursor: POINT,
    _pet_left: i32,
    _pet_top: i32,
    _pet_size: i32,
    menu_w: i32,
    menu_h: i32,
) -> (POINT, u32) {
    let (sw, sh) = work_area_size();
    let right = cursor.x + menu_w > sw;
    let bottom = cursor.y + menu_h > sh;
    let anchor = POINT {
        x: if right {
            cursor.x.clamp(menu_w.max(0), sw)
        } else {
            cursor.x.clamp(0, (sw - menu_w).max(0))
        },
        y: if bottom {
            cursor.y.clamp(menu_h.max(0), sh)
        } else {
            cursor.y.clamp(0, (sh - menu_h).max(0))
        },
    };
    let align = (if right { TPM_RIGHTALIGN } else { TPM_LEFTALIGN })
        | (if bottom {
            TPM_BOTTOMALIGN
        } else {
            TPM_TOPALIGN
        });
    (anchor, align)
}

/// Shows the native menu and sends product-level actions to the host.
pub(crate) fn show(bus: OverlayControlBus, pet_left: i32, pet_top: i32, pet_size: i32) {
    let prefs: UiPreferences = deskhud_ui::persist::load().unwrap_or_default();
    let master = prefs.hud.is_master_enabled();
    let topmost = prefs.shell.topmost;
    let labels = [
        wide(prefs.t(MessageKey::MenuSettings)),
        wide(prefs.t(MessageKey::SettingsTopmost)),
        wide(prefs.t(MessageKey::SettingsNavHud)),
        wide(prefs.t(MessageKey::MenuHudLayout)),
        wide(prefs.t(MessageKey::MenuQuit)),
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
        let (menu_w, menu_h) = measure_menu(menu, &labels);
        let (anchor, align) = popup_position(cursor, pet_left, pet_top, pet_size, menu_w, menu_h);
        let command = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | align | TPM_RIGHTBUTTON,
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
