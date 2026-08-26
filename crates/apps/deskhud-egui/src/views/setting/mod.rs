//! Settings 视口入口与 egui 运行时协调。

mod drawing;
mod window;

pub(crate) use window::SettingsWindow;

use std::{sync::Arc, time::Duration};

use deskhud_engine::EngineRegistry;
use deskhud_ui::SettingsModel;
use egui::{Context, RawInput};

use crate::views::ViewOutput;

/// 构建设置页；具体布局与控件绘制由 [`drawing`] 负责。
pub(crate) fn run(
    context: &Context,
    raw_input: RawInput,
    registry: &Arc<EngineRegistry>,
    model: &mut SettingsModel,
    font_signature: &mut Option<(String, u32)>,
) -> ViewOutput {
    let mut should_close = false;
    let mut applied_preferences = None;
    let full_output = context.run_ui(raw_input, |ctx| {
        ctx.request_repaint_after(Duration::from_millis(100));
        crate::views::theme::apply(ctx.ctx(), model.draft.shell.ui_theme);
        let signature = (
            model.draft.shell.ui_font_id.clone(),
            model.draft.shell.ui_font_size.to_bits(),
        );
        if font_signature.as_ref() != Some(&signature) {
            crate::fonts::configure_context_for(
                ctx.ctx(),
                &signature.0,
                model.draft.shell.ui_font_size,
            );
            *font_signature = Some(signature);
        }
        let result = drawing::draw(ctx, registry, model);
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
