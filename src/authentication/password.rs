use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use zxcvbn::zxcvbn;

#[derive(Debug)]
pub struct Password(Secret<String>);

pub struct PasswordFeedback {
    pub score: u8,
    pub info: Option<String>,
}

impl Password {
    const MIN_LENGTH: usize = 12;
    const MAX_LENGTH: usize = 128;
    const MAX_SCORE: u8 = 4;
    const ACCEPTABLE_SCORE: u8 = 3;

    pub fn parse(password: &Secret<String>) -> Result<Password, anyhow::Error> {
        Self::check_length(password)?;

        let feedback = Self::password_feedback(password)?;

        match feedback.score {
            Self::ACCEPTABLE_SCORE..=Self::MAX_SCORE => Ok(Self(password.clone())),
            _ => Err(PasswordError::PasswordLowScore(feedback.info.unwrap()).into()),
        }
    }

    pub fn inner_ref(&self) -> &Secret<String> {
        &self.0
    }

    pub fn password_feedback(
        password_candidate: &Secret<String>,
    ) -> Result<PasswordFeedback, anyhow::Error> {
        match zxcvbn(password_candidate.expose_secret(), &[]) {
            Ok(entropy) => {
                let mut feedback = None;

                if entropy.score() < 3 {
                    feedback = Some(
                        entropy
                            .feedback()
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Failed to get a feedback reference"))?
                            .warning()
                            .unwrap()
                            .to_string(),
                    );
                }
                Ok(PasswordFeedback {
                    score: entropy.score(),
                    info: feedback,
                })
            }
            Err(err) => Err(PasswordError::UnexpectedError(err.into()).into()),
        }
    }

    fn check_length(password: &Secret<String>) -> Result<(), PasswordError> {
        match password.expose_secret().graphemes(true).count() {
            length if length < Self::MIN_LENGTH => Err(PasswordError::PasswordTooShort),
            Self::MAX_LENGTH.. => Err(PasswordError::PasswordTooLong),
            _ => Ok(()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PasswordError {
    #[error("Password should be at most 128 characters long.")]
    PasswordTooLong,
    #[error("Password should be at least 12 characters long.")]
    PasswordTooShort,
    #[error("{0}")]
    PasswordLowScore(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(name = "Validate credentials.", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=19456,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    };

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}
#[tracing::instrument(name = "Get stored credentials.", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(name = "Change password.", skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: Password,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(&password))
        .await?
        .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user password in the database.")?;

    Ok(())
}
#[tracing::instrument(
    name = "Verify password hash.",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

fn compute_password_hash(password: &Password) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, None)
            .context("Failed to build Argon parameters.")
            .map_err(AuthError::UnexpectedError)?,
    )
    .hash_password(password.inner_ref().expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}

#[cfg(test)]
mod tests {
    use crate::authentication::Password;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::Password as fake_pw_gn;
    use fake::Fake;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand::{self, Rng};
    use secrecy::{ExposeSecret, Secret};

    #[derive(Debug, Clone)]
    struct ValidPasswordFixture(pub Secret<String>);

    impl quickcheck::Arbitrary for ValidPasswordFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut seed = StdRng::seed_from_u64(u64::arbitrary(g));
            let password_length = seed.gen_range(12..128);
            let password = fake_pw_gn(password_length..128);
            ValidPasswordFixture(Secret::new(password.fake()))
        }
    }

    #[test]
    fn password_less_then_12_character_should_failed() {
        let password = Secret::new("Short pass".into());
        let parsed_passeord = Password::parse(&password);
        assert_err!(parsed_passeord);
    }

    #[test]
    fn password_more_then_128_character_should_failed() {
        let password = Secret::new("password12".repeat(13).into());
        let parsed_passeord = Password::parse(&password);
        assert_err!(parsed_passeord);
    }

    #[test]
    fn graphemes_password_() {
        let password = Secret::new("ぁ😤😠😡🤬🤯😳🥵🥶😱".repeat(12).into());
        let parsed_passeord = Password::parse(&password);
        assert_ok!(parsed_passeord);
    }

    #[test]
    fn password_with_a_low_entropy_score() {
        let password = Secret::new("password12345".into());
        let parsed_passeord = Password::parse(&password);
        assert_err!(parsed_passeord);
    }

    // takes a while to run
    #[quickcheck_macros::quickcheck]
    fn password_with_a_passible_entropy_score(password: ValidPasswordFixture) {
        dbg!(password.0.expose_secret());
        let parsed_passeord = Password::parse(&password.0);
        assert_ok!(parsed_passeord);
    }
}
