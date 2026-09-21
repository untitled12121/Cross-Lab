use crosslab_core::{
    PairingBootstrap, PairingFlowError, PairingInstant, PairingInvitation, PairingSecret,
};
use crosslab_crypto::{Signature, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_identity_store::{MemoryIdentityStore, ProductIdentityState};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustState};
use crosslab_protocol::{
    PairingCredentialAccepted, PairingHello, PairingRole, ProductPairingMessage,
};

use super::*;

struct Fixture {
    authority: OwnerAuthorityState,
    root_key: SigningKey,
    issuer: SigningKey,
    inviter_key: SigningKey,
    joiner_key: SigningKey,
    inviter_credential: DeviceCredential,
    invitation: PairingInvitation,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x10; 32]);
        let root_key = SigningKey::from_secret_bytes([0x11; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer = SigningKey::from_secret_bytes([0x12; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let inviter_key = SigningKey::from_secret_bytes([0x13; 32]);
        let inviter_device_id = DeviceId::from_bytes([0x14; 32]);
        let inviter_credential = DeviceCredential::issue(
            owner_id,
            inviter_device_id,
            &inviter_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let invitation = PairingInvitation::from_parts(
            crosslab_core::PairingId::from_bytes([0x15; 16]),
            PairingSecret::from_bytes([0x16; 32]),
            owner_id,
            inviter_device_id,
            PairingInstant::from_ticks(0),
            PairingInstant::from_ticks(100),
        )
        .unwrap();

        Self {
            authority,
            root_key,
            issuer,
            inviter_key,
            joiner_key: SigningKey::from_secret_bytes([0x17; 32]),
            inviter_credential,
            invitation,
        }
    }

    fn coordinators(&self) -> (ProductPairingInviter, ProductPairingJoiner) {
        let code = {
            let mut invitation = self.invitation_for_test();
            invitation
                .bootstrap_code_at(PairingInstant::from_ticks(0))
                .unwrap()
        };
        let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
        let inviter = ProductPairingInviter::with_nonce(
            self.invitation_for_test(),
            self.inviter_credential,
            &self.authority,
            [0x18; 32],
        )
        .unwrap();
        let joiner = ProductPairingJoiner::with_nonce(
            bootstrap,
            DeviceId::from_bytes([0x19; 32]),
            &self.joiner_key,
            [0x1a; 32],
        )
        .unwrap();
        (inviter, joiner)
    }

    fn invitation_for_test(&self) -> PairingInvitation {
        PairingInvitation::from_parts(
            self.invitation.pairing_id(),
            PairingSecret::from_bytes([0x16; 32]),
            self.invitation.owner_id(),
            self.invitation.inviter_device_id(),
            PairingInstant::from_ticks(0),
            PairingInstant::from_ticks(100),
        )
        .unwrap()
    }
}

#[test]
fn inviter_requires_persistence_before_product_completion() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let credential = inviter
        .issue_joiner_credential(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let accepted = joiner
        .accept_credential(&fixture.authority, &credential, &fixture.joiner_key)
        .unwrap();
    let completion = inviter
        .verify_credential_acceptance(
            &accepted,
            TransitionId::from_bytes([0x1b; 32]),
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        )
        .unwrap();
    let inviter_commit = completion.commit();
    let joiner_completion = joiner
        .accept_inviter_trust(completion.reciprocal_trust(), &fixture.authority)
        .unwrap();
    let joiner_commit = joiner_completion.peer_commit();

    assert_eq!(inviter.state(), ProductPairingState::AwaitingPersistence);
    assert_eq!(joiner.state(), ProductPairingState::AwaitingPersistence);
    assert_eq!(inviter_commit.peer_credential(), credential);
    assert_eq!(inviter_commit.peer_trust().state(), TrustState::Trusted);
    assert_eq!(
        inviter_commit.peer_trust().device_id(),
        credential.device_id()
    );
    assert_eq!(joiner_completion.owner_root(), *fixture.authority.root());
    assert_eq!(
        joiner_completion.device_signing(),
        *fixture
            .authority
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
    );
    assert_eq!(joiner_completion.local_credential(), credential);
    assert_eq!(joiner_commit.peer_credential(), fixture.inviter_credential);
    assert_eq!(joiner_commit.peer_trust().state(), TrustState::Trusted);
    assert_eq!(
        joiner_commit.peer_trust().device_id(),
        fixture.inviter_credential.device_id()
    );

    inviter.mark_persisted().unwrap();
    joiner.mark_persisted().unwrap();
    assert_eq!(inviter.state(), ProductPairingState::Complete);
    assert_eq!(joiner.state(), ProductPairingState::Complete);
}

#[test]
fn successful_pairing_persists_both_sides_and_reconstructs_trust() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let joiner_credential = inviter
        .issue_joiner_credential(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let accepted = joiner
        .accept_credential(&fixture.authority, &joiner_credential, &fixture.joiner_key)
        .unwrap();
    let inviter_completion = inviter
        .verify_credential_acceptance(
            &accepted,
            TransitionId::from_bytes([0x1d; 32]),
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        )
        .unwrap();
    let joiner_completion = joiner
        .accept_inviter_trust(inviter_completion.reciprocal_trust(), &fixture.authority)
        .unwrap();

    let device_signing = *fixture
        .authority
        .current_delegation(AuthorityRole::DeviceSigning)
        .unwrap();
    let inviter_identity = ProductIdentityState::join_owner_domain(
        *fixture.authority.root(),
        device_signing,
        fixture.inviter_credential,
        &fixture.inviter_key,
    )
    .unwrap()
    .with_paired_peer(
        inviter_completion.commit().peer_credential(),
        inviter_completion.commit().peer_transition(),
    )
    .unwrap();
    let joiner_identity = ProductIdentityState::join_owner_domain(
        joiner_completion.owner_root(),
        joiner_completion.device_signing(),
        joiner_completion.local_credential(),
        &fixture.joiner_key,
    )
    .unwrap()
    .with_paired_peer(
        joiner_completion.peer_commit().peer_credential(),
        joiner_completion.peer_commit().peer_transition(),
    )
    .unwrap();

    let inviter_store = MemoryIdentityStore::default();
    inviter_store
        .compare_and_swap(None, inviter_identity.encode())
        .unwrap();
    let joiner_store = MemoryIdentityStore::default();
    joiner_store
        .compare_and_swap(None, joiner_identity.encode())
        .unwrap();

    let inviter_restored =
        ProductIdentityState::decode(inviter_store.load().unwrap().unwrap().payload()).unwrap();
    inviter_restored
        .validate_providers(&fixture.root_key, &fixture.issuer, &fixture.inviter_key)
        .unwrap();
    assert!(
        inviter_restored
            .trusted_peer(joiner_credential.device_id())
            .is_some()
    );

    let joiner_restored =
        ProductIdentityState::decode(joiner_store.load().unwrap().unwrap().payload()).unwrap();
    joiner_restored
        .validate_local_device_provider(&fixture.joiner_key)
        .unwrap();
    assert!(
        joiner_restored
            .trusted_peer(fixture.inviter_credential.device_id())
            .is_some()
    );

    inviter.mark_persisted().unwrap();
    joiner.mark_persisted().unwrap();
    assert_eq!(inviter.state(), ProductPairingState::Complete);
    assert_eq!(joiner.state(), ProductPairingState::Complete);
}

#[test]
fn scanned_bootstrap_binds_joiner_to_the_inviter_device() {
    let fixture = Fixture::new();
    let code = {
        let mut invitation = fixture.invitation_for_test();
        invitation
            .bootstrap_code_at(PairingInstant::from_ticks(0))
            .unwrap()
    };
    let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
    let mut joiner = ProductPairingJoiner::with_nonce(
        bootstrap,
        DeviceId::from_bytes([0x19; 32]),
        &fixture.joiner_key,
        [0x1a; 32],
    )
    .unwrap();

    let substituted = PairingHello::new(
        PairingRole::Inviter,
        PROTOCOL_MAJOR_V1,
        joiner.hello().pairing_id(),
        joiner.hello().owner_id(),
        DeviceId::from_bytes([0xee; 32]),
        fixture.inviter_key.verifying_key(),
        [0xef; 32],
    );

    assert_eq!(
        joiner.accept_inviter_hello(substituted),
        Err(ProductPairingError::BootstrapPeerMismatch)
    );
    assert_eq!(joiner.state(), ProductPairingState::Failed);
}

#[test]
fn replayed_confirmation_fails_the_in_progress_product_pairing() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    inviter
        .verify_joiner_confirmation(&confirmation, PairingInstant::from_ticks(20))
        .unwrap();

    assert_eq!(
        inviter.verify_joiner_confirmation(&confirmation, PairingInstant::from_ticks(21),),
        Err(ProductPairingError::InvalidState)
    );
    assert_eq!(inviter.state(), ProductPairingState::Failed);
}

#[test]
fn forged_final_proof_cannot_reach_persistence() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let credential = inviter
        .issue_joiner_credential(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let accepted = joiner
        .accept_credential(&fixture.authority, &credential, &fixture.joiner_key)
        .unwrap();
    let forged = PairingCredentialAccepted::new(
        accepted.pairing_id(),
        accepted.pairing_transcript_digest(),
        accepted.device_credential_signed_object_digest(),
        accepted.joiner_device_id(),
        accepted.joiner_device_key_id(),
        Signature::from_bytes([0xee; 64]),
    );

    assert_eq!(
        inviter.verify_credential_acceptance(
            &forged,
            TransitionId::from_bytes([0x1e; 32]),
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        ),
        Err(ProductPairingError::Flow(
            PairingFlowError::InvalidCredentialAcceptance
        ))
    );
    assert_eq!(inviter.state(), ProductPairingState::Failed);
}

#[test]
fn persistence_failure_is_terminal() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let credential = inviter
        .issue_joiner_credential(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let accepted = joiner
        .accept_credential(&fixture.authority, &credential, &fixture.joiner_key)
        .unwrap();
    let completion = inviter
        .verify_credential_acceptance(
            &accepted,
            TransitionId::from_bytes([0x1b; 32]),
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        )
        .unwrap();
    joiner
        .accept_inviter_trust(completion.reciprocal_trust(), &fixture.authority)
        .unwrap();

    inviter.persistence_failed().unwrap();
    joiner.persistence_failed().unwrap();
    assert_eq!(inviter.state(), ProductPairingState::Failed);
    assert_eq!(joiner.state(), ProductPairingState::Failed);
    assert_eq!(
        inviter.mark_persisted(),
        Err(ProductPairingError::InvalidState)
    );
    assert_eq!(
        joiner.mark_persisted(),
        Err(ProductPairingError::InvalidState)
    );
}

#[test]
fn joiner_rejects_peer_trust_from_another_pairing_evidence() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let credential = inviter
        .issue_joiner_credential(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let accepted = joiner
        .accept_credential(&fixture.authority, &credential, &fixture.joiner_key)
        .unwrap();
    inviter
        .verify_credential_acceptance(
            &accepted,
            TransitionId::from_bytes([0x1b; 32]),
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        )
        .unwrap();

    let unrelated_transition = PairingTrustTransition::issue(
        &fixture.inviter_credential,
        TransitionId::from_bytes([0x1c; 32]),
        [0xee; 32],
        &fixture.authority,
        &fixture.issuer,
    )
    .unwrap();
    let result = joiner.accept_inviter_trust(
        ProductPairingTrustBundle {
            credential: fixture.inviter_credential,
            transition: unrelated_transition,
        },
        &fixture.authority,
    );

    assert_eq!(
        result,
        Err(ProductPairingError::Flow(
            PairingFlowError::InvalidPeerTrustEvidence
        ))
    );
    assert_eq!(joiner.state(), ProductPairingState::Failed);
}

#[test]
fn cancellation_drops_uncommitted_pairing_state() {
    let fixture = Fixture::new();
    let (mut inviter, mut joiner) = fixture.coordinators();

    inviter.cancel().unwrap();
    joiner.cancel().unwrap();

    assert_eq!(inviter.state(), ProductPairingState::Cancelled);
    assert_eq!(joiner.state(), ProductPairingState::Cancelled);
    assert_eq!(
        inviter.accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10)),
        Err(ProductPairingError::InvalidState)
    );
}

#[test]
fn network_exchange_carries_pairing_to_reciprocal_completion() {
    let fixture = Fixture::new();
    let (inviter, joiner) = fixture.coordinators();
    let mut inviter = ProductPairingInviterExchange::new(inviter);
    let mut joiner = ProductPairingJoinerExchange::new(joiner);

    let inviter_hello = inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();
    let joiner_confirmation = joiner.accept_inviter_hello(inviter_hello).unwrap();
    let inviter_confirmation = inviter
        .accept_joiner_confirmation(joiner_confirmation, PairingInstant::from_ticks(20))
        .unwrap();
    joiner
        .accept_inviter_confirmation(inviter_confirmation)
        .unwrap();

    let credential_bundle = inviter
        .issue_credential_bundle(
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(30),
        )
        .unwrap();
    let credential_proof = joiner
        .accept_credential_bundle(credential_bundle, &fixture.joiner_key)
        .unwrap();
    let inviter_commit = inviter
        .accept_credential_proof(
            credential_proof,
            &fixture.authority,
            &fixture.issuer,
            PairingInstant::from_ticks(40),
        )
        .unwrap();

    assert_eq!(
        inviter.state(),
        ProductPairingExchangeState::AwaitingLocalPersistence
    );
    assert_eq!(
        inviter_commit.peer_credential().device_id(),
        DeviceId::from_bytes([0x19; 32])
    );

    let trust_bundle = inviter.local_persisted().unwrap();
    let joiner_completion = joiner.accept_trust_bundle(trust_bundle).unwrap();
    assert_eq!(
        joiner.state(),
        ProductPairingExchangeState::AwaitingLocalPersistence
    );
    assert_eq!(
        joiner_completion.peer_commit().peer_credential(),
        fixture.inviter_credential
    );

    let persisted = joiner.local_persisted().unwrap();
    let complete = inviter.accept_peer_persisted(persisted).unwrap();
    joiner.accept_complete(complete).unwrap();

    assert_eq!(inviter.state(), ProductPairingExchangeState::Complete);
    assert_eq!(joiner.state(), ProductPairingExchangeState::Complete);
}

#[test]
fn network_exchange_rejects_replayed_out_of_sequence_message() {
    let fixture = Fixture::new();
    let (inviter, joiner) = fixture.coordinators();
    let mut inviter = ProductPairingInviterExchange::new(inviter);
    let joiner = ProductPairingJoinerExchange::new(joiner);

    inviter
        .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
        .unwrap();

    assert_eq!(
        inviter.accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(11)),
        Err(ProductPairingNetworkError::UnexpectedMessage)
    );
    assert_eq!(inviter.state(), ProductPairingExchangeState::Failed);
}

#[test]
fn network_exchange_rejects_wrong_message_type_before_secret_confirmation() {
    let fixture = Fixture::new();
    let (inviter, _) = fixture.coordinators();
    let mut inviter = ProductPairingInviterExchange::new(inviter);

    let wrong = ProductPairingMessage::Cancel {
        pairing_id: [0xff; 16],
    };
    assert_eq!(
        inviter.accept_joiner_hello(wrong, PairingInstant::from_ticks(10)),
        Err(ProductPairingNetworkError::UnexpectedMessage)
    );
    assert_eq!(inviter.state(), ProductPairingExchangeState::Failed);
}
