use std::ops::{Index, IndexMut};

use miette::{LabeledSpan, SourceOffset, SourceSpan};

pub const SPAN: Span = Span::new(0, 0);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn label(self, label: impl Into<String>) -> LabeledSpan {
        LabeledSpan::new_with_span(Some(label.into()), self)
    }

    const fn size(&self) -> u32 {
        debug_assert!(self.start <= self.end);
        self.end - self.start
    }
}

impl Index<Span> for str {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self[index.start as usize..index.end as usize]
    }
}

impl IndexMut<Span> for str {
    fn index_mut(&mut self, index: Span) -> &mut Self::Output {
        &mut self[index.start as usize..index.end as usize]
    }
}

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        Self::new(SourceOffset::from(span.start as usize), span.size() as usize)
    }
}

impl From<Span> for LabeledSpan {
    fn from(span: Span) -> Self {
        Self::underline(span)
    }
}
