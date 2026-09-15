from pathlib import Path
import re


def replace_count(path: str, old: str, new: str, expected: int) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} matches, found {count}: {old!r}")
    target.write_text(text.replace(old, new))


def migrate_parts_text(text: str, expected: tuple[int, int, int]) -> str:
    pairing = re.compile(
        r"(?P<i>[ \t]*)&root,\n(?P=i)&delegation,\n(?P=i)&issuer_key,\n(?P=i)delegation\.delegation_epoch\(\),"
    )
    text, pairing_count = pairing.subn(
        lambda match: f"{match.group('i')}&authority,\n{match.group('i')}&issuer_key,", text
    )
    floor = re.compile(
        r"(?P<i>[ \t]*)&root,\n(?P=i)&delegation,\n(?P=i)delegation\.delegation_epoch\(\),"
    )
    text, floor_count = floor.subn(lambda match: f"{match.group('i')}&authority,", text)
    credential = re.compile(r"(?P<i>[ \t]*)&root,\n(?P=i)&delegation,\n(?P=i)&issuer_key,")
    text, credential_count = credential.subn(
        lambda match: f"{match.group('i')}&authority,\n{match.group('i')}&issuer_key,", text
    )
    actual = (pairing_count, floor_count, credential_count)
    if actual != expected:
        raise SystemExit(f"authority-part migration expected {expected}, found {actual}")
    return text


def migrate_authority_parts(path: str, expected: tuple[int, int, int]) -> None:
    target = Path(path)
    target.write_text(migrate_parts_text(target.read_text(), expected))


def migrate_region(
    path: str,
    start_marker: str,
    end_marker: str,
    expected: tuple[int, int, int],
) -> None:
    target = Path(path)
    text = target.read_text()
    start = text.index(start_marker)
    end = text.index(end_marker, start)
    region = migrate_parts_text(text[start:end], expected)
    target.write_text(text[:start] + region + text[end:])


def replace_test(path: str, name: str, source: str) -> None:
    target = Path(path)
    text = target.read_text()
    marker = f"#[test]\nfn {name}() {{"
    start = text.index(marker)
    end = text.find("\n#[test]\n", start + len(marker))
    if end == -1:
        end = len(text)
    target.write_text(text[:start] + source.rstrip() + "\n" + text[end:])


for path, issue_count, apply_revoked_count, apply_other_count in [
    ("apps/sim/tests/lifecycle.rs", 2, 1, 1),
    ("apps/sim/tests/stream_scenarios.rs", 1, 1, 0),
    ("apps/sim/tests/control_lifecycle.rs", 1, 1, 0),
    ("apps/sim/tests/security_hardening.rs", 1, 1, 0),
    ("crates/core/tests/session_lifecycle.rs", 1, 1, 0),
]:
    replace_count(
        path,
        "            self.authority.root(),\n            &self.root_key,",
        "            &self.authority,\n            &self.root_key,",
        issue_count,
    )
    replace_count(
        path,
        ".apply_root(&mut revoked, self.authority.root())",
        ".apply_root(&mut revoked, &self.authority)",
        apply_revoked_count,
    )
    if apply_other_count:
        replace_count(
            path,
            ".apply_root(&mut other, self.authority.root())",
            ".apply_root(&mut other, &self.authority)",
            apply_other_count,
        )

for path in [
    "crates/core/tests/stream_currentness.rs",
    "crates/policy/tests/authority_currentness.rs",
]:
    replace_count(
        path,
        "        fixture.authority.root(),\n        &fixture.root_key,",
        "        &fixture.authority,\n        &fixture.root_key,",
        1,
    )
    old = (
        ".apply_root(&mut revoked, fixture.authority.root())"
        if "stream_currentness" in path
        else ".apply_root(&mut fixture.record, fixture.authority.root())"
    )
    new = (
        ".apply_root(&mut revoked, &fixture.authority)"
        if "stream_currentness" in path
        else ".apply_root(&mut fixture.record, &fixture.authority)"
    )
    replace_count(path, old, new, 1)

replace_count(
    "transports/quic/src/session_auth_tests/lifecycle_tests.rs",
    "        fixture.authority.root(),\n        &root_key,",
    "        &fixture.authority,\n        &root_key,",
    1,
)
replace_count(
    "transports/quic/src/session_auth_tests/lifecycle_tests.rs",
    ".apply_root(&mut revoked, fixture.authority.root())",
    ".apply_root(&mut revoked, &fixture.authority)",
    1,
)

insertions = [
    (
        "crates/core/tests/control_event_authorization.rs",
        "        );\n        let local_key = SigningKey::from_secret_bytes([0x43; 32]);",
        "        );\n        let mut authority = OwnerAuthorityState::new(root);\n        authority.accept_delegation(delegation).unwrap();\n        let local_key = SigningKey::from_secret_bytes([0x43; 32]);",
        (1, 1, 2),
    ),
    (
        "crates/core/tests/control_rotation.rs",
        "    );\n    let local_key = SigningKey::from_secret_bytes([0x43; 32]);",
        "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let local_key = SigningKey::from_secret_bytes([0x43; 32]);",
        (1, 2, 3),
    ),
    (
        "crates/core/src/session/state.rs",
        "        );\n        let peer_device_id = DeviceId::from_bytes([0xf2; 32]);",
        "        );\n        let mut authority = OwnerAuthorityState::new(root);\n        authority.accept_delegation(delegation).unwrap();\n        let peer_device_id = DeviceId::from_bytes([0xf2; 32]);",
        (1, 2, 2),
    ),
    (
        "crates/policy/tests/golden_vectors.rs",
        "    );\n    let device_key = SigningKey::from_secret_bytes([0x43; 32]);",
        "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let device_key = SigningKey::from_secret_bytes([0x43; 32]);",
        (1, 2, 2),
    ),
]
for path, marker, replacement, expected in insertions:
    replace_count(path, marker, replacement, 1)
    migrate_authority_parts(path, expected)

replace_count(
    "crates/core/src/session/state.rs",
    "    use crosslab_identity::{AuthorityDelegation, AuthorityRole, OwnerRootRecord};",
    "    use crosslab_identity::{\n        AuthorityDelegation, AuthorityRole, OwnerAuthorityState, OwnerRootRecord,\n    };",
    1,
)
replace_count(
    "crates/policy/tests/golden_vectors.rs",
    "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,",
    "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,\n    OwnerRootRecord,",
    1,
)

for path, marker, replacement in [
    (
        "crates/core/tests/session_auth.rs",
        "        );\n        let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);",
        "        );\n        let mut authority = OwnerAuthorityState::new(root);\n        authority.accept_delegation(delegation).unwrap();\n        let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);",
    ),
    (
        "crates/core/tests/debug_privacy.rs",
        "    );\n    let initiator_key = SigningKey::from_secret_bytes([0xe4; 32]);",
        "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let initiator_key = SigningKey::from_secret_bytes([0xe4; 32]);",
    ),
]:
    replace_count(
        path,
        "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,\n",
        "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,\n    OwnerRootRecord,\n",
        1,
    )
    replace_count(path, marker, replacement, 1)
    migrate_authority_parts(path, (0, 0, 2))

for path, marker, replacement in [
    (
        "crates/protocol/tests/session_auth_wire.rs",
        "    );\n    let device_key = SigningKey::from_secret_bytes([0x13; 32]);",
        "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let device_key = SigningKey::from_secret_bytes([0x13; 32]);",
    ),
    (
        "crates/identity/tests/golden_vectors.rs",
        "    );\n    let device_key = SigningKey::from_secret_bytes([0x55; 32]);",
        "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let device_key = SigningKey::from_secret_bytes([0x55; 32]);",
    ),
]:
    replace_count(
        path,
        "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,\n",
        "    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,\n    OwnerRootRecord,\n",
        1,
    )
    replace_count(path, marker, replacement, 1)
    migrate_authority_parts(path, (0, 0, 1))

replace_count(
    "crates/core/tests/control_event_authorization.rs",
    "        let mut authority = OwnerAuthorityState::new(root);\n        authority.accept_delegation(delegation).unwrap();\n\n        Self {",
    "        Self {",
    1,
)
replace_count(
    "crates/core/tests/control_rotation.rs",
    "    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];",
    "    let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];",
    1,
)

replace_count(
    "crates/core/src/session/state.rs",
    "            &root,\n            &root_key,",
    "            &authority,\n            &root_key,",
    1,
)
replace_count(
    "crates/core/src/session/state.rs",
    "transition.apply_root(&mut revoked, &root)",
    "transition.apply_root(&mut revoked, &authority)",
    1,
)
replace_count(
    "crates/policy/tests/golden_vectors.rs",
    "        &root,\n        &root_key,",
    "        &authority,\n        &root_key,",
    1,
)

trust_path = "crates/policy/tests/trust_transitions.rs"
replace_count(
    trust_path,
    "    );\n    let device_id = DeviceId::from_bytes([3; 32]);",
    "    );\n    let mut authority = OwnerAuthorityState::new(root);\n    authority.accept_delegation(delegation).unwrap();\n    let device_id = DeviceId::from_bytes([3; 32]);",
    1,
)
migrate_region(
    trust_path,
    "fn fixture() -> (TrustRecord, SigningKey, OwnerRootRecord) {",
    "#[test]\nfn owner_root_signed_revocation_applies_to_matching_trust_record()",
    (1, 2, 2),
)
replace_count(
    trust_path,
    "TrustTransition::issue_root_revocation(&record, transition_id, authority.root(), &root_key)",
    "TrustTransition::issue_root_revocation(&record, transition_id, &authority, &root_key)",
    1,
)
replace_count(
    trust_path,
    ".apply_root(&mut record, authority.root())",
    ".apply_root(&mut record, &authority)",
    1,
)

replace_test(
    trust_path,
    "inactive_delegation_is_rejected",
    r'''#[test]
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
}''',
)

replace_test(
    trust_path,
    "recovery_authority_is_rejected_from_ordinary_revocation_path",
    r'''#[test]
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
}''',
)

replace_test(
    trust_path,
    "delegated_revocation_rejects_the_wrong_private_key",
    r'''#[test]
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
}''',
)

replace_test(
    trust_path,
    "transition_is_bound_to_the_credential_epoch_at_issue_time",
    r'''#[test]
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
        .accept_successor_credential(
            &successor,
            &authority,
            TransitionId::from_bytes([23; 32]),
        )
        .unwrap();

    assert_eq!(
        transition.apply_root(&mut record, &authority),
        Err(TrustTransitionError::CredentialEpochMismatch)
    );
    assert_eq!(record.state(), TrustState::Trusted);
}''',
)

replace_test(
    trust_path,
    "forged_root_signature_is_rejected",
    r'''#[test]
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
}''',
)

replace_test(
    trust_path,
    "transition_requires_exact_next_trust_revision",
    r'''#[test]
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
}''',
)

identity_path = "crates/identity/tests/credentials.rs"
replace_test(
    identity_path,
    "device_credential_rejects_mismatched_issuer_key",
    r'''#[test]
fn device_credential_rejects_mismatched_issuer_key() {
    let (owner_id, _, root, _, delegation) = fixture();
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let result = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([21; 32]),
        &SigningKey::from_secret_bytes([22; 32]),
        0,
        &authority,
        &SigningKey::from_secret_bytes([23; 32]),
    );

    assert_eq!(result.unwrap_err(), IdentityError::UnknownIssuer);
}''',
)

replace_test(
    identity_path,
    "device_credential_rejects_wrong_owner_domain",
    r'''#[test]
fn device_credential_rejects_wrong_owner_domain() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([24; 32]),
        &SigningKey::from_secret_bytes([25; 32]),
        0,
        &authority,
        &issuer_key,
    )
    .unwrap();

    let wrong_owner_id = OwnerId::from_bytes([26; 32]);
    let wrong_root_key = SigningKey::from_secret_bytes([27; 32]);
    let wrong_root = OwnerRootRecord::new(wrong_owner_id, &wrong_root_key, 0);
    let wrong_issuer_key = SigningKey::from_secret_bytes([28; 32]);
    let wrong_delegation = AuthorityDelegation::issue(
        wrong_owner_id,
        AuthorityRole::DeviceSigning,
        &wrong_issuer_key,
        0,
        &wrong_root_key,
    );
    let mut wrong_authority = OwnerAuthorityState::new(wrong_root);
    wrong_authority.accept_delegation(wrong_delegation).unwrap();

    assert_eq!(
        credential.verify(&wrong_authority, 0),
        Err(IdentityError::WrongOwner)
    );
}''',
)

replace_test(
    identity_path,
    "recovery_authority_cannot_issue_ordinary_device_credentials",
    r'''#[test]
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
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(recovery).unwrap();
    let device_key = SigningKey::from_secret_bytes([10; 32]);

    let result = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([11; 32]),
        &device_key,
        0,
        &authority,
        &recovery_key,
    );

    assert_eq!(result.unwrap_err(), IdentityError::UnknownIssuer);
}''',
)

replace_test(
    identity_path,
    "superseded_device_signing_authority_cannot_verify_current_credentials",
    r'''#[test]
fn superseded_device_signing_authority_cannot_verify_current_credentials() {
    let (owner_id, root_key, root, old_issuer_key, old_delegation) = fixture();
    let device_id = DeviceId::from_bytes([35; 32]);
    let device_key = SigningKey::from_secret_bytes([36; 32]);
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(old_delegation).unwrap();
    let credential = DeviceCredential::issue(
        owner_id,
        device_id,
        &device_key,
        0,
        &authority,
        &old_issuer_key,
    )
    .unwrap();
    let new_issuer_key = SigningKey::from_secret_bytes([37; 32]);
    let new_delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &new_issuer_key,
        1,
        &root_key,
    );
    authority.accept_delegation(new_delegation).unwrap();

    assert_eq!(
        credential.verify(&authority, 0),
        Err(IdentityError::UnknownIssuer)
    );
}''',
)

replace_test(
    identity_path,
    "device_key_can_prove_possession_for_a_session_digest",
    r'''#[test]
fn device_key_can_prove_possession_for_a_session_digest() {
    let (owner_id, _, root, issuer_key, delegation) = fixture();
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let device_key = SigningKey::from_secret_bytes([28; 32]);
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([29; 32]),
        &device_key,
        0,
        &authority,
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
}''',
)
