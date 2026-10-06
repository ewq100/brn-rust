use super::*;
use brn_store::work::{inbox_original_legacy as legacy, inbox_original_operations as current};

// The body bound includes the operation's domain metadata. This extra fixed
// allowance covers only the format/digest/envelope syntax (well below 512 bytes).
const MAX_NEW_MIRROR_BYTES: usize = current::MAX_NEW_OPERATION_BYTES + 512;
const MAX_ORIGINAL_OPERATION_BYTES: usize = current::MAX_ORIGINAL_OPERATION_BYTES;
use current::InboxOriginalNamespace;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationEnvelope<T> {
    format: u8,
    sha256: [u8; 32],
    operation: T,
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
enum OperationRef<'a> {
    Remove(&'a current::InboxOriginalRemovalRecord),
    Restore(&'a current::InboxOriginalRestoreRecord),
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
enum LegacyRef<'a> {
    Remove(&'a legacy::InboxOriginalRemovalRecord),
    Restore(&'a legacy::InboxOriginalRestoreRecord),
}
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "record",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum NewOperation {
    Remove(Box<current::InboxOriginalRemovalRecord>),
    Restore(Box<current::InboxOriginalRestoreRecord>),
}
struct Bounded(Vec<u8>, usize);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.1)
        {
            return Err(std::io::Error::other(
                "complete Inbox operation mirror exceeds its bound",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn serialize<T: Serialize>(value: &T, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Bounded(Vec::new(), max);
    serde_json::to_writer(&mut bytes, value)
        .map_err(|_| unavailable("complete Inbox operation mirror exceeds its bound"))?;
    Ok(bytes.0)
}
fn envelope<T: Serialize>(operation: T, format: u8, max: usize) -> Result<Vec<u8>> {
    let body = serialize(&operation, max)?;
    serialize(
        &OperationEnvelope {
            format,
            sha256: digest(&body),
            operation,
        },
        max,
    )
}
fn encode(operation: &OriginalOperationFile) -> Result<Vec<u8>> {
    operation.validate()?;
    match operation {
        OriginalOperationFile::Remove(r) => {
            envelope(OperationRef::Remove(r), 2, MAX_NEW_MIRROR_BYTES)
        }
        OriginalOperationFile::Restore(r) => {
            envelope(OperationRef::Restore(r), 2, MAX_NEW_MIRROR_BYTES)
        }
        OriginalOperationFile::LegacyRemove(r) => {
            envelope(LegacyRef::Remove(r), 1, MAX_ORIGINAL_OPERATION_BYTES)
        }
        OriginalOperationFile::LegacyRestore(r) => {
            envelope(LegacyRef::Restore(r), 1, MAX_ORIGINAL_OPERATION_BYTES)
        }
    }
}
fn decode(bytes: &[u8]) -> Result<OriginalOperationFile> {
    if bytes.len() > MAX_ORIGINAL_OPERATION_BYTES {
        return Err(unavailable(
            "complete Inbox operation mirror exceeds its bound",
        ));
    }
    // Inspect only the version, without allocating an untyped full graph. The
    // following typed parse and canonical equality validate every other field.
    #[derive(Deserialize)]
    struct Version {
        format: u8,
    }
    let version: Version =
        serde_json::from_slice(bytes).map_err(|_| unavailable("invalid Inbox operation mirror"))?;
    let operation = match version.format {
        1 => {
            let envelope: OperationEnvelope<legacy::Operation> = serde_json::from_slice(bytes)
                .map_err(|_| unavailable("invalid legacy Inbox operation mirror"))?;
            match envelope.operation {
                legacy::Operation::Remove(r) => OriginalOperationFile::LegacyRemove(r),
                legacy::Operation::Restore(r) => OriginalOperationFile::LegacyRestore(r),
            }
        }
        2 if bytes.len() <= MAX_NEW_MIRROR_BYTES => {
            let envelope: OperationEnvelope<NewOperation> = serde_json::from_slice(bytes)
                .map_err(|_| unavailable("invalid Inbox operation mirror"))?;
            match envelope.operation {
                NewOperation::Remove(r) => OriginalOperationFile::Remove(r),
                NewOperation::Restore(r) => OriginalOperationFile::Restore(r),
            }
        }
        _ => {
            return Err(unavailable(
                "Inbox operation mirror version or bound differs",
            ));
        }
    };
    if encode(&operation)? != bytes {
        return Err(unavailable(
            "Inbox operation mirror format, hash or canonical binding differs",
        ));
    }
    Ok(operation)
}

fn retained(id: Uuid) -> String {
    format!(".brn-inbox-removed-{id}.original")
}

impl InboxFiles {
    pub(crate) fn original_namespace(&self) -> InboxOriginalNamespace {
        InboxOriginalNamespace {
            data_device: self.root.data_device,
            data_inode: self.root.data_inode,
        }
    }
    fn check_operation(&self, operation: &OriginalOperationFile) -> Result<()> {
        operation.validate()?;
        self.validate_item(operation.item())?;
        let namespace = operation.namespace();
        if *namespace != self.original_namespace() {
            return Err(unavailable(
                "Inbox operation belongs to another data namespace",
            ));
        }
        self.validate()
    }
    pub(crate) fn operation(&self, name: &str) -> Result<OriginalOperationFile> {
        if !original_operation_name(name) || name.contains('/') {
            return Err(unavailable("invalid Inbox operation mirror name"));
        }
        self.validate()?;
        let mut file =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        let before = file.metadata().map_err(io)?;
        private_file(&before, MAX_ORIGINAL_OPERATION_BYTES)?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take((MAX_ORIGINAL_OPERATION_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        let after = file.metadata().map_err(io)?;
        private_file(&after, MAX_ORIGINAL_OPERATION_BYTES)?;
        if !same(&before, &after)
            || bytes.len() as u64 != after.len()
            || bytes.len() > MAX_ORIGINAL_OPERATION_BYTES
        {
            return Err(unavailable(
                "Inbox operation mirror changed during bounded observation",
            ));
        }
        let operation = decode(&bytes)?;
        if operation.name() != name {
            return Err(unavailable(
                "Inbox operation mirror format, hash, filename or canonical binding differs",
            ));
        }
        self.check_operation(&operation)?;
        #[cfg(test)]
        READ_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let current =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        if !same(&after, &current.metadata().map_err(io)?) {
            return Err(unavailable("Inbox operation mirror filename changed"));
        }
        self.validate()?;
        Ok(operation)
    }
    pub(crate) fn publish_operation(&self, operation: &OriginalOperationFile) -> Result<()> {
        self.check_operation(operation)?;
        let bytes = encode(operation)?;
        let name = operation.name();
        if !self.absent(&name)? {
            let mut file = open_at(&self.directory, OsStr::new(&name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            let before = file.metadata().map_err(io)?;
            if !self.matches_operation_bytes(&mut file, &name, &before, &bytes)? {
                // Untrusted unequal occupants retain the complete typed reader's
                // semantic/canonical refusals and valid-different classification.
                if encode(&self.operation(&name)?)? != bytes {
                    return Err(WorkflowError::typed(
                        ErrorKind::OperationConflict,
                        "Inbox operation mirror is occupied by another exact record",
                    ));
                }
                return Err(unavailable(
                    "Inbox operation mirror changed during observation",
                ));
            }
            return self.flush_operation(file, &name, &before, &bytes);
        }
        let temp = format!(".brn-inbox-operation-{}.stage", Uuid::new_v4());
        let mut file = open_at(
            &self.directory,
            OsStr::new(&temp),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )
        .map_err(file_error)?;
        file.write_all(&bytes).map_err(io)?;
        full_sync(&file).map_err(file_error)?;
        sync_directory(&self.directory).map_err(file_error)?;
        self.validate()?;
        let named =
            open_at(&self.directory, OsStr::new(&temp), libc::O_RDONLY, 0).map_err(file_error)?;
        let held = file.metadata().map_err(io)?;
        private_file(&held, MAX_ORIGINAL_OPERATION_BYTES)?;
        if !same(&held, &named.metadata().map_err(io)?) {
            return Err(unavailable("Inbox operation staging filename changed"));
        }
        rename_flags(
            &self.directory,
            &cstring(Path::new(&temp)).map_err(file_error)?,
            &cstring(Path::new(&name)).map_err(file_error)?,
            libc::RENAME_EXCL,
        )
        .map_err(file_error)?;
        full_sync(&file).map_err(file_error)?;
        sync_directory(&self.directory).map_err(file_error)?;
        if encode(&self.operation(&name)?)? != bytes {
            return Err(unavailable(
                "Inbox operation changed after mirror publication",
            ));
        }
        checkpoint(if operation.settled() {
            "original_operation_receipt"
        } else {
            "original_operation_intent"
        })
    }
    // A caller has already validated the expected typed operation and encoded
    // its canonical envelope. Complete byte equality can prove that same record
    // without parsing/encoding additional untrusted copies. Hold one descriptor
    // throughout both observations and durability; a new identical inode differs.
    fn matches_operation_bytes(
        &self,
        file: &mut File,
        name: &str,
        before: &Metadata,
        expected: &[u8],
    ) -> Result<bool> {
        private_file(before, MAX_ORIGINAL_OPERATION_BYTES)?;
        self.validate()?;
        let observed = file.metadata().map_err(io)?;
        private_file(&observed, MAX_ORIGINAL_OPERATION_BYTES)?;
        if !same(before, &observed) {
            return Err(unavailable("Inbox operation mirror identity changed"));
        }
        std::io::Seek::rewind(file).map_err(io)?;
        let mut reader = Read::by_ref(file).take((MAX_ORIGINAL_OPERATION_BYTES + 1) as u64);
        let mut buffer = [0u8; 64 * 1024];
        let mut offset = 0;
        let mut equal = true;
        loop {
            let count = reader.read(&mut buffer).map_err(io)?;
            if count == 0 {
                break;
            }
            let end = offset + count;
            equal &= expected.get(offset..end) == Some(&buffer[..count]);
            offset = end;
        }
        let after = file.metadata().map_err(io)?;
        private_file(&after, MAX_ORIGINAL_OPERATION_BYTES)?;
        if !same(before, &after) || offset as u64 != after.len() {
            return Err(unavailable(
                "Inbox operation mirror changed during bounded observation",
            ));
        }
        #[cfg(test)]
        READ_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let current =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        let named = current.metadata().map_err(io)?;
        private_file(&named, MAX_ORIGINAL_OPERATION_BYTES)?;
        if !same(&after, &named) {
            return Err(unavailable("Inbox operation mirror filename changed"));
        }
        self.validate()?;
        Ok(equal && offset == expected.len())
    }
    fn flush_operation(
        &self,
        mut file: File,
        name: &str,
        before: &Metadata,
        expected: &[u8],
    ) -> Result<()> {
        // Sync the same descriptor whose complete private bytes/name were proven.
        full_sync(&file).map_err(file_error)?;
        #[cfg(test)]
        SYNC_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        sync_directory(&self.directory).map_err(file_error)?;
        if !self.matches_operation_bytes(&mut file, name, before, expected)? {
            return Err(unavailable(
                "Inbox operation mirror changed during durability confirmation",
            ));
        }
        self.validate()
    }
    fn endpoints(operation: &OriginalOperationFile) -> (String, String) {
        match operation {
            OriginalOperationFile::Remove(r) => (
                original(r.request.item_id),
                retained(r.request.operation_id),
            ),
            OriginalOperationFile::Restore(r) => (
                retained(r.request.removal_operation_id),
                original(r.original.capture.id),
            ),
            OriginalOperationFile::LegacyRemove(r) => (
                original(r.request.item_id),
                retained(r.request.operation_id),
            ),
            OriginalOperationFile::LegacyRestore(r) => (
                retained(r.request.removal_operation_id),
                original(r.original.capture.id),
            ),
        }
    }
    /// A checked observation can certify an already-performed move. An intent
    /// alone never asks startup to initiate a removal or restoration.
    pub(crate) fn operation_effect_observed(
        &self,
        operation: &OriginalOperationFile,
    ) -> Result<bool> {
        self.check_operation(operation)?;
        let (from, to) = Self::endpoints(operation);
        if self.absent(&from)? && !self.absent(&to)? {
            self.flush_exact(operation.item(), &to)?;
            return Ok(true);
        }
        if !self.absent(&from)? && self.absent(&to)? {
            self.exact(operation.item(), &from)?;
            return Ok(false);
        }
        Err(unavailable(
            "Inbox operation has occupied or missing endpoints; artifacts remain retained",
        ))
    }
    fn flush_exact(&self, item: &InboxItem, name: &str) -> Result<()> {
        self.exact(item, name)?;
        let file =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        private_file(&file.metadata().map_err(io)?, crate::MAX_NOTE_BYTES)?;
        let m = file.metadata().map_err(io)?;
        if (m.dev(), m.ino()) != (item.capture.copy.file_device, item.capture.copy.file_inode) {
            return Err(unavailable(
                "Inbox exact copy changed before durability confirmation",
            ));
        }
        full_sync(&file).map_err(file_error)?;
        sync_directory(&self.directory).map_err(file_error)?;
        self.exact(item, name)?;
        self.validate()
    }
    pub(crate) fn move_original(&self, operation: &OriginalOperationFile) -> Result<()> {
        self.check_operation(operation)?;
        if operation.settled() || encode(&self.operation(&operation.name())?)? != encode(operation)?
        {
            return Err(unavailable(
                "Inbox effect requires its exact durable intent",
            ));
        }
        let (from, to) = Self::endpoints(operation);
        self.exact(operation.item(), &from)?;
        if !self.absent(&to)? {
            return Err(unavailable("Inbox original destination is occupied"));
        }
        self.validate()?;
        checkpoint("original_operation_before_rename")?;
        rename_flags(
            &self.directory,
            &cstring(Path::new(&from)).map_err(file_error)?,
            &cstring(Path::new(&to)).map_err(file_error)?,
            libc::RENAME_EXCL,
        )
        .map_err(file_error)?;
        checkpoint("original_operation_renamed")?;
        self.flush_exact(operation.item(), &to)?;
        if !self.absent(&from)? {
            return Err(unavailable(
                "Inbox original endpoint was recreated during the move",
            ));
        }
        checkpoint("original_operation_synced")
    }
    pub(crate) fn retained_copy(
        &self,
        item: &InboxItem,
        operation_id: Uuid,
        namespace: &InboxOriginalNamespace,
    ) -> Result<()> {
        if operation_id.is_nil() || *namespace != self.original_namespace() {
            return Err(unavailable(
                "Inbox retained copy belongs to another operation or namespace",
            ));
        }
        self.exact(item, &retained(operation_id))?;
        Ok(())
    }
    pub(crate) fn original_occupied(&self, item: &InboxItem) -> Result<bool> {
        self.validate_item(item)?;
        self.validate()?;
        Ok(!self.absent(&original(item.capture.id))?)
    }
}

#[cfg(test)]
thread_local! {
    static READ_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static SYNC_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
mod tests {
    include!("inbox_original_operations_tests.rs");
}
