use itertools::Itertools;
use paste::paste;
use serde::{Deserialize, Serialize};

use crate::{FatToken, Span, TokenKind};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

impl Token {
    pub fn new(span: Span, kind: TokenKind) -> Self {
        Self { span, kind }
    }

    /// Convert to an allocated [`FatToken`].
    pub fn to_fat(&self, source: &[char]) -> FatToken {
        let content = self.span.get_content(source).to_vec();

        FatToken {
            content,
            kind: self.kind,
        }
    }
}

macro_rules! create_decl_for {
    ($thing:ident) => {
        paste! {
            fn [< first_ $thing >](&self) -> Option<Token>;

            fn [< last_ $thing >](&self) -> Option<Token>;

            fn [< last_ $thing _index >](&self) -> Option<usize>;

            fn [<iter_ $thing _indices>](&self) -> impl Iterator<Item = usize> + '_;

            fn [<iter_ $thing s>](&self) -> impl Iterator<Item = Token> + '_;
        }
    };
}

macro_rules! create_fns_for {
    ($thing:ident) => {
        paste! {
            fn [< first_ $thing >](&self) -> Option<Token> {
                self.iter().find(|v| v.kind.[<is_ $thing>]()).copied()
            }

            fn [< last_ $thing >](&self) -> Option<Token> {
                self.iter().rev().find(|v| v.kind.[<is_ $thing>]()).copied()
            }

            fn [< last_ $thing _index >](&self) -> Option<usize> {
                self.iter().rev().position(|v| v.kind.[<is_ $thing>]()).map(|i| self.len() - i - 1)
            }

            fn [<iter_ $thing _indices>](&self) -> impl Iterator<Item = usize> + '_ {
                self.iter()
                    .enumerate()
                    .filter(|(_, t)| t.kind.[<is_ $thing>]())
                    .map(|(i, _)| i)
            }

            fn [<iter_ $thing s>](&self) -> impl Iterator<Item = Token> + '_ {
                self.[<iter_ $thing _indices>]().map(|i| self[i])
            }
        }
    };
}

pub trait TokenStringExt {
    fn first_sentence_word(&self) -> Option<Token>;
    fn first_non_whitespace(&self) -> Option<Token>;
    /// Grab the span that represents the beginning of the first element and the
    /// end of the last element.
    fn span(&self) -> Option<Span>;

    create_decl_for!(word);
    create_decl_for!(word_like);
    create_decl_for!(conjunction);
    create_decl_for!(space);
    create_decl_for!(apostrophe);
    create_decl_for!(pipe);
    create_decl_for!(quote);
    create_decl_for!(number);
    create_decl_for!(at);
    create_decl_for!(ellipsis);
    create_decl_for!(unlintable);
    create_decl_for!(sentence_terminator);
    create_decl_for!(paragraph_break);
    create_decl_for!(chunk_terminator);
    create_decl_for!(punctuation);
    create_decl_for!(currency);
    create_decl_for!(likely_homograph);

    fn iter_linking_verb_indices(&self) -> impl Iterator<Item = usize> + '_;
    fn iter_linking_verbs(&self) -> impl Iterator<Item = Token> + '_;

    /// Iterate over chunks.
    ///
    /// For example, the following sentence contains two chunks separated by a
    /// comma:
    ///
    /// ```text
    /// Here is an example, it is short.
    /// ```
    fn iter_chunks(&self) -> impl Iterator<Item = &'_ [Token]> + '_;

    /// Get an iterator over token slices that represent the individual
    /// paragraphs in a document.
    fn iter_paragraphs(&self) -> impl Iterator<Item = &'_ [Token]> + '_;

    /// Get an iterator over token slices that represent the individual
    /// sentences in a document.
    fn iter_sentences(&self) -> impl Iterator<Item = &'_ [Token]> + '_;
}

impl TokenStringExt for [Token] {
    create_fns_for!(word);
    create_fns_for!(word_like);
    create_fns_for!(conjunction);
    create_fns_for!(space);
    create_fns_for!(apostrophe);
    create_fns_for!(pipe);
    create_fns_for!(quote);
    create_fns_for!(number);
    create_fns_for!(at);
    create_fns_for!(punctuation);
    create_fns_for!(ellipsis);
    create_fns_for!(unlintable);
    create_fns_for!(sentence_terminator);
    create_fns_for!(paragraph_break);
    create_fns_for!(chunk_terminator);
    create_fns_for!(currency);
    create_fns_for!(likely_homograph);

    fn first_non_whitespace(&self) -> Option<Token> {
        self.iter().find(|t| !t.kind.is_whitespace()).copied()
    }

    fn first_sentence_word(&self) -> Option<Token> {
        let (w_idx, word) = self.iter().find_position(|v| v.kind.is_word())?;

        let Some(u_idx) = self.iter().position(|v| v.kind.is_unlintable()) else {
            return Some(*word);
        };

        if w_idx < u_idx {
            Some(*word)
        } else {
            None
        }
    }

    fn span(&self) -> Option<Span> {
        let min_max = self
            .iter()
            .flat_map(|v| [v.span.start, v.span.end].into_iter())
            .minmax();

        match min_max {
            itertools::MinMaxResult::NoElements => None,
            itertools::MinMaxResult::OneElement(min) => Some(Span::new(min, min)),
            itertools::MinMaxResult::MinMax(min, max) => Some(Span::new(min, max)),
        }
    }

    fn iter_linking_verb_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.iter_word_indices().filter(|idx| {
            let word = self[*idx];
            let TokenKind::Word(word) = word.kind else {
                panic!("Should be unreachable.");
            };

            word.is_linking_verb()
        })
    }

    fn iter_linking_verbs(&self) -> impl Iterator<Item = Token> + '_ {
        self.iter_linking_verb_indices().map(|idx| self[idx])
    }

    fn iter_chunks(&self) -> impl Iterator<Item = &'_ [Token]> + '_ {
        let first_chunk = self
            .iter_chunk_terminator_indices()
            .next()
            .map(|first_term| &self[0..=first_term]);

        let rest = self
            .iter_chunk_terminator_indices()
            .tuple_windows()
            .map(move |(a, b)| &self[a + 1..=b]);

        let last = if let Some(last_i) = self.last_chunk_terminator_index() {
            if last_i + 1 < self.len() {
                Some(&self[last_i + 1..])
            } else {
                None
            }
        } else {
            Some(self)
        };

        first_chunk.into_iter().chain(rest).chain(last)
    }

    fn iter_paragraphs(&self) -> impl Iterator<Item = &'_ [Token]> + '_ {
        let first_pg = self
            .iter_paragraph_break_indices()
            .next()
            .map(|first_term| &self[0..=first_term]);

        let rest = self
            .iter_paragraph_break_indices()
            .tuple_windows()
            .map(move |(a, b)| &self[a + 1..=b]);

        let last_pg = if let Some(last_i) = self.last_paragraph_break_index() {
            if last_i + 1 < self.len() {
                Some(&self[last_i + 1..])
            } else {
                None
            }
        } else {
            Some(self)
        };

        first_pg.into_iter().chain(rest).chain(last_pg)
    }

    fn iter_sentences(&self) -> impl Iterator<Item = &'_ [Token]> + '_ {
        let first_sentence = self
            .iter_sentence_terminator_indices()
            .next()
            .map(|first_term| &self[0..=first_term]);

        let rest = self
            .iter_sentence_terminator_indices()
            .tuple_windows()
            .map(move |(a, b)| &self[a + 1..=b]);

        let last_sentence = if let Some(last_i) = self.last_sentence_terminator_index() {
            if last_i + 1 < self.len() {
                Some(&self[last_i + 1..])
            } else {
                None
            }
        } else {
            Some(self)
        };

        first_sentence.into_iter().chain(rest).chain(last_sentence)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parsers::{Parser, PlainEnglish},
        TokenStringExt,
    };

    #[test]
    fn parses_sentences_correctly() {
        let text = "There were three little pigs. They built three little homes.";
        let chars: Vec<char> = text.chars().collect();
        let toks = PlainEnglish.parse(&chars);

        let mut sentence_strs = vec![];

        for sentence in toks.iter_sentences() {
            if let Some(span) = sentence.span() {
                sentence_strs.push(span.get_content_string(&chars));
            }
        }

        assert_eq!(
            sentence_strs,
            vec![
                "There were three little pigs.",
                " They built three little homes."
            ]
        )
    }
}
