//! Texts made at run time out of the credentials corpus.
//!
//! The repository is public and refuses secret-shaped text, so the probe and timing tests write
//! no credential, nor a piece of one, themselves. A recipe names a case of
//! `corpus/credentials.jsonl` and offsets into it (code points), and single neutral characters
//! to put between the cuts; this module builds the text. A recipe is numbers and filler only.
use std::sync::OnceLock;

use serde::Deserialize;

use crate::{CredentialCase, cases};

/// One part of a recipe's text: `[case, from, to]`, a cut of the case's text; `[case, from, to,
/// times]`, the cut repeated; or one character.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Part {
    Cut(usize, usize, usize),
    Run(usize, usize, usize, usize),
    Fill(String),
}

/// The text of every case of `credentials.jsonl`, read on first use.
fn texts() -> &'static [String] {
    static TEXTS: OnceLock<Vec<String>> = OnceLock::new();
    TEXTS.get_or_init(|| {
        cases::<CredentialCase>("credentials")
            .iter()
            .map(CredentialCase::assembled)
            .collect()
    })
}

/// The code points `from..to` of case `case`'s `text`. An offset past the case's end is a
/// recipe that no longer fits the corpus, and fails instead of being clamped.
fn cut(case: usize, text: &str, from: usize, to: usize) -> String {
    let width = text.chars().count();
    assert!(
        from <= to && to <= width,
        "case {case}: the cut {from}..{to} does not fit its {width} characters"
    );
    text.chars().skip(from).take(to - from).collect()
}

/// The text the parts make, each cut taken by code points of its case.
pub(crate) fn assemble(parts: &[Part]) -> String {
    let texts = texts();
    parts
        .iter()
        .map(|part| match *part {
            Part::Cut(case, from, to) => cut(case, &texts[case], from, to),
            Part::Run(case, from, to, times) => cut(case, &texts[case], from, to).repeat(times),
            Part::Fill(ref fill) => {
                assert_eq!(fill.chars().count(), 1, "a filler is one character");
                fill.clone()
            }
        })
        .collect()
}
