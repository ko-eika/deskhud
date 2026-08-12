//! 原生覆盖层到 egui 控制层的中性命令桥。
//!
//! Windows 覆盖层只产生用户意图，不创建第二套菜单或设置 UI；直接 egui 宿主
#![cfg_attr(not(windows), allow(dead_code))]
//! 消费命令并显示控制窗口。该队列不携带 HWND 或渲染器类型，供后续其它平台复用。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type CommandWaker = Arc<dyn Fn() + Send + Sync>;

/// 原生覆盖层可请求的产品级操作。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OverlayControlCommand {
    /// Wake the existing pet overlay without opening a menu.
    ActivateExisting,
    /// 打开现有 egui 宠物菜单。
    OpenMenu,
    /// Persist the pet's snapped logical-screen position.
    PetMoved { x_points: f32, y_points: f32 },
    /// 请求 DeskHud 正常退出。
    Quit,
}

/// 跨线程安全的命令队列；生产者可为原生窗口线程，消费者固定为 egui UI 线程。
#[derive(Clone, Default)]
pub(crate) struct OverlayControlBus {
    commands: Arc<Mutex<VecDeque<OverlayControlCommand>>>,
    waker: Arc<Mutex<Option<CommandWaker>>>,
    shutdown: Arc<AtomicBool>,
}

impl OverlayControlBus {
    /// 请求一个操作；锁竞争或中毒时丢弃本次请求，避免原生窗口线程崩溃。
    pub(crate) fn request(&self, command: OverlayControlCommand) {
        if let Ok(mut commands) = self.commands.lock() {
            commands.push_back(command);
        }
        if let Ok(waker) = self.waker.lock()
            && let Some(waker) = waker.as_ref()
        {
            waker();
        }
    }

    /// Connect command production to the platform event loop without polling.
    pub(crate) fn set_waker(&self, waker: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(Arc::new(waker));
        }
    }

    /// 取走目前所有命令，保持其原始顺序。
    pub(crate) fn drain(&self) -> Vec<OverlayControlCommand> {
        self.commands
            .lock()
            .map(|mut commands| commands.drain(..).collect())
            .unwrap_or_default()
    }

    pub(crate) fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
    pub(crate) fn shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{OverlayControlBus, OverlayControlCommand};

    #[test]
    fn retains_command_order() {
        let bus = OverlayControlBus::default();
        bus.request(OverlayControlCommand::OpenMenu);
        bus.request(OverlayControlCommand::PetMoved {
            x_points: 12.0,
            y_points: 34.0,
        });
        bus.request(OverlayControlCommand::Quit);
        assert_eq!(
            bus.drain(),
            vec![
                OverlayControlCommand::OpenMenu,
                OverlayControlCommand::PetMoved {
                    x_points: 12.0,
                    y_points: 34.0,
                },
                OverlayControlCommand::Quit,
            ]
        );
        assert!(bus.drain().is_empty());
    }

    #[test]
    fn wakes_consumer_when_a_command_arrives() {
        let bus = OverlayControlBus::default();
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&wakes);
        bus.set_waker(move || {
            observed.fetch_add(1, Ordering::Relaxed);
        });

        bus.request(OverlayControlCommand::OpenMenu);

        assert_eq!(wakes.load(Ordering::Relaxed), 1);
    }
}
