//! `findings` on arbitrary text: it returns, and what it returns is well formed.
#![no_main]

use dagnam_hygiene::UnicodeVersion;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let chars = text.chars().count();
    for version in UnicodeVersion::ALL {
        let mut previous_end = 0;
        for (start, end, code) in dagnam_hygiene::findings(text, version) {
            // Code-point offsets: inside the text, never empty, sorted and never overlapping.
            assert!(start < end, "empty or inverted span");
            assert!(end <= chars, "span past the end of the text");
            assert!(start >= previous_end, "spans out of order or overlapping");
            assert!(!code.is_empty(), "span without a class code");
            previous_end = end;
        }
    }
});
