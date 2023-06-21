use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(email: String) -> Result<SubscriberEmail, String> {
        if validate_email(&email) {
            Ok(Self(email))
        } else {
            Err(format!("{} is not a valid subscriber email.", email))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use claims::{assert_err, assert_ok};

    #[test]
    fn empty_email_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "John73_rdomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_local_part_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_domain_is_rejected() {
        let email = "John73_r@.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn a_valid_email_is_parsed_is_successfully() {
        let email = "John73_r@domain.com".to_string();
        assert_ok!(SubscriberEmail::parse(email));
    }
}
