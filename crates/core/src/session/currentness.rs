use crosslab_identity::{AuthorityRole, OwnerAuthorityState};
use crosslab_policy::{TrustRecord, TrustState};

use super::{LogicalSession, SessionError, SessionState};

impl LogicalSession {
    pub fn revalidate_peer_trust(&mut self, peer_trust: &TrustRecord) -> Result<(), SessionError> {
        if self.state() != SessionState::Active {
            return Err(SessionError::InvalidState);
        }

        let validation = {
            let context = self.context().ok_or(SessionError::InvalidState)?;
            if peer_trust.state() != TrustState::Trusted {
                Err(SessionError::PeerNotTrusted)
            } else if peer_trust.owner_id() != context.owner_id()
                || peer_trust.device_id() != context.peer_device_id()
            {
                Err(SessionError::PeerTrustMismatch)
            } else if peer_trust.accepted_credential_epoch() != context.peer_credential_epoch() {
                Err(SessionError::PeerCredentialEpochMismatch)
            } else if peer_trust.trust_revision() != context.peer_trust_revision() {
                Err(SessionError::PeerTrustRevisionNotAdvanced)
            } else {
                Ok(())
            }
        };

        if let Err(error) = validation {
            self.close_after_currentness_failure();
            return Err(error);
        }

        Ok(())
    }

    pub fn revalidate_authority(
        &mut self,
        authority: &OwnerAuthorityState,
    ) -> Result<(), SessionError> {
        if self.state() != SessionState::Active {
            return Err(SessionError::InvalidState);
        }

        let validation = {
            let context = self.context().ok_or(SessionError::InvalidState)?;
            let root = authority.root();
            if root.owner_id() != context.owner_id()
                || root.root_key_id() != context.root_key_id()
                || root.root_epoch() != context.root_epoch()
            {
                Err(SessionError::OwnerAuthorityChanged)
            } else {
                match authority.current_delegation(AuthorityRole::DeviceSigning) {
                    Ok(delegation)
                        if delegation.delegated_key_id() == context.device_signing_key_id()
                            && delegation.delegation_epoch() == context.device_signing_epoch() =>
                    {
                        Ok(())
                    }
                    _ => Err(SessionError::DeviceSigningAuthorityChanged),
                }
            }
        };

        if let Err(error) = validation {
            self.close_after_currentness_failure();
            return Err(error);
        }

        Ok(())
    }

    fn close_after_currentness_failure(&mut self) {
        let _ = self.begin_close();
        let _ = self.finish_close();
    }
}
