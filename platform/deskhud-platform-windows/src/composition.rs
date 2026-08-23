//! DirectComposition surfaces owned by the Windows window thread.

use std::cell::RefCell;

use windows::Win32::Foundation::{HMODULE, HWND, POINT};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DASH_STYLE_DASH, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_STROKE_STYLE_PROPERTIES1, D2D1CreateFactory, ID2D1DeviceContext, ID2D1Factory1,
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
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGIDevice};
use windows::core::{Interface, Result};
use windows_numerics::Matrix3x2;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;

struct Surface {
    _device: IDCompositionDevice,
    _d2d_device: windows::Win32::Graphics::Direct2D::ID2D1Device,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    surface: IDCompositionSurface,
    factory: ID2D1Factory1,
    width: u32,
    height: u32,
    circle: bool,
}

thread_local! {
    static SURFACES: RefCell<Vec<(isize, Surface)>> = const { RefCell::new(Vec::new()) };
}

pub fn attach(hwnd: isize, width: u32, height: u32, circle: bool) -> Result<()> {
    unsafe {
        let mut d3d: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
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
            .ok_or_else(|| windows::core::Error::from_thread())?
            .cast()?;
        let factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let d2d = factory.CreateDevice(&dxgi)?;
        let device: IDCompositionDevice = DCompositionCreateDevice2(&d2d)?;
        let target = device.CreateTargetForHwnd(HWND(hwnd as *mut _), true)?;
        let visual = device.CreateVisual()?;
        let surface = device.CreateSurface(
            width.max(1),
            height.max(1),
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )?;
        visual.SetContent(&surface)?;
        target.SetRoot(&visual)?;
        device.Commit()?;
        SURFACES.with(|surfaces| {
            surfaces.borrow_mut().push((
                hwnd,
                Surface {
                    _device: device,
                    _d2d_device: d2d,
                    _target: target,
                    _visual: visual,
                    surface,
                    factory,
                    width,
                    height,
                    circle,
                },
            ))
        });
        Ok(())
    }
}

/// Draw the first frame only after the owning HWND has entered its final
/// visible z-order. DirectComposition may otherwise discard the only commit
/// made while a newly-created popup is still transitioning into DWM.
pub fn render(hwnd: isize) -> Result<()> {
    SURFACES.with(|surfaces| {
        let surfaces = surfaces.borrow();
        let Some((_, surface)) = surfaces.iter().find(|(id, _)| *id == hwnd) else {
            return Ok(());
        };
        unsafe { render_surface(surface) }
    })
}

unsafe fn render_surface(surface: &Surface) -> Result<()> {
    unsafe {
        let mut offset = POINT::default();
        let dc: ID2D1DeviceContext = surface.surface.BeginDraw(None, &mut offset)?;
        dc.SetTransform(&Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: offset.x as f32,
            M32: offset.y as f32,
        });
        dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));
        let brush = dc.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.25,
                g: 0.65,
                b: 1.0,
                a: 1.0,
            },
            None,
        )?;
        if surface.circle {
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: windows_numerics::Vector2::new(
                        surface.width as f32 / 2.0,
                        surface.height as f32 / 2.0,
                    ),
                    radiusX: surface.width as f32 / 2.0 - 8.0,
                    radiusY: surface.height as f32 / 2.0 - 8.0,
                },
                &brush,
            );
        } else {
            let style = D2D1_STROKE_STYLE_PROPERTIES1 {
                dashStyle: D2D1_DASH_STYLE_DASH,
                ..Default::default()
            };
            let stroke = surface.factory.CreateStrokeStyle(&style, None)?;
            dc.DrawRectangle(
                &D2D_RECT_F {
                    left: 2.0,
                    top: 2.0,
                    right: surface.width as f32 - 2.0,
                    bottom: surface.height as f32 - 2.0,
                },
                &brush,
                3.0,
                &stroke,
            );
        }
        drop(dc);
        surface.surface.EndDraw()?;
        surface._device.Commit()?;
        let _ = DwmFlush();
        Ok(())
    }
}

pub fn detach(hwnd: isize) {
    SURFACES.with(|surfaces| surfaces.borrow_mut().retain(|(id, _)| *id != hwnd));
}
