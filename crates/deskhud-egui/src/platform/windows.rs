//! Windows：透明宠窗 chrome、全局键鼠、工作区几何。

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE,
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_DONOTROUND, DWMWCP_ROUND, DWMWCP_ROUNDSMALL, DWM_BB_BLURREGION, DWM_BB_ENABLE,
    DWM_BLURBEHIND,
};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateRectRgn, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW, MonitorFromPoint, ReleaseDC,
    ScreenToClient, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY,
};
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CallWindowProcW, DefWindowProcW, FindWindowW, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, IsWindow,
    SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW, ShowWindow, GWLP_HWNDPARENT, GWLP_WNDPROC,
    GWL_EXSTYLE, GWL_STYLE, HWND_TOP, MSLLHOOKSTRUCT, SM_CXSCREEN, SM_CYSCREEN, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE, WH_MOUSE_LL,
    WM_MOUSEWHEEL, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WS_BORDER, WS_CAPTION, WS_DLGFRAME,
    WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_STATICEDGE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

static SUBCLASS_HWND: AtomicIsize = AtomicIsize::new(0);
static PREV_WNDPROC: AtomicUsize = AtomicUsize::new(0);
/// 右键菜单等弹出窗：与宠窗分开的子类化槽，避免互抢 WndProc。
static POPUP_SUBCLASS_HWND: AtomicIsize = AtomicIsize::new(0);
static POPUP_PREV_WNDPROC: AtomicUsize = AtomicUsize::new(0);
static DWM_APPLIED: AtomicBool = AtomicBool::new(false);
static WHEEL_HOOK: AtomicIsize = AtomicIsize::new(0);
static WHEEL_ACCUM: AtomicI32 = AtomicI32::new(0);

fn pet_wnd_proc_addr() -> usize {
    pet_wnd_proc as *const () as usize
}

fn popup_wnd_proc_addr() -> usize {
    popup_wnd_proc as *const () as usize
}

/// 拦截非客户区绘制：点击获焦时系统会画标题条，这里直接吃掉。
unsafe extern "system" fn pet_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // 整窗都是客户区 → 没有标题栏可画。
        WM_NCCALCSIZE if wparam != 0 => return 0,
        // 激活时不让系统重绘 NC（白条闪现的主因）。
        WM_NCACTIVATE => return 1,
        WM_NCPAINT => return 0,
        _ => {}
    }

    let prev = PREV_WNDPROC.load(Ordering::SeqCst);
    // 防自递归 / 空指针：否则 CallWindowProc 易 STATUS_ACCESS_VIOLATION
    if prev == 0 || prev == pet_wnd_proc_addr() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    type WndProcFn = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
    let prev_fn: WndProcFn = unsafe { std::mem::transmute(prev) };
    unsafe { CallWindowProcW(Some(prev_fn), hwnd, msg, wparam, lparam) }
}

/// 弹出菜单：同样吃掉 NC 白条（与宠窗同因）。
unsafe extern "system" fn popup_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCALCSIZE if wparam != 0 => return 0,
        WM_NCACTIVATE => return 1,
        WM_NCPAINT => return 0,
        _ => {}
    }

    let prev = POPUP_PREV_WNDPROC.load(Ordering::SeqCst);
    if prev == 0 || prev == popup_wnd_proc_addr() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    type WndProcFn = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
    let prev_fn: WndProcFn = unsafe { std::mem::transmute(prev) };
    unsafe { CallWindowProcW(Some(prev_fn), hwnd, msg, wparam, lparam) }
}

fn restore_subclass_if_needed(old_hwnd: isize) {
    if old_hwnd == 0 {
        return;
    }
    let prev = PREV_WNDPROC.swap(0, Ordering::SeqCst);
    if prev == 0 || prev == pet_wnd_proc_addr() {
        return;
    }
    unsafe {
        let current = GetWindowLongPtrW(old_hwnd as HWND, GWLP_WNDPROC) as usize;
        if current == pet_wnd_proc_addr() {
            let _ = SetWindowLongPtrW(old_hwnd as HWND, GWLP_WNDPROC, prev as isize);
        }
    }
}

fn restore_popup_subclass_if_needed(old_hwnd: isize) {
    if old_hwnd == 0 {
        return;
    }
    let prev = POPUP_PREV_WNDPROC.swap(0, Ordering::SeqCst);
    if prev == 0 || prev == popup_wnd_proc_addr() {
        return;
    }
    unsafe {
        let current = GetWindowLongPtrW(old_hwnd as HWND, GWLP_WNDPROC) as usize;
        if current == popup_wnd_proc_addr() {
            let _ = SetWindowLongPtrW(old_hwnd as HWND, GWLP_WNDPROC, prev as isize);
        }
    }
}

fn install_subclass(hwnd: HWND) {
    let key = hwnd as isize;
    if key == 0 {
        return;
    }
    let old = SUBCLASS_HWND.load(Ordering::SeqCst);
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        if old == key && current == pet_wnd_proc_addr() {
            return;
        }
        if old != 0 && old != key {
            restore_subclass_if_needed(old);
            SUBCLASS_HWND.store(0, Ordering::SeqCst);
        }
        if current == pet_wnd_proc_addr() {
            SUBCLASS_HWND.store(key, Ordering::SeqCst);
            return;
        }
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, pet_wnd_proc_addr() as isize) as usize;
        if prev == 0 || prev == pet_wnd_proc_addr() {
            return;
        }
        PREV_WNDPROC.store(prev, Ordering::SeqCst);
        SUBCLASS_HWND.store(key, Ordering::SeqCst);
    }
}

fn install_popup_subclass(hwnd: HWND) {
    let key = hwnd as isize;
    if key == 0 {
        return;
    }
    let old = POPUP_SUBCLASS_HWND.load(Ordering::SeqCst);
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        if old == key && current == popup_wnd_proc_addr() {
            return;
        }
        if old != 0 && old != key {
            restore_popup_subclass_if_needed(old);
            POPUP_SUBCLASS_HWND.store(0, Ordering::SeqCst);
        }
        if current == popup_wnd_proc_addr() {
            POPUP_SUBCLASS_HWND.store(key, Ordering::SeqCst);
            return;
        }
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, popup_wnd_proc_addr() as isize) as usize;
        if prev == 0 || prev == popup_wnd_proc_addr() {
            return;
        }
        POPUP_PREV_WNDPROC.store(prev, Ordering::SeqCst);
        POPUP_SUBCLASS_HWND.store(key, Ordering::SeqCst);
    }
}

fn apply_styles(hwnd: HWND) -> bool {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let desired = (style
            & !(WS_CAPTION as isize
                | WS_THICKFRAME as isize
                | WS_BORDER as isize
                | WS_DLGFRAME as isize
                | WS_SYSMENU as isize
                | WS_MINIMIZEBOX as isize
                | WS_MAXIMIZEBOX as isize))
            | WS_POPUP as isize
            | WS_VISIBLE as isize;

        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        // 注意：不要加 WS_EX_NOACTIVATE，否则 StartDrag / 系统拖窗会失效。
        let mut ex_desired = ex
            & !(WS_EX_CLIENTEDGE as isize
                | WS_EX_WINDOWEDGE as isize
                | WS_EX_DLGMODALFRAME as isize
                | WS_EX_STATICEDGE as isize
                | WS_EX_APPWINDOW as isize
                | WS_EX_NOACTIVATE as isize);
        ex_desired |= WS_EX_TOOLWINDOW as isize;

        let mut changed = false;
        if desired != style {
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, desired);
            changed = true;
        }
        if ex_desired != ex {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_desired);
            changed = true;
        }
        if changed {
            let _ = SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        changed
    }
}

fn apply_pet_dwm(hwnd: HWND) {
    unsafe {
        let zero = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 0,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &zero);

        // 必须可重复：子窗若误把 Mica 打到宠窗，下一帧要清掉。
        let backdrop: i32 = DWMSBT_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            &backdrop as *const _ as *const _,
            std::mem::size_of_val(&backdrop) as u32,
        );

        let none: u32 = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &none as *const _ as *const _,
            std::mem::size_of_val(&none) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &none as *const _ as *const _,
            std::mem::size_of_val(&none) as u32,
        );

        let corner: u32 = DWMWCP_DONOTROUND as u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );

        if !DWM_APPLIED.swap(true, Ordering::SeqCst) {
            let hrgn = CreateRectRgn(0, 0, -1, -1);
            if !hrgn.is_null() {
                let bb = DWM_BLURBEHIND {
                    dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                    fEnable: 1,
                    hRgnBlur: hrgn,
                    fTransitionOnMaximized: 0,
                };
                let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
                let _ = DeleteObject(hrgn);
            }
        }
    }
}

/// 安装/维持宠窗无标题栏合成（可重复调用；风格未变时不会 FRAMECHANGED）。
pub fn ensure_pet_chrome(hwnd: isize) {
    let hwnd = hwnd as HWND;
    apply_styles(hwnd);
    install_subclass(hwnd);
    apply_pet_dwm(hwnd);
}

/// 弹出菜单 chrome：去标题栏 + 关 Acrylic + NC 拦截（与宠窗顶白线同因）。
///
/// 可重复调用；勿用 Acrylic——暗色下易留一条浅顶边，菜单已由 egui 铺不透明底。
pub fn ensure_acrylic_popup(hwnd: isize, pet_hwnd: Option<isize>, dark: bool) {
    if hwnd == 0 || pet_hwnd == Some(hwnd) {
        return;
    }
    let hwnd = hwnd as HWND;
    // 仍带系统标题栏的窗口（设置页）禁止剥 chrome
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if (style & WS_CAPTION as isize) != 0 {
            return;
        }
    }
    apply_styles(hwnd);
    install_popup_subclass(hwnd);
    // DWMSBT_NONE：避免 Acrylic/Mica 在暗色主题画出浅色顶边
    apply_backdrop(hwnd, DWMSBT_NONE, DWMWCP_ROUNDSMALL as u32);
    let use_dark: BOOL = if dark { TRUE } else { 0 };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &use_dark as *const _ as *const _,
            std::mem::size_of_val(&use_dark) as u32,
        );
    }
}

/// 菜单关闭时还原弹出窗子类化（可选；下次打开会重装）。
pub fn release_popup_chrome(hwnd: Option<isize>) {
    let Some(h) = hwnd else {
        return;
    };
    if POPUP_SUBCLASS_HWND.load(Ordering::SeqCst) == h {
        restore_popup_subclass_if_needed(h);
        POPUP_SUBCLASS_HWND.store(0, Ordering::SeqCst);
    }
}

/// 设置窗：恢复系统标题栏/可调边框（若曾被菜单 chrome 误剥）。
pub fn ensure_settings_chrome(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    // 误装在设置窗上的弹出子类化必须先卸掉
    if POPUP_SUBCLASS_HWND.load(Ordering::SeqCst) == hwnd {
        restore_popup_subclass_if_needed(hwnd);
        POPUP_SUBCLASS_HWND.store(0, Ordering::SeqCst);
    }
    let hwnd = hwnd as HWND;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let has_caption = (style & WS_CAPTION as isize) != 0;
        let is_popup = (style & WS_POPUP as isize) != 0;
        let desired = (style & !(WS_POPUP as isize))
            | WS_CAPTION as isize
            | WS_SYSMENU as isize
            | WS_THICKFRAME as isize
            | WS_MINIMIZEBOX as isize
            | WS_MAXIMIZEBOX as isize
            | WS_VISIBLE as isize;

        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let mut ex_desired = ex & !(WS_EX_TOOLWINDOW as isize);
        ex_desired |= WS_EX_APPWINDOW as isize;

        let mut changed = false;
        if !has_caption || is_popup || desired != style {
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, desired);
            changed = true;
        }
        if ex_desired != ex {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_desired);
            changed = true;
        }
        if changed {
            let _ = SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

/// 设置类窗口：Mica + 系统圆角。`pet_hwnd` 用于拒绝误绑到宠窗。
/// 当前设置窗未使用（Glow + 每帧 resize 曾导致卡死）；保留备选。
#[allow(dead_code)]
pub fn ensure_mica_window(hwnd: isize, pet_hwnd: Option<isize>) {
    if pet_hwnd == Some(hwnd) {
        return;
    }
    let hwnd = hwnd as HWND;
    apply_backdrop(hwnd, DWMSBT_MAINWINDOW, DWMWCP_ROUND as u32);
}

fn apply_backdrop(hwnd: HWND, backdrop: i32, corner: u32) {
    unsafe {
        let zero = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 0,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &zero);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            &backdrop as *const _ as *const _,
            std::mem::size_of_val(&backdrop) as u32,
        );
        let none: u32 = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &none as *const _ as *const _,
            std::mem::size_of_val(&none) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &none as *const _ as *const _,
            std::mem::size_of_val(&none) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );
    }
}

pub fn foreground_hwnd() -> Option<isize> {
    unsafe {
        let h = GetForegroundWindow();
        if h.is_null() {
            None
        } else {
            Some(h as isize)
        }
    }
}

/// 前台既不是宠窗也不是菜单 → 视为点到了别处。
pub fn foreground_is_outside(pet_hwnd: Option<isize>, menu_hwnd: Option<isize>) -> bool {
    let Some(fg) = foreground_hwnd() else {
        return false;
    };
    if pet_hwnd == Some(fg) || menu_hwnd == Some(fg) {
        return false;
    }
    true
}

pub fn cursor_screen_px() -> Option<(i32, i32)> {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return None;
        }
        Some((pt.x, pt.y))
    }
}

/// 桌面全局鼠标键是否按下（物理采样，不要求指针在宠上）。
pub fn global_mouse_buttons() -> (bool, bool, bool) {
    unsafe {
        let down = |vk: i32| (GetAsyncKeyState(vk) as u16 & 0x8000) != 0;
        (
            down(VK_LBUTTON as i32),
            down(VK_RBUTTON as i32),
            down(VK_MBUTTON as i32),
        )
    }
}

/// 全局修饰键（Shift / Ctrl / Alt）。
pub fn global_modifiers() -> (bool, bool, bool) {
    unsafe {
        let down = |vk: i32| (GetAsyncKeyState(vk) as u16 & 0x8000) != 0;
        (down(0x10), down(0x11), down(0x12)) // VK_SHIFT / CONTROL / MENU
    }
}

/// 指定虚拟键是否按下（全局采样）。
pub fn global_key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

unsafe extern "system" fn mouse_ll_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && wparam == WM_MOUSEWHEEL as usize {
            let info = &*(lparam as *const MSLLHOOKSTRUCT);
            let delta = ((info.mouseData >> 16) & 0xFFFF) as u16 as i16;
            WHEEL_ACCUM.fetch_add(delta as i32, Ordering::Relaxed);
        }
        let hook = WHEEL_HOOK.load(Ordering::SeqCst) as *mut core::ffi::c_void;
        CallNextHookEx(hook, code, wparam, lparam)
    }
}

fn ensure_wheel_hook() {
    if WHEEL_HOOK.load(Ordering::SeqCst) != 0 {
        return;
    }
    unsafe {
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_ll_proc), std::ptr::null_mut(), 0);
        if !hook.is_null() {
            WHEEL_HOOK.store(hook as isize, Ordering::SeqCst);
        }
    }
}

/// 取出并清零累计的全局滚轮 delta（Windows 约定：+120 约一格向上）。
pub fn take_wheel_delta() -> i32 {
    ensure_wheel_hook();
    WHEEL_ACCUM.swap(0, Ordering::Relaxed)
}

pub fn cursor_client_px(hwnd: isize) -> Option<(i32, i32)> {
    let hwnd = hwnd as HWND;
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return None;
        }
        if ScreenToClient(hwnd, &mut pt) == 0 {
            return None;
        }
        Some((pt.x, pt.y))
    }
}

/// 窗口屏幕矩形（物理像素：left, top, right, bottom）。
pub fn window_screen_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    let hwnd = hwnd as HWND;
    unsafe {
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rc) == 0 {
            return None;
        }
        Some((rc.left, rc.top, rc.right, rc.bottom))
    }
}

/// 窗口左上角屏幕坐标（物理像素）。
pub fn window_screen_pos(hwnd: isize) -> Option<(i32, i32)> {
    window_screen_rect(hwnd).map(|(l, t, _, _)| (l, t))
}

/// 移动窗口（不改大小/Z 序）。用于手动拖宠——`StartDrag` 依赖 HTCAPTION，与无 NC 冲突。
pub fn move_window_screen(hwnd: isize, x: i32, y: i32) {
    let hwnd = hwnd as HWND;
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// 光标所在显示器工作区（物理像素，不含任务栏）。
pub fn work_area_containing_px(x: i32, y: i32) -> (i32, i32, i32, i32) {
    unsafe {
        let pt = POINT { x, y };
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if !mon.is_null() {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                rcMonitor: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                rcWork: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                dwFlags: 0,
            };
            if GetMonitorInfoW(mon, &mut info) != 0 {
                let r = info.rcWork;
                return (r.left, r.top, r.right, r.bottom);
            }
        }
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        (0, 0, w, h)
    }
}

/// 按工作区夹紧弹出菜单左上角（逻辑像素）。
///
/// 优先在光标右下；若越界则翻到左/上，再夹到工作区内。
pub fn fit_popup_pos_points(
    cursor_points: (f32, f32),
    menu_w: f32,
    menu_h: f32,
    ppp: f32,
) -> (f32, f32) {
    let ppp = ppp.max(0.01);
    let cx = (cursor_points.0 * ppp).round() as i32;
    let cy = (cursor_points.1 * ppp).round() as i32;
    let (wl, wt, wr, wb) = work_area_containing_px(cx, cy);
    let mw = (menu_w * ppp).ceil() as i32;
    let mh = (menu_h * ppp).ceil() as i32;
    let margin = (4.0 * ppp).round() as i32;
    let gap = (2.0 * ppp).round() as i32;

    let mut x = cx + gap;
    let mut y = cy + gap;
    if x + mw > wr - margin {
        x = cx - mw - gap;
    }
    if y + mh > wb - margin {
        y = cy - mh - gap;
    }
    let max_x = (wr - margin - mw).max(wl + margin);
    let max_y = (wb - margin - mh).max(wt + margin);
    x = x.clamp(wl + margin, max_x);
    y = y.clamp(wt + margin, max_y);
    (x as f32 / ppp, y as f32 / ppp)
}

/// 一台显示器的屏幕像素几何。
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// 稳定标识；主屏为 `primary`，其它为 `x_y_w_h`。
    pub id: String,
    /// 左。
    pub x: i32,
    /// 上。
    pub y: i32,
    /// 宽。
    pub width: i32,
    /// 高。
    pub height: i32,
    /// 是否主显示器。
    pub primary: bool,
    /// 工作区左（`rcWork.left`，物理像素，已扣任务栏）。
    pub work_left: i32,
    /// 工作区上。
    pub work_top: i32,
    /// 工作区右。
    pub work_right: i32,
    /// 工作区下。
    pub work_bottom: i32,
}

struct EnumState {
    out: Vec<DisplayInfo>,
}

unsafe extern "system" fn enum_mon_proc(
    hmon: windows_sys::Win32::Graphics::Gdi::HMONITOR,
    _hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    unsafe {
        let state = &mut *(lparam as *mut EnumState);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        if GetMonitorInfoW(hmon, &mut info) == 0 {
            return TRUE;
        }
        let x = info.rcMonitor.left;
        let y = info.rcMonitor.top;
        let width = info.rcMonitor.right - info.rcMonitor.left;
        let height = info.rcMonitor.bottom - info.rcMonitor.top;
        let primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY
        let id = if primary {
            "primary".to_string()
        } else {
            format!("{x}_{y}_{width}_{height}")
        };
        state.out.push(DisplayInfo {
            id,
            x,
            y,
            width,
            height,
            primary,
            work_left: info.rcWork.left,
            work_top: info.rcWork.top,
            work_right: info.rcWork.right,
            work_bottom: info.rcWork.bottom,
        });
        TRUE
    }
}

/// 捕获指定屏幕矩形为 RGBA（物理像素）。失败返回 `None`。
pub fn capture_screen_rgba(x: i32, y: i32, width: i32, height: i32) -> Option<(u32, u32, Vec<u8>)> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let w = width as u32;
    let h = height as u32;
    unsafe {
        let hdc_screen = GetDC(std::ptr::null_mut());
        if hdc_screen.is_null() {
            return None;
        }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() {
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return None;
        }
        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbmp.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return None;
        }
        let old = SelectObject(hdc_mem, hbmp as HGDIOBJ);
        let ok = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, x, y, SRCCOPY);
        if ok == 0 {
            SelectObject(hdc_mem, old);
            DeleteObject(hbmp as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return None;
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [std::mem::zeroed()],
        };
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        let got = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            h,
            bgra.as_mut_ptr() as *mut _,
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(hdc_mem, old);
        DeleteObject(hbmp as HGDIOBJ);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);
        if got == 0 {
            return None;
        }
        // BGRA → RGBA
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        Some((w, h, bgra))
    }
}

/// 枚举所有显示器（屏幕像素）。
pub fn list_displays() -> Vec<DisplayInfo> {
    let mut state = EnumState { out: Vec::new() };
    unsafe {
        let _ = EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_mon_proc),
            &mut state as *mut _ as LPARAM,
        );
    }
    if state.out.is_empty() {
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        state.out.push(DisplayInfo {
            id: "primary".into(),
            x: 0,
            y: 0,
            width: w,
            height: h,
            primary: true,
            work_left: 0,
            work_top: 0,
            work_right: w,
            work_bottom: h,
        });
    }
    state.out
}

/// 切换点击穿透（布局编辑关时 HUD 层应穿透）。
pub fn set_click_through(hwnd: isize, enabled: bool) {
    set_click_through_inner(hwnd, enabled, false);
}

/// 强制刷新命中测试（即使 EXSTYLE 未变也刷 FRAMECHANGED）。
#[allow(dead_code)]
pub fn force_click_through(hwnd: isize, enabled: bool) {
    set_click_through_inner(hwnd, enabled, true);
}

fn set_click_through_inner(hwnd: isize, enabled: bool, force: bool) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let hwnd = hwnd as HWND;
        if IsWindow(hwnd) == 0 {
            return;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let mut desired = ex;
        if enabled {
            desired |= WS_EX_LAYERED as isize | WS_EX_TRANSPARENT as isize;
        } else {
            desired &= !(WS_EX_TRANSPARENT as isize);
            desired |= WS_EX_LAYERED as isize;
        }
        if desired != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
        } else if !force {
            return;
        }
        // 不刷 FRAMECHANGED 时，部分环境下清掉 TRANSPARENT 不生效 → 宠窗假死（点不透/无右键）
        // 置顶改 z-order 后也需要强制再刷一次。
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// 显示/隐藏原生窗（编辑模式立刻藏掉残留 HUD 小窗）。
pub fn set_window_visible(hwnd: isize, visible: bool) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindow(
            hwnd as HWND,
            if visible { SW_SHOWNOACTIVATE } else { SW_HIDE },
        );
    }
}

/// 按窗口标题查找 HWND（用于 HUD overlay）。
pub fn find_window_by_title(title: &str) -> Option<isize> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = std::ffi::OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd as isize)
    }
}

/// 窗口叠放铁律：
///
/// - **全局置顶**只跟 `prefs.shell.topmost`：宠 / HUD / 设置 / 菜单同一 `WindowLevel`。
/// - 禁止「宠置顶 + 设置普通」之类的混用（Windows 下会逼出点击穿透，并易与多窗 z-order 卡死）。
/// - 开设置时靠同层 + Focus，不再给宠窗开 click-through。
pub fn set_window_owner(window: isize, owner: Option<isize>) {
    if window == 0 {
        return;
    }
    unsafe {
        let hwnd = window as HWND;
        if IsWindow(hwnd) == 0 {
            return;
        }
        let owner_hwnd = owner.unwrap_or(0) as isize;
        let current = GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT);
        // 已是目标 owner 时勿再 SetWindowPos，否则设置窗每帧 HWND_TOP 易与 HUD 抢 z-order
        if current == owner_hwnd {
            return;
        }
        let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner_hwnd);
        // 勿 SWP_SHOWWINDOW：关窗途中再 Show 会打乱视口生命周期，易 AV
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}
