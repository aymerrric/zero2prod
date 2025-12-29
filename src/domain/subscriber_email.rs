use serde::Deserialize;
use validator::ValidateEmail;
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email.", s))
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
    use super::SubscriberEmail;
    use claim::assert_err;
    use fake::locales;
    use fake::locales::Data;
    use quickcheck::Gen;
    use quickcheck_macros::quickcheck;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[quickcheck]
    fn email_is_valid(s: ValidEmailFixture) -> bool {
        (SubscriberEmail::parse(s.0)).is_ok()
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut Gen) -> Self {
            let user_name = g
                .choose::<&str>(locales::EN::NAME_FIRST_NAME)
                .unwrap()
                .to_lowercase();
            let domains = ["com", "net", "fr", "org", "edu"];
            let domain = g.choose(&domains).unwrap();
            ValidEmailFixture(format!("{user_name}@example.{domain}"))
        }
    }

    impl AsRef<str> for ValidEmailFixture {
        fn as_ref(&self) -> &str {
            &self.0
        }
    }
}
