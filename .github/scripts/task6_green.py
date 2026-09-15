from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected 1 match, found {count}"
    return text.replace(old, new, 1)


auth_path = Path("experiments/m9-networking/src/scenarios/auth.rs")
auth = auth_path.read_text()

auth = replace_once(
    auth,
    "use crosslab_policy::{NetworkClass, PairingTrustTransition, TransitionId, TrustRecord};",
    "use crosslab_policy::{\n    NetworkClass, PairingTrustTransition, TransitionId, TrustRecord, TrustTransition,\n};",
    "trust transition import",
)

auth = replace_once(
    auth,
    """    fn responder_features() -> FeatureSet {
        FeatureSet::new(&[2, 3, 4], &[3]).unwrap()
    }
}""",
    """    fn responder_features() -> FeatureSet {
        FeatureSet::new(&[2, 3, 4], &[3]).unwrap()
    }

    pub(crate) const fn authority(&self) -> &OwnerAuthorityState {
        &self.authority
    }

    pub(crate) const fn initiator_credential(&self) -> &DeviceCredential {
        &self.initiator_credential
    }

    pub(crate) const fn responder_credential(&self) -> &DeviceCredential {
        &self.responder_credential
    }

    pub(crate) const fn initiator_trust(&self) -> TrustRecord {
        self.initiator_trust
    }

    pub(crate) const fn responder_trust(&self) -> TrustRecord {
        self.responder_trust
    }

    pub(crate) fn revoked_responder(&self) -> TrustRecord {
        let root_key = SigningKey::from_secret_bytes([0x41; 32]);
        let transition = TrustTransition::issue_root_revocation(
            &self.responder_trust,
            TransitionId::from_bytes([0xcb; 32]),
            &self.authority,
            &root_key,
        )
        .unwrap();
        let mut revoked = self.responder_trust;
        transition.apply_root(&mut revoked, &self.authority).unwrap();
        revoked
    }
}""",
    "fixture accessors",
)

auth = replace_once(
    auth,
    """pub struct AuthenticatedIrohPair {
    direct: DirectPair,
    client_transport: IrohTransportConnection,
    server_transport: IrohTransportConnection,
    client_session: LogicalSession,
    server_session: LogicalSession,
    initiator_proof: ReplayProof,
    network_class: NetworkClass,
}

impl AuthenticatedIrohPair {""",
    """pub struct AuthenticatedIrohPair {
    direct: DirectPair,
    client_transport: IrohTransportConnection,
    server_transport: IrohTransportConnection,
    client_session: LogicalSession,
    server_session: LogicalSession,
    initiator_proof: ReplayProof,
    network_class: NetworkClass,
}

pub(crate) struct AuthenticatedIrohParts {
    pub(crate) direct: DirectPair,
    pub(crate) client_transport: IrohTransportConnection,
    pub(crate) server_transport: IrohTransportConnection,
    pub(crate) client_session: LogicalSession,
    pub(crate) server_session: LogicalSession,
}

impl AuthenticatedIrohPair {""",
    "owned auth parts",
)

auth = replace_once(
    auth,
    """    pub const fn network_class(&self) -> NetworkClass {
        self.network_class
    }

    pub async fn shutdown(self) {""",
    """    pub const fn network_class(&self) -> NetworkClass {
        self.network_class
    }

    pub(crate) fn into_parts(self) -> AuthenticatedIrohParts {
        let Self {
            direct,
            client_transport,
            server_transport,
            client_session,
            server_session,
            ..
        } = self;
        AuthenticatedIrohParts {
            direct,
            client_transport,
            server_transport,
            client_session,
            server_session,
        }
    }

    pub async fn shutdown(self) {""",
    "into parts",
)

auth = replace_once(
    auth,
    """pub async fn authenticate_direct_pair(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
) -> Result<AuthenticatedIrohPair, Box<RejectedAuthentication>> {
    let mut bootstrap = bootstrap_direct_pair(fixture)""",
    """pub async fn authenticate_direct_pair(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
) -> Result<AuthenticatedIrohPair, Box<RejectedAuthentication>> {
    authenticate_direct_pair_with_peer_trust(
        fixture,
        attempt,
        &fixture.responder_trust,
        &fixture.initiator_trust,
    )
    .await
}

pub(crate) async fn authenticate_direct_pair_with_peer_trust(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
    client_peer_trust: &TrustRecord,
    server_peer_trust: &TrustRecord,
) -> Result<AuthenticatedIrohPair, Box<RejectedAuthentication>> {
    let mut bootstrap = bootstrap_direct_pair(fixture)""",
    "trust override auth entry",
)

auth = replace_once(
    auth,
    "            &fixture.responder_trust,",
    "            client_peer_trust,",
    "client peer trust",
)
auth = replace_once(
    auth,
    "            &fixture.initiator_trust,",
    "            server_peer_trust,",
    "server peer trust",
)

auth = replace_once(
    auth,
    """        _ => {
            bootstrap.shutdown().await;
            panic!(\"peers disagreed on session authentication outcome\")
        }
""",
    """        (Err(client_error), Ok(())) => {
            bootstrap
                .server_session
                .transport_lost()
                .expect(\"successful peer must fail closed when the other side rejects auth\");
            let BootstrapIrohPair {
                direct,
                client_session,
                server_session,
                ..
            } = bootstrap;
            direct.shutdown().await;
            Err(Box::new(RejectedAuthentication {
                client_session,
                server_session,
                error: client_error,
            }))
        }
        (Ok(()), Err(server_error)) => {
            bootstrap
                .client_session
                .transport_lost()
                .expect(\"successful peer must fail closed when the other side rejects auth\");
            let BootstrapIrohPair {
                direct,
                client_session,
                server_session,
                ..
            } = bootstrap;
            direct.shutdown().await;
            Err(Box::new(RejectedAuthentication {
                client_session,
                server_session,
                error: server_error,
            }))
        }
""",
    "asymmetric auth fail close",
)

auth_path.write_text(auth)

mod_path = Path("experiments/m9-networking/src/scenarios/mod.rs")
mod_text = mod_path.read_text()
mod_text = replace_once(mod_text, "pub mod auth;\n", "pub mod auth;\npub mod lifecycle;\n", "lifecycle module")
mod_path.write_text(mod_text)

lifecycle_path = Path("experiments/m9-networking/src/scenarios/lifecycle.rs")
lifecycle = lifecycle_path.read_text()
lifecycle = lifecycle.replace(
    "receiver.register_operation(authority.operation).unwrap();",
    "receiver.register_operation(authority.operation.clone()).unwrap();",
)
assert lifecycle.count("register_operation(authority.operation.clone())") == 2
lifecycle_path.write_text(lifecycle)

test_path = Path("experiments/m9-networking/tests/lifecycle.rs")
test = test_path.read_text()
test = replace_once(
    test,
    "    StreamOpenError, StreamSendError, TransportConnection,\n",
    "    StreamOpenError, StreamSendError,\n",
    "unused transport import",
)
test_path.write_text(test)
