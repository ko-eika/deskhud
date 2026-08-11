//! Windows GPU 覆盖层的能力探针。
//!
//! 该探针只验证 D3D11 硬件设备与 DirectComposition 的初始化，绝不接管现有
//! GDI 覆盖层或默认运行路径。真正的 GPU 绘制会在能力验收后另行接线。

use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext,
};
use windows::Win32::Graphics::DirectComposition::{DCompositionCreateDevice, IDCompositionDevice};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;

/// 创建硬件 D3D11 设备，并确认它能作为 DirectComposition 的 DXGI 来源。
///
/// 核显同样属于硬件驱动；只有设备/驱动创建失败时才报告不可用，调用方可继续
/// 使用既有 GDI 探针作为回退。
fn initialize_hardware_composition() -> windows::core::Result<()> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let dxgi_device: IDXGIDevice = device
            .as_ref()
            .expect("D3D11CreateDevice succeeded without a device")
            .cast()?;
        let _: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
        Ok(())
    }
}

/// 运行 GPU 能力探针。
pub fn run() -> anyhow::Result<()> {
    match initialize_hardware_composition() {
        Ok(()) => eprintln!(
            "DeskHud GPU probe: hardware D3D11 + DirectComposition is available; GDI fallback remains unchanged."
        ),
        Err(error) => eprintln!(
            "DeskHud GPU probe: hardware D3D11/DirectComposition unavailable ({error}); use the GDI probe fallback."
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::initialize_hardware_composition;

    #[test]
    fn composition_probe_is_safe_to_attempt() {
        // 设备可用性取决于执行机及其驱动；此处只确认探测不会 panic。
        let _ = initialize_hardware_composition();
    }
}
