use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId, OwnerRootRecord};
use crosslab_policy::{TransitionId, TrustRecord, TrustTransition};

#[test]
fn trust_revocation_transition_v1_golden_vector() {
    let owner_id = OwnerId::from_bytes([0x11; 32]);
    let root_key = SigningKey::from_secret_bytes([0x22; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let record = TrustRecord::trusted(
        owner_id,
        DeviceId::from_bytes([0x44; 32]),
        9,
        TransitionId::from_bytes([0x55; 32]),
    );
    let transition = TrustTransition::issue_root_revocation(
        &record,
        TransitionId::from_bytes([0x66; 32]),
        &root,
        &root_key,
    )
    .unwrap();

    assert_eq!(
        transition.transcript_digest(),
        [
            181, 222, 134, 94, 95, 78, 81, 129, 119, 53, 74, 10, 139, 65, 36, 116, 226, 239,
            148, 183, 231, 66, 179, 141, 148, 38, 231, 4, 187, 15, 171, 26,
        ]
    );
    assert_eq!(transition.signature().to_bytes(), [0; 64]);
}
