//! Persisted HUD instances and groups plus their recovery rules.

use std::collections::{HashMap, HashSet};

use deskhud_engine::{HudGroupLayout, HudInstanceId, HudSourceId};
use serde::{Deserialize, Serialize};

use super::{HudConfigValue, HudSlotLayout};

/// Per-instance values owned by the host rather than by a contribution definition.
pub type HudInstanceConfig = HashMap<String, HudConfigValue>;

/// One stable, independently configurable occurrence of a HUD contribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudInstance {
    /// Host-generated stable identity. Titles are never used as keys.
    pub id: HudInstanceId,
    /// Plugin contribution used to render this instance.
    pub source: HudSourceId,
    /// Instance-level enable switch.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Values private to this instance.
    #[serde(default)]
    pub config: HudInstanceConfig,
    /// Screen layout used while the instance is not in a group.
    #[serde(default)]
    pub layout: HudSlotLayout,
}

/// A group whose ordered children can come from different plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudGroup {
    /// Stable host-generated group identity.
    pub id: String,
    /// User-editable label; never used as an identity.
    #[serde(default)]
    pub name: String,
    /// Group-level enable switch.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// User-selected identifier color used only by the layout editor.
    #[serde(default = "default_group_color")]
    pub color: [u8; 3],
    /// Screen position and actual pixel size of the group as one virtual HUD slot.
    #[serde(default)]
    pub layout: HudSlotLayout,
    /// Arrangement, spacing, padding, grid columns and alignment inside the group.
    #[serde(default)]
    pub inner: HudGroupLayout,
    /// Ordered instance identities. An instance may occur in at most one group.
    #[serde(default)]
    pub children: Vec<HudInstanceId>,
}

/// Counts of independently recovered records after loading untrusted preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HudRecoveryReport {
    /// Invalid or duplicate instances that were ignored.
    pub removed_instances: usize,
    /// Invalid or duplicate groups that were ignored.
    pub removed_groups: usize,
    /// Missing or repeated member references that were ignored.
    pub removed_members: usize,
}

impl super::HudPrefs {
    /// Collision-free deterministic ID used by the legacy definition-to-instance mapping.
    pub fn default_instance_id(source: &HudSourceId) -> HudInstanceId {
        HudInstanceId::new(format!(
            "default:{}:{}{}:{}",
            source.plugin_id.len(),
            source.plugin_id,
            source.contribution_id.len(),
            source.contribution_id
        ))
    }

    /// Idempotently maps legacy contribution switches, layout and visual values to defaults.
    ///
    /// A missing plugin does not delete an existing instance. When it returns, the same
    /// deterministic ID restores its relationship. Explicitly deleted defaults are recorded
    /// in `suppressed_default_sources` and are not silently recreated.
    pub fn ensure_default_instances<I>(&mut self, sources: I) -> usize
    where
        I: IntoIterator<Item = (HudSourceId, bool)>,
    {
        let mut created = 0;
        for (index, (source, default_enabled)) in sources.into_iter().enumerate() {
            if !source.is_valid()
                || self
                    .instances
                    .iter()
                    .any(|instance| instance.source == source)
                || self.suppressed_default_sources.contains(&source)
            {
                continue;
            }
            let enabled =
                self.is_enabled(&source.plugin_id, &source.contribution_id, default_enabled);
            let layout = self.slot_layout(&source.plugin_id, &source.contribution_id, index);
            let config = self.legacy_instance_config(&source);
            self.clear_legacy_layout_keys(&source.plugin_id, &source.contribution_id);
            self.instances.push(HudInstance {
                id: Self::default_instance_id(&source),
                source,
                enabled,
                config,
                layout,
            });
            created += 1;
        }
        created
    }

    /// Creates a new non-default instance with the next unused host ID.
    pub fn create_instance(&mut self, source: HudSourceId, enabled: bool) -> Option<HudInstanceId> {
        if !source.is_valid() {
            return None;
        }
        let id = self.next_instance_id();
        self.instances.push(HudInstance {
            id: id.clone(),
            source,
            enabled,
            config: HashMap::new(),
            layout: HudSlotLayout::default_for_index(self.instances.len()),
        });
        Some(id)
    }

    /// Copies an instance into an ungrouped instance with a newly allocated identity.
    pub fn copy_instance(&mut self, id: &HudInstanceId) -> Option<HudInstanceId> {
        let mut copy = self
            .instances
            .iter()
            .find(|instance| &instance.id == id)?
            .clone();
        copy.id = self.next_instance_id();
        let new_id = copy.id.clone();
        self.instances.push(copy);
        Some(new_id)
    }

    /// Deletes exactly one instance and removes all group references to it.
    pub fn delete_instance(&mut self, id: &HudInstanceId) -> bool {
        let Some(index) = self
            .instances
            .iter()
            .position(|instance| &instance.id == id)
        else {
            return false;
        };
        let removed = self.instances.remove(index);
        if id == &Self::default_instance_id(&removed.source)
            && !self.suppressed_default_sources.contains(&removed.source)
        {
            self.suppressed_default_sources.push(removed.source);
        }
        for group in &mut self.groups {
            group.children.retain(|child| child != id);
        }
        true
    }

    /// Creates an empty group. Empty/orphan groups remain valid and persist.
    pub fn create_group(&mut self, name: impl Into<String>) -> String {
        let id = self.next_group_id();
        let mut layout = HudSlotLayout::default_for_index(self.groups.len());
        // A new group starts with content-sized geometry. Once the user
        // adjusts it, layout.width/height become the fixed pixel container.
        layout.width = 0.0;
        layout.height = 0.0;
        self.groups.push(HudGroup {
            id: id.clone(),
            name: name.into(),
            enabled: true,
            color: default_group_color(),
            layout,
            inner: HudGroupLayout::default(),
            children: Vec::new(),
        });
        id
    }

    /// Deletes a group while keeping all member instances as ungrouped HUDs.
    ///
    /// Group members store coordinates relative to their group. Reset their position
    /// before removing the group so the persisted coordinates remain valid when the
    /// instances are rendered as top-level HUDs after a restart.
    pub fn delete_group(&mut self, id: &str) -> bool {
        let Some(group_index) = self.groups.iter().position(|group| group.id == id) else {
            return false;
        };
        let children = self.groups.remove(group_index).children;
        for child in children {
            let Some(instance_index) = self
                .instances
                .iter()
                .position(|instance| instance.id == child)
            else {
                continue;
            };
            let default = HudSlotLayout::default_for_index(instance_index);
            let instance = &mut self.instances[instance_index];
            instance.layout.x = default.x;
            instance.layout.y = default.y;
        }
        true
    }

    /// Repairs malformed records independently, preserving unavailable plugin sources.
    ///
    /// Duplicate identities and memberships keep their first occurrence. Dangling member
    /// references are removed, but the now-empty group itself is retained.
    pub fn recover(&mut self) -> HudRecoveryReport {
        let mut report = HudRecoveryReport::default();
        let mut instance_ids = HashSet::new();
        self.instances.retain_mut(|instance| {
            let valid = instance.id.is_valid()
                && instance.source.is_valid()
                && instance_ids.insert(instance.id.clone());
            if valid {
                instance.layout = instance.layout.clone().clamp01();
            } else {
                report.removed_instances += 1;
            }
            valid
        });

        let mut group_ids = HashSet::new();
        self.groups.retain_mut(|group| {
            let valid = valid_text_id(&group.id) && group_ids.insert(group.id.clone());
            if valid {
                group.layout = group.layout.clone().clamp_position();
                group.layout.x = group.layout.x.max(0.0);
                group.layout.y = group.layout.y.max(0.0);
                if !group.layout.width.is_finite() || group.layout.width <= 0.0 {
                    group.layout.width = 0.0;
                }
                if !group.layout.height.is_finite() || group.layout.height <= 0.0 {
                    group.layout.height = 0.0;
                }
                group.inner = group.inner.clone().normalized();
            } else {
                report.removed_groups += 1;
            }
            valid
        });

        let mut assigned = HashSet::new();
        for group in &mut self.groups {
            group.children.retain(|child| {
                let keep = instance_ids.contains(child) && assigned.insert(child.clone());
                if !keep {
                    report.removed_members += 1;
                }
                keep
            });
        }
        let mut suppressed = HashSet::new();
        self.suppressed_default_sources
            .retain(|source| source.is_valid() && suppressed.insert(source.clone()));
        report
    }

    /// Moves an instance into a group, removing any previous group membership.
    pub fn add_instance_to_group(&mut self, group_id: &str, instance_id: &HudInstanceId) -> bool {
        if !self
            .instances
            .iter()
            .any(|instance| &instance.id == instance_id)
            || !self.groups.iter().any(|group| group.id == group_id)
        {
            return false;
        }
        for group in &mut self.groups {
            group.children.retain(|child| child != instance_id);
        }
        let group = self
            .groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .expect("validated group must remain present");
        group.children.push(instance_id.clone());
        true
    }

    /// Removes an instance from its group while retaining the instance itself.
    pub fn remove_instance_from_group(&mut self, instance_id: &HudInstanceId) -> bool {
        let mut removed = false;
        for group in &mut self.groups {
            let before = group.children.len();
            group.children.retain(|child| child != instance_id);
            removed |= before != group.children.len();
        }
        removed
    }

    fn legacy_instance_config(&self, source: &HudSourceId) -> HudInstanceConfig {
        let prefix = format!("{}.{}.", source.plugin_id, source.contribution_id);
        const RESERVED: &[&str] = &[
            "enable", "display", "position", "x", "y", "size", "width", "height",
        ];
        self.config
            .iter()
            .filter_map(|(key, value)| {
                let name = key.strip_prefix(&prefix)?;
                (!RESERVED.contains(&name)).then(|| (name.to_string(), value.clone()))
            })
            .collect()
    }

    fn next_instance_id(&self) -> HudInstanceId {
        for sequence in 1_u64.. {
            let candidate = HudInstanceId::new(format!("instance:{sequence}"));
            if self
                .instances
                .iter()
                .all(|instance| instance.id != candidate)
            {
                return candidate;
            }
        }
        unreachable!("finite preferences cannot exhaust all u64 instance IDs")
    }

    fn next_group_id(&self) -> String {
        for sequence in 1_u64.. {
            let candidate = format!("group:{sequence}");
            if self.groups.iter().all(|group| group.id != candidate) {
                return candidate;
            }
        }
        unreachable!("finite preferences cannot exhaust all u64 group IDs")
    }
}

fn default_enabled() -> bool {
    true
}

fn default_group_color() -> [u8; 3] {
    [86, 156, 255]
}

fn valid_text_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HudPrefs;

    fn source(plugin: &str, contribution: &str) -> HudSourceId {
        HudSourceId::new(plugin, contribution)
    }

    #[test]
    fn legacy_mapping_is_deterministic_and_idempotent() {
        let mut prefs = HudPrefs::default();
        prefs.set_enabled("hud.deskhud.demo", "clock", false);
        prefs.set_slot_layout(
            "hud.deskhud.demo",
            "clock",
            HudSlotLayout {
                x: 0.4,
                ..HudSlotLayout::default()
            },
        );
        prefs.set_visual_value("hud.deskhud.demo", "clock", "background_opacity", 0.25);
        let source = source("hud.deskhud.demo", "clock");

        assert_eq!(prefs.ensure_default_instances([(source.clone(), true)]), 1);
        assert_eq!(prefs.ensure_default_instances([(source.clone(), true)]), 0);
        let instance = &prefs.instances[0];
        assert_eq!(instance.id, HudPrefs::default_instance_id(&source));
        assert!(!instance.enabled);
        assert!((instance.layout.x - 0.4).abs() < 1e-5);
        assert_eq!(
            instance.config.get("background_opacity"),
            Some(&HudConfigValue::Float(0.25))
        );
    }

    #[test]
    fn deleting_default_suppresses_recreation_and_copy_is_independent() {
        let mut prefs = HudPrefs::default();
        let source = source("hud.deskhud.demo", "clock");
        prefs.ensure_default_instances([(source.clone(), true)]);
        let default_id = prefs.instances[0].id.clone();
        let copy_id = prefs.copy_instance(&default_id).expect("copy");
        assert_ne!(copy_id, default_id);
        assert!(prefs.delete_instance(&default_id));
        assert_eq!(prefs.ensure_default_instances([(source.clone(), true)]), 0);
        assert_eq!(prefs.instances.len(), 1);
        assert_eq!(prefs.instances[0].id, copy_id);
    }

    #[test]
    fn recovery_keeps_missing_sources_and_first_group_membership() {
        let mut prefs = HudPrefs::default();
        let missing = source("hud.missing.plugin", "gone");
        prefs.ensure_default_instances([(missing.clone(), true)]);
        let id = prefs.instances[0].id.clone();
        let first = prefs.create_group("First");
        let second = prefs.create_group("Second");
        prefs.groups[0].children = vec![id.clone(), id.clone()];
        prefs.groups[1].children = vec![id.clone(), HudInstanceId::new("missing")];

        let report = prefs.recover();
        assert_eq!(report.removed_members, 3);
        assert_eq!(prefs.instances[0].source, missing);
        assert_eq!(prefs.groups[0].id, first);
        assert_eq!(prefs.groups[0].children, vec![id]);
        assert_eq!(prefs.groups[1].id, second);
        assert!(prefs.groups[1].children.is_empty());
    }

    #[test]
    fn moving_members_between_groups_keeps_one_owner() {
        let mut prefs = HudPrefs::default();
        let source = source("hud.deskhud.demo", "clock");
        prefs.ensure_default_instances([(source, true)]);
        let instance_id = prefs.instances[0].id.clone();
        let first = prefs.create_group("First");
        let second = prefs.create_group("Second");

        assert!(prefs.add_instance_to_group(&first, &instance_id));
        assert_eq!(prefs.groups[0].children, vec![instance_id.clone()]);
        assert!(prefs.add_instance_to_group(&second, &instance_id));
        assert!(prefs.groups[0].children.is_empty());
        assert_eq!(prefs.groups[1].children, vec![instance_id.clone()]);
        assert_eq!(prefs.instances[0].layout.x, 8.0);
        assert!(prefs.remove_instance_from_group(&instance_id));
        assert!(prefs.groups.iter().all(|group| group.children.is_empty()));
    }

    #[test]
    fn deleting_group_resets_member_position_for_top_level_rendering() {
        let mut prefs = HudPrefs::default();
        let source = source("hud.deskhud.demo", "clock");
        prefs.ensure_default_instances([(source, true)]);
        prefs.instances[0].layout.x = 240.0;
        prefs.instances[0].layout.y = 180.0;
        let instance_id = prefs.instances[0].id.clone();
        let group_id = prefs.create_group("Group");
        assert!(prefs.add_instance_to_group(&group_id, &instance_id));

        assert!(prefs.delete_group(&group_id));
        assert!(prefs.groups.is_empty());
        assert_eq!(prefs.instances[0].layout.x, 8.0);
        assert_eq!(prefs.instances[0].layout.y, 8.0);
    }

    #[test]
    fn visual_overrides_are_owned_by_the_instance() {
        let mut prefs = HudPrefs::default();
        let source = source("hud.deskhud.demo", "clock");
        prefs.ensure_default_instances([(source, true)]);
        let first = prefs.instances[0].id.clone();
        let second = prefs.copy_instance(&first).expect("copy");

        assert!(prefs.set_instance_visual_value(&first, "content_opacity", 0.25));
        assert_eq!(
            prefs.instance_visual_value(&first, "content_opacity", 1.0),
            0.25
        );
        assert_eq!(
            prefs.instance_visual_value(&second, "content_opacity", 1.0),
            1.0
        );
    }
}
