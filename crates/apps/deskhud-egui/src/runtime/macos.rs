use core::{
    ffi::{c_char, c_void},
    mem,
};

type Id = *mut c_void;
type Sel = *mut c_void;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_msgSend();
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
}

pub(super) fn set_visibility(visible: bool) {
    unsafe {
        let send_id: unsafe extern "C" fn(Id, Sel) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let send_policy: unsafe extern "C" fn(Id, Sel, isize) -> bool =
            mem::transmute(objc_msgSend as *const ());
        let send_id_arg: unsafe extern "C" fn(Id, Sel, Id) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let send_data: unsafe extern "C" fn(Id, Sel, *const u8, usize) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let application = send_id(
            objc_getClass(c"NSApplication".as_ptr()),
            sel_registerName(c"sharedApplication".as_ptr()),
        );
        if application.is_null() {
            return;
        }

        // NSApplicationActivationPolicyRegular = 0; accessory = 1.
        let policy = if visible { 0 } else { 1 };
        let _ = send_policy(
            application,
            sel_registerName(c"setActivationPolicy:".as_ptr()),
            policy,
        );

        // Set the native ICNS at runtime so cargo run and an unbundled binary
        // still use the same icon as a packaged .app.
        let data_class = objc_getClass(c"NSData".as_ptr());
        let image_class = objc_getClass(c"NSImage".as_ptr());
        let data = send_data(
            data_class,
            sel_registerName(c"dataWithBytes:length:".as_ptr()),
            APP_ICON_ICNS.as_ptr(),
            APP_ICON_ICNS.len(),
        );
        let image = if data.is_null() || image_class.is_null() {
            core::ptr::null_mut()
        } else {
            let image = send_id(image_class, sel_registerName(c"alloc".as_ptr()));
            send_id_arg(image, sel_registerName(c"initWithData:".as_ptr()), data)
        };
        if !image.is_null() {
            let _ = send_id_arg(
                application,
                sel_registerName(c"setApplicationIconImage:".as_ptr()),
                image,
            );
        }
    }
}

const APP_ICON_ICNS: &[u8] = include_bytes!("../../../../../assets/icon.icns");
