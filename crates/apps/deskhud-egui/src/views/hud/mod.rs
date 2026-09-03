//! HUD 视口的 UI 入口。

mod drawing;
mod window;

pub(crate) use window::HudWindow;

use std::{collections::HashMap, time::Duration};

use deskhud_engine::{
    EngineRegistry, HudFrame, HudInstanceId, HudLogicalRect, HudLogicalSize, HudSourceId, HudVisual,
};
use deskhud_ui::{HudInstanceConfig, HudSlotLayout, UiPreferences};
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// HUD 内部子窗口的布局状态。
#[derive(Default)]
pub(crate) struct LayoutState {
    /// 按 `plugin_id/contribution_id` 保留的 HUD 条目逻辑坐标。
    pub(crate) positions: HashMap<String, egui::Pos2>,
    /// 是否处于可拖动布局模式。
    pub(crate) layout_mode: bool,
    /// 当前显示器活动区域的逻辑尺寸。
    pub(crate) activity_size: Option<egui::Vec2>,
    /// 是否等待下一帧切回紧凑窗口尺寸。
    pub(crate) compact_pending: bool,
    /// 当前高亮的 HUD 条目；右键调整窗口也绑定到此条目。
    pub(crate) selected: Option<String>,
    pub(crate) adjust_open: bool,
    pub(crate) shadow_open: bool,
    pub(crate) shadow_target: Option<ShadowTarget>,
    /// Whether layout positions should snap to the visible alignment grid.
    pub(crate) snap_to_grid: bool,
    /// Recreates HUD egui windows when entering a new editing session or
    /// when their size is changed from the adjustment panel.
    pub(crate) window_revision: u64,
    pub(crate) adjust_session: u64,
    /// Whether the adjustment panel should preserve the selected HUD aspect ratio.
    pub(crate) lock_ratio: bool,
    pub(crate) locked_ratio: Option<f32>,
    pub(crate) position_unit: AdjustmentUnit,
    pub(crate) size_unit: AdjustmentUnit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowTarget {
    Global,
    Window,
    Content,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AdjustmentUnit {
    #[default]
    Percent,
    Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HudLayoutTarget {
    Instance(HudInstanceId),
    Group(String),
}

pub(crate) struct HudRenderLayer {
    pub(crate) frame: HudFrame,
    pub(crate) rect: HudLogicalRect,
    pub(crate) clip: HudLogicalRect,
}

/// One virtual HUD slot after instance resolution and optional group composition.
pub(crate) struct HudRenderItem {
    pub(crate) key: String,
    pub(crate) target: HudLayoutTarget,
    pub(crate) source: Option<HudSourceId>,
    pub(crate) layers: Vec<HudRenderLayer>,
    pub(crate) base_size: HudLogicalSize,
    pub(crate) initial_position: egui::Pos2,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) background_enabled: bool,
    pub(crate) background_opacity: f32,
    pub(crate) background_blur: f32,
    pub(crate) content_opacity: f32,
    pub(crate) shadow_enabled: bool,
    pub(crate) window_shadow_global: bool,
    pub(crate) content_shadow_global: bool,
    pub(crate) window_shadow_enabled: bool,
    pub(crate) content_shadow_enabled: bool,
    pub(crate) window_shadow: f32,
    pub(crate) window_shadow_blur: f32,
    pub(crate) window_shadow_distance: f32,
    pub(crate) window_shadow_angle: f32,
    pub(crate) window_shadow_color: [u8; 3],
    pub(crate) window_custom_shadow: f32,
    pub(crate) window_custom_shadow_blur: f32,
    pub(crate) window_custom_shadow_distance: f32,
    pub(crate) window_custom_shadow_angle: f32,
    pub(crate) window_custom_shadow_color: [u8; 3],
    pub(crate) content_custom_shadow: f32,
    pub(crate) content_custom_shadow_blur: f32,
    pub(crate) content_custom_shadow_distance: f32,
    pub(crate) content_custom_shadow_angle: f32,
    pub(crate) content_custom_shadow_color: [u8; 3],
    pub(crate) content_color: [u8; 3],
    pub(crate) border_enabled: bool,
    pub(crate) border_opacity: f32,
    pub(crate) border_width: f32,
    pub(crate) corner_radius: f32,
    pub(crate) border_color: [u8; 3],
}

struct ActiveHudFrame {
    instance_id: HudInstanceId,
    source: HudSourceId,
    plugin_id: &'static str,
    contribution_id: &'static str,
    frame: HudFrame,
    layout: HudSlotLayout,
    config: HudInstanceConfig,
    group_id: Option<String>,
}

struct ResolvedHudSlot {
    key: String,
    target: HudLayoutTarget,
    source: Option<HudSourceId>,
    plugin_id: &'static str,
    contribution_id: &'static str,
    config: HudInstanceConfig,
    frame: HudFrame,
    layers: Vec<HudRenderLayer>,
    base_size: HudLogicalSize,
    layout: HudSlotLayout,
}

fn active_hud_frames(
    registry: &EngineRegistry,
    prefs: &UiPreferences,
    elapsed_secs: f32,
) -> Vec<ActiveHudFrame> {
    if !prefs.hud.is_master_enabled() {
        return Vec::new();
    }
    let available: HashMap<_, _> = registry
        .all_hud_contributions()
        .into_iter()
        .map(|(plugin_id, contribution)| {
            (
                HudSourceId::new(plugin_id, contribution.id),
                (plugin_id, contribution.id),
            )
        })
        .collect();
    let membership: HashMap<_, _> = prefs
        .hud
        .groups
        .iter()
        .flat_map(|group| {
            group
                .children
                .iter()
                .map(move |instance_id| (instance_id.clone(), group))
        })
        .collect();
    prefs
        .hud
        .instances
        .iter()
        .filter_map(|instance| {
            let group = membership.get(&instance.id).copied();
            if !instance.enabled
                || !prefs.hud.is_plugin_enabled(&instance.source.plugin_id)
                || !available.contains_key(&instance.source)
                || group.is_some_and(|group| !group.enabled)
            {
                return None;
            }
            let &(plugin_id, contribution_id) = available.get(&instance.source)?;
            let frame = registry.hud_frame_for_instance(&deskhud_engine::HudFrameCtx {
                instance_id: &instance.id,
                source: &instance.source,
                elapsed_secs,
            });
            (!frame.is_empty()).then(|| ActiveHudFrame {
                instance_id: instance.id.clone(),
                source: instance.source.clone(),
                plugin_id,
                contribution_id,
                frame,
                layout: instance.layout.clone(),
                config: instance.config.clone(),
                group_id: group.map(|group| group.id.clone()),
            })
        })
        .collect()
}

pub(crate) fn has_active_hud(registry: &EngineRegistry, prefs: &UiPreferences) -> bool {
    if !prefs.hud.is_master_enabled() {
        return false;
    }
    let available: std::collections::HashSet<_> = registry
        .all_hud_contributions()
        .into_iter()
        .map(|(plugin_id, contribution)| HudSourceId::new(plugin_id, contribution.id))
        .collect();
    let membership: HashMap<_, _> = prefs
        .hud
        .groups
        .iter()
        .flat_map(|group| {
            group
                .children
                .iter()
                .map(move |instance_id| (instance_id, group.enabled))
        })
        .collect();
    prefs.hud.instances.iter().any(|instance| {
        instance.enabled
            && prefs.hud.is_plugin_enabled(&instance.source.plugin_id)
            && available.contains(&instance.source)
            && membership.get(&instance.id).copied().unwrap_or(true)
    })
}

fn resolved_hud_slots(
    registry: &EngineRegistry,
    prefs: &UiPreferences,
    elapsed_secs: f32,
) -> Vec<ResolvedHudSlot> {
    let frames = active_hud_frames(registry, prefs, elapsed_secs);
    let mut by_instance: HashMap<_, _> = frames
        .into_iter()
        .map(|frame| (frame.instance_id.clone(), frame))
        .collect();
    let mut slots = Vec::new();

    for instance in &prefs.hud.instances {
        let Some(frame) = by_instance.get(&instance.id) else {
            continue;
        };
        if frame.group_id.is_some() {
            continue;
        }
        let size = measured_frame_size(&frame.frame);
        let rect = HudLogicalRect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        };
        slots.push(ResolvedHudSlot {
            key: format!("instance/{}", frame.instance_id.as_str()),
            target: HudLayoutTarget::Instance(frame.instance_id.clone()),
            source: Some(frame.source.clone()),
            plugin_id: frame.plugin_id,
            contribution_id: frame.contribution_id,
            config: frame.config.clone(),
            frame: frame.frame.clone(),
            layers: vec![HudRenderLayer {
                frame: frame.frame.clone(),
                rect,
                clip: rect,
            }],
            base_size: size,
            layout: frame.layout.clone(),
        });
    }

    for group in &prefs.hud.groups {
        if !group.enabled {
            continue;
        }
        let members: Vec<_> = group
            .children
            .iter()
            .filter_map(|id| by_instance.remove(id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let measured: Vec<_> = members
            .iter()
            .map(|member| measured_frame_size(&member.frame))
            .collect();
        let composition = group.inner.compose(&measured);
        let style_frame = HudFrame {
            visuals: members
                .iter()
                .flat_map(|member| member.frame.visuals.iter().cloned())
                .collect(),
        };
        let layers = members
            .into_iter()
            .zip(composition.members)
            .map(|(member, placement)| HudRenderLayer {
                frame: member.frame,
                rect: placement.frame,
                clip: placement.clip,
            })
            .collect();
        slots.push(ResolvedHudSlot {
            key: format!("group/{}", group.id),
            target: HudLayoutTarget::Group(group.id.clone()),
            source: None,
            plugin_id: "hud.deskhud.group",
            contribution_id: "group",
            config: HudInstanceConfig::new(),
            frame: style_frame,
            layers,
            base_size: composition.size,
            layout: group.layout.clone(),
        });
    }
    slots
}

fn measured_frame_size(frame: &HudFrame) -> HudLogicalSize {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for visual in &frame.visuals {
        match visual {
            HudVisual::Panel {
                width: panel_width,
                height: panel_height,
                ..
            } => {
                width = width.max(*panel_width);
                height = height.max(*panel_height);
            }
            HudVisual::Text {
                text, font_size, ..
            } => {
                width = width.max(text.chars().count() as f32 * font_size * 0.62 + 20.0);
                height = height.max(font_size + 16.0);
            }
        }
    }
    if width <= 0.0 || height <= 0.0 {
        HudLogicalSize::new(136.0, 72.0)
    } else {
        HudLogicalSize::new(width, height)
    }
}

/// 构建透明、无边框并带有动态虚线边框的 HUD 视图。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> ViewOutput {
    let mut content_size = [320.0, 180.0];
    let mut move_by = None;
    let mut changed = false;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(16));
        let time = ctx.input(|input| input.time) as f32;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let result = drawing::draw(ui, time, layout, items, prefs);
                content_size = result.size;
                move_by = result.move_by;
                changed = result.changed;
            });
    });

    ViewOutput {
        full_output,
        resize_to: Some(content_size),
        move_by,
        applied_preferences: changed.then(|| prefs.clone()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{HudLayoutTarget, active_hud_frames, resolved_hud_slots};
    use deskhud_engine::{
        HudContribution, HudFrame, HudGroupArrangement, HudVisual, Plugin, PluginInfo,
    };
    use std::sync::Arc;

    struct OtherPlugin;

    impl Plugin for OtherPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "hud.example.other",
                display_name: "Other",
                description: "test",
                author: "test",
                homepage: None,
                version: "0.1.0",
                engine: "0.9",
                icon: None,
            }
        }

        fn hud_contributions(&self) -> &'static [HudContribution] {
            static ITEMS: &[HudContribution] = &[HudContribution {
                id: "meter",
                label: "Meter",
                default_enabled: true,
                icon: None,
            }];
            ITEMS
        }

        fn hud_frame(&self, _contribution_id: &str, _elapsed_secs: f32) -> HudFrame {
            HudFrame {
                visuals: vec![HudVisual::Panel {
                    width: 90.0,
                    height: 40.0,
                    radius: 4.0,
                    color: [32, 64, 96, 255],
                }],
            }
        }
    }

    #[test]
    fn registry_contributions_follow_all_three_enable_levels() {
        let bootstrap = deskhud_runtime::bootstrap_registry();
        let mut prefs = deskhud_ui::UiPreferences::default();
        prefs.hud.ensure_default_instances(
            bootstrap
                .registry
                .all_hud_contributions()
                .into_iter()
                .map(|(plugin, contribution)| {
                    (
                        deskhud_engine::HudSourceId::new(plugin, contribution.id),
                        contribution.default_enabled,
                    )
                }),
        );
        let initial = active_hud_frames(&bootstrap.registry, &prefs, 1.0);
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].source.contribution_id, "clock");
        assert!(initial[0].frame.visuals.iter().any(|visual| {
            matches!(visual, HudVisual::Text { text, .. } if text.starts_with("DeskHud"))
        }));

        let tip = prefs
            .hud
            .instances
            .iter_mut()
            .find(|instance| instance.source.contribution_id == "tip")
            .expect("tip instance");
        tip.enabled = true;
        assert_eq!(active_hud_frames(&bootstrap.registry, &prefs, 1.0).len(), 2);

        prefs.hud.set_plugin_enabled("hud.deskhud.demo", false);
        assert!(active_hud_frames(&bootstrap.registry, &prefs, 1.0).is_empty());
    }

    #[test]
    fn group_switch_gates_members_without_making_them_ungrouped() {
        let bootstrap = deskhud_runtime::bootstrap_registry();
        let mut prefs = deskhud_ui::UiPreferences::default();
        prefs.hud.ensure_default_instances(
            bootstrap
                .registry
                .all_hud_contributions()
                .into_iter()
                .map(|(plugin, contribution)| {
                    (
                        deskhud_engine::HudSourceId::new(plugin, contribution.id),
                        true,
                    )
                }),
        );
        let group_id = prefs.hud.create_group("mixed");
        prefs.hud.groups[0].children = prefs
            .hud
            .instances
            .iter()
            .map(|instance| instance.id.clone())
            .collect();
        assert!(
            active_hud_frames(&bootstrap.registry, &prefs, 1.0)
                .iter()
                .all(|frame| frame.group_id.as_deref() == Some(group_id.as_str()))
        );
        prefs.hud.groups[0].enabled = false;
        assert!(active_hud_frames(&bootstrap.registry, &prefs, 1.0).is_empty());
    }

    #[test]
    fn group_becomes_one_composed_slot_in_member_order() {
        let mut bootstrap = deskhud_runtime::bootstrap_registry();
        bootstrap.registry.register_plugin(Arc::new(OtherPlugin));
        let mut prefs = deskhud_ui::UiPreferences::default();
        prefs.hud.ensure_default_instances(
            bootstrap
                .registry
                .all_hud_contributions()
                .into_iter()
                .map(|(plugin, contribution)| {
                    (
                        deskhud_engine::HudSourceId::new(plugin, contribution.id),
                        true,
                    )
                }),
        );
        let group_id = prefs.hud.create_group("combined");
        prefs.hud.groups[0].children = prefs
            .hud
            .instances
            .iter()
            .rev()
            .map(|instance| instance.id.clone())
            .collect();
        prefs.hud.groups[0].inner.arrangement = HudGroupArrangement::Vertical;
        prefs.hud.groups[0].inner.spacing = 7.0;

        let slots = resolved_hud_slots(&bootstrap.registry, &prefs, 1.0);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].target, HudLayoutTarget::Group(group_id.clone()));
        assert_eq!(slots[0].key, format!("group/{group_id}"));
        assert_eq!(slots[0].layers.len(), 3);
        assert!(slots[0].layers[1].rect.y > slots[0].layers[0].rect.y);
        assert_eq!(slots[0].layers[0].clip, slots[0].layers[0].rect);
        assert_eq!(slots[0].layers[1].clip, slots[0].layers[1].rect);
        assert_eq!(slots[0].layout, prefs.hud.groups[0].layout);
    }
}
