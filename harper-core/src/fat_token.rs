use serde::{Deserialize, Serialize};

use crate::TokenKind;

/// A [`Token`](crate::Token) that holds its content as a fat [`Vec<char>`] rather than as a
/// [`Span`](crate::Span).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FatToken {
    pub content: Vec<char>,
    pub kind: TokenKind,
}
