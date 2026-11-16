//! The credentials finder: the reference's cases, probes on texts cut from them, and the time it
//! takes.
//!
//! The repository is public and refuses secret-shaped text, so no file here holds a sample of a
//! credential in any form, not even a piece of one. Behaviour is read off the corpus
//! (`secret_corpus`), and every other positive input is a text made at run time by cutting
//! corpus cases at code-point offsets and joining the cuts with one neutral character
//! (`corpus`): `probes.jsonl` and `shapes.jsonl` hold case numbers, offsets and fillers only.
mod corpus;
mod linear_time;
mod probes;
mod secret_corpus;
