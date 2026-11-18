use crate::{Doc, Error, Span, Token};

use pyo3::prelude::*;
use pyo3::types::PyIterator;

use std::iter::FusedIterator;

fn next<'py>(it: &mut Option<Bound<'py, PyIterator>>) -> Option<Result<Bound<'py, PyAny>, Error>> {
    let item = it.as_mut()?.next();
    if !matches!(item, Some(Ok(_))) {
        *it = None;
    }
    item
}

fn size_hint(it: &Option<Bound<'_, PyIterator>>) -> (usize, Option<usize>) {
    if it.is_none() {
        (0, Some(0))
    } else {
        (0, None)
    }
}

/// A lazy, fallible iterator over document tokens or dependency children.
///
/// The Python iterator retains its backing data.
/// The first error is yielded once; subsequent calls return `None`.
/// Exhaustion is also permanent.
/// No Rust collection is materialized.
pub struct Tokens<'py> {
    inner: Option<Bound<'py, PyIterator>>,
}

impl<'py> Tokens<'py> {
    pub(crate) fn from_python(object: &Bound<'py, PyAny>) -> Result<Self, Error> {
        Ok(Self {
            inner: Some(object.try_iter()?),
        })
    }
}

impl<'py> Iterator for Tokens<'py> {
    type Item = Result<Token<'py>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        next(&mut self.inner).map(|item| item.map(Token::from_python))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        size_hint(&self.inner)
    }
}

impl FusedIterator for Tokens<'_> {}

/// A lazy, fallible iterator over sentence or entity spans.
///
/// The Python iterator retains its backing data.
/// Errors and exhaustion are terminal.
pub struct Spans<'py> {
    inner: Option<Bound<'py, PyIterator>>,
}

impl<'py> Spans<'py> {
    pub(crate) fn from_python(object: &Bound<'py, PyAny>) -> Result<Self, Error> {
        Ok(Self {
            inner: Some(object.try_iter()?),
        })
    }
}

impl<'py> Iterator for Spans<'py> {
    type Item = Result<Span<'py>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        next(&mut self.inner).map(|item| item.map(Span::from_python))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        size_hint(&self.inner)
    }
}

impl FusedIterator for Spans<'_> {}

/// A lazy, fallible iterator over documents produced by `Language.pipe()`.
///
/// The Python generator retains its pipeline and converted inputs.
/// Errors and exhaustion are terminal, and dropping this iterator drops its Python reference
/// without consuming the rest.
pub struct Docs<'py> {
    inner: Option<Bound<'py, PyIterator>>,
}

impl<'py> Docs<'py> {
    pub(crate) fn from_python(object: &Bound<'py, PyAny>) -> Result<Self, Error> {
        Ok(Self {
            inner: Some(object.try_iter()?),
        })
    }
}

impl<'py> Iterator for Docs<'py> {
    type Item = Result<Doc<'py>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        next(&mut self.inner).map(|item| item.map(Doc::from_python))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        size_hint(&self.inner)
    }
}

impl FusedIterator for Docs<'_> {}
