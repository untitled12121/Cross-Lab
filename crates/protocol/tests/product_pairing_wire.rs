use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole, ProductPairingAck,
    ProductPairingAckKind, ProductPairingCredentialBundle, ProductPairingMessage,
    ProductPairingTrustBundleMessage, decode_product_pairing, encode_product_pairing,
};

struct Fixture {
    owner_id: OwnerId,
    authority: OwnerAuthorityState,
    issuer: SigningKey,
    inviter_credential: DeviceCredential,
    joiner_credential: DeviceCredential,
    transition: PairingTrustTransition,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x11; 32]);
        let root_key = SigningKey::from_secret_bytes([0x12; 32]);
        let issuer = SigningKey::from_secret_bytes([0x13; 32]);
        let inviter_key = SigningKey::from_secret_bytes([0x14; 32]);
        let joiner_key = SigningKey::from_secret_bytes([0x15; 32]);

        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let inviter_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x21; 32]),
            &inviter_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let joiner_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x22; 32]),
            &joiner_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let transition = PairingTrustTransition::issue(
            &joiner_credential,
            TransitionId::from_bytes([0x31; 32]),
            [0x32; 32],
            &authority,
            &issuer,
        )
        .unwrap();

        Self {
            owner_id,
            authority,
            issuer,
            inviter_credential,
            joiner_credential,
            transition,
        }
    }

    fn round_trip(&self, message: ProductPairingMessage) {
        let encoded = encode_product_pairing(&message).unwrap();
        assert_eq!(decode_product_pairing(&encoded).unwrap(), message);
    }
}

#[test]
fn product_pairing_messages_round_trip() {
    let fixture = Fixture::new();
    let pairing_id = [0x41; 16];

    fixture.round_trip(ProductPairingMessage::Hello(PairingHello::new(
        PairingRole::Inviter,
        1,
        pairing_id,
        fixture.owner_id,
        fixture.inviter_credential.device_id(),
        fixture.inviter_credential.device_public_key(),
        [0x42; 32],
    )));
    fixture.round_trip(ProductPairingMessage::Confirmation(
        PairingConfirmation::new(PairingRole::Joiner, pairing_id, [0x43; 32]),
    ));

    let device_signing = *fixture
        .authority
        .current_delegation(AuthorityRole::DeviceSigning)
        .unwrap();
    fixture.round_trip(ProductPairingMessage::CredentialBundle(Box::new(
        ProductPairingCredentialBundle::new(
            *fixture.authority.root(),
            device_signing,
            fixture.joiner_credential,
        ),
    )));

    fixture.round_trip(ProductPairingMessage::CredentialAccepted(
        PairingCredentialAccepted::new(
            pairing_id,
            [0x44; 32],
            [0x45; 32],
            fixture.joiner_credential.device_id(),
            fixture.joiner_credential.device_key_id(),
            fixture.issuer.sign_message(b"wire-test"),
        ),
    ));

    fixture.round_trip(ProductPairingMessage::TrustBundle(
        ProductPairingTrustBundleMessage::new(fixture.joiner_credential, fixture.transition),
    ));
    fixture.round_trip(ProductPairingMessage::Ack(ProductPairingAck::new(
        pairing_id,
        PairingRole::Joiner,
        ProductPairingAckKind::Persisted,
    )));
    fixture.round_trip(ProductPairingMessage::Ack(ProductPairingAck::new(
        pairing_id,
        PairingRole::Inviter,
        ProductPairingAckKind::Complete,
    )));
    fixture.round_trip(ProductPairingMessage::Cancel { pairing_id });
}

#[test]
fn product_pairing_decoder_rejects_trailing_and_wrong_profile_frames() {
    let fixture = Fixture::new();
    let message = ProductPairingMessage::Hello(PairingHello::new(
        PairingRole::Inviter,
        1,
        [0x51; 16],
        fixture.owner_id,
        fixture.inviter_credential.device_id(),
        fixture.inviter_credential.device_public_key(),
        [0x52; 32],
    ));

    let mut frame = encode_product_pairing(&message).unwrap();
    frame.push(0);
    assert!(decode_product_pairing(&frame).is_err());

    let mut frame = encode_product_pairing(&message).unwrap();
    let payload_offset = 4;
    assert_eq!(frame[payload_offset], 0x08);
    frame[payload_offset + 1] = 0x02;
    assert!(decode_product_pairing(&frame).is_err());
}
