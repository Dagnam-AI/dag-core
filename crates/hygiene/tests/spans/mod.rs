//! The span resolver: `findings` against the reference's corpora, and the time it takes.
//!
//! No file here holds a credential in any form: the credential cases are texts the credentials
//! corpus assembles at run time, and failure messages carry case numbers, class codes and
//! lengths, never the text.
mod corpus;
mod linear_time;
