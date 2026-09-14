use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, IdentityError, OwnerAuthorityState, OwnerId,
    OwnerRootRecord, RootSuccessor,
};

#[test]
fn first_observed_delegation_can_start_above_zero() {
    let owner_id = OwnerId::from_bytes([0x10; 32]);
    let root_key = SigningKey::from_secret_bytes([0x11; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegated_key = SigningKey::from_secret_bytes([0x12; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &delegated_key,
        7,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(delegation).unwrap();

    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(7)
    );
    assert_eq!(
        state
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
            .delegated_key_id(),
        delegation.delegated_key_id()
    );
}

#[test]
fn delegation_replacement_is_strictly_monotonic() {
    let owner_id = OwnerId::from_bytes([0x20; 32]);
    let root_key = SigningKey::from_secret_bytes([0x21; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let old_key = SigningKey::from_secret_bytes([0x22; 32]);
    let new_key = SigningKey::from_secret_bytes([0x23; 32]);
    let old = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &old_key,
        4,
        &root_key,
    );
    let new = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &new_key,
        5,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(old).unwrap();
    state.accept_delegation(new).unwrap();

    assert_eq!(
        state.accept_delegation(old),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(5)
    );
    assert_eq!(
        state
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
            .delegated_key_id(),
        new.delegated_key_id()
    );
}

#[test]
fn rejected_same_epoch_replacement_leaves_current_delegation_unchanged() {
    let owner_id = OwnerId::from_bytes([0x30; 32]);
    let root_key = SigningKey::from_secret_bytes([0x31; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let first_key = SigningKey::from_secret_bytes([0x32; 32]);
    let alternate_key = SigningKey::from_secret_bytes([0x33; 32]);
    let first = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &first_key,
        9,
        &root_key,
    );
    let alternate = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &alternate_key,
        9,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(first).unwrap();

    assert_eq!(
        state.accept_delegation(alternate),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
    assert_eq!(
        state
            .current_delegation(AuthorityRole::Administrative)
            .unwrap()
            .delegated_key_id(),
        first.delegated_key_id()
    );
}

#[test]
fn delegated_role_slots_advance_independently() {
    let owner_id = OwnerId::from_bytes([0x40; 32]);
    let root_key = SigningKey::from_secret_bytes([0x41; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let device_signing_key = SigningKey::from_secret_bytes([0x42; 32]);
    let administrative_key = SigningKey::from_secret_bytes([0x43; 32]);
    let device_signing = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        2,
        &root_key,
    );
    let administrative = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &administrative_key,
        11,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(device_signing).unwrap();
    state.accept_delegation(administrative).unwrap();

    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(2)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::Administrative)
            .unwrap(),
        Some(11)
    );
}

#[test]
fn wrong_owner_delegation_is_rejected_without_mutation() {
    let owner_id = OwnerId::from_bytes([0x44; 32]);
    let wrong_owner_id = OwnerId::from_bytes([0x45; 32]);
    let root_key = SigningKey::from_secret_bytes([0x46; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegated_key = SigningKey::from_secret_bytes([0x47; 32]);
    let wrong_owner = AuthorityDelegation::issue(
        wrong_owner_id,
        AuthorityRole::DeviceSigning,
        &delegated_key,
        1,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    assert_eq!(
        state.accept_delegation(wrong_owner),
        Err(IdentityError::WrongOwner)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        None
    );
    assert_eq!(
        state.current_delegation(AuthorityRole::DeviceSigning),
        Err(IdentityError::UnknownIssuer)
    );
}

#[test]
fn inactive_root_delegation_is_rejected_without_mutation() {
    let owner_id = OwnerId::from_bytes([0x48; 32]);
    let root_key = SigningKey::from_secret_bytes([0x49; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let inactive_root_key = SigningKey::from_secret_bytes([0x4a; 32]);
    let delegated_key = SigningKey::from_secret_bytes([0x4b; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::Administrative,
        &delegated_key,
        3,
        &inactive_root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    assert_eq!(
        state.accept_delegation(delegation),
        Err(IdentityError::UnknownIssuer)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::Administrative)
            .unwrap(),
        None
    );
}

#[test]
fn root_successor_clears_active_roles_but_keeps_epoch_floors() {
    let owner_id = OwnerId::from_bytes([0x50; 32]);
    let root_key = SigningKey::from_secret_bytes([0x51; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let old_device_signing_key = SigningKey::from_secret_bytes([0x52; 32]);
    let old_device_signing = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &old_device_signing_key,
        7,
        &root_key,
    );
    let next_root_key = SigningKey::from_secret_bytes([0x53; 32]);
    let successor = RootSuccessor::issue(&root, &root_key, &next_root_key).unwrap();

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(old_device_signing).unwrap();
    state.accept_root_successor(&successor).unwrap();

    assert_eq!(state.root().root_epoch(), 1);
    assert_eq!(
        state.current_delegation(AuthorityRole::DeviceSigning),
        Err(IdentityError::UnknownIssuer)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(7)
    );

    let stale_epoch_key = SigningKey::from_secret_bytes([0x54; 32]);
    let stale_epoch = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &stale_epoch_key,
        7,
        &next_root_key,
    );
    assert_eq!(
        state.accept_delegation(stale_epoch),
        Err(IdentityError::InvalidAuthorityEpoch)
    );

    let advanced_key = SigningKey::from_secret_bytes([0x55; 32]);
    let advanced = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &advanced_key,
        8,
        &next_root_key,
    );
    state.accept_delegation(advanced).unwrap();
    assert_eq!(
        state
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
            .delegated_key_id(),
        advanced.delegated_key_id()
    );
}

#[test]
fn failed_root_successor_leaves_root_and_active_delegation_unchanged() {
    let owner_id = OwnerId::from_bytes([0x56; 32]);
    let root_key = SigningKey::from_secret_bytes([0x57; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegated_key = SigningKey::from_secret_bytes([0x58; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &delegated_key,
        4,
        &root_key,
    );
    let foreign_root_key = SigningKey::from_secret_bytes([0x59; 32]);
    let foreign_root = OwnerRootRecord::new(owner_id, &foreign_root_key, 0);
    let foreign_next_key = SigningKey::from_secret_bytes([0x5a; 32]);
    let foreign_successor =
        RootSuccessor::issue(&foreign_root, &foreign_root_key, &foreign_next_key).unwrap();

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(delegation).unwrap();

    assert_eq!(
        state.accept_root_successor(&foreign_successor),
        Err(IdentityError::InvalidRootSuccessor)
    );
    assert_eq!(state.root().root_key_id(), root.root_key_id());
    assert_eq!(state.root().root_epoch(), 0);
    assert_eq!(
        state
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
            .delegated_key_id(),
        delegation.delegated_key_id()
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(4)
    );
}

#[test]
fn owner_root_is_not_a_delegated_role_slot() {
    let owner_id = OwnerId::from_bytes([0x60; 32]);
    let root_key = SigningKey::from_secret_bytes([0x61; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegated_key = SigningKey::from_secret_bytes([0x62; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::OwnerRoot,
        &delegated_key,
        1,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    assert_eq!(
        state.accept_delegation(delegation),
        Err(IdentityError::WrongIssuerRole)
    );
    assert_eq!(
        state.current_delegation(AuthorityRole::OwnerRoot),
        Err(IdentityError::WrongIssuerRole)
    );
    assert_eq!(
        state.accepted_delegation_epoch(AuthorityRole::OwnerRoot),
        Err(IdentityError::WrongIssuerRole)
    );
}
