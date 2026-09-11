use crosslab_protocol::{ControlSequence, SequenceError};

#[test]
fn first_control_message_uses_sequence_zero() {
    let mut sequence = ControlSequence::new();

    assert_eq!(sequence.expected(), Some(0));
    sequence.accept(0).unwrap();
    assert_eq!(sequence.expected(), Some(1));
}

#[test]
fn ordered_sequence_advances_exactly_one() {
    let mut sequence = ControlSequence::new();

    sequence.accept(0).unwrap();
    sequence.accept(1).unwrap();
    sequence.accept(2).unwrap();

    assert_eq!(sequence.expected(), Some(3));
}

#[test]
fn duplicate_or_lower_sequence_is_replay() {
    let mut sequence = ControlSequence::new();
    sequence.accept(0).unwrap();
    sequence.accept(1).unwrap();

    assert_eq!(
        sequence.accept(1).unwrap_err(),
        SequenceError::ReplayDetected {
            expected: 2,
            received: 1,
        }
    );
    assert_eq!(
        sequence.accept(0).unwrap_err(),
        SequenceError::ReplayDetected {
            expected: 2,
            received: 0,
        }
    );
}

#[test]
fn sequence_gap_is_rejected_without_advancing_state() {
    let mut sequence = ControlSequence::new();
    sequence.accept(0).unwrap();

    assert_eq!(
        sequence.accept(2).unwrap_err(),
        SequenceError::Gap {
            expected: 1,
            received: 2,
        }
    );
    assert_eq!(sequence.expected(), Some(1));
    sequence.accept(1).unwrap();
}

#[test]
fn sequence_space_exhaustion_is_terminal() {
    let mut sequence = ControlSequence::from_expected(u64::MAX);

    sequence.accept(u64::MAX).unwrap();
    assert_eq!(sequence.expected(), None);
    assert_eq!(
        sequence.accept(u64::MAX).unwrap_err(),
        SequenceError::Exhausted
    );
}
