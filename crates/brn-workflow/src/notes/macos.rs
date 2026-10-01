use super::files::{NoteFileNotice, NoteNoticeKind, NoteNoticeSink, note_unsupported};
use block2::RcBlock;
use brn_store::notes::{FileOutcome, NoteErrorCode, NoteResult};
use objc2::{
    AnyThread, DefinedClass, define_class, msg_send, rc::Retained, runtime::ProtocolObject,
};
use objc2_foundation::{
    NSFileCoordinator, NSFileCoordinatorReadingOptions, NSFileCoordinatorWritingOptions,
    NSFilePresenter, NSObject, NSObjectProtocol, NSOperationQueue, NSURL,
};
use std::{
    cell::RefCell,
    ffi::{CStr, CString, OsStr},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

struct PresenterState {
    vault_id: Uuid,
    root: PathBuf,
    url: Retained<NSURL>,
    queue: Retained<NSOperationQueue>,
    notices: NoteNoticeSink,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. State is immutable except
    // for the mutex-protected queue; every callback only enqueues an observation hint.
    #[unsafe(super = NSObject)]
    #[ivars = PresenterState]
    struct NotePresenter;

    // SAFETY: NSObjectProtocol imposes no additional requirements.
    unsafe impl NSObjectProtocol for NotePresenter {}

    // SAFETY: required URL and dedicated serial operation queue remain retained.
    unsafe impl NSFilePresenter for NotePresenter {
        #[unsafe(method_id(presentedItemURL))]
        fn presented_url(&self) -> Option<Retained<NSURL>> {
            Some(self.ivars().url.clone())
        }

        #[unsafe(method_id(presentedItemOperationQueue))]
        fn operation_queue(&self) -> Retained<NSOperationQueue> {
            self.ivars().queue.clone()
        }

        #[unsafe(method(presentedItemDidChange))]
        fn changed(&self) {
            self.enqueue(None, NoteNoticeKind::Changed);
        }

        #[unsafe(method(presentedItemDidMoveToURL:))]
        fn moved(&self, _: &NSURL) {
            self.enqueue(None, NoteNoticeKind::Moved);
        }

        #[unsafe(method(presentedSubitemDidChangeAtURL:))]
        fn subitem_changed(&self, url: &NSURL) {
            self.enqueue(Some(url), NoteNoticeKind::Changed);
        }

        #[unsafe(method(presentedSubitemDidAppearAtURL:))]
        fn subitem_appeared(&self, url: &NSURL) {
            self.enqueue(Some(url), NoteNoticeKind::Changed);
        }

        #[unsafe(method(presentedSubitemAtURL:didMoveToURL:))]
        fn subitem_moved(&self, old: &NSURL, _: &NSURL) {
            self.enqueue(Some(old), NoteNoticeKind::Moved);
        }

        #[unsafe(method(accommodatePresentedItemDeletionWithCompletionHandler:))]
        fn deleted(&self, completion: &block2::DynBlock<dyn Fn(*mut objc2_foundation::NSError)>) {
            self.enqueue(None, NoteNoticeKind::Deleted);
            completion.call((std::ptr::null_mut(),));
        }

        #[unsafe(method(accommodatePresentedSubitemDeletionAtURL:completionHandler:))]
        fn subitem_deleted(
            &self,
            url: &NSURL,
            completion: &block2::DynBlock<dyn Fn(*mut objc2_foundation::NSError)>,
        ) {
            self.enqueue(Some(url), NoteNoticeKind::Deleted);
            completion.call((std::ptr::null_mut(),));
        }
    }
);

impl NotePresenter {
    fn enqueue(&self, url: Option<&NSURL>, kind: NoteNoticeKind) {
        let state = self.ivars();
        let relative_path = url
            .map(url_path)
            .and_then(|path| path.strip_prefix(&state.root).ok().map(Path::to_owned));
        let notice = NoteFileNotice {
            vault_id: state.vault_id,
            kind: if url.is_some() && relative_path.is_none() {
                NoteNoticeKind::RescanRequired
            } else {
                kind
            },
            relative_path,
        };
        // Preserve the observation requirement even if an earlier consumer panicked.
        let mut notices = state
            .notices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        notices.push(notice);
    }
}

pub(super) struct Coordination {
    presenter: Retained<NotePresenter>,
}

impl Coordination {
    pub(super) fn new(vault_id: Uuid, root: &Path, notices: NoteNoticeSink) -> NoteResult<Self> {
        let url = file_url(root, true)?;
        let queue = NSOperationQueue::new();
        queue.setMaxConcurrentOperationCount(1);
        let state = PresenterState {
            vault_id,
            root: root.to_owned(),
            url,
            queue,
            notices,
        };
        // SAFETY: initializing an allocated NSObject subclass with initialized ivars.
        let presenter: Retained<NotePresenter> =
            unsafe { msg_send![super(NotePresenter::alloc().set_ivars(state)), init] };
        NSFileCoordinator::addFilePresenter(ProtocolObject::from_ref(&*presenter));
        Ok(Self { presenter })
    }

    pub(super) fn read<T>(
        &self,
        path: &Path,
        action: impl FnOnce() -> NoteResult<T>,
    ) -> NoteResult<T> {
        self.access(path, false, action)
    }

    pub(super) fn write<T>(
        &self,
        path: &Path,
        action: impl FnOnce() -> NoteResult<T>,
    ) -> NoteResult<T> {
        self.access(path, true, action)
    }

    fn access<T>(
        &self,
        path: &Path,
        write: bool,
        action: impl FnOnce() -> NoteResult<T>,
    ) -> NoteResult<T> {
        objc2::rc::autoreleasepool(|_| {
            let url = file_url(path, false)?;
            let coordinator = NSFileCoordinator::initWithFilePresenter(
                NSFileCoordinator::alloc(),
                Some(ProtocolObject::from_ref(&*self.presenter)),
            );
            let action = RefCell::new(Some(action));
            let result = RefCell::new(None);
            let block = RcBlock::new(|provided: std::ptr::NonNull<NSURL>| {
                // SAFETY: Foundation supplies a live URL for the synchronous accessor.
                let provided = unsafe { provided.as_ref() };
                if url_path(provided) != path {
                    *result.borrow_mut() =
                        Some(Err(note_unsupported("coordinator relocated the note")));
                    return;
                }
                let Some(action) = action.borrow_mut().take() else {
                    return;
                };
                // Never unwind through an Objective-C block boundary.
                *result.borrow_mut() = Some(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).unwrap_or_else(
                        |_| {
                            let mut failure = note_unsupported("coordinated accessor panicked");
                            if write {
                                failure.code = NoteErrorCode::SaveUncertain;
                                failure.filesystem_outcome = FileOutcome::Unknown;
                            }
                            Err(failure)
                        },
                    ),
                );
            });
            let mut error = None;
            if write {
                coordinator.coordinateWritingItemAtURL_options_error_byAccessor(
                    &url,
                    NSFileCoordinatorWritingOptions::ForReplacing,
                    Some(&mut error),
                    &block,
                );
            } else {
                coordinator.coordinateReadingItemAtURL_options_error_byAccessor(
                    &url,
                    NSFileCoordinatorReadingOptions::empty(),
                    Some(&mut error),
                    &block,
                );
            }
            if let Some(error) = error {
                let mut failure = note_unsupported(&format!(
                    "macOS coordination failed: {}",
                    error.localizedDescription()
                ));
                failure.code = NoteErrorCode::Io;
                if write && action.borrow().is_none() {
                    failure.filesystem_outcome = FileOutcome::Unknown;
                }
                return Err(failure);
            }
            drop(block);
            result
                .into_inner()
                .unwrap_or_else(|| Err(note_unsupported("coordinator did not run its accessor")))
        })
    }
}

impl Drop for Coordination {
    fn drop(&mut self) {
        NSFileCoordinator::removeFilePresenter(ProtocolObject::from_ref(&*self.presenter));
        self.presenter
            .ivars()
            .queue
            .waitUntilAllOperationsAreFinished();
    }
}

fn file_url(path: &Path, directory: bool) -> NoteResult<Retained<NSURL>> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| note_unsupported("NUL in coordinated path"))?;
    // SAFETY: path is a valid live NUL-terminated filesystem representation;
    // absolute paths are used, so no relative URL is necessary.
    Ok(unsafe {
        NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
            std::ptr::NonNull::new(path.as_ptr().cast_mut()).unwrap(),
            directory,
            None,
        )
    })
}

fn url_path(url: &NSURL) -> PathBuf {
    // SAFETY: Foundation returns a NUL-terminated representation owned by the live URL.
    let representation = unsafe { CStr::from_ptr(url.fileSystemRepresentation().as_ptr()) };
    PathBuf::from(OsStr::from_bytes(representation.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::super::files::NoteNoticeQueue;
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn adapter_can_move_to_the_owned_worker() {
        fn assert_send<T: Send>() {}
        assert_send::<super::super::files::MacFiles>();
    }

    struct DenyingState {
        url: Retained<NSURL>,
        queue: Retained<NSOperationQueue>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements.
        #[unsafe(super = NSObject)]
        #[ivars = DenyingState]
        struct DenyingPresenter;

        // SAFETY: NSObjectProtocol has no additional requirements.
        unsafe impl NSObjectProtocol for DenyingPresenter {}
        // SAFETY: the two required properties are retained for the object's lifetime.
        unsafe impl NSFilePresenter for DenyingPresenter {
            #[unsafe(method_id(presentedItemURL))]
            fn url(&self) -> Option<Retained<NSURL>> {
                Some(self.ivars().url.clone())
            }
            #[unsafe(method_id(presentedItemOperationQueue))]
            fn queue(&self) -> Retained<NSOperationQueue> {
                self.ivars().queue.clone()
            }
            #[unsafe(method(savePresentedItemChangesWithCompletionHandler:))]
            fn save(&self, completion: &block2::DynBlock<dyn Fn(*mut objc2_foundation::NSError)>) {
                // SAFETY: no userInfo dictionary is supplied; the domain is a live NSString.
                let error = unsafe {
                    objc2_foundation::NSError::errorWithDomain_code_userInfo(
                        &objc2_foundation::NSString::from_str("org.brn.synthetic.denial"),
                        1,
                        None,
                    )
                };
                completion.call((&*error as *const objc2_foundation::NSError as *mut _,));
            }
        }
    );

    #[test]
    fn foundation_coordination_error_never_calls_accessor() {
        let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let path = root.path().join("plan.md");
        std::fs::write(&path, "unchanged").unwrap();
        let coordination = Coordination::new(Uuid::new_v4(), root.path(), Arc::default()).unwrap();
        let state = DenyingState {
            url: file_url(&path, false).unwrap(),
            queue: NSOperationQueue::new(),
        };
        state.queue.setMaxConcurrentOperationCount(1);
        // SAFETY: ivars are initialized before NSObject initialization.
        let denying: Retained<DenyingPresenter> =
            unsafe { msg_send![super(DenyingPresenter::alloc().set_ivars(state)), init] };
        let protocol = ProtocolObject::from_ref(&*denying);
        NSFileCoordinator::addFilePresenter(protocol);
        let ran = std::cell::Cell::new(false);
        let result = coordination.read(&path, || {
            ran.set(true);
            Ok(())
        });
        NSFileCoordinator::removeFilePresenter(protocol);
        denying.ivars().queue.waitUntilAllOperationsAreFinished();
        assert!(result.is_err(), "{result:?}");
        assert_eq!(
            result.unwrap_err().filesystem_outcome,
            FileOutcome::NotApplied
        );
        assert!(!ran.get());
        assert_eq!(std::fs::read(&path).unwrap(), b"unchanged");
    }

    #[test]
    fn foreign_coordinated_write_delivers_observation_hints() {
        let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let path = root.path().join("plan.md");
        std::fs::write(&path, "before").unwrap();
        let notices = Arc::new(Mutex::new(NoteNoticeQueue::default()));
        let _coordination =
            Coordination::new(Uuid::new_v4(), root.path(), notices.clone()).unwrap();
        let foreign = NSFileCoordinator::initWithFilePresenter(NSFileCoordinator::alloc(), None);
        let url = file_url(&path, false).unwrap();
        let block = RcBlock::new(|_: std::ptr::NonNull<NSURL>| {
            std::fs::write(&path, "external writer").unwrap();
        });
        let mut error = None;
        foreign.coordinateWritingItemAtURL_options_error_byAccessor(
            &url,
            NSFileCoordinatorWritingOptions::empty(),
            Some(&mut error),
            &block,
        );
        assert!(error.is_none());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let delivered = notices.lock().unwrap().drain();
            if delivered
                .iter()
                .any(|notice| notice.kind == NoteNoticeKind::Changed)
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Foundation delivered no changed notice"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"external writer");
    }

    #[test]
    fn notices_coalesce_and_overflow_requires_rescan() {
        let vault_id = Uuid::new_v4();
        let mut queue = NoteNoticeQueue::default();
        for index in 0..300 {
            queue.push(NoteFileNotice {
                vault_id,
                relative_path: Some(PathBuf::from(format!("{index}.md"))),
                kind: NoteNoticeKind::Changed,
            });
        }
        assert_eq!(
            queue.drain(),
            vec![NoteFileNotice {
                vault_id,
                relative_path: None,
                kind: NoteNoticeKind::RescanRequired,
            }]
        );
        let notice = NoteFileNotice {
            vault_id,
            relative_path: None,
            kind: NoteNoticeKind::Moved,
        };
        queue.push(notice.clone());
        queue.push(notice);
        assert_eq!(queue.drain().len(), 1);
    }

    #[test]
    fn unmapped_subitem_url_requires_rescan_instead_of_a_root_event() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let notices = Arc::new(Mutex::new(NoteNoticeQueue::default()));
        let vault_id = Uuid::new_v4();
        let coordination = Coordination::new(vault_id, root.path(), notices.clone()).unwrap();
        let outside_url = file_url(&outside.path().join("outside.md"), false).unwrap();
        coordination
            .presenter
            .enqueue(Some(&outside_url), NoteNoticeKind::Changed);
        let drained = notices.lock().unwrap().drain();
        assert_eq!(
            drained,
            vec![NoteFileNotice {
                vault_id,
                relative_path: None,
                kind: NoteNoticeKind::RescanRequired,
            }]
        );
        coordination.presenter.enqueue(None, NoteNoticeKind::Moved);
        let drained = notices.lock().unwrap().drain();
        assert_eq!(drained[0].kind, NoteNoticeKind::Moved);
        drop(coordination);
    }

    #[test]
    fn presenter_lifecycle_serial_callback_only_enqueues() {
        let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let notices = Arc::new(Mutex::new(NoteNoticeQueue::default()));
        let coordination = Coordination::new(Uuid::new_v4(), root.path(), notices.clone()).unwrap();
        let protocol = ProtocolObject::from_ref(&*coordination.presenter);
        assert!(NSFileCoordinator::filePresenters().containsObject(protocol));
        assert_eq!(
            coordination
                .presenter
                .ivars()
                .queue
                .maxConcurrentOperationCount(),
            1
        );
        let callback_url = file_url(&root.path().join("plan.md"), false).unwrap();
        let presenter = coordination.presenter.clone();
        let block = RcBlock::new(move || {
            presenter.enqueue(Some(&callback_url), NoteNoticeKind::Changed);
        });
        // SAFETY: the captured retained presenter/URL and mutex state may be used
        // on the dedicated queue; the block outlives the enqueued operation.
        unsafe {
            coordination
                .presenter
                .ivars()
                .queue
                .addOperationWithBlock(&block)
        };
        coordination
            .presenter
            .ivars()
            .queue
            .waitUntilAllOperationsAreFinished();
        assert_eq!(
            notices.lock().unwrap().drain()[0].relative_path,
            Some(PathBuf::from("plan.md"))
        );
        assert!(!root.path().join("plan.md").exists());
        let retained = coordination.presenter.clone();
        drop(coordination);
        let protocol = ProtocolObject::from_ref(&*retained);
        assert!(!NSFileCoordinator::filePresenters().containsObject(protocol));
    }

    #[test]
    fn coordinator_rejects_panics_without_uncoordinated_fallback() {
        let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let coordination = Coordination::new(Uuid::new_v4(), root.path(), Arc::default()).unwrap();
        let result: NoteResult<()> = coordination.write(&root.path().join("plan.md"), || {
            panic!("synthetic accessor failure")
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().filesystem_outcome, FileOutcome::Unknown);
        assert!(!root.path().join("plan.md").exists());
    }
}
