use crosslab_crypto::{Signature, SigningKey, SigningProvider, SigningProviderError, VerifyingKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    PairingTrustTransition, TransitionId, TrustRecord, TrustState, TrustTransition,
    TrustTransitionError,
};

struct FailingSigningProvider {
    verifying_key: VerifyingKey,
}

impl SigningProvider for FailingSigningProvider {
    fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }

    fn sign_message(&self, _: &[u8]) -> Result<Signature, SigningProviderError> {
        Err(SigningProviderError)
    }
}

fn fixture() -> (TrustRecord, SigningKey, OwnerRootRecord) {
    let owner_id = OwnerId::from_bytes([1; 32]);
    let root_key = SigningKey::from_secret_bytes([2; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([24; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let device_id = DeviceId::from_bytes([3; 32]);
    let device_key = SigningKey::from_secret_bytes([25; 32]);
    let initial_credential =
        DeviceCredential::issue(owner_id, device_id, &device_key, 0, &authority, &issuer_key)
            .unwrap();
    let pairing = PairingTrustTransition::issue(
        &initial_credential,
        TransitionId::from_bytes([4; 32]),
        [26; 32],
        &authority,
        &issuer_key,
    )
    .unwrap();
    let mut record = pairing.establish(&initial_credential, &authority).unwrap();

    for epoch in 1..=4 {
        let successor = DeviceCredential::issue(
            owner_id,
            device_id,
            &device_key,
            epoch,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let mut transition_id = [4; 32];
        transition_id[..8].copy_from_slice(&epoch.to_be_bytes());
        record
            .accept_successor_credential(
                &successor,
                &authority,
                TransitionId::from_bytes(transition_id),
            )
            .unwrap();
    }

    (record, root_key, root)
}

#[test]
fn root_revocation_fails_closed_when_signing_provider_fails() {
    let (record, root_key, root) = fixture();
    let authority = OwnerAuthorityState::new(root);
    let provider = FailingSigningProvider {
        verifying_key: root_key.verifying_key(),
    };

    assert_eq!(
        TrustTransition::issue_root_revocation_with_provider(
            &record,
            TransitionId::from_bytes([0x7f; 32]),
            &authority,
            &provider,
        ),
        Err(TrustTransitionError::SigningFailed)
    );
}

#[test]
fn owner_root_signed_revocation_applies_to_matching_trust_record() {
    let (mut record, root_key, root) = fixture();
    let authority = OwnerAuthorityState::new(root);
    let transition_id = TransitionId::from_bytes([5; 32]);
    let transition =
        TrustTransition::issue_root_revocation(&record, transition_id, &authority, &root_key)
            .unwrap();

    transition.apply_root(&mut record, &authority).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
    assert_eq!(record.trust_revision(), 5);
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
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let transition = TrustTransition::issue_delegated_revocation(
        &record,
        TransitionId::from_bytes([7; 32]),
        &authority,
        AuthorityRole::Administrative,
        &administrative_key,
    )
    .unwrap();

    transition.apply_delegated(&mut record, &authority).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
}

#[test]
fn device_signing_authority_can_sign_ordinary_revocation() {
    let (mut record, root_key, root) = fixture();
    let device_signing_key = SigningKey::from_secret_bytes([17; 32]);
    let delegation = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        2,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let transition = TrustTransition::issue_delegated_revocation(
        &record,
        TransitionId::from_bytes([18; 32]),
        &authority,
        AuthorityRole::DeviceSigning,
        &device_signing_key,
    )
    .unwrap();

    transition.apply_delegated(&mut record, &authority).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
}

#[test]
fn superseded_administrative_delegation_is_rejected() {
    let (record, root_key, root) = fixture();
    let old_key = SigningKey::from_secret_bytes([19; 32]);
    let old = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::Administrative,
        &old_key,
        0,
        &root_key,
    );
    let current_key = SigningKey::from_secret_bytes([20; 32]);
    let current = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::Administrative,
        &current_key,
        1,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old).unwrap();
    authority.accept_delegation(current).unwrap();

    assert_eq!(
        TrustTransition::issue_delegated_revocation(
            &record,
            TransitionId::from_bytes([21; 32]),
            &authority,
            AuthorityRole::Administrative,
            &old_key,
        ),
        Err(TrustTransitionError::UnknownIssuer)
    );
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
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(recovery).unwrap();

    assert_eq!(
        TrustTransition::issue_delegated_revocation(
            &record,
            TransitionId::from_bytes([9; 32]),
            &authority,
            AuthorityRole::Recovery,
            &recovery_key,
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
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();

    assert_eq!(
        TrustTransition::issue_delegated_revocation(
            &record,
            TransitionId::from_bytes([11; 32]),
            &authority,
            AuthorityRole::Administrative,
            &SigningKey::from_secret_bytes([12; 32]),
        ),
        Err(TrustTransitionError::UnknownIssuer)
    );
}

#[test]
fn transition_is_bound_to_the_credential_epoch_at_issue_time() {
    let (mut record, root_key, root) = fixture();
    let mut authority = OwnerAuthorityState::new(root);
    let transition = TrustTransition::issue_root_revocation(
        &record,
        TransitionId::from_bytes([13; 32]),
        &authority,
        &root_key,
    )
    .unwrap();
    let device_signing_key = SigningKey::from_secret_bytes([21; 32]);
    let delegation = AuthorityDelegation::issue(
        record.owner_id(),
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        0,
        &root_key,
    );
    authority.accept_delegation(delegation).unwrap();
    let successor = DeviceCredential::issue(
        record.owner_id(),
        record.device_id(),
        &SigningKey::from_secret_bytes([22; 32]),
        5,
        &authority,
        &device_signing_key,
    )
    .unwrap();
    record
        .accept_successor_credential(&successor, &authority, TransitionId::from_bytes([23; 32]))
        .unwrap();

    assert_eq!(
        transition.apply_root(&mut record, &authority),
        Err(TrustTransitionError::CredentialEpochMismatch)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}

#[test]
fn forged_root_signature_is_rejected() {
    let (mut record, root_key, root) = fixture();
    let authority = OwnerAuthorityState::new(root);
    let valid = TrustTransition::issue_root_revocation(
        &record,
        TransitionId::from_bytes([14; 32]),
        &authority,
        &root_key,
    )
    .unwrap();
    let forged_signature =
        SigningKey::from_secret_bytes([15; 32]).sign_digest(&valid.transcript_digest());
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
        forged.apply_root(&mut record, &authority),
        Err(TrustTransitionError::InvalidSignature)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}

#[test]
fn transition_requires_exact_next_trust_revision() {
    let (mut record, root_key, root) = fixture();
    let authority = OwnerAuthorityState::new(root);
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
        transition.apply_root(&mut record, &authority),
        Err(TrustTransitionError::InvalidRevision)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}
