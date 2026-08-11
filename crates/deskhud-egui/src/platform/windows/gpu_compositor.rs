//! Windows GPU 覆盖层合成器。
//!
//! 该模块只负责 Direct3D、Direct2D 与 DirectComposition 资源；宠物运行时、窗口消息
//! 和产品 UI 均由调用方管理，因而可从探针平滑迁移到正式覆盖层后端。

use std::collections::HashMap;

use deskhud_engine::{OverlayColor, OverlayScene, OverlayVisual};
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_ROUNDED_RECT, D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, IDXGIDevice,
};
use windows::core::{Interface, w};
use windows_numerics::{Matrix3x2, Vector2};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;

/// 单个原生覆盖窗的 GPU 合成资源。
pub(crate) struct GpuCompositor {
    composition: IDCompositionDevice,
    _d2d_device: ID2D1Device,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    surface: IDCompositionSurface,
    text_factory: IDWriteFactory,
    text_formats: HashMap<u32, IDWriteTextFormat>,
}

impl GpuCompositor {
    /// 创建绑定到既有原生窗口的合成器；窗口生命周期仍属于调用方。
    pub(crate) unsafe fn create(
        hwnd: isize,
        width: i32,
        height: i32,
    ) -> windows::core::Result<Self> {
        unsafe {
            let mut d3d: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d),
                None,
                Some(&mut context),
            )?;
            let dxgi: IDXGIDevice = d3d
                .as_ref()
                .expect("D3D11CreateDevice succeeded without a device")
                .cast()?;
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_device = factory.CreateDevice(&dxgi)?;
            // DCompositionCreateDevice2 仍须请求基础 IDCompositionDevice IID。
            let composition: IDCompositionDevice = DCompositionCreateDevice2(&d2d_device)?;
            let target = composition.CreateTargetForHwnd(HWND(hwnd as *mut _), true)?;
            let visual = composition.CreateVisual()?;
            let surface = composition.CreateSurface(
                width.max(1) as u32,
                height.max(1) as u32,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            visual.SetContent(&surface)?;
            target.SetRoot(&visual)?;
            composition.Commit()?;
            let text_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            Ok(Self {
                composition,
                _d2d_device: d2d_device,
                _target: target,
                _visual: visual,
                surface,
                text_factory,
                text_formats: HashMap::new(),
            })
        }
    }

    /// 提交一帧平台无关场景，并与桌面合成器同步。
    pub(crate) unsafe fn render(&mut self, scene: &OverlayScene) -> windows::core::Result<()> {
        unsafe {
            self.prepare_text_formats(scene)?;
            let mut offset = windows::Win32::Foundation::POINT::default();
            let context: ID2D1DeviceContext = self.surface.BeginDraw(None, &mut offset)?;
            let draw_result = (|| -> windows::core::Result<()> {
                let transparent = D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                };
                context.Clear(Some(&transparent));
                context.SetTransform(&Matrix3x2 {
                    M11: 1.0,
                    M12: 0.0,
                    M21: 0.0,
                    M22: 1.0,
                    M31: offset.x as f32,
                    M32: offset.y as f32,
                });
                draw_scene(&context, scene, &self.text_formats)
            })();
            drop(context);
            let end_result = self.surface.EndDraw();
            draw_result?;
            end_result?;
            self.composition.Commit()?;
            let flush_result = DwmFlush();
            if flush_result < 0 {
                tracing::debug!(
                    hresult = format_args!("0x{:08x}", flush_result as u32),
                    "GPU overlay DwmFlush failed"
                );
            }
            Ok(())
        }
    }

    fn prepare_text_formats(&mut self, scene: &OverlayScene) -> windows::core::Result<()> {
        for visual in &scene.visuals {
            let OverlayVisual::Text(text) = visual else {
                continue;
            };
            let size = text.font_size.clamp(8.0, 96.0);
            let key = size.to_bits();
            if self.text_formats.contains_key(&key) {
                continue;
            }
            let format = unsafe {
                self.text_factory.CreateTextFormat(
                    w!("Segoe UI"),
                    None,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!("zh-CN"),
                )?
            };
            unsafe {
                format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            }
            self.text_formats.insert(key, format);
        }
        Ok(())
    }
}

/// 仅这组错误可以安全地通过丢弃并重建设备资源来恢复。
pub(crate) fn is_device_lost(error: &windows::core::Error) -> bool {
    matches!(
        error.code(),
        DXGI_ERROR_DEVICE_HUNG | DXGI_ERROR_DEVICE_REMOVED | DXGI_ERROR_DEVICE_RESET
    )
}

unsafe fn draw_scene(
    context: &ID2D1DeviceContext,
    scene: &OverlayScene,
    text_formats: &HashMap<u32, IDWriteTextFormat>,
) -> windows::core::Result<()> {
    unsafe {
        for visual in &scene.visuals {
            match visual {
                OverlayVisual::Circle(circle) => {
                    let brush = context.CreateSolidColorBrush(&d2d_color(circle.color), None)?;
                    context.FillEllipse(
                        &D2D1_ELLIPSE {
                            point: Vector2 {
                                X: circle.center.x,
                                Y: circle.center.y,
                            },
                            radiusX: circle.radius,
                            radiusY: circle.radius,
                        },
                        &brush,
                    );
                }
                OverlayVisual::Ellipse(ellipse) => {
                    let brush = context.CreateSolidColorBrush(&d2d_color(ellipse.color), None)?;
                    context.FillEllipse(
                        &D2D1_ELLIPSE {
                            point: Vector2 {
                                X: ellipse.center.x,
                                Y: ellipse.center.y,
                            },
                            radiusX: ellipse.radius_x,
                            radiusY: ellipse.radius_y,
                        },
                        &brush,
                    );
                }
                OverlayVisual::RoundedRect(rounded) => {
                    let brush = context.CreateSolidColorBrush(&d2d_color(rounded.color), None)?;
                    context.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: d2d_rect(rounded.rect),
                            radiusX: rounded.corner_radius,
                            radiusY: rounded.corner_radius,
                        },
                        &brush,
                    );
                }
                OverlayVisual::Text(text) => {
                    let Some(format) = text_formats.get(&text.font_size.clamp(8.0, 96.0).to_bits())
                    else {
                        continue;
                    };
                    let brush = context.CreateSolidColorBrush(&d2d_color(text.color), None)?;
                    let utf16: Vec<u16> = text.text.encode_utf16().collect();
                    context.DrawText(
                        &utf16,
                        format,
                        &d2d_rect(text.rect),
                        &brush,
                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                        DWRITE_MEASURING_MODE_NATURAL,
                    );
                }
            }
        }
        Ok(())
    }
}

fn d2d_color(color: OverlayColor) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: color.red as f32 / 255.0,
        g: color.green as f32 / 255.0,
        b: color.blue as f32 / 255.0,
        a: color.alpha as f32 / 255.0,
    }
}

fn d2d_rect(rect: deskhud_engine::OverlayRect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.origin.x,
        top: rect.origin.y,
        right: rect.origin.x + rect.width,
        bottom: rect.origin.y + rect.height,
    }
}
