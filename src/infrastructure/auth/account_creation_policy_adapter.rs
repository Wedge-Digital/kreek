//! Qui peut créer un compte coach — la réponse de l'hôte à `auth` (carte 573).
//!
//! `auth` est extractible : il ne connaît ni les espaces ni le compte
//! exploitant. Il déclare `IAccountCreationPolicy` ; kreek y répond ici, en
//! posant la question au service commun « qui est admin » — l'exception du
//! compte exploitant y vit déjà, et nulle part ailleurs.

use crate::app::auth::ports::IAccountCreationPolicy;
use crate::app::shared_kernel::bloodbowl::admin_access::IAdminAccessPort;
use crate::app::shared_kernel::identity::ids::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// Admin d'au moins un espace, ou le compte exploitant : ceux qui voient le
/// panneau de création aujourd'hui. La route n'a pas d'espace dans son chemin.
pub struct AccountCreationPolicyAdapter {
    admin_access: Arc<dyn IAdminAccessPort>,
}

impl AccountCreationPolicyAdapter {
    pub fn new(admin_access: Arc<dyn IAdminAccessPort>) -> Self {
        Self { admin_access }
    }
}

#[async_trait]
impl IAccountCreationPolicy for AccountCreationPolicyAdapter {
    async fn may_create_accounts(&self, user_id: &UserId) -> bool {
        self.admin_access.is_admin_of_any_space(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::ids::{EntityId, SpaceId};

    #[tokio::test]
    async fn a_space_admin_may_create_accounts() {
        let (coach, space): (UserId, SpaceId) = (EntityId::new(), EntityId::new());
        let policy = AccountCreationPolicyAdapter::new(Arc::new(
            FakeAdminAccess::new().space_admin(&coach, &space),
        ));
        assert!(policy.may_create_accounts(&coach).await);
    }

    #[tokio::test]
    async fn a_simple_member_may_not() {
        let policy = AccountCreationPolicyAdapter::new(Arc::new(FakeAdminAccess::new()));
        assert!(!policy.may_create_accounts(&EntityId::new()).await);
    }
}
