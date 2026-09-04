//! 平台级全局输入快照；仅由 UI 壳读取并转换为引擎中性状态。
#![allow(clippy::needless_return)]

#[cfg(target_os = "macos")]
use deskhud_engine::{PetEvent, PetKey, PetModifiers};

#[cfg(not(target_os = "macos"))]
use deskhud_engine::PetKey;

/// 将获得焦点窗口的物理键码映射为宠物契约支持的完整标准键盘集合。
#[allow(dead_code)]
pub(crate) fn winit_key_to_pet_key(key: winit::keyboard::KeyCode) -> Option<PetKey> {
    use winit::keyboard::KeyCode as Key;
    Some(match key {
        Key::Escape => PetKey::Escape,
        Key::Tab => PetKey::Tab,
        Key::Enter => PetKey::Enter,
        Key::Space => PetKey::Space,
        Key::Backspace => PetKey::Backspace,
        Key::Delete => PetKey::Delete,
        Key::Insert => PetKey::Insert,
        Key::ArrowUp => PetKey::ArrowUp,
        Key::ArrowDown => PetKey::ArrowDown,
        Key::ArrowLeft => PetKey::ArrowLeft,
        Key::ArrowRight => PetKey::ArrowRight,
        Key::Home => PetKey::Home,
        Key::End => PetKey::End,
        Key::PageUp => PetKey::PageUp,
        Key::PageDown => PetKey::PageDown,
        Key::PrintScreen => PetKey::PrintScreen,
        Key::ScrollLock => PetKey::ScrollLock,
        Key::Pause => PetKey::Pause,
        Key::ContextMenu => PetKey::ContextMenu,
        Key::ShiftLeft | Key::ShiftRight => PetKey::Shift,
        Key::ControlLeft | Key::ControlRight => PetKey::Ctrl,
        Key::AltLeft | Key::AltRight => PetKey::Alt,
        Key::SuperLeft | Key::SuperRight => PetKey::Super,
        Key::CapsLock => PetKey::CapsLock,
        Key::NumLock => PetKey::NumLock,
        Key::NumpadClear => PetKey::Clear,
        Key::NumpadEnter => PetKey::NumpadEnter,
        Key::Numpad0 => PetKey::NumpadDigit(0),
        Key::Numpad1 => PetKey::NumpadDigit(1),
        Key::Numpad2 => PetKey::NumpadDigit(2),
        Key::Numpad3 => PetKey::NumpadDigit(3),
        Key::Numpad4 => PetKey::NumpadDigit(4),
        Key::Numpad5 => PetKey::NumpadDigit(5),
        Key::Numpad6 => PetKey::NumpadDigit(6),
        Key::Numpad7 => PetKey::NumpadDigit(7),
        Key::Numpad8 => PetKey::NumpadDigit(8),
        Key::Numpad9 => PetKey::NumpadDigit(9),
        Key::NumpadAdd => PetKey::NumpadAdd,
        Key::NumpadSubtract => PetKey::NumpadSubtract,
        Key::NumpadMultiply => PetKey::NumpadMultiply,
        Key::NumpadDivide => PetKey::NumpadDivide,
        Key::NumpadDecimal => PetKey::NumpadDecimal,
        Key::NumpadComma => PetKey::NumpadSeparator,
        Key::F1 => PetKey::Function(1),
        Key::F2 => PetKey::Function(2),
        Key::F3 => PetKey::Function(3),
        Key::F4 => PetKey::Function(4),
        Key::F5 => PetKey::Function(5),
        Key::F6 => PetKey::Function(6),
        Key::F7 => PetKey::Function(7),
        Key::F8 => PetKey::Function(8),
        Key::F9 => PetKey::Function(9),
        Key::F10 => PetKey::Function(10),
        Key::F11 => PetKey::Function(11),
        Key::F12 => PetKey::Function(12),
        Key::F13 => PetKey::Function(13),
        Key::F14 => PetKey::Function(14),
        Key::F15 => PetKey::Function(15),
        Key::F16 => PetKey::Function(16),
        Key::F17 => PetKey::Function(17),
        Key::F18 => PetKey::Function(18),
        Key::F19 => PetKey::Function(19),
        Key::F20 => PetKey::Function(20),
        Key::F21 => PetKey::Function(21),
        Key::F22 => PetKey::Function(22),
        Key::F23 => PetKey::Function(23),
        Key::F24 => PetKey::Function(24),
        Key::KeyA => PetKey::Letter('A'),
        Key::KeyB => PetKey::Letter('B'),
        Key::KeyC => PetKey::Letter('C'),
        Key::KeyD => PetKey::Letter('D'),
        Key::KeyE => PetKey::Letter('E'),
        Key::KeyF => PetKey::Letter('F'),
        Key::KeyG => PetKey::Letter('G'),
        Key::KeyH => PetKey::Letter('H'),
        Key::KeyI => PetKey::Letter('I'),
        Key::KeyJ => PetKey::Letter('J'),
        Key::KeyK => PetKey::Letter('K'),
        Key::KeyL => PetKey::Letter('L'),
        Key::KeyM => PetKey::Letter('M'),
        Key::KeyN => PetKey::Letter('N'),
        Key::KeyO => PetKey::Letter('O'),
        Key::KeyP => PetKey::Letter('P'),
        Key::KeyQ => PetKey::Letter('Q'),
        Key::KeyR => PetKey::Letter('R'),
        Key::KeyS => PetKey::Letter('S'),
        Key::KeyT => PetKey::Letter('T'),
        Key::KeyU => PetKey::Letter('U'),
        Key::KeyV => PetKey::Letter('V'),
        Key::KeyW => PetKey::Letter('W'),
        Key::KeyX => PetKey::Letter('X'),
        Key::KeyY => PetKey::Letter('Y'),
        Key::KeyZ => PetKey::Letter('Z'),
        Key::Digit0 => PetKey::Digit('0'),
        Key::Digit1 => PetKey::Digit('1'),
        Key::Digit2 => PetKey::Digit('2'),
        Key::Digit3 => PetKey::Digit('3'),
        Key::Digit4 => PetKey::Digit('4'),
        Key::Digit5 => PetKey::Digit('5'),
        Key::Digit6 => PetKey::Digit('6'),
        Key::Digit7 => PetKey::Digit('7'),
        Key::Digit8 => PetKey::Digit('8'),
        Key::Digit9 => PetKey::Digit('9'),
        Key::Backquote => PetKey::Punct('`'),
        Key::Minus => PetKey::Punct('-'),
        Key::Equal => PetKey::Punct('='),
        Key::BracketLeft => PetKey::Punct('['),
        Key::BracketRight => PetKey::Punct(']'),
        Key::Backslash | Key::IntlBackslash | Key::IntlRo => PetKey::Punct('\\'),
        Key::Semicolon => PetKey::Punct(';'),
        Key::Quote => PetKey::Punct('\''),
        Key::Comma => PetKey::Punct(','),
        Key::Period => PetKey::Punct('.'),
        Key::Slash => PetKey::Punct('/'),
        _ => return None,
    })
}

#[derive(Clone, Copy, Default)]
pub(crate) struct GlobalMouseButtons {
    pub(crate) primary_down: bool,
    pub(crate) secondary_down: bool,
    pub(crate) middle_down: bool,
}

#[cfg(target_os = "macos")]
pub(crate) fn global_mouse_buttons() -> GlobalMouseButtons {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceButtonState(state_id: u32, button: i32) -> bool;
    }
    // kCGEventSourceStateCombinedSessionState; button values match CGMouseButton.
    unsafe {
        GlobalMouseButtons {
            primary_down: CGEventSourceButtonState(0, 0),
            secondary_down: CGEventSourceButtonState(0, 1),
            middle_down: CGEventSourceButtonState(0, 2),
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn global_pointer_position() -> Option<[f64; 2]> {
    #[repr(C)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreate(source: *const core::ffi::c_void) -> *mut core::ffi::c_void;
        fn CGEventGetLocation(event: *const core::ffi::c_void) -> Point;
        fn CFRelease(value: *const core::ffi::c_void);
    }
    unsafe {
        let event = CGEventCreate(core::ptr::null());
        if event.is_null() {
            return None;
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        Some([point.x, point.y])
    }
}

/// macOS 全局输入监听。系统要求用户在「辅助功能」中授予本应用权限；监听器
/// 为 listen-only，不会拦截或修改任何键鼠事件。
#[cfg(target_os = "macos")]
pub(crate) struct GlobalKeyMonitor {
    _tap: core_graphics::event::CGEventTap<'static>,
    _source: core_foundation::runloop::CFRunLoopSource,
}

#[cfg(target_os = "macos")]
pub(crate) fn install_global_key_monitor(
    proxy: winit::event_loop::EventLoopProxy<crate::runtime::viewport::UserEvent>,
    keyboard: bool,
    mouse: bool,
) -> Option<GlobalKeyMonitor> {
    use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventType, CallbackResult, EventField,
    };

    let mut events = Vec::new();
    if keyboard {
        events.extend([
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ]);
    }
    if mouse {
        events.push(CGEventType::ScrollWheel);
    }
    if events.is_empty() {
        return None;
    }
    let tap = unsafe {
        CGEventTap::new_unchecked(
            CGEventTapLocation::Session,
            core_graphics::event::CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            events,
            move |_proxy, event_type, event| {
                let modifiers = mac_modifiers(event.get_flags());
                if mouse && matches!(event_type, CGEventType::ScrollWheel) {
                    let delta = event
                        .get_integer_value_field(EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1)
                        .clamp(i8::MIN as i64, i8::MAX as i64)
                        as i8;
                    if delta != 0 {
                        let _ = proxy.send_event(crate::runtime::viewport::UserEvent::PetEvent(
                            PetEvent::GlobalMouseWheel { delta, modifiers },
                        ));
                    }
                    return CallbackResult::Keep;
                }
                if !keyboard {
                    return CallbackResult::Keep;
                }
                let keycode =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                if let Some(key) = mac_key(keycode) {
                    let pressed = match event_type {
                        CGEventType::KeyDown => true,
                        CGEventType::KeyUp => false,
                        // macOS 的修饰键不保证发送 KeyDown/KeyUp，改以当前 flags
                        // 中对应位判断其新状态。
                        CGEventType::FlagsChanged => match key {
                            PetKey::Shift => modifiers.shift,
                            PetKey::Ctrl => modifiers.ctrl,
                            PetKey::Alt => modifiers.alt,
                            PetKey::Super => modifiers.meta,
                            _ => return CallbackResult::Keep,
                        },
                        _ => return CallbackResult::Keep,
                    };
                    let event = if pressed {
                        PetEvent::GlobalKeyPressed { key, modifiers }
                    } else {
                        PetEvent::GlobalKeyReleased { key, modifiers }
                    };
                    let _ = proxy.send_event(crate::runtime::viewport::UserEvent::PetEvent(event));
                }
                CallbackResult::Keep
            },
        )
        .ok()?
    };
    let source = tap.mach_port().create_runloop_source(0).ok()?;
    CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
    Some(GlobalKeyMonitor {
        _tap: tap,
        _source: source,
    })
}

#[cfg(target_os = "macos")]
fn mac_modifiers(flags: core_graphics::event::CGEventFlags) -> PetModifiers {
    use core_graphics::event::CGEventFlags;
    PetModifiers {
        shift: flags.contains(CGEventFlags::CGEventFlagShift),
        ctrl: flags.contains(CGEventFlags::CGEventFlagControl),
        alt: flags.contains(CGEventFlags::CGEventFlagAlternate),
        meta: flags.contains(CGEventFlags::CGEventFlagCommand),
    }
}

#[cfg(target_os = "macos")]
fn mac_key(code: u16) -> Option<PetKey> {
    use core_graphics::event::KeyCode;
    Some(match code {
        KeyCode::ANSI_A => PetKey::Letter('A'),
        KeyCode::ANSI_B => PetKey::Letter('B'),
        KeyCode::ANSI_C => PetKey::Letter('C'),
        KeyCode::ANSI_D => PetKey::Letter('D'),
        KeyCode::ANSI_E => PetKey::Letter('E'),
        KeyCode::ANSI_F => PetKey::Letter('F'),
        KeyCode::ANSI_G => PetKey::Letter('G'),
        KeyCode::ANSI_H => PetKey::Letter('H'),
        KeyCode::ANSI_I => PetKey::Letter('I'),
        KeyCode::ANSI_J => PetKey::Letter('J'),
        KeyCode::ANSI_K => PetKey::Letter('K'),
        KeyCode::ANSI_L => PetKey::Letter('L'),
        KeyCode::ANSI_M => PetKey::Letter('M'),
        KeyCode::ANSI_N => PetKey::Letter('N'),
        KeyCode::ANSI_O => PetKey::Letter('O'),
        KeyCode::ANSI_P => PetKey::Letter('P'),
        KeyCode::ANSI_Q => PetKey::Letter('Q'),
        KeyCode::ANSI_R => PetKey::Letter('R'),
        KeyCode::ANSI_S => PetKey::Letter('S'),
        KeyCode::ANSI_T => PetKey::Letter('T'),
        KeyCode::ANSI_U => PetKey::Letter('U'),
        KeyCode::ANSI_V => PetKey::Letter('V'),
        KeyCode::ANSI_W => PetKey::Letter('W'),
        KeyCode::ANSI_X => PetKey::Letter('X'),
        KeyCode::ANSI_Y => PetKey::Letter('Y'),
        KeyCode::ANSI_Z => PetKey::Letter('Z'),
        KeyCode::ANSI_0 => PetKey::Digit('0'),
        KeyCode::ANSI_1 => PetKey::Digit('1'),
        KeyCode::ANSI_2 => PetKey::Digit('2'),
        KeyCode::ANSI_3 => PetKey::Digit('3'),
        KeyCode::ANSI_4 => PetKey::Digit('4'),
        KeyCode::ANSI_5 => PetKey::Digit('5'),
        KeyCode::ANSI_6 => PetKey::Digit('6'),
        KeyCode::ANSI_7 => PetKey::Digit('7'),
        KeyCode::ANSI_8 => PetKey::Digit('8'),
        KeyCode::ANSI_9 => PetKey::Digit('9'),
        KeyCode::ANSI_GRAVE => PetKey::Punct('`'),
        KeyCode::ANSI_MINUS => PetKey::Punct('-'),
        KeyCode::ANSI_EQUAL => PetKey::Punct('='),
        KeyCode::ANSI_LEFT_BRACKET => PetKey::Punct('['),
        KeyCode::ANSI_RIGHT_BRACKET => PetKey::Punct(']'),
        KeyCode::ANSI_BACKSLASH => PetKey::Punct('\\'),
        KeyCode::ANSI_SEMICOLON => PetKey::Punct(';'),
        KeyCode::ANSI_QUOTE => PetKey::Punct('\''),
        KeyCode::ANSI_COMMA => PetKey::Punct(','),
        KeyCode::ANSI_PERIOD => PetKey::Punct('.'),
        KeyCode::ANSI_SLASH => PetKey::Punct('/'),
        KeyCode::RETURN => PetKey::Enter,
        KeyCode::TAB => PetKey::Tab,
        KeyCode::SPACE => PetKey::Space,
        KeyCode::DELETE => PetKey::Backspace,
        KeyCode::FORWARD_DELETE => PetKey::Delete,
        KeyCode::ESCAPE => PetKey::Escape,
        KeyCode::HOME => PetKey::Home,
        KeyCode::END => PetKey::End,
        KeyCode::PAGE_UP => PetKey::PageUp,
        KeyCode::PAGE_DOWN => PetKey::PageDown,
        KeyCode::CAPS_LOCK => PetKey::CapsLock,
        KeyCode::ANSI_KEYPAD_CLEAR => PetKey::Clear,
        KeyCode::ANSI_KEYPAD_ENTER => PetKey::NumpadEnter,
        KeyCode::ANSI_KEYPAD_0 => PetKey::NumpadDigit(0),
        KeyCode::ANSI_KEYPAD_1 => PetKey::NumpadDigit(1),
        KeyCode::ANSI_KEYPAD_2 => PetKey::NumpadDigit(2),
        KeyCode::ANSI_KEYPAD_3 => PetKey::NumpadDigit(3),
        KeyCode::ANSI_KEYPAD_4 => PetKey::NumpadDigit(4),
        KeyCode::ANSI_KEYPAD_5 => PetKey::NumpadDigit(5),
        KeyCode::ANSI_KEYPAD_6 => PetKey::NumpadDigit(6),
        KeyCode::ANSI_KEYPAD_7 => PetKey::NumpadDigit(7),
        KeyCode::ANSI_KEYPAD_8 => PetKey::NumpadDigit(8),
        KeyCode::ANSI_KEYPAD_9 => PetKey::NumpadDigit(9),
        KeyCode::ANSI_KEYPAD_PLUS => PetKey::NumpadAdd,
        KeyCode::ANSI_KEYPAD_MINUS => PetKey::NumpadSubtract,
        KeyCode::ANSI_KEYPAD_MULTIPLY => PetKey::NumpadMultiply,
        KeyCode::ANSI_KEYPAD_DIVIDE => PetKey::NumpadDivide,
        KeyCode::ANSI_KEYPAD_DECIMAL => PetKey::NumpadDecimal,
        KeyCode::JIS_KEYPAD_COMMA => PetKey::NumpadSeparator,
        KeyCode::ANSI_KEYPAD_EQUAL => PetKey::Punct('='),
        KeyCode::ISO_SECTION => PetKey::Punct('§'),
        KeyCode::JIS_YEN => PetKey::Punct('¥'),
        KeyCode::JIS_UNDERSCORE => PetKey::Punct('_'),
        // Apple 键盘没有独立 Insert；旧式 Help 键是最接近的可报告来源。
        KeyCode::HELP => PetKey::Insert,
        KeyCode::F1 => PetKey::Function(1),
        KeyCode::F2 => PetKey::Function(2),
        KeyCode::F3 => PetKey::Function(3),
        KeyCode::F4 => PetKey::Function(4),
        KeyCode::F5 => PetKey::Function(5),
        KeyCode::F6 => PetKey::Function(6),
        KeyCode::F7 => PetKey::Function(7),
        KeyCode::F8 => PetKey::Function(8),
        KeyCode::F9 => PetKey::Function(9),
        KeyCode::F10 => PetKey::Function(10),
        KeyCode::F11 => PetKey::Function(11),
        KeyCode::F12 => PetKey::Function(12),
        KeyCode::SHIFT | KeyCode::RIGHT_SHIFT => PetKey::Shift,
        KeyCode::CONTROL | KeyCode::RIGHT_CONTROL => PetKey::Ctrl,
        KeyCode::OPTION | KeyCode::RIGHT_OPTION => PetKey::Alt,
        KeyCode::COMMAND | KeyCode::RIGHT_COMMAND => PetKey::Super,
        KeyCode::LEFT_ARROW => PetKey::ArrowLeft,
        KeyCode::RIGHT_ARROW => PetKey::ArrowRight,
        KeyCode::UP_ARROW => PetKey::ArrowUp,
        KeyCode::DOWN_ARROW => PetKey::ArrowDown,
        _ => return None,
    })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::mac_key;
    use core_graphics::event::KeyCode;
    use deskhud_engine::PetKey;

    #[test]
    fn mac_key_map_covers_standard_navigation_modifiers_and_keypad() {
        let cases = [
            (KeyCode::ANSI_GRAVE, PetKey::Punct('`')),
            (KeyCode::DELETE, PetKey::Backspace),
            (KeyCode::FORWARD_DELETE, PetKey::Delete),
            (KeyCode::HOME, PetKey::Home),
            (KeyCode::END, PetKey::End),
            (KeyCode::PAGE_UP, PetKey::PageUp),
            (KeyCode::PAGE_DOWN, PetKey::PageDown),
            (KeyCode::LEFT_ARROW, PetKey::ArrowLeft),
            (KeyCode::CAPS_LOCK, PetKey::CapsLock),
            (KeyCode::RIGHT_SHIFT, PetKey::Shift),
            (KeyCode::RIGHT_CONTROL, PetKey::Ctrl),
            (KeyCode::RIGHT_OPTION, PetKey::Alt),
            (KeyCode::RIGHT_COMMAND, PetKey::Super),
            (KeyCode::ANSI_KEYPAD_ENTER, PetKey::NumpadEnter),
            (KeyCode::ANSI_KEYPAD_0, PetKey::NumpadDigit(0)),
            (KeyCode::ANSI_KEYPAD_DECIMAL, PetKey::NumpadDecimal),
            (KeyCode::JIS_KEYPAD_COMMA, PetKey::NumpadSeparator),
            (KeyCode::F12, PetKey::Function(12)),
        ];
        for (code, expected) in cases {
            assert_eq!(mac_key(code), Some(expected));
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn global_mouse_buttons() -> GlobalMouseButtons {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VIRTUAL_KEY, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
        };
        let is_down = |key: VIRTUAL_KEY| unsafe { GetAsyncKeyState(key.0 as i32) < 0 };
        return GlobalMouseButtons {
            primary_down: is_down(VK_LBUTTON),
            secondary_down: is_down(VK_RBUTTON),
            middle_down: is_down(VK_MBUTTON),
        };
    }
    #[cfg(not(target_os = "windows"))]
    {
        GlobalMouseButtons::default()
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn global_pointer_position() -> Option<[f64; 2]> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = POINT::default();
        return unsafe { GetCursorPos(&mut point).ok() }.map(|_| [point.x as f64, point.y as f64]);
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Windows low-level hooks are listen-only. They run on a dedicated thread
/// with its own Win32 message pump, because a low-level hook callback is
/// delivered to the thread that installed it and must not share winit's
/// window/event-loop workload.
#[cfg(target_os = "windows")]
pub(crate) struct GlobalKeyMonitor {
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "windows")]
static WINDOWS_PROXY: std::sync::OnceLock<
    std::sync::Mutex<
        Option<winit::event_loop::EventLoopProxy<crate::runtime::viewport::UserEvent>>,
    >,
> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
pub(crate) fn install_global_key_monitor(
    proxy: winit::event_loop::EventLoopProxy<crate::runtime::viewport::UserEvent>,
    keyboard: bool,
    mouse: bool,
) -> Option<GlobalKeyMonitor> {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, SetWindowsHookExW,
        TranslateMessage, WH_KEYBOARD_LL, WH_MOUSE_LL,
    };

    let slot = WINDOWS_PROXY.get_or_init(|| std::sync::Mutex::new(None));
    *slot.lock().ok()? = Some(proxy);

    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("deskhud-global-hook".to_owned())
        .spawn(move || {
            use windows::Win32::System::LibraryLoader::GetModuleHandleW;
            use windows::Win32::System::Threading::GetCurrentThreadId;
            use windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx;

            let thread_id = unsafe { GetCurrentThreadId() };
            let mut initial_message = MSG::default();
            unsafe {
                let _ = PeekMessageW(&mut initial_message, None, 0, 0, PM_NOREMOVE);
            }
            let module = unsafe {
                GetModuleHandleW(None)
                    .ok()
                    .map(|module| windows::Win32::Foundation::HINSTANCE(module.0))
            };
            let keyboard_hook = if keyboard {
                unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0).ok() }
            } else {
                None
            };
            let mouse_hook = if mouse {
                unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0).ok() }
            } else {
                None
            };
            tracing::info!(
                thread_id,
                module_loaded = module.is_some(),
                keyboard_hook = keyboard_hook.is_some(),
                mouse_hook = mouse_hook.is_some(),
                "global hook thread initialized"
            );
            if (keyboard && keyboard_hook.is_none()) || (mouse && mouse_hook.is_none()) {
                if let Some(hook) = keyboard_hook {
                    unsafe {
                        let _ = UnhookWindowsHookEx(hook);
                    }
                }
                if let Some(hook) = mouse_hook {
                    unsafe {
                        let _ = UnhookWindowsHookEx(hook);
                    }
                }
                let _ = ready_tx.send(Err(()));
                return;
            }

            let _ = ready_tx.send(Ok(thread_id));
            tracing::info!(thread_id, "global hook message pump started");
            let mut message = MSG::default();
            while unsafe { GetMessageW(&mut message, None, 0, 0).as_bool() } {
                unsafe {
                    let _ = TranslateMessage(&message);
                    let _ = DispatchMessageW(&message);
                }
            }
            if let Some(hook) = keyboard_hook {
                unsafe {
                    let _ = UnhookWindowsHookEx(hook);
                }
            }
            if let Some(hook) = mouse_hook {
                unsafe {
                    let _ = UnhookWindowsHookEx(hook);
                }
            }
            tracing::info!(thread_id, "global hook message pump stopped");
        })
        .ok()?;
    let thread_id = match ready_rx.recv() {
        Ok(Ok(thread_id)) => thread_id,
        _ => {
            let _ = thread.join();
            return None;
        }
    };
    Some(GlobalKeyMonitor {
        thread_id,
        thread: Some(thread),
    })
}

#[cfg(target_os = "windows")]
impl Drop for GlobalKeyMonitor {
    fn drop(&mut self) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            let _ = PostThreadMessageW(
                self.thread_id,
                WM_QUIT,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_os = "windows")]
fn send_global(event: deskhud_engine::PetEvent) {
    // The proxy is installed before either hook is registered. The static is
    // deliberately process-local; hooks are removed before the event loop exits.
    if let Ok(guard) = WINDOWS_PROXY
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        && let Some(proxy) = guard.as_ref()
    {
        if proxy
            .send_event(crate::runtime::viewport::UserEvent::PetEvent(event))
            .is_err()
        {
            tracing::warn!("global input event dropped: event loop proxy unavailable");
        }
    } else {
        tracing::warn!("global input event dropped: event loop proxy not installed");
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
    };
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        tracing::info!(vk, message = wparam.0, "raw global keyboard hook event");
        if let Some(key) = windows_vk_to_pet_key(vk) {
            let pressed = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            // GetAsyncKeyState may be sampled just before Windows updates the
            // state for this hook callback. Include the current modifier event
            // explicitly so a standalone Ctrl/Shift/Alt is never lost.
            let modifiers = windows_modifiers_for_event(info.vkCode as u16, pressed);
            tracing::info!(
                vk = info.vkCode,
                ?key,
                pressed,
                ?modifiers,
                "global keyboard hook event"
            );
            send_global(if pressed {
                deskhud_engine::PetEvent::GlobalKeyPressed { key, modifiers }
            } else {
                deskhud_engine::PetEvent::GlobalKeyReleased { key, modifiers }
            });
        } else {
            tracing::warn!(vk, "unmapped global keyboard hook event");
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
        WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
    };
    if code >= 0 {
        let _info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        let button = match message {
            WM_LBUTTONDOWN | WM_LBUTTONUP => Some(deskhud_engine::PetMouseButton::Primary),
            WM_RBUTTONDOWN | WM_RBUTTONUP => Some(deskhud_engine::PetMouseButton::Secondary),
            WM_MBUTTONDOWN | WM_MBUTTONUP => Some(deskhud_engine::PetMouseButton::Middle),
            _ => None,
        };
        if let Some(button) = button {
            send_global(
                if matches!(message, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN) {
                    deskhud_engine::PetEvent::GlobalMousePressed {
                        button,
                        modifiers: windows_modifiers(),
                    }
                } else {
                    deskhud_engine::PetEvent::GlobalMouseReleased {
                        button,
                        modifiers: windows_modifiers(),
                    }
                },
            );
        }
        if matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
            let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            let delta = ((info.mouseData >> 16) as i16).signum() as i8;
            if delta != 0 {
                send_global(deskhud_engine::PetEvent::GlobalMouseWheel {
                    delta,
                    modifiers: windows_modifiers(),
                });
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(target_os = "windows")]
fn windows_modifiers() -> deskhud_engine::PetModifiers {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let down = |key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| unsafe {
        GetAsyncKeyState(key.0 as i32) < 0
    };
    deskhud_engine::PetModifiers {
        shift: down(VK_SHIFT) || down(VK_LSHIFT),
        ctrl: down(VK_CONTROL),
        alt: down(VK_MENU) || down(VK_LMENU),
        meta: down(VK_LWIN) || down(VK_RWIN),
    }
}

#[cfg(target_os = "windows")]
fn windows_modifiers_for_event(vk: u16, pressed: bool) -> deskhud_engine::PetModifiers {
    let mut modifiers = windows_modifiers();
    match vk {
        0x10 | 0xA0 | 0xA1 => modifiers.shift = pressed, // VK_SHIFT / left/right
        0x11 | 0xA2 | 0xA3 => modifiers.ctrl = pressed,  // VK_CONTROL / left/right
        0x12 | 0xA4 | 0xA5 => modifiers.alt = pressed,   // VK_MENU / left/right
        0x5B | 0x5C => modifiers.meta = pressed,         // VK_LWIN / VK_RWIN
        _ => {}
    }
    modifiers
}

#[cfg(target_os = "windows")]
fn windows_vk_to_pet_key(vk: u16) -> Option<PetKey> {
    Some(match vk {
        0x08 => PetKey::Backspace,
        0x09 => PetKey::Tab,
        0x0D => PetKey::Enter,
        0x1B => PetKey::Escape,
        0x20 => PetKey::Space,
        0x24 => PetKey::Home,
        0x23 => PetKey::End,
        0x21 => PetKey::PageUp,
        0x22 => PetKey::PageDown,
        0x25 => PetKey::ArrowLeft,
        0x26 => PetKey::ArrowUp,
        0x27 => PetKey::ArrowRight,
        0x28 => PetKey::ArrowDown,
        0x2D => PetKey::Insert,
        0x2E => PetKey::Delete,
        0x2C => PetKey::PrintScreen,
        0x91 => PetKey::ScrollLock,
        0x13 => PetKey::Pause,
        0x5D => PetKey::ContextMenu,
        0x10 | 0xA0 | 0xA1 => PetKey::Shift,
        0x11 | 0xA2 | 0xA3 => PetKey::Ctrl,
        0x12 | 0xA4 | 0xA5 => PetKey::Alt,
        0x5B | 0x5C => PetKey::Super,
        0x14 => PetKey::CapsLock,
        0x90 => PetKey::NumLock,
        0x60..=0x69 => PetKey::NumpadDigit((vk - 0x60) as u8),
        0x6A => PetKey::NumpadMultiply,
        0x6B => PetKey::NumpadAdd,
        0x6C => PetKey::NumpadSeparator,
        0x6D => PetKey::NumpadSubtract,
        0x6E => PetKey::NumpadDecimal,
        0x6F => PetKey::NumpadDivide,
        0x70..=0x7B => PetKey::Function((vk - 0x6F) as u8),
        0x7C..=0x87 => PetKey::Function((vk - 0x6F) as u8),
        0x30..=0x39 => PetKey::Digit((b'0' + (vk - 0x30) as u8) as char),
        0x41..=0x5A => PetKey::Letter((b'A' + (vk - 0x41) as u8) as char),
        0xBA => PetKey::Punct(';'),
        0xBB => PetKey::Punct('='),
        0xBC => PetKey::Punct(','),
        0xBD => PetKey::Punct('-'),
        0xBE => PetKey::Punct('.'),
        0xBF => PetKey::Punct('/'),
        0xC0 => PetKey::Punct('`'),
        0xDB => PetKey::Punct('['),
        0xDC => PetKey::Punct('\\'),
        0xDD => PetKey::Punct(']'),
        0xDE => PetKey::Punct('\''),
        _ => return None,
    })
}
