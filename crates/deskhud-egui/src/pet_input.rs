//! egui 输入 → 宿主中性 [`PetEvent`] / 键鼠类型。

use std::sync::LazyLock;

use deskhud_engine::{PetKey, PetModifiers, PetMouseButton};
use eframe::egui::{self, Key, PointerButton};

pub fn modifiers_from_egui(m: &egui::Modifiers) -> PetModifiers {
    PetModifiers {
        shift: m.shift,
        ctrl: m.ctrl,
        alt: m.alt,
        meta: m.command || m.mac_cmd,
    }
}

pub fn mouse_button_from_egui(b: PointerButton) -> Option<PetMouseButton> {
    match b {
        PointerButton::Primary => Some(PetMouseButton::Primary),
        PointerButton::Secondary => Some(PetMouseButton::Secondary),
        PointerButton::Middle => Some(PetMouseButton::Middle),
        _ => None,
    }
}

pub fn key_from_egui(key: Key) -> Option<PetKey> {
    Some(match key {
        Key::Escape => PetKey::Escape,
        Key::Tab => PetKey::Tab,
        Key::Enter => PetKey::Enter,
        Key::Space => PetKey::Space,
        Key::Backspace => PetKey::Backspace,
        Key::Delete => PetKey::Delete,
        Key::ArrowUp => PetKey::ArrowUp,
        Key::ArrowDown => PetKey::ArrowDown,
        Key::ArrowLeft => PetKey::ArrowLeft,
        Key::ArrowRight => PetKey::ArrowRight,
        Key::Home => PetKey::Home,
        Key::End => PetKey::End,
        Key::PageUp => PetKey::PageUp,
        Key::PageDown => PetKey::PageDown,
        Key::A => PetKey::Letter('A'),
        Key::B => PetKey::Letter('B'),
        Key::C => PetKey::Letter('C'),
        Key::D => PetKey::Letter('D'),
        Key::E => PetKey::Letter('E'),
        Key::F => PetKey::Letter('F'),
        Key::G => PetKey::Letter('G'),
        Key::H => PetKey::Letter('H'),
        Key::I => PetKey::Letter('I'),
        Key::J => PetKey::Letter('J'),
        Key::K => PetKey::Letter('K'),
        Key::L => PetKey::Letter('L'),
        Key::M => PetKey::Letter('M'),
        Key::N => PetKey::Letter('N'),
        Key::O => PetKey::Letter('O'),
        Key::P => PetKey::Letter('P'),
        Key::Q => PetKey::Letter('Q'),
        Key::R => PetKey::Letter('R'),
        Key::S => PetKey::Letter('S'),
        Key::T => PetKey::Letter('T'),
        Key::U => PetKey::Letter('U'),
        Key::V => PetKey::Letter('V'),
        Key::W => PetKey::Letter('W'),
        Key::X => PetKey::Letter('X'),
        Key::Y => PetKey::Letter('Y'),
        Key::Z => PetKey::Letter('Z'),
        Key::Num0 => PetKey::Digit('0'),
        Key::Num1 => PetKey::Digit('1'),
        Key::Num2 => PetKey::Digit('2'),
        Key::Num3 => PetKey::Digit('3'),
        Key::Num4 => PetKey::Digit('4'),
        Key::Num5 => PetKey::Digit('5'),
        Key::Num6 => PetKey::Digit('6'),
        Key::Num7 => PetKey::Digit('7'),
        Key::Num8 => PetKey::Digit('8'),
        Key::Num9 => PetKey::Digit('9'),
        Key::Minus => PetKey::Punct('-'),
        Key::Equals => PetKey::Punct('='),
        Key::Comma => PetKey::Punct(','),
        Key::Period => PetKey::Punct('.'),
        Key::Slash => PetKey::Punct('/'),
        Key::Backslash => PetKey::Punct('\\'),
        Key::Semicolon => PetKey::Punct(';'),
        Key::Quote => PetKey::Punct('\''),
        Key::Backtick => PetKey::Punct('`'),
        Key::OpenBracket => PetKey::Punct('['),
        Key::CloseBracket => PetKey::Punct(']'),
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
        _ => return None,
    })
}

/// US 布局 OEM 标点：(显示字符, VK)。
const OEM_PUNCT: &[(char, i32)] = &[
    (';', 0xBA),
    ('=', 0xBB),
    (',', 0xBC),
    ('-', 0xBD),
    ('.', 0xBE),
    ('/', 0xBF),
    ('`', 0xC0),
    ('[', 0xDB),
    ('\\', 0xDC),
    (']', 0xDD),
    ('\'', 0xDE),
];

fn build_global_tracked_keys() -> Vec<PetKey> {
    let mut keys = vec![
        PetKey::Space,
        PetKey::Enter,
        PetKey::Escape,
        PetKey::Tab,
        PetKey::Backspace,
        PetKey::Delete,
        PetKey::ArrowUp,
        PetKey::ArrowDown,
        PetKey::ArrowLeft,
        PetKey::ArrowRight,
        PetKey::Home,
        PetKey::End,
        PetKey::PageUp,
        PetKey::PageDown,
        PetKey::Shift,
        PetKey::Ctrl,
        PetKey::Alt,
        PetKey::Super,
        PetKey::CapsLock,
    ];
    for c in b'A'..=b'Z' {
        keys.push(PetKey::Letter(c as char));
    }
    for c in b'0'..=b'9' {
        keys.push(PetKey::Digit(c as char));
    }
    for &(ch, _) in OEM_PUNCT {
        keys.push(PetKey::Punct(ch));
    }
    for n in 1u8..=12 {
        keys.push(PetKey::Function(n));
    }
    keys
}

static GLOBAL_TRACKED_KEYS: LazyLock<Vec<PetKey>> = LazyLock::new(build_global_tracked_keys);

/// 桌面全局采样的按键（含修饰键、字母数字、标点、F 键）。
pub fn global_tracked_keys() -> &'static [PetKey] {
    &GLOBAL_TRACKED_KEYS
}

fn vks_for(key: PetKey) -> Vec<i32> {
    match key {
        PetKey::Space => vec![0x20],
        PetKey::Enter => vec![0x0D],
        PetKey::Escape => vec![0x1B],
        PetKey::Tab => vec![0x09],
        PetKey::Backspace => vec![0x08],
        PetKey::Delete => vec![0x2E],
        PetKey::ArrowLeft => vec![0x25],
        PetKey::ArrowUp => vec![0x26],
        PetKey::ArrowRight => vec![0x27],
        PetKey::ArrowDown => vec![0x28],
        PetKey::Home => vec![0x24],
        PetKey::End => vec![0x23],
        PetKey::PageUp => vec![0x21],
        PetKey::PageDown => vec![0x22],
        PetKey::Shift => vec![0x10, 0xA0, 0xA1],
        PetKey::Ctrl => vec![0x11, 0xA2, 0xA3],
        PetKey::Alt => vec![0x12, 0xA4, 0xA5],
        PetKey::Super => vec![0x5B, 0x5C],
        PetKey::CapsLock => vec![0x14],
        PetKey::Letter(c) if c.is_ascii_uppercase() => vec![c as i32],
        PetKey::Digit(c) if c.is_ascii_digit() => vec![c as i32],
        PetKey::Punct(ch) => OEM_PUNCT
            .iter()
            .find(|(c, _)| *c == ch)
            .map(|(_, vk)| vec![*vk])
            .unwrap_or_default(),
        PetKey::Function(n) if (1..=12).contains(&n) => vec![0x70 + (n as i32 - 1)],
        _ => vec![],
    }
}

/// 全局是否按下该 [`PetKey`]（修饰键合并左右）。
pub fn global_pet_key_down(key: PetKey, sample: impl Fn(i32) -> bool) -> bool {
    vks_for(key).into_iter().any(sample)
}
