//! Calling a superclass' designated initializer through an `...Init` trait,
//! in the shape that `header-translator` emits for framework classes.
#![allow(non_snake_case, clippy::missing_safety_doc)]
use objc2::rc::{PartialInit, Retained};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{
    class, define_class, extern_class, msg_send, AnyThread, DefinedClass, ExternClass, Inherits,
    Ivars,
};

extern_class!(
    #[unsafe(super(NSObject))]
    pub struct NSException;
);

// What `header-translator` emits for `NSException`'s designated initializer.

/// The designated initializers of [`NSException`], callable on a
/// [`PartialInit`] of any subclass, to initialize the superclass.
pub trait NSExceptionInit<T, I> {
    unsafe fn initWithName_reason_userInfo(
        self,
        name: &AnyObject,
        reason: Option<&AnyObject>,
        user_info: Option<&AnyObject>,
    ) -> Retained<T>;
}

impl<T, I> NSExceptionInit<T, I> for PartialInit<T>
where
    T: DefinedClass,
    T::Super: Inherits<NSException, I> + ExternClass,
{
    #[inline]
    #[track_caller]
    unsafe fn initWithName_reason_userInfo(
        self,
        name: &AnyObject,
        reason: Option<&AnyObject>,
        user_info: Option<&AnyObject>,
    ) -> Retained<T> {
        // SAFETY: Upheld by the caller, the superclass inherits from
        // the class that declares this initializer.
        unsafe { msg_send![super(self), initWithName: name, reason: reason, userInfo: user_info] }
    }
}

define_class!(
    #[unsafe(super(NSException))]
    struct Child {
        extra: u32,
    }
);

#[cfg(any())]
define_class!(
    #[unsafe(super(Child))]
    struct GrandChild {
        more: u32,
    }
);

fn string(s: &std::ffi::CStr) -> Retained<AnyObject> {
    unsafe { msg_send![class!(NSString), stringWithUTF8String: s.as_ptr()] }
}

#[test]
fn subclass() {
    let name = string(c"MyName");
    let obj = unsafe {
        Child::alloc()
            .set_ivars(Ivars::<Child> { extra: 2 })
            .initWithName_reason_userInfo(&name, None, None)
    };
    let actual: Retained<AnyObject> = unsafe { msg_send![&*obj, name] };
    let equal: bool = unsafe { msg_send![&*actual, isEqual: &*name] };
    assert!(equal);
    assert_eq!(*obj.extra(), 2);
}

/// `Child`'s ivars would be left uninitialized, so this must not compile.
#[cfg(any())]
fn _subclass_of_rust_class() {
    let name = string(c"MyName");
    let _ = unsafe {
        GrandChild::alloc()
            .set_ivars(Ivars::<GrandChild> { more: 3 })
            .initWithName_reason_userInfo(&name, None, None)
    };
}
