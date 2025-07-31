use std::{borrow::Cow, fmt, ops};

use miette::{Diagnostic as MietteDiagnostic, LabeledSpan, Severity, SourceCode};

pub type Error = miette::Error;

/// Alias for a `Result` with the error type as [`Diagnostic`].
pub type Result<T> = std::result::Result<T, Diagnostic>;

/// Describes an error or warning that occurred during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    // Boxed to make `Diagnostic` 8 bytes so that `Result` is small.
    // This is due to rust not supporting return value optimization.
    // <https://users.rust-lang.org/t/does-rust-have-return-value-optimization/10389>
    inner: Box<DiagnosticInner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticInner {
    pub message: Cow<'static, str>,
    pub labels: Option<Vec<LabeledSpan>>,
    pub help: Option<Cow<'static, str>>,
    pub severity: Severity,
}

impl Diagnostic {
    /// Creates a new error-level [`Diagnostic`].
    pub fn error(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: Box::new(DiagnosticInner {
                message: message.into(),
                labels: None,
                help: None,
                severity: Severity::Error,
            }),
        }
    }

    /// Creates a new warning-level [`Diagnostic`].
    pub fn warning(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: Box::new(DiagnosticInner {
                message: message.into(),
                labels: None,
                help: None,
                severity: Severity::Warning,
            }),
        }
    }

    /// Sets a possible suggestion for a problem to the user.
    pub fn with_help(mut self, help: impl Into<Cow<'static, str>>) -> Self {
        self.inner.help = Some(help.into());
        self
    }

    /// Sets a label covering the problematic portion of the source code.
    ///
    /// Existing labels will be removed. Use [`Diagnostic::add_label`] to append
    /// labels instead.
    pub fn with_label(mut self, label: impl Into<LabeledSpan>) -> Self {
        self.inner.labels = Some(vec![label.into()]);
        self
    }

    /// Appends a label to this diagnostic without affecting previous ones.
    pub fn add_label(mut self, label: impl Into<LabeledSpan>) -> Self {
        let mut labels = self.inner.labels.unwrap_or_default();
        labels.push(label.into());
        self.inner.labels = Some(labels);
        self
    }

    /// Adds a source to this diagnostic and converts it into an [`Error`].
    pub fn with_source_code(self, code: impl SourceCode + 'static) -> Error {
        Error::from(self).with_source_code(code)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl std::error::Error for Diagnostic {}

impl ops::Deref for Diagnostic {
    type Target = DiagnosticInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl MietteDiagnostic for Diagnostic {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help.as_ref().map(Box::new).map(|help| help as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.labels
            .as_ref()
            .map(|labels| labels.iter().cloned())
            .map(Box::new)
            .map(|labels| labels as Box<dyn Iterator<Item = LabeledSpan>>)
    }
}

pub mod errors {
    use crate::span::Span;

    use super::*;

    #[cold]
    pub fn invalid_number(span: Span) -> Diagnostic {
        Diagnostic::error("Invalid number").with_label(span)
    }

    #[cold]
    pub fn for_each_wrong_first_arg(span: Span) -> Diagnostic {
        Diagnostic::error("`for_each` first argument must be either a `variable.` or a `temp.`")
            .with_label(span)
    }

    #[cold]
    pub fn unexpected_token(span: Span) -> Diagnostic {
        Diagnostic::error("Unexpected token").with_label(span)
    }

    #[cold]
    pub fn expected_token(expected: &str, found: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!("Expected `{expected}` but found `{found}`"))
            .with_label(span.label("Here"))
    }

    #[cold]
    pub fn semi_required(span: Span) -> Diagnostic {
        Diagnostic::error(
            "Semicolons are required for complex Molang expressions (contain `=` or `;`)",
        )
        .with_help("Try inserting a semicolon here")
        .with_label(span)
    }

    #[cold]
    pub fn semi_required_in_block_expression(span: Span) -> Diagnostic {
        Diagnostic::error("Expressions inside `{}` must be delimited by `;`")
            .with_help("Try inserting a semicolon here")
            .with_label(span)
    }

    #[cold]
    pub fn empty_block_expression(span: Span) -> Diagnostic {
        Diagnostic::error("Block expressions must contain at least one expression").with_label(span)
    }

    #[cold]
    pub fn illegal_string_operators(span: Span) -> Diagnostic {
        Diagnostic::error("Strings only support `==` and `!=` operators").with_label(span)
    }

    #[cold]
    pub fn break_outside_loop(span: Span) -> Diagnostic {
        Diagnostic::error("`break` is only supported inside `loop` and `for_each` expressions")
            .with_label(span)
    }

    #[cold]
    pub fn continue_outside_loop(span: Span) -> Diagnostic {
        Diagnostic::error("`continue` is only supported inside `loop` and `for_each` expressions")
            .with_label(span)
    }

    #[cold]
    pub fn assigning_context(span: Span) -> Diagnostic {
        Diagnostic::error("`context.` variables are read-only")
            .with_help("Try assigning to `variable.` or `temp.` instead")
            .with_label(span)
    }

    #[cold]
    pub fn unterminated_string(span: Span) -> Diagnostic {
        Diagnostic::error("Unterminated string").with_label(span)
    }

    #[cold]
    pub fn empty_parenthesized_expression(span: Span) -> Diagnostic {
        Diagnostic::error("Empty parenthesized expression").with_label(span)
    }

    #[cold]
    pub fn illegal_update_operation(span: Span) -> Diagnostic {
        Diagnostic::error("`++` and `--` can only be used on `variable.*` and `temp.*`")
            .with_label(span)
    }
}
