#![cfg(all(
    feature = "NSAppleEventDescriptor",
    feature = "NSAppleEventManager",
    feature = "NSArray",
    feature = "NSEnumerator",
    feature = "NSString",
    feature = "NSURL",
    feature = "objc2-core-services",
    target_vendor = "apple"
))]
use alloc::string::ToString;
use alloc::vec::Vec;
use core::cell::RefCell;

use objc2_core_services::{
    kAEOpenDocuments, kAEReopenApplication, kAnyTransactionID, kAutoGenerateReturnID,
    kCoreEventClass, keyAESearchText, keyDirectObject, AEEventClass, AEEventID, AEReturnID,
    AETransactionID,
};

use crate::{ns_string, AppleEventHandler, AppleEventKind, NSAppleEventDescriptor, NSURL};

fn event(
    event_class: AEEventClass,
    event_id: AEEventID,
) -> objc2::rc::Retained<NSAppleEventDescriptor> {
    NSAppleEventDescriptor::appleEventWithEventClass_eventID_targetDescriptor_returnID_transactionID(
        event_class,
        event_id,
        None,
        kAutoGenerateReturnID as AEReturnID,
        kAnyTransactionID as AETransactionID,
    )
}

fn call<const CLASS: AEEventClass, const ID: AEEventID, F>(
    handler: F,
    event: &NSAppleEventDescriptor,
) where
    AppleEventKind<CLASS, ID>: AppleEventHandler<F>,
{
    let reply = NSAppleEventDescriptor::nullDescriptor();
    <AppleEventKind<CLASS, ID> as AppleEventHandler<F>>::call(&handler, event, &reply);
}

#[test]
fn open_documents() {
    let event = event(kCoreEventClass, kAEOpenDocuments);
    let files = NSAppleEventDescriptor::listDescriptor();
    for (index, path) in ["/tmp/first movie.swf", "/tmp/second.swf"]
        .into_iter()
        .enumerate()
    {
        let url = NSURL::fileURLWithPath(&crate::NSString::from_str(path));
        files.insertDescriptor_atIndex(
            &NSAppleEventDescriptor::descriptorWithFileURL(&url),
            index as isize + 1,
        );
    }
    event.setParamDescriptor_forKeyword(&files, keyDirectObject);
    event.setParamDescriptor_forKeyword(
        &NSAppleEventDescriptor::descriptorWithString(ns_string!("movie")),
        keyAESearchText,
    );

    let received = RefCell::new(Vec::new());
    call::<kCoreEventClass, kAEOpenDocuments, _>(
        |files, event, _reply| {
            // Descriptor lists are 1-based.
            for index in 1..=files.numberOfItems() {
                let file = files.descriptorAtIndex(index).unwrap();
                let url = file.fileURLValue().unwrap();

                received.borrow_mut().push(url.path().unwrap().to_string());
            }
            // Optional parameters are read from the event itself.
            let search_text = event
                .paramDescriptorForKeyword(keyAESearchText)
                .and_then(|search_text| search_text.stringValue());
            assert_eq!(search_text.unwrap().to_string(), "movie");
        },
        &event,
    );

    assert_eq!(
        *received.borrow(),
        ["/tmp/first movie.swf", "/tmp/second.swf"]
    );
}

#[test]
fn open_documents_without_files() {
    let event = event(kCoreEventClass, kAEOpenDocuments);

    let called = RefCell::new(false);
    call::<kCoreEventClass, kAEOpenDocuments, _>(
        |files, event, _reply| {
            assert_eq!(files.numberOfItems(), 0);
            assert!(event.paramDescriptorForKeyword(keyAESearchText).is_none());
            *called.borrow_mut() = true;
        },
        &event,
    );

    assert!(*called.borrow());
}

#[test]
fn reopen_application() {
    let event = event(kCoreEventClass, kAEReopenApplication);

    let called = RefCell::new(false);
    call::<kCoreEventClass, kAEReopenApplication, _>(
        |_event, _reply| *called.borrow_mut() = true,
        &event,
    );

    assert!(*called.borrow());
}
