pub async fn execute(
    cmd: RegisterNewSpaceCommand,
    repo: &dyn IUserRepository,
) -> Result<(), Vec<RegisterError>> {
    let mut errors: Vec<RegisterError> = Vec::new();

    // --- validation : tous les champs sont vérifiés sans court-circuit ---

    if cmd.password != cmd.password_confirm {
        errors.push(RegisterError::PasswordMismatch);
    }
    if cmd.password.len() < 8 {
        errors.push(RegisterError::PasswordTooShort);
    }

    let coach_name = match CoachName::try_new(&cmd.coach_name) {
        Ok(v)  => Some(v),
        Err(e) => { errors.push(RegisterError::InvalidCoachName(e.into())); None }
    };
    let email = match Email::try_new(&cmd.email) {
        Ok(v)  => Some(v),
        Err(e) => { errors.push(RegisterError::InvalidEmail(e.into())); None }
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    // --- à partir d'ici les valeurs sont garanties valides ---

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(cmd.password.as_bytes(), &salt)
        .map_err(|_| vec![RegisterError::PasswordHashError])?
        .to_string();

    let user = User::new(
        UserId::new(),
        coach_name.unwrap(),
        email.unwrap(),
        password_hash,
    );

    repo.create(&user).await.map_err(|e| vec![RegisterError::from(e)])
}
