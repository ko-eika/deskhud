//! 渲染线程：唯一执行 egui UI 构建、OpenGL 绘制和 buffer swap 的线程。

use std::{sync::mpsc, thread, thread::JoinHandle, time::Duration};

use winit::{event::WindowEvent, event_loop::EventLoopProxy, window::WindowId};

use super::{viewport::UserEvent, window_manager::WindowManager};

pub(crate) fn frame_rate_for(graphics: &deskhud_ui::GraphicsPreferences) -> u32 {
    match graphics.fps_limit {
        deskhud_ui::FpsLimit::Auto => match graphics.power_mode {
            deskhud_ui::PowerMode::Saving => 30,
            deskhud_ui::PowerMode::Balanced => 60,
            deskhud_ui::PowerMode::Smooth => 120,
        },
        deskhud_ui::FpsLimit::Fps30 => 30,
        deskhud_ui::FpsLimit::Fps60 => 60,
        deskhud_ui::FpsLimit::Fps120 => 120,
    }
}

#[cfg(test)]
mod tests {
    use super::frame_rate_for;
    use deskhud_ui::{FpsLimit, GraphicsPreferences, PowerMode};

    #[test]
    fn auto_frame_rate_follows_power_mode() {
        let mut graphics = GraphicsPreferences {
            power_mode: PowerMode::Saving,
            ..Default::default()
        };
        assert_eq!(frame_rate_for(&graphics), 30);
        graphics.power_mode = PowerMode::Balanced;
        assert_eq!(frame_rate_for(&graphics), 60);
        graphics.power_mode = PowerMode::Smooth;
        assert_eq!(frame_rate_for(&graphics), 120);
    }

    #[test]
    fn explicit_frame_rate_overrides_power_mode() {
        let graphics = GraphicsPreferences {
            power_mode: PowerMode::Saving,
            fps_limit: FpsLimit::Fps120,
            ..Default::default()
        };
        assert_eq!(frame_rate_for(&graphics), 120);
    }
}

#[derive(Debug)]
pub(crate) enum WindowCommand {
    /// 请求窗口管理器开始原生窗口拖动。
    Drag,
    /// 请求原生窗口调整客户区大小。
    Resize { width: u32, height: u32 },
    /// 请求原生窗口移动到指定的屏幕坐标。
    Move {
        position: winit::dpi::PhysicalPosition<i32>,
    },
    /// macOS：切换应用是否以普通应用显示在 Dock 中。
    #[cfg(target_os = "macos")]
    SetDockIcon { visible: bool },
}

#[derive(Debug)]
pub(crate) enum RenderResult {
    ShouldClose,
    Stopped,
}

pub(crate) enum RenderCommand {
    /// 将 winit 线程收到的窗口事件转交给渲染线程。
    WindowEvent {
        window_id: WindowId,
        event: WindowEvent,
    },
    /// 修改普通帧的最大频率；实际渲染仍在渲染线程中串行执行。
    SetFrameRate { frames_per_second: u32 },
    /// 请求渲染一帧，但仍遵守帧率限制。
    RenderNow,
    /// 停止渲染线程并释放所有 OpenGL 资源。
    Shutdown,
}

pub(crate) struct Renderer {
    sender: mpsc::Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
}

impl Renderer {
    pub(crate) fn start(manager: WindowManager, proxy: EventLoopProxy<UserEvent>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let initial_frame_rate = manager.frame_rate();
        let thread = thread::Builder::new()
            .name("egui-render".to_owned())
            .spawn(move || run(manager, proxy, receiver))
            .expect("启动渲染线程失败");
        let _ = sender.send(RenderCommand::SetFrameRate {
            frames_per_second: initial_frame_rate,
        });
        Self {
            sender,
            thread: Some(thread),
        }
    }

    pub(crate) fn send(&self, command: RenderCommand) {
        let _ = self.sender.send(command);
    }

    pub(crate) fn shutdown(mut self) {
        let _ = self.sender.send(RenderCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    mut manager: WindowManager,
    proxy: EventLoopProxy<UserEvent>,
    receiver: mpsc::Receiver<RenderCommand>,
) {
    // 普通重绘采用固定帧率，避免 egui 的 request_repaint_after 或高频输入事件
    // 造成无界的渲染循环。窗口尺寸变化属于交互反馈，需要走下面的立即重绘路径。
    let mut frame_duration = Duration::from_nanos(1_000_000_000 / 60);
    let mut next_frame = std::time::Instant::now();
    let mut applied_frame_rate = manager.frame_rate();
    // 非窗口事件暂存到下一轮处理，避免从通道中取出后丢失。
    let mut pending_command = None;
    loop {
        let timeout = next_frame.saturating_duration_since(std::time::Instant::now());
        let command = match pending_command.take() {
            Some(command) => Ok(command),
            None => receiver.recv_timeout(timeout),
        };
        match command {
            Ok(RenderCommand::WindowEvent { window_id, event }) => {
                // 调整窗口大小时不能等待普通帧率节流，否则原生窗口的尺寸变化
                // 可能会领先于设置窗口的内容刷新。Resize 事件处理完后立即补一帧。
                let mut should_render_immediately = matches!(event, WindowEvent::Resized(_));
                if manager.handle_event(window_id, event) {
                    let _ = proxy.send_event(UserEvent::RenderResult(RenderResult::ShouldClose));
                    break;
                }
                let requested_frame_rate = manager.frame_rate();
                if requested_frame_rate != applied_frame_rate {
                    frame_duration =
                        Duration::from_nanos(1_000_000_000 / requested_frame_rate.max(1) as u64);
                    applied_frame_rate = requested_frame_rate;
                    next_frame = std::time::Instant::now();
                }
                // 合并当前已经排队的输入事件。高频 CursorMoved 事件只需要推动一次
                // 绘制，但点击、菜单等事件仍会按顺序交给 WindowManager 处理。
                loop {
                    match receiver.try_recv() {
                        Ok(RenderCommand::WindowEvent { window_id, event }) => {
                            should_render_immediately |= matches!(event, WindowEvent::Resized(_));
                            if manager.handle_event(window_id, event) {
                                let _ = proxy
                                    .send_event(UserEvent::RenderResult(RenderResult::ShouldClose));
                                #[cfg(not(target_os = "macos"))]
                                manager.destroy_all();
                                let _ = proxy
                                    .send_event(UserEvent::RenderResult(RenderResult::Stopped));
                                return;
                            }
                        }
                        Ok(command) => {
                            pending_command = Some(command);
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => {
                            break;
                        }
                    }
                }
                // Resized 事件必须立即绘制：原生窗口已经变了尺寸，如果继续等待
                // next_frame，设置窗口在拖动时会出现明显的内容滞后。
                if should_render_immediately || std::time::Instant::now() >= next_frame {
                    if manager.render_all() {
                        let _ =
                            proxy.send_event(UserEvent::RenderResult(RenderResult::ShouldClose));
                        break;
                    }
                    next_frame = std::time::Instant::now() + frame_duration;
                }
            }
            Ok(RenderCommand::SetFrameRate { frames_per_second }) => {
                // 防止传入 0 导致除零；修改帧率后从当前时间重新开始计时。
                let fps = frames_per_second.max(1);
                frame_duration = Duration::from_nanos(1_000_000_000 / fps as u64);
                next_frame = std::time::Instant::now();
                applied_frame_rate = fps;
            }
            Ok(RenderCommand::RenderNow) => {
                // egui 的重绘回调只是提示，不允许绕过帧率限制；否则
                // request_repaint_after 可能演变成无界的渲染循环。
                if std::time::Instant::now() >= next_frame {
                    if manager.render_all() {
                        let _ =
                            proxy.send_event(UserEvent::RenderResult(RenderResult::ShouldClose));
                        break;
                    }
                    next_frame = std::time::Instant::now() + frame_duration;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if manager.render_all() {
                    let _ = proxy.send_event(UserEvent::RenderResult(RenderResult::ShouldClose));
                    break;
                }
                next_frame = std::time::Instant::now() + frame_duration;
            }
            Ok(RenderCommand::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    // macOS 退出时不要逐个 make_current/释放 AppKit 关联的 OpenGL view；
    // 让渲染线程自然结束并释放对象，避免与主线程的 AppKit 退出流程互相等待。
    #[cfg(not(target_os = "macos"))]
    manager.destroy_all();
    let _ = proxy.send_event(UserEvent::RenderResult(RenderResult::Stopped));
}
