use std::{collections::BTreeMap, path::PathBuf};

use crosslab_agent::{
    FileTransferDataEvent, FileTransferIntegrityError, FileTransferStateMatch,
};
use crosslab_identity::DeviceId;
use crosslab_protocol::{
    FileTransferOffer, FileTransferTerminalOutcome, RequestId, StreamId, TransferId,
};

use super::{
    LinuxFileTransferError, LinuxFileTransferReceiver, LinuxFileTransferRecovery,
    LinuxFileTransferStateStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxIncomingFileTransfer {
    request_id: RequestId,
    source_device_id: DeviceId,
    offer: FileTransferOffer,
}

impl LinuxIncomingFileTransfer {
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    pub const fn source_device_id(&self) -> DeviceId {
        self.source_device_id
    }

    pub const fn offer(&self) -> &FileTransferOffer {
        &self.offer
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxFileTransferRequestAction {
    ChooseDestination(LinuxIncomingFileTransfer),
    Ready {
        request_id: RequestId,
        transfer_id: TransferId,
        resume_offset: u64,
    },
    AlreadyComplete {
        request_id: RequestId,
        transfer_id: TransferId,
    },
}

#[derive(Debug)]
pub enum LinuxFileTransferDataAction {
    Continue,
    Terminal {
        transfer_id: TransferId,
        outcome: FileTransferTerminalOutcome,
    },
    Abort {
        transfer_id: TransferId,
        stream_id: StreamId,
        outcome: FileTransferTerminalOutcome,
        error: LinuxFileTransferError,
    },
}

struct ActiveReceive {
    receiver: LinuxFileTransferReceiver,
    stream_id: Option<StreamId>,
}

pub struct LinuxFileTransferService {
    store: LinuxFileTransferStateStore,
    active: BTreeMap<TransferId, ActiveReceive>,
}

impl LinuxFileTransferService {
    pub fn from_environment() -> Result<Self, LinuxFileTransferError> {
        Ok(Self::new(LinuxFileTransferStateStore::from_environment()?))
    }

    fn new(store: LinuxFileTransferStateStore) -> Self {
        Self {
            store,
            active: BTreeMap::new(),
        }
    }

    pub fn handle_request(
        &mut self,
        request_id: RequestId,
        source_device_id: DeviceId,
        offer: FileTransferOffer,
        now_unix_secs: u64,
    ) -> Result<LinuxFileTransferRequestAction, LinuxFileTransferError> {
        let transfer_id = offer.transfer_id();
        if self.active.contains_key(&transfer_id) {
            return Err(LinuxFileTransferError::AlreadyActive);
        }

        let snapshot = self.store.load()?;
        match snapshot.find(source_device_id, &offer)? {
            None => Ok(LinuxFileTransferRequestAction::ChooseDestination(
                LinuxIncomingFileTransfer {
                    request_id,
                    source_device_id,
                    offer,
                },
            )),
            Some(FileTransferStateMatch::AlreadyComplete(_)) => {
                Ok(LinuxFileTransferRequestAction::AlreadyComplete {
                    request_id,
                    transfer_id,
                })
            }
            Some(FileTransferStateMatch::Partial(_)) => {
                match LinuxFileTransferReceiver::recover(
                    self.store.clone(),
                    source_device_id,
                    offer,
                    now_unix_secs,
                )? {
                    LinuxFileTransferRecovery::Ready(receiver) => {
                        let resume_offset = receiver.offset();
                        self.active.insert(
                            transfer_id,
                            ActiveReceive {
                                receiver: *receiver,
                                stream_id: None,
                            },
                        );
                        Ok(LinuxFileTransferRequestAction::Ready {
                            request_id,
                            transfer_id,
                            resume_offset,
                        })
                    }
                    LinuxFileTransferRecovery::AlreadyComplete => {
                        Ok(LinuxFileTransferRequestAction::AlreadyComplete {
                            request_id,
                            transfer_id,
                        })
                    }
                }
            }
        }
    }

    pub fn accept_destination(
        &mut self,
        incoming: LinuxIncomingFileTransfer,
        final_path: PathBuf,
        now_unix_secs: u64,
    ) -> Result<LinuxFileTransferRequestAction, LinuxFileTransferError> {
        let transfer_id = incoming.offer.transfer_id();
        if self.active.contains_key(&transfer_id) {
            return Err(LinuxFileTransferError::AlreadyActive);
        }

        let receiver = LinuxFileTransferReceiver::create(
            self.store.clone(),
            incoming.source_device_id,
            incoming.offer,
            final_path,
            now_unix_secs,
        )?;
        let resume_offset = receiver.offset();
        self.active.insert(
            transfer_id,
            ActiveReceive {
                receiver,
                stream_id: None,
            },
        );
        Ok(LinuxFileTransferRequestAction::Ready {
            request_id: incoming.request_id,
            transfer_id,
            resume_offset,
        })
    }

    pub fn release_transfer(&mut self, transfer_id: TransferId) -> bool {
        self.active.remove(&transfer_id).is_some()
    }

    pub fn handle_data(
        &mut self,
        event: FileTransferDataEvent,
        now_unix_secs: u64,
    ) -> LinuxFileTransferDataAction {
        match event {
            FileTransferDataEvent::Opened {
                transfer_id,
                stream_id,
                resume_offset,
            } => {
                let Some(active) = self.active.get_mut(&transfer_id) else {
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                };
                if active.stream_id.is_some() || active.receiver.offset() != resume_offset {
                    self.active.remove(&transfer_id);
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                }
                active.stream_id = Some(stream_id);
                LinuxFileTransferDataAction::Continue
            }
            FileTransferDataEvent::Chunk(chunk) => {
                let transfer_id = chunk.transfer_id();
                let stream_id = chunk.stream_id();
                let result = match self.active.get_mut(&transfer_id) {
                    Some(active) if active.stream_id == Some(stream_id) => {
                        active.receiver.write_chunk(chunk.bytes(), now_unix_secs)
                    }
                    _ => Err(LinuxFileTransferError::InvalidStream),
                };
                match result {
                    Ok(()) => LinuxFileTransferDataAction::Continue,
                    Err(error) => {
                        self.active.remove(&transfer_id);
                        abort(transfer_id, stream_id, error)
                    }
                }
            }
            FileTransferDataEvent::Finished {
                transfer_id,
                stream_id,
            } => {
                let Some(active) = self.active.remove(&transfer_id) else {
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                };
                if active.stream_id != Some(stream_id) {
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                }
                match active.receiver.finish(now_unix_secs) {
                    Ok(()) => LinuxFileTransferDataAction::Terminal {
                        transfer_id,
                        outcome: FileTransferTerminalOutcome::Completed,
                    },
                    Err(error) => abort(transfer_id, stream_id, error),
                }
            }
            FileTransferDataEvent::Cancelled {
                transfer_id,
                stream_id,
            } => {
                let Some(active) = self.active.remove(&transfer_id) else {
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                };
                if active.stream_id != Some(stream_id) {
                    return abort(
                        transfer_id,
                        stream_id,
                        LinuxFileTransferError::InvalidStream,
                    );
                }
                LinuxFileTransferDataAction::Terminal {
                    transfer_id,
                    outcome: FileTransferTerminalOutcome::Cancelled,
                }
            }
        }
    }

    #[cfg(test)]
    fn for_test(store: LinuxFileTransferStateStore) -> Self {
        Self::new(store)
    }
}

fn abort(
    transfer_id: TransferId,
    stream_id: StreamId,
    error: LinuxFileTransferError,
) -> LinuxFileTransferDataAction {
    let outcome = match error {
        LinuxFileTransferError::Integrity(
            FileTransferIntegrityError::SizeMismatch
            | FileTransferIntegrityError::DigestMismatch
            | FileTransferIntegrityError::SizeOverflow,
        ) => FileTransferTerminalOutcome::IntegrityFailed,
        _ => FileTransferTerminalOutcome::StorageFailed,
    };
    LinuxFileTransferDataAction::Abort {
        transfer_id,
        stream_id,
        outcome,
        error,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fmt::Write as _, fs};

    use crosslab_agent::{FileTransferDataChunk, FileTransferStateError};
    use crosslab_crypto::{blake3_256, random_bytes};
    use crosslab_protocol::{FileTransferDigest, FILE_TRANSFER_CHECKPOINT_BYTES};

    use super::*;

    #[test]
    fn fresh_offer_requires_owner_destination_before_receiver_exists() {
        let root = test_root("fresh");
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x31; 32]);
        let offer = test_offer(0x32, b"payload");
        let mut service = LinuxFileTransferService::for_test(
            LinuxFileTransferStateStore::for_test(state_path),
        );

        let action = service
            .handle_request(
                RequestId::from_bytes([0x33; 16]),
                source,
                offer.clone(),
                10,
            )
            .unwrap();
        let LinuxFileTransferRequestAction::ChooseDestination(incoming) = action else {
            panic!("fresh transfer should require owner destination");
        };
        assert_eq!(incoming.source_device_id(), source);
        assert_eq!(incoming.offer(), &offer);
        assert!(!final_path.exists());

        let action = service.accept_destination(incoming, final_path, 11).unwrap();
        assert!(matches!(
            action,
            LinuxFileTransferRequestAction::Ready {
                transfer_id,
                resume_offset: 0,
                ..
            } if transfer_id == offer.transfer_id()
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn data_events_publish_verified_file_and_complete_idempotently() {
        let root = test_root("complete");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x41; 32]);
        let payload = b"verified-payload";
        let offer = test_offer(0x42, payload);
        let transfer_id = offer.transfer_id();
        let request_id = RequestId::from_bytes([0x43; 16]);
        let stream_id = StreamId::from_bytes([0x44; 16]);
        let mut service = LinuxFileTransferService::for_test(
            LinuxFileTransferStateStore::for_test(state_path),
        );

        let LinuxFileTransferRequestAction::ChooseDestination(incoming) = service
            .handle_request(request_id, source, offer.clone(), 20)
            .unwrap()
        else {
            panic!("fresh transfer should require owner destination");
        };
        service
            .accept_destination(incoming, final_path.clone(), 21)
            .unwrap();

        assert!(matches!(
            service.handle_data(
                FileTransferDataEvent::Opened {
                    transfer_id,
                    stream_id,
                    resume_offset: 0,
                },
                22
            ),
            LinuxFileTransferDataAction::Continue
        ));
        assert!(matches!(
            service.handle_data(
                FileTransferDataEvent::Chunk(FileTransferDataChunk::new(
                    transfer_id,
                    stream_id,
                    payload.to_vec(),
                )),
                23
            ),
            LinuxFileTransferDataAction::Continue
        ));
        assert!(matches!(
            service.handle_data(
                FileTransferDataEvent::Finished {
                    transfer_id,
                    stream_id,
                },
                24
            ),
            LinuxFileTransferDataAction::Terminal {
                transfer_id: completed,
                outcome: FileTransferTerminalOutcome::Completed,
            } if completed == transfer_id
        ));
        assert_eq!(fs::read(&final_path).unwrap(), payload);

        assert!(matches!(
            service
                .handle_request(
                    RequestId::from_bytes([0x45; 16]),
                    source,
                    offer,
                    25
                )
                .unwrap(),
            LinuxFileTransferRequestAction::AlreadyComplete {
                transfer_id: completed,
                ..
            } if completed == transfer_id
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn retained_partial_resumes_only_from_durable_checkpoint() {
        let root = test_root("resume");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x51; 32]);
        let checkpoint = usize::try_from(FILE_TRANSFER_CHECKPOINT_BYTES).unwrap();
        let payload = vec![0x5a; checkpoint + 17];
        let offer = test_offer(0x52, &payload);
        let transfer_id = offer.transfer_id();

        {
            let mut service = LinuxFileTransferService::for_test(
                LinuxFileTransferStateStore::for_test(state_path.clone()),
            );
            let LinuxFileTransferRequestAction::ChooseDestination(incoming) = service
                .handle_request(
                    RequestId::from_bytes([0x53; 16]),
                    source,
                    offer.clone(),
                    30,
                )
                .unwrap()
            else {
                panic!("fresh transfer should require owner destination");
            };
            service
                .accept_destination(incoming, final_path.clone(), 31)
                .unwrap();
            let stream_id = StreamId::from_bytes([0x54; 16]);
            service.handle_data(
                FileTransferDataEvent::Opened {
                    transfer_id,
                    stream_id,
                    resume_offset: 0,
                },
                32,
            );
            for chunk in payload[..checkpoint + 5].chunks(64 * 1024) {
                assert!(matches!(
                    service.handle_data(
                        FileTransferDataEvent::Chunk(FileTransferDataChunk::new(
                            transfer_id,
                            stream_id,
                            chunk.to_vec(),
                        )),
                        33
                    ),
                    LinuxFileTransferDataAction::Continue
                ));
            }
        }

        let mut service = LinuxFileTransferService::for_test(
            LinuxFileTransferStateStore::for_test(state_path),
        );
        assert!(matches!(
            service
                .handle_request(
                    RequestId::from_bytes([0x55; 16]),
                    source,
                    offer,
                    34
                )
                .unwrap(),
            LinuxFileTransferRequestAction::Ready {
                transfer_id: resumed,
                resume_offset: FILE_TRANSFER_CHECKPOINT_BYTES,
                ..
            } if resumed == transfer_id
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn retained_transfer_rejects_changed_authenticated_source() {
        let root = test_root("identity");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x61; 32]);
        let offer = test_offer(0x62, b"payload");

        let mut service = LinuxFileTransferService::for_test(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
        );
        let LinuxFileTransferRequestAction::ChooseDestination(incoming) = service
            .handle_request(
                RequestId::from_bytes([0x63; 16]),
                source,
                offer.clone(),
                40,
            )
            .unwrap()
        else {
            panic!("fresh transfer should require owner destination");
        };
        service
            .accept_destination(incoming, final_path, 41)
            .unwrap();
        drop(service);

        let mut service = LinuxFileTransferService::for_test(
            LinuxFileTransferStateStore::for_test(state_path),
        );
        assert!(matches!(
            service.handle_request(
                RequestId::from_bytes([0x64; 16]),
                DeviceId::from_bytes([0x65; 32]),
                offer,
                42
            ),
            Err(LinuxFileTransferError::State(
                FileTransferStateError::IdentityMismatch
            ))
        ));

        let _ = fs::remove_dir_all(root);
    }

    fn test_offer(tag: u8, payload: &[u8]) -> FileTransferOffer {
        FileTransferOffer::new(
            TransferId::from_bytes([tag; 32]),
            "payload.bin".into(),
            u64::try_from(payload.len()).unwrap(),
            FileTransferDigest::from_bytes(blake3_256(payload)),
        )
        .unwrap()
    }

    fn test_root(label: &str) -> PathBuf {
        let nonce = random_bytes::<8>().unwrap();
        let mut suffix = String::with_capacity(16);
        for byte in nonce {
            write!(&mut suffix, "{byte:02x}").unwrap();
        }
        env::temp_dir().join(format!("crosslab-file-service-{label}-{suffix}"))
    }
}
