//! EU VAT numbers: a member state's prefix and its format (no per-state checksum).
use super::{Pattern, compile, group_span, not_ascii_alnum, scan};
use crate::text::char_before;
use crate::unicode::{UnicodeVersion, Versioned, digit_class};

/// Each member state's format after its prefix (`EL` is Greece), in the reference's order and
/// spelled as the reference spells it: `\d` is a decimal digit of any script.
const FORMATS: [(&str, &str); 27] = [
    ("AT", r"U\d{8}"),
    ("BE", r"[01]\d{9}"),
    ("BG", r"\d{9,10}"),
    ("CY", r"\d{8}[A-Z]"),
    ("CZ", r"\d{8,10}"),
    ("DE", r"\d{9}"),
    ("DK", r"\d{8}"),
    ("EE", r"\d{9}"),
    ("EL", r"\d{9}"),
    ("ES", r"[A-Z0-9]\d{7}[A-Z0-9]"),
    ("FI", r"\d{8}"),
    ("FR", r"[A-HJ-NP-Z0-9]{2}\d{9}"),
    ("HR", r"\d{11}"),
    ("HU", r"\d{8}"),
    ("IE", r"\d{7}[A-W][A-I]?|\d[A-Z+*]\d{5}[A-W]"),
    ("IT", r"\d{11}"),
    ("LT", r"\d{9}|\d{12}"),
    ("LU", r"\d{8}"),
    ("LV", r"\d{11}"),
    ("MT", r"\d{8}"),
    ("NL", r"\d{9}B\d{2}"),
    ("PL", r"\d{10}"),
    ("PT", r"\d{9}"),
    // Romania's CUI runs 2-10 digits; under 6 it is too short to tell from a code.
    ("RO", r"[1-9]\d{5,9}"),
    ("SE", r"\d{10}01"),
    ("SI", r"\d{8}"),
    ("SK", r"\d{10}"),
];

/// One alternative per state: the prefix, an optional space and the format; then a character that
/// is not an ASCII letter or digit (or the end), looked at and left for the next search, so the
/// shorter of two formats (`LT`) gives way to the longer (`docs/patterns.md`, P30).
static EU_VAT: Pattern = Versioned::new(|version| {
    let digit = digit_class(version);
    let states: Vec<String> = FORMATS
        .iter()
        .map(|(state, format)| format!("{state} ?(?:{})", format.replace(r"\d", digit)))
        .collect();
    compile(&format!("((?:{}))(?:\\z|[^A-Za-z0-9])", states.join("|")))
});

/// EU VAT numbers, by the member state's format.
#[must_use]
pub fn find_eu_vat(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    scan(EU_VAT.get(version), text, |start, caps| {
        group_span(caps, 1).filter(|_| not_ascii_alnum(char_before(text, start)))
    })
}

#[cfg(test)]
mod tests {
    use super::FORMATS;

    /// `\d` is the only backslash a format holds, so replacing it cannot touch an escaped
    /// backslash followed by a `d`, or any other literal.
    #[test]
    fn a_format_has_no_backslash_but_the_digit_class() {
        for (_, format) in FORMATS {
            assert!(!format.replace(r"\d", "").contains('\\'));
        }
    }
}
