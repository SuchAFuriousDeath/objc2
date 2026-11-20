#[doc(hidden)]
#[macro_export]
macro_rules! __fallback_if_not_set {
    (
        ()
        ($($fallback:tt)*)
    ) => {
        $($fallback)*
    };
    (
        ($($actual:tt)+)
        ($($_fallback:tt)*)
    ) => {
        $($actual)+
    };
}

/// Maps the value of `#[init = ...]` to a [`ClassType::Init`] type.
///
/// [`ClassType::Init`]: crate::ClassType::Init
#[doc(hidden)]
#[macro_export]
macro_rules! __init_kind {
    (available) => {
        $crate::InitAvailable
    };
    (unavailable) => {
        $crate::InitUnavailable
    };
    ($($invalid:tt)*) => {
        $crate::__macros::compile_error!($crate::__macros::concat!(
            "#[init = ",
            $crate::__macros::stringify!($($invalid)*),
            "] is not supported, use #[init = available] or #[init = unavailable]",
        ))
    };
}
