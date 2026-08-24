/// Path constants — use in Rust code: `path::LOGIN`, router definitions, `HX-Redirect` headers.
pub mod path {
    pub const AUTH_LAYOUT: &str = "/auth";
    pub const LOGIN: &str = "/auth/login";
    pub const LOGIN_SUCCESS: &str = "/auth/login/success";
    pub const REGISTER: &str = "/auth/register";
    pub const REGISTER_SUCCESS: &str = "/auth/register/success";
    pub const FORGOT_PASSWORD: &str = "/auth/forgot-password";
    pub const FORGOT_PASSWORD_SENT: &str = "/auth/forgot-password/sent";
    /// Même opération que `FORGOT_PASSWORD`, sans page ni redirection.
    pub const RESET_PASSWORD_REQUEST: &str = "/auth/password/request";
    /// Fragment de création de compte, destiné à être affiché par un hôte.
    pub const COACH_CREATION_WIDGET: &str = "/auth/widgets/coach-creation";
    // Route pattern for Axum — parameter in braces
    pub const LOGOUT: &str = "/auth/logout";
    pub const RESET_PASSWORD_PATTERN: &str = "/auth/password/update/{reset_token}";
    // Base used to build concrete URLs
    pub const RESET_PASSWORD_BASE: &str = "/auth/password/update";
}

/// Struct exposing route helpers to Askama templates.
///
/// Add `routes: Routes` to a template struct and call `{{ routes.login() }}` in the template.
/// `Routes` is `Copy + Default` — `..Default::default()` initialises it for free.
#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn auth_layout(&self) -> &'static str {
        path::AUTH_LAYOUT
    }
    pub fn login(&self) -> &'static str {
        path::LOGIN
    }
    pub fn logout(&self) -> &'static str {
        path::LOGOUT
    }
    pub fn login_success(&self) -> &'static str {
        path::LOGIN_SUCCESS
    }
    pub fn register(&self) -> &'static str {
        path::REGISTER
    }
    pub fn register_success(&self) -> &'static str {
        path::REGISTER_SUCCESS
    }
    pub fn forgot_password(&self) -> &'static str {
        path::FORGOT_PASSWORD
    }
    /// Demande d'envoi d'un lien de réinitialisation, pour un appelant qui
    /// gère lui-même son retour visuel. Rend 204.
    pub fn reset_password_request(&self) -> &'static str {
        path::RESET_PASSWORD_REQUEST
    }
    pub fn forgot_password_sent(&self) -> &'static str {
        path::FORGOT_PASSWORD_SENT
    }

    /// Builds the concrete password-reset URL for a given token.
    pub fn reset_password_form(&self, token: &str) -> String {
        format!("{}/{token}", path::RESET_PASSWORD_BASE)
    }
}
