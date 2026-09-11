//! Owner and device identity domain for Cross-Lab.

mod authority;
mod credential;
mod error;
mod ids;
mod root;
mod root_successor;

pub use authority::{AuthorityDelegation, AuthorityRole};
pub use credential::DeviceCredential;
pub use error::IdentityError;
pub use ids::{DeviceId, IdGenerationError, KeyId, OwnerId};
pub use root::OwnerRootRecord;
pub use root_successor::RootSuccessor;
