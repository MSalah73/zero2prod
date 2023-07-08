use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(name: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > 256;
        let forbidden_characters = [
            '\\', '/', '<', '>', '{', '}', '"', '(', ')', '\'', ';', ':', '|',
        ];
        let contain_forbidden_characters = name
            .chars()
            .any(|char| forbidden_characters.contains(&char));
        if is_empty_or_whitespace || is_too_long || contain_forbidden_characters {
            Err(format!("{} is not a valid subscriber name.", name))
        } else {
            Ok(Self(name))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_graphmem_long_name_is_valid() {
        let name = "火".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphmem_is_rejected() {
        let name = "体".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_name_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_forbidden_characters_are_rejected() {
        for name in &[
            '\\', '/', '<', '>', '{', '}', '"', '(', ')', '\'', ';', ':', '|',
        ] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "John Marie Schmoe".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
