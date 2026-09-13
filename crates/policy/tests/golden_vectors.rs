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
            181, 222, 134, 94, 95, 78, 81, 129, 119, 53, 74, 10, 139, 65, 36, 116, 226, 239, 148,
            183, 231, 66, 179, 141, 148, 38, 231, 4, 187, 15, 171, 26,
        ]
    );
    assert_eq!(
        transition.signature().to_bytes(),
        [
            16, 90, 89, 164, 103, 20, 161, 112, 149, 88, 180, 233, 182, 122, 8, 94, 105, 50, 241,
            32, 232, 143, 235, 184, 245, 22, 88, 52, 196, 238, 169, 132, 84, 116, 74, 224, 126,
            127, 145, 74, 35, 91, 206, 182, 190, 199, 19, 151, 6, 205, 176, 151, 129, 122, 184, 67,
            70, 145, 186, 155, 250, 233, 60, 10,
        ]
    );
}
