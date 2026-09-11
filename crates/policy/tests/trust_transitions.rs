use crosslab_crypto::{Signature, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    TransitionId, TrustRecord, TrustState, TrustTransition, TrustTransitionError,
};

fn fixture() -> (TrustRecord, SigningKey, OwnerRootRecord) {
    let owner_id = OwnerId::from_bytes([1; 32]);
    let root_key = SigningKey::from_secret_bytes([2; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let record = TrustRecord::trusted(
        owner_id,
        DeviceId::from_bytes([3; 32]),
        4,
        TransitionId::from_bytes([4; 32]),
    );
    (record, root_key, root)
}

#[test]
fn owner_root_signed_revocation_applies_to_matching_trust_record() {
    let (mut record, root_key, root) = fixture();
    let transition_id = TransitionId::from_bytes([5; 32]);
    let transition =
        TrustTransition::issue_root_revocation(&record, transition_id, &root, &root_key).unwrap();

    transition.apply_root(&mut record, &root).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
    assert_eq!(record.trust_revision(), 1);
    assert_eq!(record.last_transition_id(), transition_id);
}

#[test]
fn administrative_authority_can_sign_ordinary_revocation() {
    let (mut record, root_key, root) = fixture();
    let administrative_key = SigningKey::from_secret_bytes([6; 32]);
    let delegation = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::Administrative,
        &administrative_key,
        0,
        &root_key,
    );
    let transition = TrustTransition::issue_delegated_revocation(
        &record,
        TransitionId::from_bytes([7; 32]),
        &root,
        &delegation,
        &administrative_key,
        0,
    )
    .unwrap();

    transition
        .apply_delegated(&mut record, &root, &delegation, 0)
        .unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
}

#[test]
fn recovery_authority_is_rejected_from_ordinary_revocation_path() {
    let (record, root_key, root) = fixture();
    let recovery_key = SigningKey::from_secret_bytes([8; 32]);
    let recovery = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::Recovery,
        &recovery_key,
        0,
        &root_key,
    );

    assert_eq!(
        TrustTransition::issue_delegated_revocation(
            &record,
            TransitionId::from_bytes([9; 32]),
            &root,
            &recovery,
            &recovery_key,
            0,
        ),
        Err(TrustTransitionError::WrongIssuerRole)
    );
}

#[test]
fn delegated_revocation_rejects_the_wrong_private_key() {
    let (record, root_key, root) = fixture();
    let administrative_key = SigningKey::from_secret_bytes([10; 32]);
    let delegation = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::Administrative,
        &administrative_key,
        0,
        &root_key,
    );

    assert_eq!(
        TrustTransition::issue_delegated_revocation(
            &record,
            TransitionId::from_bytes([11; 32]),
            &root,
            &delegation,
            &SigningKey::from_secret_bytes([12; 32]),
            0,
        ),
        Err(TrustTransitionError::UnknownIssuer)
    );
}

#[test]
fn transition_is_bound_to_the_credential_epoch_at_issue_time() {
    let (mut record, root_key, root) = fixture();
    let transition = TrustTransition::issue_root_revocation(
        &record,
        TransitionId::from_bytes([13; 32]),
        &root,
        &root_key,
    )
    .unwrap();
    record.advance_credential_epoch(5).unwrap();

    assert_eq!(
        transition.apply_root(&mut record, &root),
        Err(TrustTransitionError::CredentialEpochMismatch)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}

#[test]
fn forged_root_signature_is_rejected() {
    let (mut record, root_key, root) = fixture();
    let valid = TrustTransition::issue_root_revocation(
        &record,
        TransitionId::from_bytes([14; 32]),
        &root,
        &root_key,
    )
    .unwrap();
    let forged_signature = SigningKey::from_secret_bytes([15; 32]).sign_digest(&valid.transcript_digest());
    let forged = TrustTransition::from_signed_revocation(
        record.owner_id(),
        record.device_id(),
        valid.transition_id(),
        valid.previous_revision(),
        valid.new_revision(),
        valid.credential_epoch_context(),
        AuthorityRole::OwnerRoot,
        root.root_key_id(),
        forged_signature,
    );

    assert_eq!(
        forged.apply_root(&mut record, &root),
        Err(TrustTransitionError::InvalidSignature)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}

#[test]
fn transition_requires_exact_next_trust_revision() {
    let (mut record, root_key, root) = fixture();
    let transition_id = TransitionId::from_bytes([16; 32]);
    let unsigned = TrustTransition::from_signed_revocation(
        record.owner_id(),
        record.device_id(),
        transition_id,
        record.trust_revision(),
        record.trust_revision() + 2,
        record.accepted_credential_epoch(),
        AuthorityRole::OwnerRoot,
        root.root_key_id(),
        Signature::from_bytes([0; 64]),
    );
    let signature = root_key.sign_digest(&unsigned.transcript_digest());
    let transition = TrustTransition::from_signed_revocation(
        record.owner_id(),
        record.device_id(),
        transition_id,
        record.trust_revision(),
        record.trust_revision() + 2,
        record.accepted_credential_epoch(),
        AuthorityRole::OwnerRoot,
        root.root_key_id(),
        signature,
    );

    assert_eq!(
        transition.apply_root(&mut record, &root),
        Err(TrustTransitionError::InvalidRevision)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}
