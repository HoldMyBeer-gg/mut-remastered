use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

/// Hash a password using Argon2id with a random salt.
///
/// Returns the PHC string format (`$argon2id$v=19$...`).
/// This function is synchronous and CPU-intensive — call it via
/// `tokio::task::spawn_blocking` from async contexts.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id v19, m=19456, t=2, p=1
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hashing failed: {}", e))?;
    Ok(password_hash.to_string())
}

/// Verify a plaintext password against a stored PHC hash string.
///
/// Returns `Ok(true)` if the password matches, `Ok(false)` if wrong.
/// This function is synchronous and CPU-intensive — call it via
/// `tokio::task::spawn_blocking` from async contexts.
pub fn verify_password(password: &str, phc_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(phc_hash)
        .map_err(|e| anyhow::anyhow!("invalid PHC hash: {}", e))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("password verification error: {}", e)),
    }
}
