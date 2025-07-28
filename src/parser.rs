use logos::{Lexer, Logos};

use crate::{
    ast::*,
    diagnostic::{errors, Diagnostic, Result},
    span::Span,
    token::{Kind, Token},
};

/// Return value of [`Parser::parse`] which contains the AST and errors.
///
/// ## AST
///
/// [`program`] will always contain structurally valid AST, even if there
/// are syntax errors. However, the AST may be semantically invalid. To ensure it is valid:
///
/// 1. Check that [`errors`] is empty
/// 2. Analyze the AST semantically with [`SemanticChcker`][`crate::semantic::SemanticChecker`]
///
/// ## Errors
///
/// Nolana is able to recover from most syntax errors and continue parsing
/// anyway. When this happens:
/// 1. [`program`] will contain an AST
/// 2. [`errors`] will be non-empty
/// 3. [`panicked`] will be false
///
/// [`program`]: ParserReturn::program
/// [`errors`]: ParserReturn::errors
/// [`panicked`]: ParserReturn::panicked
#[derive(Debug)]
pub struct ParserReturn<'a> {
    pub program: Program<'a>,
    pub errors: Vec<Diagnostic>,
    pub panicked: bool,
}

/// Recursive Descent Parser for [Molang](https://bedrock.dev/docs/stable/Molang).
pub struct Parser<'a> {
    lexer: Lexer<'a, Kind>,
    source_code: &'a str,
    token: Token,
    prev_token_end: u32,
    /// An expression is considered a [`complex expression`] if there is at
    /// least one `=` or `;`.
    ///
    /// [`complex expression`]: https://bedrock.dev/docs/stable/Molang#Simple%20vs%20Complex%20Expressions
    is_complex: bool,
    errors: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    /// Creates a new [`Parser`].
    pub fn new(source_code: &'a str) -> Self {
        Self {
            lexer: Logos::lexer(source_code),
            source_code,
            token: Token::default(),
            prev_token_end: 0,
            is_complex: false,
            errors: Vec::new(),
        }
    }

    /// Main entry point.
    ///
    /// See [`ParserReturn`] for more info.
    pub fn parse(mut self) -> ParserReturn<'a> {
        self.bump(); // First token.
        let (program, panicked) = match self.parse_program() {
            Ok(program) => (program, false),
            Err(error) => {
                self.error(error);
                let program = Program {
                    span: Span::default(),
                    source: self.source_code,
                    is_complex: false,
                    body: Vec::new(),
                };
                (program, true)
            }
        };
        ParserReturn { program, errors: self.errors, panicked }
    }

    fn parse_program(&mut self) -> Result<Program<'a>> {
        let span = self.start_span();
        let mut body = Vec::new();
        while !self.at(Kind::Eof) {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
        }
        Ok(Program {
            span: self.end_span(span),
            source: self.source_code,
            is_complex: self.is_complex,
            body,
        })
    }

    fn parse_statement(&mut self) -> Result<Option<Statement<'a>>> {
        let stmt = match self.current_kind() {
            Kind::Semi => None, // We skip expressions that start with `;`.
            v if v.is_variable() => Some(self.parse_assignment_statement_or_expression()?),
            Kind::Return => Some(Statement::Return(self.parse_return_statement()?.into())),
            Kind::Break => Some(Statement::Break(self.parse_break_statement()?.into())),
            Kind::Continue => Some(Statement::Continue(self.parse_continue_statement()?.into())),
            _ => Some(Statement::Expression(self.parse_expression(0)?.into())),
        };
        if self.eat(Kind::Semi) {
            if !self.is_complex {
                self.is_complex = true;
            }
        } else if self.is_complex {
            self.error(errors::semi_required(self.current_token().span()));
        }
        Ok(stmt)
    }

    fn parse_assignment_statement_or_expression(&mut self) -> Result<Statement<'a>> {
        let span = self.start_span();
        let left = self.parse_variable_expression()?;
        let operator = match self.current_kind() {
            Kind::Eq => AssignmentOperator::Assign,
            Kind::PlugEq => AssignmentOperator::Addition,
            Kind::MinusEq => AssignmentOperator::Subtraction,
            Kind::StarEq => AssignmentOperator::Multiplication,
            Kind::SlashEq => AssignmentOperator::Division,
            Kind::Star2Eq => AssignmentOperator::Exponential,
            Kind::PercentEq => AssignmentOperator::Remainder,
            _ => {
                return Ok(Statement::Expression(
                    self.parse_expression_rest(0, Expression::Variable(left.into()), span)?.into(),
                ))
            }
        };
        self.bump();
        if !self.is_complex {
            self.is_complex = true;
        }
        let right = self.parse_expression(0)?;
        Ok(Statement::Assignment(
            AssignmentStatement { span: self.end_span(span), left, operator, right }.into(),
        ))
    }

    fn parse_return_statement(&mut self) -> Result<ReturnStatement<'a>> {
        let span = self.start_span();
        self.expect(Kind::Return)?;
        let argument = self.parse_expression(0)?;
        Ok(ReturnStatement { span: self.end_span(span), argument })
    }

    fn parse_break_statement(&mut self) -> Result<BreakStatement> {
        let span = self.start_span();
        self.expect(Kind::Break)?;
        Ok(BreakStatement { span: self.end_span(span) })
    }

    fn parse_continue_statement(&mut self) -> Result<ContinueStatement> {
        let span = self.start_span();
        self.expect(Kind::Continue)?;
        Ok(ContinueStatement { span: self.end_span(span) })
    }

    fn parse_expression(&mut self, min_bp: u8) -> Result<Expression<'a>> {
        let span = self.start_span();
        let left = match self.current_kind() {
            Kind::True | Kind::False => self.parse_literal_boolean()?,
            Kind::Number => self.parse_literal_number()?,
            Kind::String => self.parse_literal_string()?,
            v if v.is_variable() => {
                self.parse_variable_expression().map(|expr| Expression::Variable(expr.into()))?
            }
            Kind::LeftParen => self.parse_parenthesized_expression()?,
            Kind::LeftBrace => {
                self.parse_block_expression().map(|expr| Expression::Block(expr.into()))?
            }
            v if v.is_unary_operator() => self.parse_unary_expression()?,
            Kind::Query | Kind::Math => self.parse_call_expression()?,
            v if v.is_resource() => self.parse_resource_expression()?,
            Kind::Array => self.parse_array_access_expression()?,
            Kind::Loop => self.parse_loop_expression()?,
            Kind::ForEach => self.parse_for_each_expression()?,
            Kind::This => self.parse_this_expression()?,
            Kind::UnterminatedString => {
                return Err(errors::unterminated_string(self.end_span(span)))
            }
            _ => return Err(errors::unexpected_token(self.current_token().span())),
        };
        self.parse_expression_rest(min_bp, left, span)
    }

    fn parse_expression_rest(
        &mut self,
        min_bp: u8,
        mut left: Expression<'a>,
        span: Span,
    ) -> Result<Expression<'a>> {
        loop {
            let kind = self.current_kind();

            if kind == Kind::Arrow {
                left = self.parse_arrow_access_expression(span, left)?;
                break;
            }

            let Some((lbp, rbp)) = kind.binding_power() else {
                break;
            };
            if lbp < min_bp {
                break;
            }

            match self.current_kind() {
                kind if kind.is_binary_operator() => {
                    left = self.parse_binary_expression(span, left, rbp)?;
                }
                kind if kind.is_update_operator() => match left {
                    Expression::Variable(variable)
                        if variable.lifetime != VariableLifetime::Context =>
                    {
                        left = self.parse_update_expression(span, *variable)?;
                    }
                    _ => return Err(errors::illegal_update_operation(self.end_span(span))),
                },
                Kind::Question => {
                    left = self.parse_ternary_or_conditional_expression(span, left)?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_literal_number(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let raw = self.current_src();
        self.expect(Kind::Number)?;
        let value = raw.parse::<f32>().map_err(|_| errors::invalid_number(self.end_span(span)))?;
        Ok(Expression::NumericLiteral(
            NumericLiteral { span: self.end_span(span), value, raw }.into(),
        ))
    }

    fn parse_literal_boolean(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let value = match self.current_kind() {
            Kind::True => true,
            Kind::False => false,
            kind => unreachable!("Boolean Literal: {kind:?}"),
        };
        self.bump();
        Ok(Expression::BooleanLiteral(BooleanLiteral { span: self.end_span(span), value }.into()))
    }

    pub fn parse_literal_string(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let value = self.current_src();
        let value = &value[1..value.len() - 1];
        self.expect(Kind::String)?;
        Ok(Expression::StringLiteral(StringLiteral { span: self.end_span(span), value }.into()))
    }

    #[inline(always)] // Hot path
    fn parse_identifier_reference(&mut self) -> Result<IdentifierReference<'a>> {
        let span = self.start_span();
        let name = self.current_src();
        match self.current_kind() {
            v if v.is_variable() | v.is_call() => self.bump(),
            _ => self.expect(Kind::Identifier)?,
        }
        Ok(IdentifierReference { span: self.end_span(span), name })
    }

    fn parse_parenthesized_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        self.expect(Kind::LeftParen)?;
        if self.at(Kind::RightParen) {
            let span = self.start_span();
            self.bump();
            return Err(errors::empty_parenthesized_expression(self.end_span(span)));
        }
        let first_statement = match self.current_kind() {
            v if v.is_variable() => self.parse_assignment_statement_or_expression()?,
            Kind::Return => Statement::Return(self.parse_return_statement()?.into()),
            Kind::Break => Statement::Break(self.parse_break_statement()?.into()),
            Kind::Continue => Statement::Continue(self.parse_continue_statement()?.into()),
            _ => Statement::Expression(self.parse_expression(0)?.into()),
        };
        if let Statement::Expression(expression) = first_statement {
            match self.current_kind() {
                Kind::RightParen => {
                    self.bump();
                    Ok(Expression::Parenthesized(
                        ParenthesizedExpression::Single {
                            span: self.end_span(span),
                            expression: *expression,
                        }
                        .into(),
                    ))
                }
                Kind::Semi => self
                    .parse_parenthesized_expression_rest(Statement::Expression(expression), span),
                Kind::Eof => Err(errors::expected_token(
                    Kind::RightParen.as_str(),
                    self.current_kind().as_str(),
                    Span::new(self.prev_token_end, self.current_token().start),
                )),
                _ => Err(errors::unexpected_token(self.current_token().span())),
            }
        } else {
            self.parse_parenthesized_expression_rest(first_statement, span)
        }
    }

    fn parse_parenthesized_expression_rest(
        &mut self,
        first_statement: Statement<'a>,
        span: Span,
    ) -> Result<Expression<'a>> {
        let mut statements = vec![first_statement];
        loop {
            if self.at(Kind::RightParen) {
                break;
            }
            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }
        }
        self.expect(Kind::RightParen)?;
        Ok(Expression::Parenthesized(
            ParenthesizedExpression::Complex { span: self.end_span(span), statements }.into(),
        ))
    }

    fn parse_block_expression(&mut self) -> Result<BlockExpression<'a>> {
        // This deviates from Molang a little bit. However, because every
        // expression inside `{}` must be delimited with a `;`, it is grammatically
        // correct to do this early.
        if !self.is_complex {
            self.is_complex = true;
        }
        let span = self.start_span();
        self.expect(Kind::LeftBrace)?;
        let mut statements = Vec::new();
        while !self.at(Kind::RightBrace) {
            statements.push(match self.current_kind() {
                v if v.is_variable() => self.parse_assignment_statement_or_expression()?,
                Kind::Return => Statement::Return(self.parse_return_statement()?.into()),
                Kind::Break => Statement::Break(self.parse_break_statement()?.into()),
                Kind::Continue => Statement::Continue(self.parse_continue_statement()?.into()),
                _ => Statement::Expression(self.parse_expression(0)?.into()),
            });
            if !self.eat(Kind::Semi) {
                self.error(errors::semi_required_in_block_expression(self.current_token().span()))
            }
        }
        self.expect(Kind::RightBrace)?;
        Ok(BlockExpression { span: self.end_span(span), statements })
    }

    fn parse_binary_expression(
        &mut self,
        left_span: Span,
        left: Expression<'a>,
        rbp: u8,
    ) -> Result<Expression<'a>> {
        let operator = self.current_kind().into();
        self.bump();
        let right = self.parse_expression(rbp)?;
        Ok(Expression::Binary(
            BinaryExpression { span: self.end_span(left_span), left, operator, right }.into(),
        ))
    }

    fn parse_unary_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let operator = self.current_kind().into();
        self.bump();
        let argument = self.parse_expression(0)?;
        Ok(Expression::Unary(
            UnaryExpression { span: self.end_span(span), operator, argument }.into(),
        ))
    }

    fn parse_ternary_or_conditional_expression(
        &mut self,
        test_span: Span,
        test: Expression<'a>,
    ) -> Result<Expression<'a>> {
        self.expect(Kind::Question)?;
        let consequent = self.parse_expression(0)?;
        if self.eat(Kind::Colon) {
            let alternate = self.parse_expression(0)?;
            Ok(Expression::Ternary(
                TernaryExpression { span: self.end_span(test_span), test, consequent, alternate }
                    .into(),
            ))
        } else {
            Ok(Expression::Conditional(
                ConditionalExpression { span: self.end_span(test_span), test, consequent }.into(),
            ))
        }
    }

    fn parse_variable_expression(&mut self) -> Result<VariableExpression<'a>> {
        let span = self.start_span();
        let lifetime: VariableLifetime = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let property = self.parse_identifier_reference()?;
        let mut member = VariableMember::Property { span: self.end_span(span), property };
        while self.eat(Kind::Dot) {
            let property = self.parse_identifier_reference()?;
            member = VariableMember::Object {
                span: self.end_span(span),
                object: member.into(),
                property,
            };
        }
        Ok(VariableExpression { span: self.end_span(span), lifetime, member })
    }

    fn parse_update_expression(
        &mut self,
        span: Span,
        variable: VariableExpression<'a>,
    ) -> Result<Expression<'a>> {
        let operator = self.current_kind().into();
        self.bump();
        Ok(Expression::Update(
            UpdateExpression { span: self.end_span(span), variable, operator }.into(),
        ))
    }

    fn parse_resource_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let section: ResourceSection = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let name = self.parse_identifier_reference()?;
        Ok(Expression::Resource(
            ResourceExpression { span: self.end_span(span), section, name }.into(),
        ))
    }

    fn parse_array_access_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        self.expect(Kind::Array)?;
        self.expect(Kind::Dot)?;
        let name = self.parse_identifier_reference()?;
        self.expect(Kind::LeftBracket)?;
        let index = self.parse_expression(0)?;
        self.expect(Kind::RightBracket)?;
        Ok(Expression::ArrayAccess(
            ArrayAccessExpression { span: self.end_span(span), name, index }.into(),
        ))
    }

    fn parse_arrow_access_expression(
        &mut self,
        left_span: Span,
        left: Expression<'a>,
    ) -> Result<Expression<'a>> {
        self.expect(Kind::Arrow)?;
        let right = self.parse_expression(0)?;
        Ok(Expression::ArrowAccess(
            ArrowAccessExpression { span: self.end_span(left_span), left, right }.into(),
        ))
    }

    fn parse_call_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        let kind: CallKind = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let callee = self.parse_identifier_reference()?;
        let arguments = if self.eat(Kind::LeftParen) {
            let mut arguments = Vec::new();
            let mut first = true;
            loop {
                if self.at(Kind::RightParen) || self.at(Kind::Eof) {
                    break;
                }
                if first {
                    first = false;
                } else {
                    self.expect(Kind::Comma)?;
                    if self.at(Kind::RightParen) {
                        break;
                    }
                }
                arguments.push(self.parse_expression(0)?);
            }
            self.expect(Kind::RightParen)?;
            Some(arguments)
        } else {
            None
        };
        Ok(Expression::Call(
            CallExpression { span: self.end_span(span), kind, callee, arguments }.into(),
        ))
    }

    fn parse_loop_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        self.expect(Kind::Loop)?;
        self.expect(Kind::LeftParen)?;
        let count = self.parse_expression(0)?;
        self.expect(Kind::Comma)?;
        let block = self.parse_block_expression()?;
        self.expect(Kind::RightParen)?;
        Ok(Expression::Loop(LoopExpression { span: self.end_span(span), count, block }.into()))
    }

    fn parse_for_each_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        self.expect(Kind::ForEach)?;
        self.expect(Kind::LeftParen)?;
        if !(self.at(Kind::Variable) || self.at(Kind::Temporary)) {
            return Err(errors::for_each_wrong_first_arg(self.current_token().span()));
        }
        let variable = self.parse_variable_expression()?;
        self.expect(Kind::Comma)?;
        let array = self.parse_expression(0)?;
        self.expect(Kind::Comma)?;
        let block = self.parse_block_expression()?;
        self.expect(Kind::RightParen)?;
        Ok(Expression::ForEach(
            ForEachExpression { span: self.end_span(span), variable, array, block }.into(),
        ))
    }

    fn parse_this_expression(&mut self) -> Result<Expression<'a>> {
        let span = self.start_span();
        self.expect(Kind::This)?;
        Ok(Expression::This(ThisExpression { span: self.end_span(span) }.into()))
    }

    #[inline]
    fn current_token(&self) -> Token {
        self.token
    }

    #[inline]
    fn current_kind(&self) -> Kind {
        self.token.kind
    }

    #[inline]
    fn current_src(&self) -> &'a str {
        self.lexer.slice()
    }

    #[inline]
    fn start_span(&self) -> Span {
        Span::new(self.token.start, 0)
    }

    #[inline]
    fn end_span(&self, mut span: Span) -> Span {
        span.end = self.prev_token_end;
        debug_assert!(span.end >= span.start);
        span
    }

    #[inline]
    fn at(&self, kind: Kind) -> bool {
        self.current_kind() == kind
    }

    #[inline(always)] // Hot path
    fn bump(&mut self) {
        self.prev_token_end = self.token.end;
        let kind = self.lexer.next().unwrap_or(Ok(Kind::Eof)).unwrap_or(Kind::UnterminatedString);
        let span = self.lexer.span();
        self.token = Token { kind, start: span.start as u32, end: span.end as u32 };
    }

    #[inline]
    fn eat(&mut self, kind: Kind) -> bool {
        if self.at(kind) {
            self.bump();
            return true;
        }
        false
    }

    #[inline(always)] // Hot path
    fn expect(&mut self, kind: Kind) -> Result<()> {
        if !self.eat(kind) {
            return Err(self.expected_token(kind));
        }
        Ok(())
    }

    fn expected_token(&self, kind: Kind) -> Diagnostic {
        let curr_token = self.current_token();
        errors::expected_token(kind.as_str(), curr_token.kind.as_str(), curr_token.span())
    }

    fn error(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }
}
