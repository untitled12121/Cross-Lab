from pathlib import Path


def replace_count(path: str, old: str, new: str, expected: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} matches, found {count}: {old!r}")
    target.write_text(text.replace(old, new))


path = "crates/policy/tests/authority_currentness.rs"

# Fixture setup already has canonical authority state; pairing/trust must use it too.
replace_count(
    path,
    """        let pairing = PairingTrustTransition::issue(
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
            .establish(&credential, authority.root(), &dsa, dsa.delegation_epoch())
            .unwrap();
""",
    """        let pairing = PairingTrustTransition::issue(
            &credential,
            TransitionId::from_bytes([0x15; 32]),
            [0x16; 32],
            &authority,
            &dsa_key,
        )
        .unwrap();
        let record = pairing.establish(&credential, &authority).unwrap();
""",
)

# Issue approval while the old administrative authority is current, then supersede it.
replace_count(
    path,
    """    let evidence = OwnerApprovalEvidence::issue(
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
""",
    """    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old).unwrap();
    let evidence = OwnerApprovalEvidence::issue(
        approval_scope(),
        ApprovalInstant::from_ticks(10),
        ApprovalInstant::from_ticks(20),
        &authority,
        &old_key,
    )
    .unwrap();
    let new_key = SigningKey::from_secret_bytes([0x33; 32]);
""",
)
replace_count(
    path,
    """    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old).unwrap();
    authority.accept_delegation(new).unwrap();

    assert_eq!(
        evidence.verify(&authority),
""",
    """    authority.accept_delegation(new).unwrap();

    assert_eq!(
        evidence.verify(&authority),
""",
)

# Issue delegated revocation under the old administrative authority before superseding it.
replace_count(
    path,
    """    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x51; 32]),
        fixture.authority.root(),
        &old,
        &old_key,
        0,
    )
    .unwrap();
""",
    """    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x51; 32]),
        &fixture.authority,
        AuthorityRole::Administrative,
        &old_key,
    )
    .unwrap();
""",
)

# Device-signing revocation is issued while the original DSA is current, then rotated.
replace_count(
    path,
    """    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x53; 32]),
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
        fixture.dsa.delegation_epoch(),
    )
    .unwrap();
""",
    """    let transition = TrustTransition::issue_delegated_revocation(
        &fixture.record,
        TransitionId::from_bytes([0x53; 32]),
        &fixture.authority,
        AuthorityRole::DeviceSigning,
        &fixture.dsa_key,
    )
    .unwrap();
""",
)

# Pairing transition is issued before rotating the DSA; establishment must reject it afterward.
replace_count(
    path,
    """    let transition = PairingTrustTransition::issue(
        &fixture.credential,
        TransitionId::from_bytes([0x62; 32]),
        [0x63; 32],
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
        fixture.dsa.delegation_epoch(),
    )
    .unwrap();
""",
    """    let transition = PairingTrustTransition::issue(
        &fixture.credential,
        TransitionId::from_bytes([0x62; 32]),
        [0x63; 32],
        &fixture.authority,
        &fixture.dsa_key,
    )
    .unwrap();
""",
)

# Successor credential is issued while the original DSA is current, then rejected after rotation.
replace_count(
    path,
    """    let successor = DeviceCredential::issue(
        fixture.owner_id,
        fixture.device_id,
        &fixture.device_key,
        1,
        fixture.authority.root(),
        &fixture.dsa,
        &fixture.dsa_key,
    )
    .unwrap();
""",
    """    let successor = DeviceCredential::issue(
        fixture.owner_id,
        fixture.device_id,
        &fixture.device_key,
        1,
        &fixture.authority,
        &fixture.dsa_key,
    )
    .unwrap();
""",
)

# RotationFixture no longer needs to retain a raw delegation once all public APIs use authority state.
replace_count(
    "crates/policy/tests/trust.rs",
    """    issuer_key: SigningKey,
    delegation: AuthorityDelegation,
    authority: OwnerAuthorityState,
""",
    """    issuer_key: SigningKey,
    authority: OwnerAuthorityState,
""",
)
replace_count(
    "crates/policy/tests/trust.rs",
    """            device_id: DeviceId::from_bytes([0x23; 32]),
            issuer_key,
            delegation,
            authority,
""",
    """            device_id: DeviceId::from_bytes([0x23; 32]),
            issuer_key,
            authority,
""",
)
