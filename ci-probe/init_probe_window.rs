//! Probe: call a superclass designated initializer through the generated
//! `...Init` traits, with everything glob-imported like users would.
#![allow(dead_code)]
use objc2::rc::Retained;
use objc2::{define_class, Ivars, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::*;
use objc2_foundation::*;

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    struct MyWindow {
        x: u32,
    }
);

fn make(mtm: MainThreadMarker) -> Retained<MyWindow> {
    unsafe {
        MyWindow::alloc(mtm)
            .set_ivars(Ivars::<MyWindow> { x: 1 })
            .initWithContentRect_styleMask_backing_defer(NSRect::ZERO, NSWindowStyleMask::Titled, NSBackingStoreType::Buffered, false)
    }
}

#[test]
fn compiles() {}
