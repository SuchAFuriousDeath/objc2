//! Typed handlers for Apple Events.
use alloc::boxed::Box;

use core::fmt;
use objc2::rc::Retained;
use objc2::runtime::NSObject;

use objc2::{define_class, msg_send, sel, Ivars, MainThreadMarker, MainThreadOnly};
use objc2_core_services::{
    kAEOpenDocuments, kAEReopenApplication, kCoreEventClass, keyDirectObject, typeAEList,
    AEEventClass, AEEventID,
};

use crate::{NSAppleEventDescriptor, NSAppleEventManager};

/// An Apple Event, identified by its event class and event ID.
///
/// Used with [`NSAppleEventManager::set_event_handler`] to select the
/// parameters that the handler receives.
pub struct AppleEventKind<const CLASS: AEEventClass, const ID: AEEventID>;

/// Calls a handler of type `F` for an Apple Event.
///
/// Implemented for each known Apple Event, for closures taking the
/// parameters of that event, followed by the event itself (for optional
/// parameters) and the reply event.
///
/// Handlers with the wrong parameters for the event are rejected:
///
/// ```compile_fail
/// # use objc2::MainThreadMarker;
/// # use objc2_core_services::{kAEOpenDocuments, kCoreEventClass};
/// # use objc2_foundation::NSAppleEventManager;
/// # let mtm = MainThreadMarker::new().unwrap();
/// // `kAEOpenDocuments` passes the files, the event and the reply.
/// let _registration = NSAppleEventManager::sharedAppleEventManager()
///     .set_event_handler::<kCoreEventClass, kAEOpenDocuments, _>(mtm, |_event, _reply| {});
/// ```
///
/// So are events that aren't known, e.g. with the class and ID swapped:
///
/// ```compile_fail
/// # use objc2::MainThreadMarker;
/// # use objc2_core_services::{kAEOpenDocuments, kCoreEventClass};
/// # use objc2_foundation::NSAppleEventManager;
/// # let mtm = MainThreadMarker::new().unwrap();
/// let _registration = NSAppleEventManager::sharedAppleEventManager()
///     .set_event_handler::<kAEOpenDocuments, kCoreEventClass, _>(mtm, |_event, _reply| {});
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a known Apple Event, or the handler has the wrong parameters",
    label = "no handler for this event class and event ID",
    note = "the event class and event ID might be swapped, or this event isn't known to `objc2-foundation`",
    note = "for other events, use `NSAppleEventManager::set_raw_event_handler`"
)]
pub trait AppleEventHandler<F> {
    /// Extracts the parameters from `event` and calls `handler` with them.
    fn call(handler: &F, event: &NSAppleEventDescriptor, reply: &NSAppleEventDescriptor);
}

/// `kAEOpenDocuments`: open the given files.
///
/// The handler receives a list descriptor of the files, each of which has a
/// [`fileURLValue`](NSAppleEventDescriptor::fileURLValue).
impl<F> AppleEventHandler<F> for AppleEventKind<kCoreEventClass, kAEOpenDocuments>
where
    F: Fn(&NSAppleEventDescriptor, &NSAppleEventDescriptor, &NSAppleEventDescriptor),
{
    fn call(handler: &F, event: &NSAppleEventDescriptor, reply: &NSAppleEventDescriptor) {
        let files = event
            .paramDescriptorForKeyword(keyDirectObject)
            .and_then(|direct| direct.coerceToDescriptorType(typeAEList))
            .unwrap_or_else(NSAppleEventDescriptor::listDescriptor);

        handler(&files, event, reply);
    }
}

/// `kAEReopenApplication`: the running application is opened again, e.g. by
/// clicking its icon in the Dock.
impl<F> AppleEventHandler<F> for AppleEventKind<kCoreEventClass, kAEReopenApplication>
where
    F: Fn(&NSAppleEventDescriptor, &NSAppleEventDescriptor),
{
    fn call(handler: &F, event: &NSAppleEventDescriptor, reply: &NSAppleEventDescriptor) {
        handler(event, reply);
    }
}

type RawHandler = Box<dyn Fn(&NSAppleEventDescriptor, &NSAppleEventDescriptor)>;

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `Target` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "__objc2_foundation_AppleEventHandlerTarget"]
    struct Target {
        handler: RawHandler,
    }

    impl Target {
        // SAFETY: This is the signature that `NSAppleEventManager` calls
        // handlers with.
        #[unsafe(method(handleAppleEvent:withReplyEvent:))]
        fn handle(&self, event: &NSAppleEventDescriptor, reply: &NSAppleEventDescriptor) {
            (self.handler())(event, reply);
        }
    }
);

impl Target {
    fn new(mtm: MainThreadMarker, handler: RawHandler) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars::<Self> { handler });
        // SAFETY: `NSObject`'s `init` is its designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// An Apple Event handler, which is removed when this is dropped.
#[must_use = "the handler is removed when this is dropped"]
pub struct EventHandlerRegistration {
    _target: Retained<Target>,
    event_class: AEEventClass,
    event_id: AEEventID,
}

impl fmt::Debug for EventHandlerRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventHandlerRegistration")
            .field("event_class", &self.event_class)
            .field("event_id", &self.event_id)
            .finish_non_exhaustive()
    }
}

impl Drop for EventHandlerRegistration {
    fn drop(&mut self) {
        NSAppleEventManager::sharedAppleEventManager()
            .removeEventHandlerForEventClass_andEventID(self.event_class, self.event_id);
    }
}

impl NSAppleEventManager {
    /// Sets `handler` as the handler for the Apple Event with the given
    /// event class and event ID, replacing any previous handler.
    ///
    /// The parameters of `handler` depend on the event, see the
    /// implementations of [`AppleEventHandler`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use objc2::MainThreadMarker;
    /// use objc2_core_services::{kAEOpenDocuments, kCoreEventClass};
    /// use objc2_foundation::NSAppleEventManager;
    ///
    /// let mtm = MainThreadMarker::new().unwrap();
    /// let _registration = NSAppleEventManager::sharedAppleEventManager()
    ///     .set_event_handler::<kCoreEventClass, kAEOpenDocuments, _>(mtm, |files, _event, _reply| {
    ///         // Descriptor lists are 1-based.
    ///         for index in 1..=files.numberOfItems() {
    ///             let Some(url) = files.descriptorAtIndex(index).and_then(|file| file.fileURLValue()) else {
    ///                 continue;
    ///             };
    ///
    ///             println!("open {url:?}");
    ///         }
    ///     });
    /// ```
    pub fn set_event_handler<const CLASS: AEEventClass, const ID: AEEventID, F>(
        &self,
        mtm: MainThreadMarker,
        handler: F,
    ) -> EventHandlerRegistration
    where
        AppleEventKind<CLASS, ID>: AppleEventHandler<F>,
        F: 'static,
    {
        self.set_raw_event_handler(mtm, CLASS, ID, move |event, reply| {
            <AppleEventKind<CLASS, ID> as AppleEventHandler<F>>::call(&handler, event, reply);
        })
    }

    /// Sets `handler` as the handler for the Apple Event with the given
    /// event class and event ID, replacing any previous handler.
    ///
    /// The handler receives the event and the reply event as they are.
    pub fn set_raw_event_handler(
        &self,
        mtm: MainThreadMarker,
        event_class: AEEventClass,
        event_id: AEEventID,
        handler: impl Fn(&NSAppleEventDescriptor, &NSAppleEventDescriptor) + 'static,
    ) -> EventHandlerRegistration {
        let target = Target::new(mtm, Box::new(handler));

        // SAFETY: `Target` implements the selector with the signature that
        // event handlers are called with, and the registration keeps it
        // alive until it removes the handler.
        unsafe {
            self.setEventHandler_andSelector_forEventClass_andEventID(
                &target,
                sel!(handleAppleEvent:withReplyEvent:),
                event_class,
                event_id,
            );
        }

        EventHandlerRegistration {
            _target: target,
            event_class,
            event_id,
        }
    }
}
