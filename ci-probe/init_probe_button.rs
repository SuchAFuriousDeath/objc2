//! Probe: call a superclass designated initializer through the generated
//! `...Init` traits, with everything glob-imported like users would.
#![allow(dead_code)]
use objc2::rc::Retained;
use objc2::{define_class, Ivars, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::*;
use objc2_foundation::*;

define_class!(
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    struct MyButton {
        x: u32,
    }
);

fn make(mtm: MainThreadMarker) -> Retained<MyButton> {
    unsafe {
        MyButton::alloc(mtm)
            .set_ivars(Ivars::<MyButton> { x: 1 })
            .initWithFrame(NSRect::ZERO)
    }
}

#[test]
fn compiles() {}
