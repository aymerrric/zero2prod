use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SuscriberName(String);

impl SuscriberName {
    pub fn parse(s: String) -> Result<SuscriberName, String> {
        let empty = s.trim().is_empty();
        let too_long = s.graphemes(true).count() > 256;
        let forbidden_chars = [
            '/', '(', ')', '{', '}', '%', '/', '\\', '"', '[', ']', '<', '>',
        ];
        let contains_forbidden_char = s.chars().any(|c| forbidden_chars.contains(&c));

        if empty || too_long || contains_forbidden_char {
            Err("The name input is ill formed ".to_string())
        } else {
            Ok(SuscriberName(s))
        }
    }
}

impl AsRef<str> for SuscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SuscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    pub fn return_suscribername() {
        let string = "grosse banane";
        assert_ok!(SuscriberName::parse(string.to_string()));
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SuscriberName::parse(name));
    }
    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SuscriberName::parse(name));
        }
    }
    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SuscriberName::parse(name));
    }
}
