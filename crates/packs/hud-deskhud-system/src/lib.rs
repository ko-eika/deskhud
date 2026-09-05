//! Cached, platform-neutral system and application performance HUDs.

mod sampler;

use std::{collections::HashMap, sync::Mutex, time::Instant};

use deskhud_engine::{
    HudConfigChoice, HudConfigDynamicChoice, HudConfigKind, HudConfigOption, HudContribution,
    HudFrame, HudFrameCtx, HudTextAlign, HudVisual, Plugin, PluginInfo,
};
use sampler::{ApplicationState, SampleRequest, Sampler};

const UNIT_CHOICES: &[HudConfigChoice] = &[
    HudConfigChoice {
        value: "binary",
        label: "Binary (GiB/MiB)",
    },
    HudConfigChoice {
        value: "decimal",
        label: "Decimal (GB/MB)",
    },
];

const REFRESH_OPTION: HudConfigOption = HudConfigOption {
    key: "refresh_seconds",
    label: "Refresh interval",
    description: "Sampling interval in seconds",
    kind: HudConfigKind::Number {
        default: 1.0,
        min: 0.5,
        max: 30.0,
        step: 0.5,
    },
};
const UNIT_OPTION: HudConfigOption = HudConfigOption {
    key: "unit",
    label: "Memory unit",
    description: "Binary or decimal memory units",
    kind: HudConfigKind::Choice {
        default: "binary",
        choices: UNIT_CHOICES,
    },
};
const SHOW_LABELS_OPTION: HudConfigOption = HudConfigOption {
    key: "show_labels",
    label: "Show labels",
    description: "Show metric names beside values",
    kind: HudConfigKind::Bool { default: true },
};

const SYSTEM_CONFIG: &[HudConfigOption] = &[REFRESH_OPTION, UNIT_OPTION, SHOW_LABELS_OPTION];
const PROCESS_CONFIG: &[HudConfigOption] = &[REFRESH_OPTION, UNIT_OPTION, SHOW_LABELS_OPTION];

const APPLICATION_CONFIG: &[HudConfigOption] = &[
    REFRESH_OPTION,
    UNIT_OPTION,
    SHOW_LABELS_OPTION,
    HudConfigOption {
        key: "process_name",
        label: "Application",
        description: "Choose a running application process",
        kind: HudConfigKind::DynamicChoice { default: "" },
    },
];

/// Official built-in performance plugin.
pub struct SystemHudPlugin {
    sampler: Sampler,
    frame_rates: Mutex<HashMap<String, FrameRateSample>>,
}

struct FrameRateSample {
    window_start: Instant,
    frames: u32,
    smoothed: f32,
}

impl Default for SystemHudPlugin {
    fn default() -> Self {
        Self {
            sampler: Sampler::new(),
            frame_rates: Mutex::new(HashMap::new()),
        }
    }
}

impl Plugin for SystemHudPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hud.deskhud.system",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "System Monitor",
            description: "Cached real-time system and application performance metrics",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            icon: Some(include_bytes!("../assets/icon.svg")),
        }
    }

    fn hud_contributions(&self) -> &'static [HudContribution] {
        const ITEMS: &[HudContribution] = &[
            HudContribution {
                id: "system_cpu",
                label: "System status",
                default_enabled: true,
                icon: Some(include_bytes!("../assets/icon_overview.svg")),
                config: SYSTEM_CONFIG,
            },
            HudContribution {
                id: "deskhud",
                label: "Current application process status",
                default_enabled: true,
                icon: Some(include_bytes!("../assets/icon_deskhud.svg")),
                config: PROCESS_CONFIG,
            },
            HudContribution {
                id: "application",
                label: "Selected application process status",
                default_enabled: false,
                icon: Some(include_bytes!("../assets/icon_application.svg")),
                config: APPLICATION_CONFIG,
            },
        ];
        ITEMS
    }

    fn hud_config_choices(
        &self,
        contribution_id: &str,
        option_key: &str,
    ) -> Vec<HudConfigDynamicChoice> {
        if contribution_id != "application" || option_key != "process_name" {
            return Vec::new();
        }
        self.sampler
            .process_names()
            .into_iter()
            .map(|name| HudConfigDynamicChoice {
                value: name.clone(),
                label: name,
            })
            .collect()
    }

    fn hud_frame_for_instance(&self, ctx: &HudFrameCtx<'_>) -> HudFrame {
        let request =
            SampleRequest::from_config(ctx.config, ctx.source.contribution_id == "application");
        self.sampler.request(ctx.instance_id.as_str(), request);
        let snapshot = self.sampler.snapshot();
        let zh = ctx.locale.starts_with("zh");
        let labels = config_bool(ctx, "show_labels", true);
        let binary = config_text(ctx, "unit", "binary") == "binary";
        match ctx.source.contribution_id.as_str() {
            "system_cpu" => system_frame(&snapshot, zh, labels, binary),
            "deskhud" => deskhud_frame(
                &snapshot,
                zh,
                labels,
                binary,
                self.measure_frame_rate(ctx.instance_id.as_str()),
            ),
            "application" => application_frame(&snapshot, zh, labels, binary),
            _ => HudFrame::empty(),
        }
    }
}

impl SystemHudPlugin {
    fn measure_frame_rate(&self, instance_id: &str) -> f32 {
        let now = Instant::now();
        let Ok(mut samples) = self.frame_rates.lock() else {
            return 0.0;
        };
        let sample = samples
            .entry(instance_id.to_owned())
            .or_insert(FrameRateSample {
                window_start: now,
                frames: 0,
                smoothed: 0.0,
            });
        sample.frames = sample.frames.saturating_add(1);
        let elapsed = now.duration_since(sample.window_start).as_secs_f32();
        if elapsed >= 0.5 {
            let current = (sample.frames as f32 / elapsed).clamp(0.0, 999.0);
            sample.smoothed = if sample.smoothed <= f32::EPSILON {
                current
            } else {
                sample.smoothed * 0.8 + current * 0.2
            };
            sample.window_start = now;
            sample.frames = 0;
        }
        sample.smoothed
    }
}

fn system_frame(
    snapshot: &sampler::SystemSnapshot,
    zh: bool,
    labels: bool,
    binary: bool,
) -> HudFrame {
    let memory_ratio = if snapshot.memory_total == 0 {
        0.0
    } else {
        snapshot.memory_used as f32 / snapshot.memory_total as f32
    };
    let memory_value = if snapshot.memory_total == 0 {
        if zh { "暂不可用" } else { "Unavailable" }.to_owned()
    } else {
        format_memory_pair(snapshot.memory_used, snapshot.memory_total, binary)
    };
    let footer_left = if zh {
        format!(
            "{} 个进程 · 已运行 {}",
            snapshot.process_count,
            format_duration(snapshot.uptime_seconds, true)
        )
    } else {
        format!(
            "{} processes · up {}",
            snapshot.process_count,
            format_duration(snapshot.uptime_seconds, false)
        )
    };
    let footer_right = if snapshot.swap_total == 0 {
        String::new()
    } else if zh {
        format!(
            "交换 {}/{}",
            format_bytes(snapshot.swap_used, binary),
            format_bytes(snapshot.swap_total, binary)
        )
    } else {
        format!(
            "Swap {}/{}",
            format_bytes(snapshot.swap_used, binary),
            format_bytes(snapshot.swap_total, binary)
        )
    };
    status_frame(StatusFrameSpec {
        title: if zh { "系统状态" } else { "System status" },
        cpu: snapshot.system_cpu,
        memory_text: memory_value,
        memory_ratio,
        footer_left,
        footer_right,
        header_right: String::new(),
        history: &snapshot.cpu_history,
        zh,
        labels,
    })
}

fn deskhud_frame(
    snapshot: &sampler::SystemSnapshot,
    zh: bool,
    labels: bool,
    binary: bool,
    frames_per_second: f32,
) -> HudFrame {
    let memory = format_bytes(snapshot.deskhud.memory_bytes.unwrap_or(0), binary);
    let memory_ratio = snapshot
        .deskhud
        .memory_bytes
        .filter(|_| snapshot.memory_total > 0)
        .map_or(0.0, |used| used as f32 / snapshot.memory_total as f32);
    let footer_left = if snapshot.deskhud.name.is_empty() {
        "DeskHud".to_owned()
    } else if let Some(pid) = snapshot.deskhud.pid {
        format!("{} · PID {pid}", snapshot.deskhud.name)
    } else {
        snapshot.deskhud.name.clone()
    };
    status_frame(StatusFrameSpec {
        title: if zh {
            "当前应用进程状态"
        } else {
            "Current application status"
        },
        cpu: snapshot.deskhud.cpu_percent,
        memory_text: memory,
        memory_ratio,
        footer_left,
        footer_right: if zh {
            "当前进程"
        } else {
            "Current process"
        }
        .to_owned(),
        header_right: format!("{frames_per_second:.0} FPS"),
        history: &snapshot.deskhud_history,
        zh,
        labels,
    })
}

fn application_frame(
    snapshot: &sampler::SystemSnapshot,
    zh: bool,
    labels: bool,
    binary: bool,
) -> HudFrame {
    let (cpu, memory, memory_ratio, footer_left, footer_right) = match &snapshot.application {
        ApplicationState::NotSelected => (
            None,
            "—".to_owned(),
            0.0,
            if zh {
                "请选择监控应用"
            } else {
                "Select an application"
            }
            .to_owned(),
            String::new(),
        ),
        ApplicationState::NotRunning { selector } => (
            None,
            "—".to_owned(),
            0.0,
            selector.clone(),
            if zh { "未运行" } else { "Not running" }.to_owned(),
        ),
        ApplicationState::Running {
            name,
            pid,
            cpu_percent,
            memory_bytes,
            matches,
        } => {
            let duplicate = if *matches > 1 {
                if zh {
                    format!(" · {} 个同名", matches)
                } else {
                    format!(" · {matches} matches")
                }
            } else {
                String::new()
            };
            (
                Some(*cpu_percent),
                format_bytes(*memory_bytes, binary),
                if snapshot.memory_total == 0 {
                    0.0
                } else {
                    *memory_bytes as f32 / snapshot.memory_total as f32
                },
                format!("{name} · PID {pid}"),
                duplicate.trim_start_matches(" · ").to_owned(),
            )
        }
    };
    status_frame(StatusFrameSpec {
        title: if zh {
            "应用进程状态"
        } else {
            "Application process status"
        },
        cpu,
        memory_text: memory,
        memory_ratio,
        footer_left,
        footer_right,
        header_right: String::new(),
        history: &snapshot.application_history,
        zh,
        labels,
    })
}

struct StatusFrameSpec<'a> {
    title: &'a str,
    cpu: Option<f32>,
    memory_text: String,
    memory_ratio: f32,
    footer_left: String,
    footer_right: String,
    header_right: String,
    history: &'a [f32],
    zh: bool,
    labels: bool,
}

fn status_frame(spec: StatusFrameSpec<'_>) -> HudFrame {
    let cpu = spec.cpu.unwrap_or(0.0);
    let cpu_text = spec.cpu.map_or_else(
        || {
            if spec.zh {
                "暂不可用"
            } else {
                "Unavailable"
            }
            .to_owned()
        },
        |value| format!("{value:.1}%"),
    );
    HudFrame {
        visuals: vec![
            HudVisual::Panel {
                width: 360.0,
                height: 164.0,
                radius: 16.0,
                color: [20, 27, 39, 244],
            },
            HudVisual::label(
                if spec.labels { spec.title } else { "" },
                18.0,
                20.0,
                HudTextAlign::Left,
                13.0,
                [248, 248, 252, 255],
            ),
            HudVisual::label(
                spec.header_right,
                342.0,
                20.0,
                HudTextAlign::Right,
                11.0,
                [248, 248, 252, 255],
            ),
            HudVisual::label(
                if spec.labels {
                    format!("CPU  {cpu_text}")
                } else {
                    cpu_text
                },
                18.0,
                50.0,
                HudTextAlign::Left,
                20.0,
                [248, 248, 252, 255],
            ),
            HudVisual::label(
                if spec.labels {
                    format!(
                        "{}  {} · {:.1}%",
                        if spec.zh { "内存" } else { "Memory" },
                        spec.memory_text,
                        spec.memory_ratio * 100.0
                    )
                } else {
                    format!("{} · {:.1}%", spec.memory_text, spec.memory_ratio * 100.0)
                },
                342.0,
                50.0,
                HudTextAlign::Right,
                11.0,
                [248, 248, 252, 255],
            ),
            HudVisual::line_chart(
                18.0,
                68.0,
                324.0,
                36.0,
                spec.history.iter().copied(),
                0.0,
                100.0,
                1.75,
                [86, 181, 255, 235],
            ),
            HudVisual::progress_bar(
                18.0,
                113.0,
                324.0,
                5.0,
                2.5,
                cpu / 100.0,
                [49, 61, 81, 190],
                [74, 167, 255, 255],
            ),
            HudVisual::progress_bar(
                18.0,
                126.0,
                324.0,
                5.0,
                2.5,
                spec.memory_ratio,
                [49, 61, 81, 190],
                [147, 105, 255, 255],
            ),
            HudVisual::label(
                spec.footer_left,
                18.0,
                151.0,
                HudTextAlign::Left,
                10.0,
                [248, 248, 252, 255],
            ),
            HudVisual::label(
                spec.footer_right,
                342.0,
                151.0,
                HudTextAlign::Right,
                10.0,
                [248, 248, 252, 255],
            ),
        ],
    }
}

fn config_bool(ctx: &HudFrameCtx<'_>, key: &str, default: bool) -> bool {
    ctx.config
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}
fn config_text<'a>(ctx: &'a HudFrameCtx<'_>, key: &str, default: &'a str) -> &'a str {
    ctx.config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
}
fn format_memory_pair(used: u64, total: u64, binary: bool) -> String {
    format!(
        "{} / {}",
        format_bytes(used, binary),
        format_bytes(total, binary)
    )
}
fn format_bytes(bytes: u64, binary: bool) -> String {
    let (base, labels) = if binary {
        (1024.0, ["B", "KiB", "MiB", "GiB", "TiB"])
    } else {
        (1000.0, ["B", "KB", "MB", "GB", "TB"])
    };
    let mut value = bytes as f64;
    let mut index = 0;
    while value >= base && index + 1 < labels.len() {
        value /= base;
        index += 1;
    }
    if index == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", labels[index])
    }
}

fn format_duration(seconds: u64, zh: bool) -> String {
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    if days > 0 {
        if zh {
            format!("{days}天{hours}小时")
        } else {
            format!("{days}d {hours}h")
        }
    } else if zh {
        format!("{hours}小时")
    } else {
        format!("{hours}h")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_independent_metric_huds() {
        let plugin = SystemHudPlugin::default();
        let ids = plugin
            .hud_contributions()
            .iter()
            .map(|contribution| contribution.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["system_cpu", "deskhud", "application"]);
        assert!(matches!(
            plugin.hud_contributions()[2].config[3].kind,
            HudConfigKind::DynamicChoice { .. }
        ));
    }

    #[test]
    fn byte_units_are_explicit() {
        assert_eq!(format_bytes(1_073_741_824, true), "1.0 GiB");
        assert_eq!(format_bytes(1_000_000_000, false), "1.0 GB");
    }

    #[test]
    fn system_card_combines_cpu_memory_and_host_details() {
        let frame = system_frame(
            &sampler::SystemSnapshot {
                system_cpu: Some(25.0),
                memory_used: 8 * 1024 * 1024 * 1024,
                memory_total: 16 * 1024 * 1024 * 1024,
                swap_used: 1024 * 1024 * 1024,
                swap_total: 4 * 1024 * 1024 * 1024,
                process_count: 128,
                uptime_seconds: 90_000,
                cpu_history: vec![10.0, 25.0],
                ..Default::default()
            },
            false,
            true,
            true,
        );
        assert_eq!(
            frame
                .visuals
                .iter()
                .filter(|visual| matches!(visual, HudVisual::ProgressBar { .. }))
                .count(),
            2
        );
        assert!(frame.visuals.iter().any(|visual| matches!(
            visual,
            HudVisual::Panel { width, height, .. } if *width == 360.0 && *height == 164.0
        )));
        assert!(
            frame
                .visuals
                .iter()
                .any(|visual| matches!(visual, HudVisual::LineChart { .. }))
        );
        assert!(frame.visuals.iter().any(
            |visual| matches!(visual, HudVisual::Label { text, .. } if text.contains("128 processes"))
        ));
    }

    #[test]
    fn current_application_card_shows_render_frame_rate() {
        let frame = deskhud_frame(
            &sampler::SystemSnapshot {
                memory_total: 16 * 1024 * 1024 * 1024,
                deskhud: sampler::ProcessMetric {
                    name: "deskhud-egui.exe".to_owned(),
                    pid: Some(42),
                    cpu_percent: Some(12.0),
                    memory_bytes: Some(512 * 1024 * 1024),
                },
                ..Default::default()
            },
            false,
            true,
            true,
            60.0,
        );
        assert!(
            frame
                .visuals
                .iter()
                .any(|visual| matches!(visual, HudVisual::Label { text, .. } if text == "60 FPS"))
        );
        assert!(frame.visuals.iter().any(|visual| matches!(
            visual,
            HudVisual::Panel { width, height, .. } if *width == 360.0 && *height == 164.0
        )));
    }
}
