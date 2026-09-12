use quinn::{ReadExactError, RecvStream, SendStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordError {
    Empty,
    TooLarge { declared: usize, max: usize },
    Finished,
    Read,
    Write,
}

pub(crate) async fn write_record(
    send: &mut SendStream,
    bytes: &[u8],
    max: usize,
) -> Result<(), RecordError> {
    if bytes.is_empty() {
        return Err(RecordError::Empty);
    }
    if bytes.len() > max {
        return Err(RecordError::TooLarge {
            declared: bytes.len(),
            max,
        });
    }

    let length = u32::try_from(bytes.len()).map_err(|_| RecordError::TooLarge {
        declared: bytes.len(),
        max: max.min(u32::MAX as usize),
    })?;
    send.write_all(&length.to_be_bytes())
        .await
        .map_err(|_| RecordError::Write)?;
    send.write_all(bytes)
        .await
        .map_err(|_| RecordError::Write)?;
    Ok(())
}

pub(crate) async fn read_record(
    recv: &mut RecvStream,
    max: usize,
    allow_empty: bool,
) -> Result<Vec<u8>, RecordError> {
    let mut prefix = [0_u8; 4];
    match recv.read_exact(&mut prefix).await {
        Ok(()) => {}
        Err(ReadExactError::FinishedEarly(0)) => return Err(RecordError::Finished),
        Err(_) => return Err(RecordError::Read),
    }

    let declared = u32::from_be_bytes(prefix) as usize;
    if declared > max {
        return Err(RecordError::TooLarge { declared, max });
    }
    if declared == 0 && !allow_empty {
        return Err(RecordError::Empty);
    }

    let mut bytes = vec![0_u8; declared];
    recv.read_exact(&mut bytes)
        .await
        .map_err(|_| RecordError::Read)?;
    Ok(bytes)
}
