use crosslab_protocol::{FeatureSet, FeatureNegotiationError, negotiate_features};

#[test]
fn optional_unknown_features_are_ignored() {
    let local = FeatureSet::new(&[1, 2], &[]).unwrap();
    let peer = FeatureSet::new(&[2, 99], &[]).unwrap();

    assert_eq!(negotiate_features(&local, &peer).unwrap(), vec![2]);
}

#[test]
fn required_features_must_be_supported_by_the_other_peer() {
    let local = FeatureSet::new(&[1, 2], &[2]).unwrap();
    let peer = FeatureSet::new(&[1], &[]).unwrap();

    assert_eq!(
        negotiate_features(&local, &peer).unwrap_err(),
        FeatureNegotiationError::UnsupportedRequiredFeature(2)
    );
}

#[test]
fn peer_required_features_are_checked_symmetrically() {
    let local = FeatureSet::new(&[1], &[]).unwrap();
    let peer = FeatureSet::new(&[1, 2], &[2]).unwrap();

    assert_eq!(
        negotiate_features(&local, &peer).unwrap_err(),
        FeatureNegotiationError::UnsupportedRequiredFeature(2)
    );
}

#[test]
fn feature_sets_are_bounded() {
    let supported = [1_u16; 65];
    let required = [1_u16; 33];

    assert_eq!(
        FeatureSet::new(&supported, &[]).unwrap_err(),
        FeatureNegotiationError::TooManySupportedFeatures
    );
    assert_eq!(
        FeatureSet::new(&[], &required).unwrap_err(),
        FeatureNegotiationError::TooManyRequiredFeatures
    );
}

#[test]
fn duplicate_feature_ids_are_normalized() {
    let local = FeatureSet::new(&[3, 1, 3, 2], &[2, 2]).unwrap();
    let peer = FeatureSet::new(&[2, 3], &[]).unwrap();

    assert_eq!(local.supported(), &[1, 2, 3]);
    assert_eq!(local.required(), &[2]);
    assert_eq!(negotiate_features(&local, &peer).unwrap(), vec![2, 3]);
}
