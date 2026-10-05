use super::*;
use brn_store::work::inbox_original_operations::{
    InboxOriginalNamespace, InboxOriginalRemovalRecord, MAX_ORIGINAL_OPERATION_BYTES,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationEnvelope {
    format: u8,
    sha256: [u8; 32],
    operation: OriginalOperationFile,
}
fn encode(operation: &OriginalOperationFile) -> Result<Vec<u8>> {
    operation.validate()?;
    let body = serde_json::to_vec(operation)
        .map_err(|_| unavailable("could not encode Inbox operation"))?;
    let bytes = serde_json::to_vec(&OperationEnvelope {
        format: 1,
        sha256: digest(&body),
        operation: operation.clone(),
    })
    .map_err(|_| unavailable("could not encode Inbox operation mirror"))?;
    if bytes.len() > MAX_ORIGINAL_OPERATION_BYTES {
        return Err(unavailable(
            "complete Inbox operation mirror exceeds its bound",
        ));
    }
    Ok(bytes)
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
        let namespace = match operation {
            OriginalOperationFile::Remove(r) => &r.namespace,
            OriginalOperationFile::Restore(r) => &r.namespace,
        };
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
        let envelope: OperationEnvelope = serde_json::from_slice(&bytes)
            .map_err(|_| unavailable("invalid Inbox operation mirror"))?;
        if envelope.format != 1
            || envelope.operation.name() != name
            || encode(&envelope.operation)? != bytes
        {
            return Err(unavailable(
                "Inbox operation mirror format, hash, filename or canonical binding differs",
            ));
        }
        self.check_operation(&envelope.operation)?;
        let current =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        if !same(&after, &current.metadata().map_err(io)?) {
            return Err(unavailable("Inbox operation mirror filename changed"));
        }
        self.validate()?;
        Ok(envelope.operation)
    }
    pub(crate) fn publish_operation(&self, operation: &OriginalOperationFile) -> Result<()> {
        self.check_operation(operation)?;
        let bytes = encode(operation)?;
        let name = operation.name();
        if !self.absent(&name)? {
            if encode(&self.operation(&name)?)? != bytes {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "Inbox operation mirror is occupied by another exact record",
                ));
            }
            self.flush_operation(&name)?;
            return Ok(());
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
    fn flush_operation(&self, name: &str) -> Result<()> {
        let expected = encode(&self.operation(name)?)?;
        let file =
            open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0).map_err(file_error)?;
        private_file(&file.metadata().map_err(io)?, MAX_ORIGINAL_OPERATION_BYTES)?;
        full_sync(&file).map_err(file_error)?;
        sync_directory(&self.directory).map_err(file_error)?;
        if encode(&self.operation(name)?)? != expected {
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
    pub(crate) fn retained_original(&self, record: &InboxOriginalRemovalRecord) -> Result<()> {
        record.validate()?;
        self.retained_copy(
            &record.evidence.snapshot.review.original,
            record.request.operation_id,
            &record.namespace,
        )
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
