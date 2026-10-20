use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferStage {
    Idle,
    AwaitingDestination,
    Preparing,
    WaitingForPeer,
    Ready,
    Transferring,
    Finalizing,
    Completed,
    AlreadyComplete,
    Cancelled,
    Failed,
}

impl FileTransferStage {
    pub const fn active(self) -> bool {
        matches!(
            self,
            Self::AwaitingDestination
                | Self::Preparing
                | Self::WaitingForPeer
                | Self::Ready
                | Self::Transferring
                | Self::Finalizing
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferFailure {
    Source,
    NotConnected,
    NotNegotiated,
    Denied,
    ResourceLimit,
    TimedOut,
    Integrity,
    Storage,
    Connection,
    Failed,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferOperationState {
    stage: FileTransferStage,
    display_name: Option<String>,
    transferred_bytes: u64,
    total_bytes: u64,
    failure: Option<FileTransferFailure>,
}

impl FileTransferOperationState {
    const fn idle() -> Self {
        Self {
            stage: FileTransferStage::Idle,
            display_name: None,
            transferred_bytes: 0,
            total_bytes: 0,
            failure: None,
        }
    }

    pub const fn stage(&self) -> FileTransferStage {
        self.stage
    }

    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    pub const fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub const fn failure(&self) -> Option<FileTransferFailure> {
        self.failure
    }

    pub const fn active(&self) -> bool {
        self.stage.active()
    }

    fn set(
        &mut self,
        stage: FileTransferStage,
        transferred_bytes: u64,
        total_bytes: u64,
        failure: Option<FileTransferFailure>,
    ) {
        self.stage = stage;
        self.transferred_bytes = transferred_bytes.min(total_bytes);
        self.total_bytes = total_bytes;
        self.failure = failure;
    }

    fn reset(&mut self) {
        *self = Self::idle();
    }
}

impl fmt::Debug for FileTransferOperationState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferOperationState")
            .field("stage", &self.stage)
            .field(
                "display_name",
                &self
                    .display_name
                    .as_ref()
                    .map(|name| format!("[REDACTED; {} bytes]", name.len())),
            )
            .field("transferred_bytes", &self.transferred_bytes)
            .field("total_bytes", &self.total_bytes)
            .field("failure", &self.failure)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransferFeatureState {
    available: bool,
    send: FileTransferOperationState,
    receive: FileTransferOperationState,
}

impl FileTransferFeatureState {
    pub const fn new() -> Self {
        Self {
            available: false,
            send: FileTransferOperationState::idle(),
            receive: FileTransferOperationState::idle(),
        }
    }

    pub const fn available(&self) -> bool {
        self.available
    }

    pub const fn send(&self) -> &FileTransferOperationState {
        &self.send
    }

    pub const fn receive(&self) -> &FileTransferOperationState {
        &self.receive
    }

    pub fn set_available(&mut self, available: bool) {
        self.available = available;
        if !available {
            if self.send.active() {
                self.send.set(
                    FileTransferStage::Failed,
                    self.send.transferred_bytes,
                    self.send.total_bytes,
                    Some(FileTransferFailure::Connection),
                );
            }
            if self.receive.active() {
                self.receive.set(
                    FileTransferStage::Failed,
                    self.receive.transferred_bytes,
                    self.receive.total_bytes,
                    Some(FileTransferFailure::Connection),
                );
            }
        }
    }

    pub fn begin_send(&mut self, display_name: String) {
        if !self.available {
            return;
        }
        self.send.display_name = Some(display_name);
        self.send.set(FileTransferStage::Preparing, 0, 0, None);
    }

    pub fn send_waiting(&mut self, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.send
            .set(FileTransferStage::WaitingForPeer, 0, total_bytes, None);
    }

    pub fn send_progress(&mut self, transferred_bytes: u64, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.send.set(
            FileTransferStage::Transferring,
            transferred_bytes,
            total_bytes,
            None,
        );
    }

    pub fn send_finalizing(&mut self, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.send.set(
            FileTransferStage::Finalizing,
            total_bytes,
            total_bytes,
            None,
        );
    }

    pub fn send_completed(&mut self, total_bytes: u64, already_complete: bool) {
        if !self.available {
            return;
        }
        self.send.set(
            if already_complete {
                FileTransferStage::AlreadyComplete
            } else {
                FileTransferStage::Completed
            },
            total_bytes,
            total_bytes,
            None,
        );
    }

    pub fn send_cancelled(&mut self, transferred_bytes: u64, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.send.set(
            FileTransferStage::Cancelled,
            transferred_bytes,
            total_bytes,
            None,
        );
    }

    pub fn send_failed(
        &mut self,
        transferred_bytes: u64,
        total_bytes: u64,
        failure: FileTransferFailure,
    ) {
        if !self.available {
            return;
        }
        self.send.set(
            FileTransferStage::Failed,
            transferred_bytes,
            total_bytes,
            Some(failure),
        );
    }

    pub fn incoming_offer(&mut self, display_name: String, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.receive.display_name = Some(display_name);
        self.receive
            .set(FileTransferStage::AwaitingDestination, 0, total_bytes, None);
    }

    pub fn receive_ready(&mut self, resume_offset: u64, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.receive
            .set(FileTransferStage::Ready, resume_offset, total_bytes, None);
    }

    pub fn receive_progress(&mut self, received_bytes: u64, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.receive.set(
            FileTransferStage::Transferring,
            received_bytes,
            total_bytes,
            None,
        );
    }

    pub fn receive_completed(&mut self, total_bytes: u64, already_complete: bool) {
        if !self.available {
            return;
        }
        self.receive.set(
            if already_complete {
                FileTransferStage::AlreadyComplete
            } else {
                FileTransferStage::Completed
            },
            total_bytes,
            total_bytes,
            None,
        );
    }

    pub fn receive_cancelled(&mut self, received_bytes: u64, total_bytes: u64) {
        if !self.available {
            return;
        }
        self.receive.set(
            FileTransferStage::Cancelled,
            received_bytes,
            total_bytes,
            None,
        );
    }

    pub fn receive_failed(
        &mut self,
        received_bytes: u64,
        total_bytes: u64,
        failure: FileTransferFailure,
    ) {
        if !self.available {
            return;
        }
        self.receive.set(
            FileTransferStage::Failed,
            received_bytes,
            total_bytes,
            Some(failure),
        );
    }

    pub fn reset_send(&mut self) {
        self.send.reset();
    }

    pub fn reset_receive(&mut self) {
        self.receive.reset();
    }
}

impl Default for FileTransferFeatureState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keeps_send_and_receive_independent() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("send.bin".into());
        state.incoming_offer("receive.bin".into(), 100);
        state.send_progress(40, 80);
        state.receive_ready(20, 100);

        assert_eq!(state.send().stage(), FileTransferStage::Transferring);
        assert_eq!(state.send().transferred_bytes(), 40);
        assert_eq!(state.receive().stage(), FileTransferStage::Ready);
        assert_eq!(state.receive().transferred_bytes(), 20);
    }

    #[test]
    fn finalizing_send_stays_active_without_fake_remaining_bytes() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("send.bin".into());
        state.send_finalizing(128);

        assert_eq!(state.send().stage(), FileTransferStage::Finalizing);
        assert_eq!(state.send().transferred_bytes(), 128);
        assert_eq!(state.send().total_bytes(), 128);
        assert!(state.send().active());
    }

    #[test]
    fn debug_redacts_file_names() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("private-name.txt".into());

        let debug = format!("{state:?}");
        assert!(!debug.contains("private-name.txt"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn unavailable_runtime_cancels_active_operations() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("send.bin".into());
        state.incoming_offer("receive.bin".into(), 100);
        state.set_available(false);

        assert_eq!(state.send().stage(), FileTransferStage::Failed);
        assert_eq!(
            state.send().failure(),
            Some(FileTransferFailure::Connection)
        );
        assert_eq!(state.receive().stage(), FileTransferStage::Failed);
        assert_eq!(
            state.receive().failure(),
            Some(FileTransferFailure::Connection)
        );
    }
    #[test]
    fn connection_loss_preserves_bounded_progress_and_does_not_claim_owner_cancel() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("first.bin".into());
        state.send_progress(64, 128);
        state.incoming_offer("second.bin".into(), 200);
        state.receive_progress(90, 200);
        state.set_available(false);

        assert_eq!(
            state.send().failure(),
            Some(FileTransferFailure::Connection)
        );
        assert_eq!(
            state.receive().failure(),
            Some(FileTransferFailure::Connection)
        );
        assert_eq!(state.send().transferred_bytes(), 64);
        assert_eq!(state.receive().transferred_bytes(), 90);

        state.send_completed(128, false);
        state.receive_completed(200, false);
        state.send_progress(128, 128);
        state.receive_progress(200, 200);
        assert_eq!(state.send().stage(), FileTransferStage::Failed);
        assert_eq!(state.receive().stage(), FileTransferStage::Failed);
    }

    #[test]
    fn stopped_service_refuses_late_offers_or_new_transfer_authority() {
        let mut state = FileTransferFeatureState::new();
        state.begin_send("unavailable.bin".into());
        state.incoming_offer("ignored.bin".into(), 22);
        assert_eq!(state.send().stage(), FileTransferStage::Idle);
        assert_eq!(state.receive().stage(), FileTransferStage::Idle);

        state.set_available(true);
        state.begin_send("active.bin".into());
        state.set_available(false);
        state.send_waiting(50);
        state.send_finalizing(50);
        state.send_cancelled(0, 50);
        state.send_failed(0, 50, FileTransferFailure::Storage);
        assert_eq!(
            state.send().failure(),
            Some(FileTransferFailure::Connection)
        );

        state.set_available(true);
        state.begin_send("fresh.bin".into());
        assert_eq!(state.send().stage(), FileTransferStage::Preparing);
        assert_eq!(state.send().failure(), None);
    }

    #[test]
    fn completed_transfer_remains_completed_when_service_goes_offline() {
        let mut state = FileTransferFeatureState::new();
        state.set_available(true);
        state.begin_send("complete.bin".into());
        state.send_completed(1024, false);
        state.set_available(false);
        assert_eq!(state.send().stage(), FileTransferStage::Completed);
        assert_eq!(state.send().transferred_bytes(), 1024);
        assert_eq!(state.send().failure(), None);
    }
}
