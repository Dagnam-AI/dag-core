//! The rules the corpus's whole texts leave to their edges, probed on texts made at run time by
//! cutting corpus cases by code-point offsets and joining the cuts with one neutral character.
//!
//! The repository is public and refuses secret-shaped text, so this directory holds no sample of
//! a credential in any form, not even a piece: `probes.jsonl` names a case of
//! `corpus/credentials.jsonl` and offsets into it, and the single filler characters between the
//! cuts. Each line is one probe, `{"p": [[case, from, to], "x", ...], "s": [[start, end], ...]}`:
//! the parts, joined, are the text, and `s` is what the reference finds in it (code points).
//! The offsets and spans were computed once from the reference over the corpus and are numbers
//! only. A probe that names a case or an offset the corpus no longer has fails here, loudly.
use dagnam_hygiene::{find_secrets, to_char_offsets};
use serde::Deserialize;

use super::corpus::{Part, assemble};

/// One line of `probes.jsonl`.
#[derive(Deserialize)]
struct Probe {
    p: Vec<Part>,
    s: Vec<(usize, usize)>,
}

/// What the finder reports on `text`, as code points.
fn found(text: &str) -> Vec<(usize, usize)> {
    let bytes: Vec<usize> = find_secrets(text)
        .iter()
        .flat_map(|&(s, e)| [s, e])
        .collect();
    to_char_offsets(text, &bytes)
        .chunks(2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

#[test]
fn every_probe_finds_what_the_reference_finds() {
    let lines = include_str!("probes.jsonl");
    let mut failed = Vec::new();
    let mut count = 0;
    for (index, line) in lines.lines().enumerate() {
        let probe: Probe =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {}: {e}", index + 1));
        count += 1;
        if found(&assemble(&probe.p)) != probe.s {
            failed.push(index + 1);
        }
    }
    assert!(count > 0, "no probes");
    assert!(failed.is_empty(), "probes that differ: {failed:?}");
}
