from pathlib import Path


def replace_test(path: str, name: str, source: str) -> None:
    target = Path(path)
    text = target.read_text()
    marker = f"#[test]\nfn {name}() {{"
    start = text.index(marker)
    end = text.find("\n#[test]\n", start + len(marker))
    if end == -1:
        end = len(text)
    target.write_text(text[:start] + source.rstrip() + "\n" + text[end:])


replace_test(
    "crates/policy/tests/authorization.rs",
    "non_administrative_delegation_cannot_authorize_owner_approval",
    r'''#[test]
fn non_administrative_delegation_cannot_authorize_owner_approval() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let owner_id = OwnerId::from_bytes([0x50; 32]);
    let root_key = SigningKey::from_secret_bytes([0x51; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let device_signing_key = SigningKey::from_secret_bytes([0x52; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();

    assert_eq!(
        OwnerApprovalEvidence::issue(
            ApprovalScope::from_context(&context),
            ApprovalInstant::from_ticks(10),
            ApprovalInstant::from_ticks(20),
            &authority,
            &device_signing_key,
        ),
        Err(ApprovalError::UnknownIssuer)
    );
}''',
)

replace_test(
    "crates/policy/tests/trust.rs",
    "successor_credential_rejects_stale_skipped_wrong_device_and_wrong_authority",
    r'''#[test]
fn successor_credential_rejects_stale_skipped_wrong_device_and_wrong_authority() {
    let fixture = RotationFixture::new();

    let mut stale_record = fixture.trust();
    let stale = fixture.credential(fixture.device_id, 0);
    assert_eq!(
        stale_record.accept_successor_credential(
            &stale,
            &fixture.authority,
            TransitionId::from_bytes([0x27; 32]),
        ),
        Err(CredentialRotationError::StaleCredentialEpoch)
    );

    let mut skipped_record = fixture.trust();
    let skipped = fixture.credential(fixture.device_id, 2);
    assert_eq!(
        skipped_record.accept_successor_credential(
            &skipped,
            &fixture.authority,
            TransitionId::from_bytes([0x28; 32]),
        ),
        Err(CredentialRotationError::UnexpectedCredentialEpoch)
    );

    let mut wrong_device_record = fixture.trust();
    let wrong_device = fixture.credential(DeviceId::from_bytes([0x29; 32]), 1);
    assert_eq!(
        wrong_device_record.accept_successor_credential(
            &wrong_device,
            &fixture.authority,
            TransitionId::from_bytes([0x2a; 32]),
        ),
        Err(CredentialRotationError::WrongDevice)
    );

    let mut wrong_authority_record = fixture.trust();
    let successor = fixture.credential(fixture.device_id, 1);
    let other_root_key = SigningKey::from_secret_bytes([0x2b; 32]);
    let other_root = OwnerRootRecord::new(fixture.owner_id, &other_root_key, 0);
    let other_authority = OwnerAuthorityState::new(other_root);
    assert_eq!(
        wrong_authority_record.accept_successor_credential(
            &successor,
            &other_authority,
            TransitionId::from_bytes([0x2c; 32]),
        ),
        Err(CredentialRotationError::Identity(
            IdentityError::UnknownIssuer
        ))
    );
}''',
)

replace_test(
    "crates/policy/tests/trust.rs",
    "signed_revocation_is_terminal_and_advances_trust_revision",
    r'''#[test]
fn signed_revocation_is_terminal_and_advances_trust_revision() {
    let mut record = trusted_record();
    let root_key = SigningKey::from_secret_bytes([4; 32]);
    let root = OwnerRootRecord::new(record.owner_id(), &root_key, 0);
    let authority = OwnerAuthorityState::new(root);
    let transition_id = TransitionId::from_bytes([5; 32]);
    let transition =
        TrustTransition::issue_root_revocation(&record, transition_id, &authority, &root_key)
            .unwrap();

    transition.apply_root(&mut record, &authority).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
    assert_eq!(record.trust_revision(), 1);
    assert_eq!(record.last_transition_id(), transition_id);
    assert_eq!(
        TrustTransition::issue_root_revocation(
            &record,
            TransitionId::from_bytes([6; 32]),
            &authority,
            &root_key,
        ),
        Err(TrustTransitionError::AlreadyRevoked)
    );
}''',
)
