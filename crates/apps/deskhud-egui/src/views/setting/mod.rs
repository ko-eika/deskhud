//! Settings 视口入口与 egui 运行时协调。

mod drawing;
mod window;

pub(crate) use window::SettingsWindow;

use std::{sync::Arc, time::Duration};

use deskhud_engine::EngineRegistry;
use deskhud_ui::{CatalogStore, SettingsModel};
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// 构建设置页；具体布局与控件绘制由 [`drawing`] 负责。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    registry: &Arc<EngineRegistry>,
    catalogs: &CatalogStore,
    model: &mut SettingsModel,
) -> ViewOutput {
    let mut should_close = false;
    let mut applied_preferences = None;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(100));
        let result = drawing::draw(ctx, registry, catalogs, model);
        should_close = result.0;
        applied_preferences = result.1;
    });

    ViewOutput {
        full_output,
        should_close,
        applied_preferences,
        ..Default::default()
    }
}
