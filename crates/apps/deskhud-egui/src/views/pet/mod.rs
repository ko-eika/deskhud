//! Pet 主视口的 UI 入口。

mod drawing;
pub(crate) mod menu;
mod window;

pub(crate) use menu::{PetMenu, PetMenuAction};
pub(crate) use window::PetWindow;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use deskhud_engine::{
    DockState, DragState, MouseState, PetConfigBag, PetEvent, PetKind, PetModifiers,
    PetMouseButton, PetPaintCtx, PetTheme,
};
use deskhud_ui::UiPreferences;
use egui::{Context, RawInput};

use crate::input;
use crate::views::ViewOutput;

/// 构建透明、可拖动并带有右键菜单的 Pet 视图。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    pet: &dyn PetKind,
    prefs: &UiPreferences,
    elapsed: f32,
    last_hit: &mut bool,
    dock: DockState,
    last_drag: &mut bool,
    dt: f32,
    last_scene: &mut deskhud_engine::PetScene,
    last_global_mouse: &mut input::GlobalMouseButtons,
    screen_center: Option<[f64; 2]>,
    window_size: [f64; 2],
    global_mouse_input: bool,
) -> ViewOutput {
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let info = pet.info();
        let options: Vec<(&str, bool)> = pet
            .config_options()
            .iter()
            .map(|option| (option.key, option.default))
            .collect();
        let map = prefs.pet.short_map_for(info.id, &options);
        let center = ctx.max_rect().center();
        let (pointer_dir, mouse) = ctx.input(|input| {
            let pointer_dir = global_mouse_input
                .then(input::global_pointer_position)
                .flatten()
                .zip(screen_center)
                .map_or_else(
                    || {
                        input.pointer.interact_pos().map_or([0.0, 0.0], |pos| {
                            [
                                ((pos.x - center.x) / center.x.max(1.0)).clamp(-1.0, 1.0),
                                ((pos.y - center.y) / center.y.max(1.0)).clamp(-1.0, 1.0),
                            ]
                        })
                    },
                    |(pointer, center)| {
                        [
                            ((pointer[0] - center[0]) / (window_size[0] / 2.0).max(1.0))
                                .clamp(-1.0, 1.0) as f32,
                            ((pointer[1] - center[1]) / (window_size[1] / 2.0).max(1.0))
                                .clamp(-1.0, 1.0) as f32,
                        ]
                    },
                );
            let pointer = &input.pointer;
            let global = global_mouse_input
                .then(input::global_mouse_buttons)
                .unwrap_or_default();
            (
                pointer_dir,
                MouseState {
                    hovering: input.pointer.hover_pos().is_some(),
                    primary_down: pointer.primary_down(),
                    secondary_down: pointer.secondary_down(),
                    middle_down: pointer.middle_down(),
                    global_primary_down: global.primary_down,
                    global_secondary_down: global.secondary_down,
                    global_middle_down: global.middle_down,
                },
            )
        });
        let hit_scene = &*last_scene;
        let modifiers = ctx.input(|input| PetModifiers {
            shift: input.modifiers.shift,
            ctrl: input.modifiers.ctrl,
            alt: input.modifiers.alt,
            meta: input.modifiers.command,
        });
        let global_mouse = global_mouse_input
            .then(input::global_mouse_buttons)
            .unwrap_or_default();
        for (was_down, is_down, button) in [
            (
                last_global_mouse.primary_down,
                global_mouse.primary_down,
                PetMouseButton::Primary,
            ),
            (
                last_global_mouse.secondary_down,
                global_mouse.secondary_down,
                PetMouseButton::Secondary,
            ),
            (
                last_global_mouse.middle_down,
                global_mouse.middle_down,
                PetMouseButton::Middle,
            ),
        ] {
            if !was_down && is_down {
                pet.on_event(PetEvent::GlobalMousePressed { button, modifiers });
            } else if was_down && !is_down {
                pet.on_event(PetEvent::GlobalMouseReleased { button, modifiers });
            }
        }
        *last_global_mouse = global_mouse;
        let local_point = ctx.input(|input| input.pointer.hover_pos()).map(|pos| {
            [
                (pos.x - center.x) / (ctx.max_rect().width().min(ctx.max_rect().height()) * 0.32),
                (pos.y - center.y) / (ctx.max_rect().width().min(ctx.max_rect().height()) * 0.32),
            ]
        });
        let inside = local_point.is_some_and(|point| hit_scene.hit_test(point));
        if inside != *last_hit {
            pet.on_event(PetEvent::MouseHover { inside });
            *last_hit = inside;
        }
        // Once a drag has crossed the pet's transparent edge, the local hit
        // test may become false even though the mouse button is still down.
        // Keep the drag state until release; otherwise the snap-back path can
        // race the native drag and make the window jump back and forth.
        let dragging = if *last_drag {
            ctx.input(|input| input.pointer.primary_down())
        } else {
            ctx.input(|input| {
                input.pointer.primary_down()
                    && input.pointer.press_origin().is_some_and(|origin| {
                        input
                            .pointer
                            .interact_pos()
                            .is_some_and(|position| position.distance_sq(origin) >= 16.0)
                    })
            }) && inside
        };
        if dragging != *last_drag {
            pet.on_event(if dragging {
                PetEvent::DragStarted
            } else {
                PetEvent::DragEnded {
                    drag: DragState::IDLE,
                }
            });
            *last_drag = dragging;
        }

        pet.tick(dt);
        let scene_result = catch_unwind(AssertUnwindSafe(|| {
            pet.scene(PetPaintCtx {
                time_secs: elapsed as f64,
                pointer_dir,
                status_line: "",
                dock,
                drag: DragState { active: *last_drag },
                mouse,
                config: PetConfigBag::new(&map),
                theme: match ctx.ctx().theme() {
                    egui::Theme::Light => PetTheme::Light,
                    egui::Theme::Dark => PetTheme::Dark,
                },
                shadows: prefs.graphics.shadows,
            })
        }));
        let scene = scene_result.unwrap_or_else(|_| {
            eprintln!("pet scene generation panicked; skipping frame");
            deskhud_engine::PetScene::default()
        });
        *last_scene = scene.clone();

        let valid_scene = match scene.validate() {
            Ok(()) => true,
            Err(error) => {
                eprintln!("pet scene rejected: {error:?}");
                false
            }
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                if valid_scene {
                    drawing::EguiSceneRenderer::render(ui, &scene);
                }
            });
    });

    ViewOutput {
        full_output,
        ..Default::default()
    }
}
