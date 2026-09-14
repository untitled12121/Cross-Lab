use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord, RootSuccessor,
};
use crosslab_policy::{
    ApprovalError, ApprovalInstant, ApprovalScope, CapabilityId, CredentialRotationError,
    OperationName, OwnerApprovalEvidence, PairingTrustTransition, PairingTrustTransitionError,
    SessionId, TransitionId, TrustRecord, TrustState, TrustTransition, TrustTransitionError,
};

struct TrustFixture {
    owner_id: OwnerId,
    root_key: SigningKey,
    authority: OwnerAuthorityState,
    dsa_key: SigningKey,
    dsa: AuthorityDelegation,
    device_id: DeviceId,
    device_key: SigningKey,
    credential: DeviceCredential,
    record: TrustRecord,
}

impl TrustFixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x10; 32]);
        let root_key = SigningKey::from_secret_bytes([0x11; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let dsa_key = SigningKey::from_secret_bytes([0x12; 32]);
        let dsa = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &dsa_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(dsa).unwrap();
        let device_id = DeviceId::from_bytes([0x13; 32]);
        let device_key = SigningKey::from_secret_bytes([0x14; 32]);
        let credential = DeviceCredential::issue_current(
            owner_id,
            device_id,
            &device_key,
            0,
            &authority,
            &dsa_key,
        )
        .unwrap();
        let pairing = PairingTrustTransition::issue(
            &credential,
            TransitionId::from_bytes([0x15; 32]),
            [0x16; 32],
            authority.root(),
            &dsa,
            &dsa_key,
            dsa.delegation_epoch(),
        )
        .unwrap();
        let record = pairing
            .establish(
                &credential,
                authority.root(),
                &dsa,
                dsa.delegation_epoch(),
            )
            .unwrap();

        Self {
            owner_id,
            root_key,
            authority,
            dsa_key,
            dsa,
            device_id,
            device_key,
            credential,
            record,
        }
    }

    fn rotate_device_signing(&mut self) -> (SigningKey, AuthorityDelegation) {
        let key = SigningKey::from_secret_bytes([0x17; 32]);
        let delegation = AuthorityDelegation::issue(
            self.owner_id,
            AuthorityRole::DeviceSigning,
            &key,
            self.dsa.delegation_epoch() + 1,
            &self.root_key,
        );
        self.authority.accept_delegation(delegation).unwrap();
        (key, delegation)
    }
}

fn approval_scope() -> ApprovalScope {
    ApprovalScope::new(
        DeviceId::from_bytes([0x20; 32]),
        DeviceId::from_bytes([0x21; 32]),
        SessionId::from_bytes([0x22; 32]),
        CapabilityId::parse("files.transfer").unwrap(),
        OperationName::parse("receive").unwrap(),
    )
}

#[test]
fn superseded_administrative_authority_cannot_verify_current_approval() {
    let owner_id = OwnerId::from_bytes([0x30; 32]);
    let root_key = SigningKey::from_secret_bytes([0x31; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let old_key = SigningKey::from_secret_bytes([0x32; 32]);
    let old = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &old_key,
        0,
        &root_key,
    );
    let evidence = OwnerApprovalEvidence::issue(
        approval_scope(),
        ApprovalInstant::from_ticks(10),
        ApprovalInstant::from_ticks(20),
        &root,
        &old,
        &old_key,
        0,
    )
    .unwrap();
    let new_key = SigningKey::from_secret_bytes([0x33; 32]);
    let new = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &new_key,
        1,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old).unwrap();
    authority.accept_delegation(new).unwrap();

    assert_eq!(
        evidence.verify_current(&authority),
        Err(ApprovalError::UnknownIssuer)
    );
}

#[test]
fn superseded_administrative_key_cannot_issue_current_approval() {
    let owner_id = OwnerId::from_bytes([0x34; 32]);
    let root_key = SigningKey::from_secret_bytes([0x35; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let old_key = SigningKey::from_secret_bytes([0x36; 32]);
    let old = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &old_key,
        0,
        &root_key,
    );
    let new_key = SigningKey::from_secret_bytes([0x37; 32]);
    let new = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &new_key,
        1,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old).unwrap();
    authority.accept_delegation(new).unwrap();

    assert_eq!(
        OwnerApprovalEvidence::issue_current(
            approval_scope(),
            ApprovalInstant::from_ticks(10),
            ApprovalInstant::from_ticks(20),
            &authority,
            &old_key,
        ),
        Err(ApprovalError::UnknownIssuer)
    );
}

#[test]
fn old_root_revocation_cannot_apply_after_root_successor() {
    let mut fixture = TrustFixture::new();
    let transition = TrustTransition::issue_root_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x40; 32]),
        fixture.authority.root(),
        &fixture.root_key,
    )
    .unwrap();
    let next_root_key = SigningKey::from_secret_bytes([0x41; 32]);
    let successor =
        RootSuccessor::issue(fixture.authority.root(), &fixture.root_key, &next_root_key).unwrap();
    fixture.authority.accept_root_successor(&successor).unwrap();

    assert_eq!(
        transition.apply_root(&mut fixture.record, fixture.authority.root()),
        Err(TrustTransitionError::UnknownIssuer)
    );
    assert_eq!(fixture.record.state(), TrustState::Trusted);
}

#[test]
fn superseded_administrative_revocation_cannot_apply_current() {
    let mut fixture = TrustFixture::new();
    let old_key = SigningKey::from_secret_bytes([0x50; 32]);
    let old = AuthorityDelegation::issue(
        fixture.owner_id,
        AuthorityRole::Administrative,
        &old_key,
        0,
        &fixture.root_key,
    );
    fixture.authority.accept_delegation(old).unwrap();
    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x51; 32]),
        fixture.authority.root(),
        &old,
        &old_key,
        0,
    )
    .unwrap();
    let new_key = SigningKey::from_secret_bytes([0x52; 32]);
    let new = AuthorityDelegation::issue(
        fixture.owner_id,
        AuthorityRole::Administrative,
        &new_key,
        1,
        &fixture.root_key,
    );
    fixture.authority.accept_delegation(new).unwrap();

    assert_eq!(
        transition.apply_delegated_current(&mut fixture.record, &fixture.authority),
        Err(TrustTransitionError::UnknownIssuer)
    );
    assert_eq!(fixture.record.state(), TrustState::Trusted);
}

#[test]
fn superseded_device_signing_revocation_cannot_apply_current() {
    let mut fixture = TrustFixture::new();
    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x53; 32]),
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
        fixture.dsa.delegation_epoch(),
    )
    .unwrap();
    fixture.rotate_device_signing();

    assert_eq!(
        transition.apply_delegated_current(&mut fixture.record, &fixture.authority),
        Err(TrustTransitionError::UnknownIssuer)
    );
    assert_eq!(fixture.record.state(), TrustState::Trusted);
}

#[test]
fn recovery_remains_invalid_for_current_delegated_revocation() {
    let mut fixture = TrustFixture::new();
    let recovery_key = SigningKey::from_secret_bytes([0x54; 32]);
    let recovery = AuthorityDelegation::issue(
        fixture.owner_id,
        AuthorityRole::Recovery,
        &recovery_key,
        0,
        &fixture.root_key,
    );
    fixture.authority.accept_delegation(recovery).unwrap();

    assert_eq!(
        TrustTransition::issue_delegated_revocation_current(
            &fixture.record,
            TransitionId::from_bytes([0x55; 32]),
            &fixture.authority,
            AuthorityRole::Recovery,
            &recovery_key,
        ),
        Err(TrustTransitionError::WrongIssuerRole)
    );
}

#[test]
fn superseded_device_signing_key_cannot_issue_current_pairing_transition() {
    let mut fixture = TrustFixture::new();
    fixture.rotate_device_signing();

    assert_eq!(
        PairingTrustTransition::issue_current(
            &fixture.credential,
            TransitionId::from_bytes([0x60; 32]),
            [0x61; 32],
            &fixture.authority,
            &fixture.dsa_key,
        ),
        Err(PairingTrustTransitionError::Identity(
            IdentityError::UnknownIssuer
        ))
    );
}

#[test]
fn superseded_device_signing_authority_cannot_establish_current_pairing() {
    let mut fixture = TrustFixture::new();
    let transition = PairingTrustTransition::issue(
        &fixture.credential,
        TransitionId::from_bytes([0x62; 32]),
        [0x63; 32],
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
        fixture.dsa.delegation_epoch(),
    )
    .unwrap();
    fixture.rotate_device_signing();

    assert_eq!(
        transition.establish_current(&fixture.credential, &fixture.authority),
        Err(PairingTrustTransitionError::Identity(
            IdentityError::UnknownIssuer
        ))
    );
}

#[test]
fn successor_credential_rejects_superseded_device_signing_authority_current() {
    let mut fixture = TrustFixture::new();
    let successor = DeviceCredential::issue(
        fixture.owner_id,
        fixture.device_id,
        &fixture.device_key,
        1,
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
    )
    .unwrap();
    fixture.rotate_device_signing();

    assert_eq!(
        fixture.record.accept_successor_credential_current(
            &successor,
            &fixture.authority,
            TransitionId::from_bytes([0x64; 32]),
        ),
        Err(CredentialRotationError::Identity(
            IdentityError::UnknownIssuer
        ))
    );
}
