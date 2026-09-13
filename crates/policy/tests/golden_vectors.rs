use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustTransition};

#[test]
fn trust_revocation_transition_v1_golden_vector() {
    let owner_id = OwnerId::from_bytes([0x11; 32]);
    let root_key = SigningKey::from_secret_bytes([0x22; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x33; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let device_key = SigningKey::from_secret_bytes([0x43; 32]);
    let device_id = DeviceId::from_bytes([0x44; 32]);
    let initial_credential = DeviceCredential::issue(
        owner_id,
        device_id,
        &device_key,
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let pairing_transition = PairingTrustTransition::issue(
        &initial_credential,
        TransitionId::from_bytes([0x55; 32]),
        [0x45; 32],
        &root,
        &delegation,
        &issuer_key,
        delegation.delegation_epoch(),
    )
    .unwrap();
    let mut record = pairing_transition
        .establish(
            &initial_credential,
            &root,
            &delegation,
            delegation.delegation_epoch(),
        )
        .unwrap();

    for epoch in 1_u64..=9 {
        let credential = DeviceCredential::issue(
            owner_id,
            device_id,
            &device_key,
            epoch,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let mut transition_id = [0x56; 32];
        transition_id[..8].copy_from_slice(&epoch.to_be_bytes());
        record
            .accept_successor_credential(
                &credential,
                &root,
                &delegation,
                delegation.delegation_epoch(),
                TransitionId::from_bytes(transition_id),
            )
            .unwrap();
    }

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
            33, 112, 221, 247, 124, 124, 55, 197, 241, 100, 101, 60, 52, 20, 117, 166, 222, 114,
            188, 91, 135, 4, 189, 245, 103, 137, 226, 39, 112, 123, 212, 202,
        ]
    );
    assert_eq!(
        transition.signature().to_bytes(),
        [
            210, 46, 103, 202, 49, 230, 72, 110, 163, 68, 88, 108, 148, 174, 109, 101, 174,
            132, 205, 189, 89, 94, 215, 0, 102, 248, 246, 191, 155, 16, 38, 200, 160, 7, 250,
            4, 41, 55, 103, 202, 171, 121, 64, 134, 36, 194, 213, 136, 60, 56, 228, 220, 153,
            4, 109, 137, 96, 124, 28, 236, 184, 1, 42, 14,
        ]
    );
}
