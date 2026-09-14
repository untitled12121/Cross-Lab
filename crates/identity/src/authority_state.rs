use crate::{
    AuthorityDelegation, AuthorityRole, IdentityError, OwnerRootRecord, RootSuccessor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DelegatedRoleState {
    accepted_epoch: Option<u64>,
    active: Option<AuthorityDelegation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerAuthorityState {
    root: OwnerRootRecord,
    device_signing: DelegatedRoleState,
    administrative: DelegatedRoleState,
    recovery: DelegatedRoleState,
}

impl OwnerAuthorityState {
    pub fn new(root: OwnerRootRecord) -> Self {
        Self {
            root,
            device_signing: DelegatedRoleState::default(),
            administrative: DelegatedRoleState::default(),
            recovery: DelegatedRoleState::default(),
        }
    }

    pub const fn root(&self) -> &OwnerRootRecord {
        &self.root
    }

    pub fn current_delegation(
        &self,
        role: AuthorityRole,
    ) -> Result<&AuthorityDelegation, IdentityError> {
        self.role_state(role)?
            .active
            .as_ref()
            .ok_or(IdentityError::UnknownIssuer)
    }

    pub fn accepted_delegation_epoch(
        &self,
        role: AuthorityRole,
    ) -> Result<Option<u64>, IdentityError> {
        Ok(self.role_state(role)?.accepted_epoch)
    }

    pub fn accept_delegation(
        &mut self,
        delegation: AuthorityDelegation,
    ) -> Result<(), IdentityError> {
        let role = delegation.role();
        if role == AuthorityRole::OwnerRoot {
            return Err(IdentityError::WrongIssuerRole);
        }

        delegation.verify(&self.root, 0)?;

        let state = self.role_state_mut(role)?;
        if state
            .accepted_epoch
            .is_some_and(|epoch| delegation.delegation_epoch() <= epoch)
        {
            return Err(IdentityError::InvalidAuthorityEpoch);
        }

        state.accepted_epoch = Some(delegation.delegation_epoch());
        state.active = Some(delegation);
        Ok(())
    }

    pub fn accept_root_successor(
        &mut self,
        successor: &RootSuccessor,
    ) -> Result<(), IdentityError> {
        let next_root = successor.verify(&self.root)?;

        self.root = next_root;
        self.device_signing.active = None;
        self.administrative.active = None;
        self.recovery.active = None;
        Ok(())
    }

    fn role_state(&self, role: AuthorityRole) -> Result<&DelegatedRoleState, IdentityError> {
        match role {
            AuthorityRole::DeviceSigning => Ok(&self.device_signing),
            AuthorityRole::Administrative => Ok(&self.administrative),
            AuthorityRole::Recovery => Ok(&self.recovery),
            AuthorityRole::OwnerRoot => Err(IdentityError::WrongIssuerRole),
        }
    }

    fn role_state_mut(
        &mut self,
        role: AuthorityRole,
    ) -> Result<&mut DelegatedRoleState, IdentityError> {
        match role {
            AuthorityRole::DeviceSigning => Ok(&mut self.device_signing),
            AuthorityRole::Administrative => Ok(&mut self.administrative),
            AuthorityRole::Recovery => Ok(&mut self.recovery),
            AuthorityRole::OwnerRoot => Err(IdentityError::WrongIssuerRole),
        }
    }
}
