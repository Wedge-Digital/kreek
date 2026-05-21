use std::collections::HashMap;
use axum::extract::{FromRef, FromRequestParts, Path};
use axum::http::request::Parts;
use axum::http::StatusCode;
use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;

/// Permissions de l'utilisateur courant sur l'espace courant.
///
/// À utiliser comme paramètre de handler sur les routes paramétrées par `{space_id}`.
/// Retourne 403 si l'utilisateur n'est pas membre de l'espace.
pub struct SpacePermissions {
    pub space_id: SpaceId,
    pub role: SpaceProfile,
}

impl SpacePermissions {
    pub fn is_admin(&self) -> bool {
        self.role == SpaceProfile::SpaceAdmin
    }

    pub fn can_report_match(&self) -> bool {
        matches!(self.role, SpaceProfile::SpaceAdmin)
    }
}

impl<S> FromRequestParts<S> for SpacePermissions
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_session = AuthSession::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let user = auth_session.user.ok_or(StatusCode::UNAUTHORIZED)?;

        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let raw_id = params.get("space_id").ok_or(StatusCode::BAD_REQUEST)?;
        let space_id = SpaceId::try_new(raw_id).map_err(|_| StatusCode::BAD_REQUEST)?;

        let role = app_state
            .spaces.space_repository
            .find_member_profile(&user.id, &space_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::FORBIDDEN)?;

        Ok(SpacePermissions { space_id, role })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::common_types::SpaceId;

    fn perms(role: SpaceProfile) -> SpacePermissions {
        SpacePermissions { space_id: SpaceId::new(), role }
    }

    #[test]
    fn admin_is_admin() {
        assert!(perms(SpaceProfile::SpaceAdmin).is_admin());
    }

    #[test]
    fn simple_user_is_not_admin() {
        assert!(!perms(SpaceProfile::SpaceUser).is_admin());
    }

    #[test]
    fn admin_can_report_match() {
        assert!(perms(SpaceProfile::SpaceAdmin).can_report_match());
    }

    #[test]
    fn simple_user_cannot_report_match() {
        assert!(!perms(SpaceProfile::SpaceUser).can_report_match());
    }
}