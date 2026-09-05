//! HUD 视口的 UI 入口。

mod drawing;
mod window;

pub(crate) use window::HudWindow;

use std::{collections::HashMap, time::Duration};

use deskhud_engine::{
    EngineRegistry, HudFrame, HudInstanceId, HudLogicalRect, HudLogicalSize, HudSourceId, HudVisual,
};
use deskhud_ui::{CatalogStore, HudInstanceConfig, HudSlotLayout, UiPreferences};
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// Shadow blur is stored as a 0..=1 factor and displayed as 0..=24 px.
pub(crate) const DEFAULT_SHADOW_BLUR: f32 = 1.0 / 24.0;

/// HUD 内部子窗口的布局状态。
#[derive(Default)]
pub(crate) struct LayoutState {
    /// 布局编辑器活动画布中的 egui 逻辑坐标。
    pub(crate) positions: HashMap<String, egui::Pos2>,
    /// Layout-session positions in global physical screen pixels.
    /// Persisted slots are converted to/from these coordinates only when
    /// entering or leaving layout mode.
    pub(crate) absolute_positions: HashMap<String, egui::Pos2>,
    /// Exact rectangles painted in the most recent layout frame. These are
    /// the WYSIWYG source used when committing a layout session.
    pub(crate) rendered_rects: HashMap<String, egui::Rect>,
    /// Global physical pixel coordinate of the expanded layout canvas origin.
    pub(crate) activity_origin: Option<egui::Pos2>,
    /// Scale used to project physical screen pixels into the egui canvas.
    pub(crate) scale_factor: f32,
    /// Group sizes temporarily held stable while a member is being dragged.
    /// These must never be persisted as the group's manual size.
    pub(crate) transient_group_sizes: HashMap<String, egui::Vec2>,
    /// HUD currently being dragged in layout-window coordinates.
    ///
    /// A grouped HUD is detached when this state is created and is assigned
    /// back to a group (or to the screen) only after the drag finishes.
    pub(crate) active_hud_drag: Option<HudDragState>,
    /// Whether the preview surface is being dragged as a virtual root group.
    pub(crate) root_dragging: bool,
    /// 是否处于可拖动布局模式。
    pub(crate) layout_mode: bool,
    /// The in-canvas completion action requests the same transition as Escape.
    pub(crate) finish_layout_requested: bool,
    /// The canvas menu can close the editor while restoring its entry snapshot.
    pub(crate) discard_layout_requested: bool,
    /// 当前显示器活动区域的逻辑尺寸。
    pub(crate) activity_size: Option<egui::Vec2>,
    /// 是否等待下一帧切回紧凑窗口尺寸。
    pub(crate) compact_pending: bool,
    /// 当前高亮的 HUD 或组；右侧调节窗口绑定到此条目。
    pub(crate) selected: Option<String>,
    /// Whether the plugin/HUD switch tree is visible in this layout session.
    pub(crate) information_tree_open: bool,
    /// Whether the enabled instance/group navigation tree is visible.
    pub(crate) active_tree_open: bool,
    /// Tree panels in their current top-to-bottom opening order.
    pub(crate) tree_panel_order: Vec<&'static str>,
    /// Recreates tree windows when their ordered column changes.
    pub(crate) tree_window_revision: u64,
    /// Selection that the adjustment panels were last synchronized to.
    pub(crate) adjustment_selection: Option<String>,
    /// Adjustment panel kinds in the order in which they were opened.
    pub(crate) adjustment_order: Vec<&'static str>,
    /// Revision used to reset panel geometry when a panel is reopened.
    pub(crate) adjustment_window_revision: u64,
    /// Position/size snapshots captured when an adjustment panel opens.
    pub(crate) adjustment_reset_sizes: HashMap<String, egui::Vec2>,
    pub(crate) adjust_open: bool,
    pub(crate) group_adjust_open: bool,
    pub(crate) hud_adjust_open: bool,
    pub(crate) hud_adjust_key: Option<String>,
    pub(crate) group_adjust_key: Option<String>,
    pub(crate) shadow_open: bool,
    pub(crate) shadow_target: Option<ShadowTarget>,
    /// Whether layout positions should snap to the visible alignment grid.
    pub(crate) snap_to_grid: bool,
    /// Recreates HUD egui windows when entering a new editing session.
    /// Size changes are applied to existing windows to preserve position.
    pub(crate) window_revision: u64,
    pub(crate) adjust_session: u64,
    /// Whether layout editing should preserve the selected HUD aspect ratio.
    pub(crate) lock_ratio: bool,
    pub(crate) locked_ratio: Option<f32>,
}

pub(crate) struct HudDragState {
    pub(crate) instance_id: HudInstanceId,
    pub(crate) source_group_id: Option<String>,
    pub(crate) source_group_rect: Option<egui::Rect>,
    pub(crate) position: egui::Pos2,
    pub(crate) size: egui::Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowTarget {
    Global,
    Window,
    Content,
    Border,
    Background,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HudLayoutTarget {
    Instance(HudInstanceId),
    Group(String),
}

#[derive(Clone)]
pub(crate) struct HudRenderLayer {
    pub(crate) instance_id: HudInstanceId,
    pub(crate) source: HudSourceId,
    /// Names belonging to this member, even when rendered inside a group.
    pub(crate) plugin_name: String,
    pub(crate) contribution_name: String,
    pub(crate) config: HudInstanceConfig,
    pub(crate) base_size: HudLogicalSize,
    pub(crate) frame: HudFrame,
    pub(crate) rect: HudLogicalRect,
    pub(crate) clip: HudLogicalRect,
}

/// One virtual HUD slot after instance resolution and optional group composition.
#[derive(Clone)]
pub(crate) struct HudRenderItem {
    pub(crate) key: String,
    pub(crate) target: HudLayoutTarget,
    pub(crate) source: Option<HudSourceId>,
    /// User-facing plugin name resolved from the registry.
    pub(crate) plugin_name: String,
    /// User-facing contribution name resolved from the registry.
    pub(crate) contribution_name: String,
    pub(crate) layers: Vec<HudRenderLayer>,
    pub(crate) base_size: HudLogicalSize,
    /// Optional actual container size for virtual groups.
    pub(crate) container_size: Option<egui::Vec2>,
    pub(crate) initial_position: egui::Pos2,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) background_enabled: bool,
    pub(crate) background_opacity: f32,
    pub(crate) background_blur: f32,
    pub(crate) content_opacity: f32,
    pub(crate) shadow_enabled: bool,
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
    /// Layout-editor-only group identifier color.
    pub(crate) group_color: Option<[u8; 3]>,
    pub(crate) group_padding: Option<[f32; 4]>,
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
                config: &instance.config,
                locale: &prefs.locale.tag(),
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
    include_empty_groups: bool,
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
        let (plugin_name, contribution_name) =
            source_names(registry, frame.plugin_id, frame.contribution_id);
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
                instance_id: frame.instance_id.clone(),
                source: frame.source.clone(),
                plugin_name,
                contribution_name,
                config: frame.config.clone(),
                base_size: size,
                frame: frame.frame.clone(),
                rect,
                clip: rect,
            }],
            base_size: size,
            layout: frame.layout.clone(),
        });
    }

    for group in &prefs.hud.groups {
        if !group.enabled && !include_empty_groups {
            continue;
        }
        let members: Vec<_> = group
            .children
            .iter()
            .filter_map(|id| {
                let frame = by_instance.remove(id)?;
                let member_layout = frame.layout.clone();
                Some((frame, member_layout))
            })
            .collect();
        if members.is_empty() && !include_empty_groups {
            continue;
        }
        let measured: Vec<_> = members
            .iter()
            .map(|(member, _member_layout)| {
                let size = measured_frame_size(&member.frame);
                let instance_layout = prefs
                    .hud
                    .instances
                    .iter()
                    .find(|instance| instance.id == member.instance_id)
                    .map(|instance| &instance.layout);
                HudLogicalSize::new(
                    size.width * instance_layout.map(|layout| layout.width).unwrap_or(1.0),
                    size.height * instance_layout.map(|layout| layout.height).unwrap_or(1.0),
                )
            })
            .collect();
        let mut inner_layout = group.inner.clone();
        if group.layout.width > 0.0 && group.layout.height > 0.0 {
            let horizontal_limit = group.layout.width * 0.25;
            let vertical_limit = group.layout.height * 0.25;
            inner_layout.padding[0] = inner_layout.padding[0].min(vertical_limit).floor();
            inner_layout.padding[2] = inner_layout.padding[2].min(vertical_limit).floor();
            inner_layout.padding[1] = inner_layout.padding[1].min(horizontal_limit).floor();
            inner_layout.padding[3] = inner_layout.padding[3].min(horizontal_limit).floor();
        }
        // Groups currently have one layout mode: freely positioned members.
        // The persisted member rectangles are the source of truth.
        let frames = measured
            .iter()
            .zip(&members)
            .map(|(size, (_, member_layout))| HudLogicalRect {
                x: member_layout.x,
                y: member_layout.y,
                width: size.width,
                height: size.height,
            })
            .collect::<Vec<_>>();
        let composition = inner_layout.compose_free(&frames);
        let style_frame = HudFrame {
            visuals: members
                .iter()
                .flat_map(|(member, _)| member.frame.visuals.iter().cloned())
                .collect(),
        };
        let layers = members
            .into_iter()
            .zip(composition.members)
            .map(|((member, _), placement)| {
                let base_size = measured_frame_size(&member.frame);
                let (plugin_name, contribution_name) =
                    source_names(registry, member.plugin_id, member.contribution_id);
                HudRenderLayer {
                    instance_id: member.instance_id,
                    source: member.source,
                    plugin_name,
                    contribution_name,
                    config: member.config,
                    base_size,
                    frame: member.frame,
                    rect: placement.frame,
                    clip: placement.clip,
                }
            })
            .collect::<Vec<_>>();
        let base_size = if layers.is_empty() {
            HudLogicalSize::new(240.0, 140.0)
        } else {
            composition.size
        };
        slots.push(ResolvedHudSlot {
            key: format!("group/{}", group.id),
            target: HudLayoutTarget::Group(group.id.clone()),
            source: None,
            plugin_id: "hud.deskhud.group",
            contribution_id: "group",
            config: HudInstanceConfig::new(),
            frame: style_frame,
            layers,
            base_size,
            layout: group.layout.clone(),
        });
    }
    slots
}

fn source_names(
    registry: &EngineRegistry,
    plugin_id: &str,
    contribution_id: &str,
) -> (String, String) {
    let plugin_name = registry
        .plugin_infos()
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .map_or_else(
            || plugin_id.to_owned(),
            |plugin| plugin.display_name.to_owned(),
        );
    let contribution_name = registry
        .all_hud_contributions()
        .into_iter()
        .find(|(id, contribution)| *id == plugin_id && contribution.id == contribution_id)
        .map_or_else(
            || contribution_id.to_owned(),
            |(_, contribution)| contribution.label.to_owned(),
        );
    (plugin_name, contribution_name)
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
                let lines = text.lines().collect::<Vec<_>>();
                let line_count = lines.len().max(1) as f32;
                let longest = lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0) as f32;
                width = width.max(longest * font_size * 0.62 + 20.0);
                height = height.max(line_count * font_size * 1.25 + 20.0);
            }
            HudVisual::Label {
                text,
                x,
                y,
                font_size,
                align,
                ..
            } => {
                let longest = text
                    .lines()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0) as f32
                    * font_size
                    * 0.62;
                let left = match align {
                    deskhud_engine::HudTextAlign::Left => 0.0,
                    deskhud_engine::HudTextAlign::Center => longest * 0.5,
                    deskhud_engine::HudTextAlign::Right => longest,
                };
                width = width.max(x - left + longest);
                height = height.max(y + font_size * 0.65);
            }
            HudVisual::ProgressBar {
                x,
                y,
                width: visual_width,
                height: visual_height,
                ..
            }
            | HudVisual::LineChart {
                x,
                y,
                width: visual_width,
                height: visual_height,
                ..
            } => {
                width = width.max(x + visual_width);
                height = height.max(y + visual_height);
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
    registry: &EngineRegistry,
    catalogs: &CatalogStore,
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
                let result = drawing::draw(ui, time, layout, items, registry, catalogs, prefs);
                content_size = result.size;
                move_by = result.move_by;
                changed = result.changed;
            });
    });

    ViewOutput {
        full_output,
        // Normal mode is intentionally driven by the persisted window preset.
        // The activity canvas remains the only dynamically sized surface.
        resize_to: layout.layout_mode.then_some(content_size),
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
                config: &[],
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
        let initial = active_hud_frames(&bootstrap.registry, &prefs, 1.0)
            .into_iter()
            .filter(|frame| frame.source.plugin_id == "hud.deskhud.demo")
            .collect::<Vec<_>>();
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
        assert_eq!(
            active_hud_frames(&bootstrap.registry, &prefs, 1.0)
                .iter()
                .filter(|frame| frame.source.plugin_id == "hud.deskhud.demo")
                .count(),
            2
        );

        prefs.hud.set_plugin_enabled("hud.deskhud.demo", false);
        assert!(
            active_hud_frames(&bootstrap.registry, &prefs, 1.0)
                .iter()
                .all(|frame| frame.source.plugin_id != "hud.deskhud.demo")
        );
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
                .filter(|(plugin, _)| {
                    *plugin == "hud.deskhud.demo" || *plugin == "hud.example.other"
                })
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
                .filter(|(plugin, _)| {
                    *plugin == "hud.deskhud.demo" || *plugin == "hud.example.other"
                })
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
        prefs.hud.groups[0].inner.arrangement = HudGroupArrangement::Free;
        prefs.hud.groups[0].inner.spacing = 7.0;
        for (index, instance_id) in prefs.hud.groups[0].children.iter().enumerate() {
            let instance = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| &instance.id == instance_id)
                .unwrap();
            instance.layout.x = index as f32 * 40.0;
            instance.layout.y = index as f32 * 24.0;
        }

        let slots = resolved_hud_slots(&bootstrap.registry, &prefs, 1.0, false);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].target, HudLayoutTarget::Group(group_id.clone()));
        assert_eq!(slots[0].key, format!("group/{group_id}"));
        assert_eq!(slots[0].layers.len(), 3);
        assert!(slots[0].layers[1].rect.y > slots[0].layers[0].rect.y);
        assert_eq!(slots[0].layers[0].clip, slots[0].layers[0].rect);
        assert_eq!(slots[0].layers[1].clip, slots[0].layers[1].rect);
        assert_eq!(slots[0].layout, prefs.hud.groups[0].layout);
    }

    #[test]
    fn free_group_uses_persisted_member_geometry() {
        let bootstrap = deskhud_runtime::bootstrap_registry();
        let mut prefs = deskhud_ui::UiPreferences::default();
        prefs.hud.ensure_default_instances(
            bootstrap
                .registry
                .all_hud_contributions()
                .into_iter()
                .filter(|(plugin, _)| *plugin == "hud.deskhud.demo")
                .map(|(plugin, contribution)| {
                    (
                        deskhud_engine::HudSourceId::new(plugin, contribution.id),
                        true,
                    )
                }),
        );
        let member = prefs.hud.instances[0].id.clone();
        prefs.hud.instances[0].layout.width = 1.5;
        let group_id = prefs.hud.create_group("free");
        prefs.hud.add_instance_to_group(&group_id, &member);
        let group = &mut prefs.hud.groups[0];
        group.inner.arrangement = HudGroupArrangement::Free;
        group.layout.width = 4096.0;
        group.layout.height = 4096.0;
        prefs.hud.instances[0].layout.x = 42.0;
        prefs.hud.instances[0].layout.y = 17.0;

        let slots = resolved_hud_slots(&bootstrap.registry, &prefs, 1.0, true);
        assert_eq!(slots.len(), 2);
        let group = slots
            .iter()
            .find(|slot| slot.target == HudLayoutTarget::Group(group_id.clone()))
            .expect("group slot");
        assert_eq!(group.layers.len(), 1);
        assert!(group.layers[0].rect.x >= 42.0);
        assert!(group.layers[0].rect.y >= 17.0);
        assert!(group.layers[0].rect.width > group.layers[0].base_size.width);
        // Free layout derives the container from member bounds; a persisted
        // manual size must not override that calculation.
        assert!(group.base_size.width < 4096.0);
        assert!(group.base_size.height < 4096.0);
    }

    #[test]
    fn empty_groups_only_produce_editor_slots() {
        let bootstrap = deskhud_runtime::bootstrap_registry();
        let mut prefs = deskhud_ui::UiPreferences::default();
        let group_id = prefs.hud.create_group("empty");
        assert!(resolved_hud_slots(&bootstrap.registry, &prefs, 1.0, false).is_empty());
        let slots = resolved_hud_slots(&bootstrap.registry, &prefs, 1.0, true);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].target, HudLayoutTarget::Group(group_id));
        assert!(slots[0].layers.is_empty());
    }
}
