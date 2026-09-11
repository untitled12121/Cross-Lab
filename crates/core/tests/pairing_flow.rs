use crosslab_core::{
    PairingFlowError, PairingId, PairingInvitation, PairingInvitationState, PairingInviterFlow,
    PairingInviterState, PairingJoinerFlow, PairingJoinerState, PairingSecret,
};
use crosslab_crypto::{Signature, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{TransitionId, TrustState};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole,
};

struct Fixture {
    owner_id: OwnerId,
    root: OwnerRootRecord,
    issuer_key: SigningKey,
    delegation: AuthorityDelegation,
    inviter_key: SigningKey,
    joiner_key: SigningKey,
    inviter_device_id: DeviceId,
    joiner_device_id: DeviceId,
    pairing_id: PairingId,
    secret: [u8; 32],
    inviter_hello: PairingHello,
    joiner_hello: PairingHello,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x10; 32]);
        let root_key = SigningKey::from_secret_bytes([0x11; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x12; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let inviter_key = SigningKey::from_secret_bytes([0x13; 32]);
        let joiner_key = SigningKey::from_secret_bytes([0x14; 32]);
        let inviter_device_id = DeviceId::from_bytes([0x15; 32]);
        let joiner_device_id = DeviceId::from_bytes([0x16; 32]);
        let pairing_id = PairingId::from_bytes([0x17; 16]);
        let secret = [0x18; 32];
        let inviter_hello = PairingHello::new(
            PairingRole::Inviter,
            1,
            pairing_id.to_bytes(),
            owner_id,
            inviter_device_id,
            inviter_key.verifying_key(),
            [0x19; 32],
        );
        let joiner_hello = PairingHello::new(
            PairingRole::Joiner,
            1,
            pairing_id.to_bytes(),
            owner_id,
            joiner_device_id,
            joiner_key.verifying_key(),
            [0x1a; 32],
        );

        Self {
            owner_id,
            root,
            issuer_key,
            delegation,
            inviter_key,
            joiner_key,
            inviter_device_id,
            joiner_device_id,
            pairing_id,
            secret,
            inviter_hello,
            joiner_hello,
        }
    }

    fn invitation(&self) -> PairingInvitation {
        PairingInvitation::from_parts(
            self.pairing_id,
            PairingSecret::from_bytes(self.secret),
            self.owner_id,
            self.inviter_device_id,
        )
    }

    fn inviter_flow(&self) -> PairingInviterFlow {
        PairingInviterFlow::new(self.invitation(), self.inviter_hello, self.joiner_hello).unwrap()
    }

    fn joiner_flow(&self) -> PairingJoinerFlow {
        PairingJoinerFlow::new(
            PairingSecret::from_bytes(self.secret),
            self.inviter_hello,
            self.joiner_hello,
        )
        .unwrap()
    }

    fn confirmed_flows(&self) -> (PairingInviterFlow, PairingJoinerFlow) {
        let mut inviter = self.inviter_flow();
        let mut joiner = self.joiner_flow();
        let joiner_confirmation = joiner.joiner_confirmation().unwrap();
        let inviter_confirmation = inviter
            .verify_joiner_confirmation(&joiner_confirmation)
            .unwrap();
        joiner
            .verify_inviter_confirmation(&inviter_confirmation)
            .unwrap();
        (inviter, joiner)
    }
}

#[test]
fn s002_pairing_commits_trust_only_after_final_joiner_proof() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.confirmed_flows();

    assert_eq!(inviter.state(), PairingInviterState::ReadyToIssueCredential);
    assert_eq!(joiner.state(), PairingJoinerState::AwaitingCredential);
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Pending);

    let credential = inviter
        .issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 7)
        .unwrap();
    assert_eq!(
        inviter.state(),
        PairingInviterState::AwaitingCredentialAcceptance
    );
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Pending);

    let accepted = joiner
        .accept_credential(
            &fixture.root,
            &fixture.delegation,
            &credential,
            &fixture.joiner_key,
        )
        .unwrap();
    assert_eq!(joiner.state(), PairingJoinerState::Accepted);

    let transition_id = TransitionId::from_bytes([0x1b; 32]);
    let trust = inviter.commit_trust(&accepted, transition_id).unwrap();

    assert_eq!(trust.owner_id(), fixture.owner_id);
    assert_eq!(trust.device_id(), fixture.joiner_device_id);
    assert_eq!(trust.state(), TrustState::Trusted);
    assert_eq!(trust.accepted_credential_epoch(), 7);
    assert_eq!(trust.trust_revision(), 0);
    assert_eq!(trust.last_transition_id(), transition_id);
    assert_eq!(inviter.state(), PairingInviterState::Trusted);
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Consumed);
}

#[test]
fn n010_wrong_pairing_secret_rejects_and_consumes_invitation() {
    let fixture = Fixture::new();
    let mut inviter = fixture.inviter_flow();
    let wrong_joiner = PairingJoinerFlow::new(
        PairingSecret::from_bytes([0xee; 32]),
        fixture.inviter_hello,
        fixture.joiner_hello,
    )
    .unwrap();
    let confirmation = wrong_joiner.joiner_confirmation().unwrap();

    assert_eq!(
        inviter.verify_joiner_confirmation(&confirmation),
        Err(PairingFlowError::InvalidConfirmation)
    );
    assert_eq!(inviter.state(), PairingInviterState::Failed);
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Consumed);
}

#[test]
fn n011_replayed_pairing_confirmation_is_rejected_fail_closed() {
    let fixture = Fixture::new();
    let mut inviter = fixture.inviter_flow();
    let joiner = fixture.joiner_flow();
    let confirmation = joiner.joiner_confirmation().unwrap();

    inviter.verify_joiner_confirmation(&confirmation).unwrap();
    assert_eq!(inviter.state(), PairingInviterState::ReadyToIssueCredential);
    assert_eq!(
        inviter.verify_joiner_confirmation(&confirmation),
        Err(PairingFlowError::UnexpectedState)
    );
    assert_eq!(inviter.state(), PairingInviterState::Failed);
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Consumed);
}

#[test]
fn n012_pairing_id_and_nonce_substitution_are_rejected() {
    let fixture = Fixture::new();

    let mut pairing_id_inviter = fixture.inviter_flow();
    let joiner = fixture.joiner_flow();
    let valid = joiner.joiner_confirmation().unwrap();
    let wrong_pairing_id =
        PairingConfirmation::new(PairingRole::Joiner, [0xfe; 16], valid.confirmation());
    assert_eq!(
        pairing_id_inviter.verify_joiner_confirmation(&wrong_pairing_id),
        Err(PairingFlowError::InvalidConfirmation)
    );
    assert_eq!(
        pairing_id_inviter.invitation_state(),
        PairingInvitationState::Consumed
    );

    let substituted_joiner_hello = PairingHello::new(
        PairingRole::Joiner,
        1,
        fixture.pairing_id.to_bytes(),
        fixture.owner_id,
        fixture.joiner_device_id,
        fixture.joiner_key.verifying_key(),
        [0xfd; 32],
    );
    let substituted_joiner = PairingJoinerFlow::new(
        PairingSecret::from_bytes(fixture.secret),
        fixture.inviter_hello,
        substituted_joiner_hello,
    )
    .unwrap();
    let substituted_confirmation = substituted_joiner.joiner_confirmation().unwrap();
    let mut nonce_inviter = fixture.inviter_flow();

    assert_eq!(
        nonce_inviter.verify_joiner_confirmation(&substituted_confirmation),
        Err(PairingFlowError::InvalidConfirmation)
    );
    assert_eq!(nonce_inviter.state(), PairingInviterState::Failed);
    assert_eq!(
        nonce_inviter.invitation_state(),
        PairingInvitationState::Consumed
    );
}

#[test]
fn n013_joiner_key_substitution_after_confirmation_is_rejected() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.confirmed_flows();
    let wrong_joiner_key = SigningKey::from_secret_bytes([0xef; 32]);
    let substituted = DeviceCredential::issue_for_public_key(
        fixture.owner_id,
        fixture.joiner_device_id,
        wrong_joiner_key.verifying_key(),
        0,
        &fixture.root,
        &fixture.delegation,
        &fixture.issuer_key,
    )
    .unwrap();

    assert_eq!(
        joiner.accept_credential(
            &fixture.root,
            &fixture.delegation,
            &substituted,
            &fixture.joiner_key,
        ),
        Err(PairingFlowError::CredentialMismatch)
    );
    assert_eq!(joiner.state(), PairingJoinerState::Failed);

    let result =
        inviter.issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 0);
    assert!(result.is_ok());
    assert_ne!(inviter.state(), PairingInviterState::Trusted);
}

#[test]
fn n014_cancelled_or_consumed_invitation_cannot_be_reused() {
    let fixture = Fixture::new();

    let mut cancelled = fixture.invitation();
    cancelled.cancel().unwrap();
    assert_eq!(
        PairingInviterFlow::new(cancelled, fixture.inviter_hello, fixture.joiner_hello).err(),
        Some(PairingFlowError::InvitationNotPending)
    );

    let mut consumed = fixture.invitation();
    consumed.consume().unwrap();
    assert_eq!(
        PairingInviterFlow::new(consumed, fixture.inviter_hello, fixture.joiner_hello).err(),
        Some(PairingFlowError::InvitationNotPending)
    );
}

#[test]
fn n015_wrong_credential_accepted_device_key_never_commits_trust() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.confirmed_flows();
    let credential = inviter
        .issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 0)
        .unwrap();

    assert_eq!(
        joiner.accept_credential(
            &fixture.root,
            &fixture.delegation,
            &credential,
            &SigningKey::from_secret_bytes([0xf0; 32]),
        ),
        Err(PairingFlowError::JoinerKeyMismatch)
    );
    assert_eq!(joiner.state(), PairingJoinerState::Failed);

    let forged = PairingCredentialAccepted::new(
        fixture.pairing_id.to_bytes(),
        [0x21; 32],
        [0x22; 32],
        fixture.joiner_device_id,
        credential.device_key_id(),
        Signature::from_bytes([0x23; 64]),
    );
    assert_eq!(
        inviter.commit_trust(&forged, TransitionId::from_bytes([0x24; 32])),
        Err(PairingFlowError::InvalidCredentialAcceptance)
    );
    assert_eq!(inviter.state(), PairingInviterState::Failed);
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Consumed);
}

#[test]
fn mismatched_owner_or_invalid_delegation_fails_before_acceptance() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.confirmed_flows();
    let credential = inviter
        .issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 0)
        .unwrap();

    let wrong_owner = OwnerId::from_bytes([0x31; 32]);
    let wrong_root_key = SigningKey::from_secret_bytes([0x32; 32]);
    let wrong_root = OwnerRootRecord::new(wrong_owner, &wrong_root_key, 0);
    assert_eq!(
        joiner.accept_credential(
            &wrong_root,
            &fixture.delegation,
            &credential,
            &fixture.joiner_key,
        ),
        Err(PairingFlowError::Identity(IdentityError::WrongOwner))
    );
    assert_eq!(joiner.state(), PairingJoinerState::Failed);

    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.confirmed_flows();
    let credential = inviter
        .issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 0)
        .unwrap();
    let wrong_root_key = SigningKey::from_secret_bytes([0x33; 32]);
    let wrong_issuer_key = SigningKey::from_secret_bytes([0x34; 32]);
    let invalid_delegation = AuthorityDelegation::issue(
        fixture.owner_id,
        AuthorityRole::DeviceSigning,
        &wrong_issuer_key,
        0,
        &wrong_root_key,
    );

    assert_eq!(
        joiner.accept_credential(
            &fixture.root,
            &invalid_delegation,
            &credential,
            &fixture.joiner_key,
        ),
        Err(PairingFlowError::Identity(IdentityError::UnknownIssuer))
    );
    assert_eq!(joiner.state(), PairingJoinerState::Failed);
}

#[test]
fn mismatched_pairing_hello_context_is_rejected_before_confirmation() {
    let fixture = Fixture::new();
    let wrong_owner_hello = PairingHello::new(
        PairingRole::Joiner,
        1,
        fixture.pairing_id.to_bytes(),
        OwnerId::from_bytes([0x41; 32]),
        fixture.joiner_device_id,
        fixture.joiner_key.verifying_key(),
        fixture.joiner_hello.nonce(),
    );

    assert_eq!(
        PairingInviterFlow::new(
            fixture.invitation(),
            fixture.inviter_hello,
            wrong_owner_hello,
        )
        .err(),
        Some(PairingFlowError::InvalidPairingContext)
    );
    assert_eq!(
        PairingJoinerFlow::new(
            PairingSecret::from_bytes(fixture.secret),
            fixture.inviter_hello,
            wrong_owner_hello,
        )
        .err(),
        Some(PairingFlowError::InvalidPairingContext)
    );
}

#[test]
fn inviter_credential_issuance_does_not_require_joiner_private_key() {
    let fixture = Fixture::new();
    let (mut inviter, _) = fixture.confirmed_flows();

    let credential = inviter
        .issue_joiner_credential(&fixture.root, &fixture.delegation, &fixture.issuer_key, 2)
        .unwrap();

    assert_eq!(credential.device_id(), fixture.joiner_device_id);
    assert_eq!(
        credential.device_public_key(),
        fixture.joiner_key.verifying_key()
    );
    credential
        .verify(&fixture.root, &fixture.delegation, 2, 0)
        .unwrap();
    assert_eq!(
        fixture.inviter_key.verifying_key(),
        fixture.inviter_hello.device_public_key()
    );
}
