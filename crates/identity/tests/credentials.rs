use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError, OwnerId,
    OwnerRootRecord, RootSuccessor,
};

fn fixture() -> (
    OwnerId,
    SigningKey,
    OwnerRootRecord,
    SigningKey,
    AuthorityDelegation,
) {
    let owner_id = OwnerId::from_bytes([1; 32]);
    let root_key = SigningKey::from_secret_bytes([2; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let device_signing_key = SigningKey::from_secret_bytes([3; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        0,
        &root_key,
    );

    (owner_id, root_key, root, device_signing_key, delegation)
}

#[test]
fn device_signing_delegation_verifies_against_owner_root() {
    let (_, _, root, _, delegation) = fixture();
    delegation.verify(&root, 0).unwrap();
}

#[test]
fn delegation_rejects_unknown_root_key() {
    let (owner_id, _, _, _, delegation) = fixture();
    let unknown_root = OwnerRootRecord::new(owner_id, &SigningKey::from_secret_bytes([20; 32]), 0);

    assert_eq!(
        delegation.verify(&unknown_root, 0),
        Err(IdentityError::UnknownIssuer)
    );
}

#[test]
fn stale_authority_epoch_is_rejected() {
    let (_, _, root, _, delegation) = fixture();

    assert_eq!(
        delegation.verify(&root, 1),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
}

#[test]
fn device_credential_verifies_through_root_and_device_signing_authority() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let device_id = DeviceId::from_bytes([4; 32]);
    let device_key = SigningKey::from_secret_bytes([5; 32]);

    let credential = DeviceCredential::issue(
        owner_id,
        device_id,
        &device_key,
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    credential.verify(&root, &delegation, 0, 0).unwrap();
    assert_eq!(credential.device_id(), device_id);
    assert_eq!(credential.credential_epoch(), 0);
}

#[test]
fn device_credential_rejects_mismatched_issuer_key() {
    let (owner_id, _, root, _, delegation) = fixture();
    let result = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([21; 32]),
        &SigningKey::from_secret_bytes([22; 32]),
        0,
        &root,
        &delegation,
        &SigningKey::from_secret_bytes([23; 32]),
    );

    assert_eq!(result.unwrap_err(), IdentityError::UnknownIssuer);
}

#[test]
fn device_credential_rejects_wrong_owner_domain() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([24; 32]),
        &SigningKey::from_secret_bytes([25; 32]),
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let wrong_owner_root = OwnerRootRecord::new(
        OwnerId::from_bytes([26; 32]),
        &SigningKey::from_secret_bytes([27; 32]),
        0,
    );

    assert_eq!(
        credential.verify(&wrong_owner_root, &delegation, 0, 0),
        Err(IdentityError::WrongOwner)
    );
}

#[test]
fn recovery_authority_cannot_issue_ordinary_device_credentials() {
    let owner_id = OwnerId::from_bytes([7; 32]);
    let root_key = SigningKey::from_secret_bytes([8; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let recovery_key = SigningKey::from_secret_bytes([9; 32]);
    let recovery = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Recovery,
        &recovery_key,
        0,
        &root_key,
    );
    let device_key = SigningKey::from_secret_bytes([10; 32]);

    let result = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([11; 32]),
        &device_key,
        0,
        &root,
        &recovery,
        &recovery_key,
    );

    assert_eq!(result.unwrap_err(), IdentityError::WrongIssuerRole);
}

#[test]
fn stale_credential_epoch_is_rejected() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([12; 32]),
        &SigningKey::from_secret_bytes([13; 32]),
        1,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    assert_eq!(
        credential.verify(&root, &delegation, 2, 0),
        Err(IdentityError::StaleCredentialEpoch)
    );
}

#[test]
fn device_key_rotation_preserves_device_id_and_advances_epoch() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let device_id = DeviceId::from_bytes([14; 32]);
    let current = DeviceCredential::issue(
        owner_id,
        device_id,
        &SigningKey::from_secret_bytes([15; 32]),
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    let rotated = current
        .rotate(
            &SigningKey::from_secret_bytes([16; 32]),
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();

    assert_eq!(rotated.device_id(), device_id);
    assert_eq!(rotated.credential_epoch(), 1);
    rotated.verify(&root, &delegation, 1, 0).unwrap();
}

#[test]
fn device_key_can_prove_possession_for_a_session_digest() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let device_key = SigningKey::from_secret_bytes([28; 32]);
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([29; 32]),
        &device_key,
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let session_digest = [0xa5; 32];
    let proof = device_key.sign_digest(&session_digest);

    credential
        .device_public_key()
        .verify_digest(&session_digest, &proof)
        .unwrap();

    let mut modified_digest = session_digest;
    modified_digest[0] ^= 1;
    assert!(
        credential
            .device_public_key()
            .verify_digest(&modified_digest, &proof)
            .is_err()
    );
}

#[test]
fn normal_root_successor_requires_continuity_from_both_keys() {
    let owner_id = OwnerId::from_bytes([17; 32]);
    let current_key = SigningKey::from_secret_bytes([18; 32]);
    let current = OwnerRootRecord::new(owner_id, &current_key, 3);
    let next_key = SigningKey::from_secret_bytes([19; 32]);

    let successor = RootSuccessor::issue(&current, &current_key, &next_key).unwrap();
    let next = successor.verify(&current).unwrap();

    assert_eq!(next.owner_id(), owner_id);
    assert_eq!(next.root_epoch(), 4);
    assert_eq!(next.root_public_key(), next_key.verifying_key());
}

#[test]
fn root_successor_rejects_the_wrong_current_private_key() {
    let owner_id = OwnerId::from_bytes([30; 32]);
    let current_key = SigningKey::from_secret_bytes([31; 32]);
    let current = OwnerRootRecord::new(owner_id, &current_key, 5);
    let wrong_current_key = SigningKey::from_secret_bytes([32; 32]);
    let next_key = SigningKey::from_secret_bytes([33; 32]);

    assert_eq!(
        RootSuccessor::issue(&current, &wrong_current_key, &next_key),
        Err(IdentityError::InvalidRootSuccessor)
    );
}
