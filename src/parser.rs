use logos::{Lexer, Logos};

use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result},
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
///
/// [`program`]: ParseResult::program
/// [`errors`]: ParseResult::errors
#[derive(Debug)]
pub struct ParseResult<'src> {
    pub program: Program<'src>,
    pub errors: Vec<Diagnostic>,
}

/// Recursive Descent Parser for [Molang](https://bedrock.dev/docs/stable/Molang).
pub struct Parser<'src> {
    lexer: Lexer<'src, Kind>,
    source_code: &'src str,
    token: Token,
    prev_token_end: u32,
    is_complex: bool,
    function_depth: u8,
    errors: Vec<Diagnostic>,
}

impl<'src> Parser<'src> {
    /// Creates a new [`Parser`].
    pub fn new(source_code: &'src str) -> Self {
        Self {
            lexer: Logos::lexer(source_code),
            source_code,
            token: Token::default(),
            prev_token_end: 0,
            is_complex: false,
            function_depth: 0,
            errors: Vec::new(),
        }
    }

    /// Main entry point.
    ///
    /// See [`ParseResult`] for more info.
    pub fn parse(mut self) -> ParseResult<'src> {
        self.bump(); // First token.
        let program = match self.parse_program() {
            Ok(program) => program,
            Err(error) => {
                self.error(error);
                Program {
                    span: Span::default(),
                    source: self.source_code,
                    body: ProgramBody::Empty,
                }
            }
        };
        ParseResult { program, errors: self.errors }
    }

    fn parse_program(&mut self) -> Result<Program<'src>> {
        let span = self.start_span();
        let mut body = ProgramBody::Empty;
        while !self.at(Kind::Eof) {
            let stmt = self.parse_statement()?;
            if !self.parse_semi(&stmt) && self.is_complex {
                self.error(semi_required_in_complex(self.current_token().span()));
            }
            match &mut body {
                ProgramBody::Complex(stmts) => stmts.push(stmt),
                ProgramBody::Empty => {
                    body = match stmt {
                        Statement::Expression(expr) if !self.is_complex && self.at(Kind::Eof) => {
                            ProgramBody::Simple(*expr)
                        }
                        stmt => ProgramBody::Complex(vec![stmt]),
                    };
                }
                // Simple is only set when it's the end of the program, so this
                // is not possible to reach.
                ProgramBody::Simple(_) => unreachable!(),
            }
        }
        Ok(Program { span: self.end_span(span), source: self.source_code, body })
    }

    fn parse_statement(&mut self) -> Result<Statement<'src>> {
        let stmt = match self.current_kind() {
            Kind::Semi => self.parse_empty_statement()?,
            v if v.is_variable() => self.parse_assignment_statement_or_expression()?,
            Kind::Function => self.parse_function_statement()?,
            Kind::Loop => self.parse_loop_statement()?,
            Kind::ForEach => self.parse_for_each_statement()?,
            Kind::Return => self.parse_return_statement()?.into(),
            Kind::Break => self.parse_break_statement()?.into(),
            Kind::Continue => self.parse_continue_statement()?.into(),
            _ => self.parse_expression(0)?.into(),
        };
        Ok(stmt)
    }

    fn parse_semi(&mut self, stmt: &Statement<'src>) -> bool {
        if !stmt.is_empty() && self.eat(Kind::Semi) {
            self.is_complex = true;
            return true;
        }
        false
    }

    fn parse_assignment_statement_or_expression(&mut self) -> Result<Statement<'src>> {
        let span = self.start_span();
        let left = self.parse_variable_expression()?;
        let kind = self.current_kind();
        Ok(if kind.is_assignment_operator() {
            let operator = kind.into();
            self.bump();

            if !self.is_complex {
                self.is_complex = true;
            }

            let right = self.parse_expression(0)?;
            Statement::Assignment(
                AssignmentStatement { span: self.end_span(span), left, operator, right }.into(),
            )
        } else {
            Statement::Expression(
                self.parse_expression_rest(0, Expression::Variable(left.into()), span)?.into(),
            )
        })
    }

    fn parse_function_statement(&mut self) -> Result<Statement<'src>> {
        let span = self.start_span();
        self.expect(Kind::Function)?;
        self.expect(Kind::Dot)?;
        let name = self.parse_identifier()?;
        self.expect(Kind::Eq)?;
        self.expect(Kind::Function)?;
        self.expect(Kind::LeftParen)?;
        let mut parameters = Vec::new();
        loop {
            if self.at(Kind::LeftBrace) {
                break;
            }
            parameters.push(self.parse_literal_string()?);
            if self.eat(Kind::Comma) && self.at(Kind::LeftBrace) {
                break;
            }
        }
        self.enter_function();
        let body = self.parse_block_expression()?;
        self.exit_function();
        self.expect(Kind::RightParen)?;
        Ok(FunctionStatement {
            span: self.end_span(span),
            name,
            parameters: (!parameters.is_empty()).then_some(parameters),
            body,
        }
        .into())
    }

    fn parse_loop_statement(&mut self) -> Result<Statement<'src>> {
        let span = self.start_span();
        self.expect(Kind::Loop)?;
        self.expect(Kind::LeftParen)?;
        let count = self.parse_expression(0)?;
        self.expect(Kind::Comma)?;
        let block = self.parse_block_expression()?;
        self.expect(Kind::RightParen)?;
        Ok(LoopStatement { span: self.end_span(span), count, block }.into())
    }

    fn parse_for_each_statement(&mut self) -> Result<Statement<'src>> {
        let span = self.start_span();
        self.expect(Kind::ForEach)?;
        self.expect(Kind::LeftParen)?;
        if !self.current_kind().is_variable() {
            return Err(invalid_for_each_first_arg(self.current_token().span()));
        }
        let variable = self.parse_variable_expression()?;
        self.expect(Kind::Comma)?;
        let array = self.parse_expression(0)?;
        self.expect(Kind::Comma)?;
        let block = self.parse_block_expression()?;
        self.expect(Kind::RightParen)?;
        Ok(ForEachStatement { span: self.end_span(span), variable, array, block }.into())
    }

    fn parse_return_statement(&mut self) -> Result<ReturnStatement<'src>> {
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

    fn parse_empty_statement(&mut self) -> Result<Statement<'src>> {
        self.expect(Kind::Semi)?;
        Ok(EmptyStatement { span: self.end_span_single(self.current_token().span()) }.into())
    }

    fn parse_expression(&mut self, min_bp: u8) -> Result<Expression<'src>> {
        let span = self.start_span();
        let left = match self.current_kind() {
            Kind::True | Kind::False => self.parse_literal_boolean()?,
            Kind::Number => self.parse_literal_number()?,
            Kind::String => self.parse_literal_string().map(Into::into)?,
            v if v.is_variable() => self.parse_variable_expression().map(Into::into)?,
            Kind::LeftParen => self.parse_parenthesized_expression()?,
            Kind::LeftBrace => self.parse_block_expression().map(Into::into)?,
            v if v.is_unary_operator() => self.parse_unary_expression()?,
            v if v.is_call() => self.parse_call_expression()?,
            v if v.is_resource() => self.parse_resource_expression()?,
            Kind::Array => self.parse_array_access_expression()?,
            Kind::Loop | Kind::ForEach => {
                return Err(loop_in_expression(self.end_span_single(span)));
            }
            Kind::This => self.parse_this_expression()?,
            Kind::UnterminatedString => {
                return Err(unterminated_string(self.end_span(span)));
            }
            _ => return Err(unexpected_token(self.current_token().span())),
        };
        self.parse_expression_rest(min_bp, left, span)
    }

    fn parse_expression_rest(
        &mut self,
        min_bp: u8,
        mut left: Expression<'src>,
        span: Span,
    ) -> Result<Expression<'src>> {
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
                    Expression::Variable(variable) => {
                        left = self.parse_update_expression(span, *variable)?;
                    }
                    _ => return Err(illegal_update_operation(self.end_span(span))),
                },
                Kind::Question => {
                    left = self.parse_ternary_or_conditional_expression(span, left)?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_literal_number(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        let raw = self.current_src();
        self.expect(Kind::Number)?;
        let value = raw.parse::<f32>().map_err(|_| invalid_number(self.end_span(span)))?;
        Ok(NumericLiteral { span: self.end_span(span), value, raw }.into())
    }

    fn parse_literal_boolean(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        let value = match self.current_kind() {
            Kind::True => true,
            Kind::False => false,
            kind => unreachable!("Boolean Literal: {kind:?}"),
        };
        self.bump();
        Ok(BooleanLiteral { span: self.end_span(span), value }.into())
    }

    fn parse_literal_string(&mut self) -> Result<StringLiteral<'src>> {
        let span = self.start_span();
        let value = self.current_src();
        let value = &value[1..value.len() - 1];
        self.expect(Kind::String)?;
        Ok(StringLiteral { span: self.end_span(span), value })
    }

    #[inline(always)] // Hot path
    fn parse_identifier(&mut self) -> Result<Identifier<'src>> {
        let span = self.start_span();
        let name = self.current_src();
        match self.current_kind() {
            v if v.is_variable() | v.is_call() => self.bump(),
            _ => self.expect(Kind::Identifier)?,
        }
        Ok(Identifier { span: self.end_span(span), name: name.into() })
    }

    fn parse_parenthesized_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        self.expect(Kind::LeftParen)?;
        let first_stmt = self.parse_statement()?;
        if self.parse_semi(&first_stmt) {
            self.parse_parenthesized_expression_rest(first_stmt, span)
        } else if let Statement::Expression(expr) = first_stmt
            && self.eat(Kind::RightParen)
        {
            Ok(ParenthesizedExpression {
                span: self.end_span(span),
                body: ParenthesizedBody::Single(*expr),
            }
            .into())
        } else if self.eat(Kind::Eof) {
            Err(expected_token(
                Kind::RightParen.as_str(),
                self.current_kind().as_str(),
                Span::new(self.prev_token_end, self.current_token().start),
            ))
        } else {
            Err(unexpected_token(self.current_token().span()))
        }
    }

    fn parse_parenthesized_expression_rest(
        &mut self,
        first_statement: Statement<'src>,
        span: Span,
    ) -> Result<Expression<'src>> {
        let mut statements = vec![first_statement];
        loop {
            if self.at(Kind::RightParen) {
                break;
            }
            let stmt = self.parse_statement()?;
            if !self.parse_semi(&stmt) {
                self.error(semi_required_in_parenthesized(self.current_token().span()));
            }
            statements.push(stmt);
        }
        self.expect(Kind::RightParen)?;
        Ok(ParenthesizedExpression {
            span: self.end_span(span),
            body: ParenthesizedBody::Multiple(statements),
        }
        .into())
    }

    fn parse_block_expression(&mut self) -> Result<BlockExpression<'src>> {
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
            let stmt = self.parse_statement()?;
            if !self.parse_semi(&stmt) && self.is_complex {
                self.error(semi_required_in_block_expression(self.current_token().span()));
            }
            statements.push(stmt)
        }
        self.expect(Kind::RightBrace)?;
        Ok(BlockExpression { span: self.end_span(span), statements })
    }

    fn parse_binary_expression(
        &mut self,
        left_span: Span,
        left: Expression<'src>,
        rbp: u8,
    ) -> Result<Expression<'src>> {
        let operator = self.current_kind().into();
        self.bump();
        let right = self.parse_expression(rbp)?;
        Ok(BinaryExpression { span: self.end_span(left_span), left, operator, right }.into())
    }

    fn parse_unary_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        let operator = self.current_kind().into();
        self.bump();
        let argument = self.parse_expression(0)?;
        Ok(UnaryExpression { span: self.end_span(span), operator, argument }.into())
    }

    fn parse_ternary_or_conditional_expression(
        &mut self,
        test_span: Span,
        test: Expression<'src>,
    ) -> Result<Expression<'src>> {
        self.expect(Kind::Question)?;
        let consequent = self.parse_expression(0)?;
        if self.eat(Kind::Colon) {
            let alternate = self.parse_expression(0)?;
            Ok(TernaryExpression { span: self.end_span(test_span), test, consequent, alternate }
                .into())
        } else {
            Ok(ConditionalExpression { span: self.end_span(test_span), test, consequent }.into())
        }
    }

    fn parse_variable_expression(&mut self) -> Result<VariableExpression<'src>> {
        let span = self.start_span();
        let lifetime: VariableLifetime = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let property = self.parse_identifier()?;
        let mut member = VariableMember::Property { property };
        while self.eat(Kind::Dot) {
            let property = self.parse_identifier()?;
            member = VariableMember::Object { object: member.into(), property };
        }
        if lifetime == VariableLifetime::Parameter && !self.is_in_function() {
            return Err(function_variable_outside_function(self.end_span(span)));
        }
        Ok(VariableExpression { span: self.end_span(span), lifetime, member })
    }

    fn parse_update_expression(
        &mut self,
        span: Span,
        variable: VariableExpression<'src>,
    ) -> Result<Expression<'src>> {
        let operator = self.current_kind().into();
        self.bump();
        Ok(Expression::Update(
            UpdateExpression { span: self.end_span(span), variable, operator }.into(),
        ))
    }

    fn parse_resource_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        let section: ResourceSection = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let name = self.parse_identifier()?;
        Ok(ResourceExpression { span: self.end_span(span), section, name }.into())
    }

    fn parse_array_access_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        self.expect(Kind::Array)?;
        self.expect(Kind::Dot)?;
        let name = self.parse_identifier()?;
        self.expect(Kind::LeftBracket)?;
        let index = self.parse_expression(0)?;
        self.expect(Kind::RightBracket)?;
        Ok(ArrayAccessExpression { span: self.end_span(span), name, index }.into())
    }

    fn parse_arrow_access_expression(
        &mut self,
        left_span: Span,
        left: Expression<'src>,
    ) -> Result<Expression<'src>> {
        self.expect(Kind::Arrow)?;
        let right = self.parse_expression(0)?;
        Ok(ArrowAccessExpression { span: self.end_span(left_span), left, right }.into())
    }

    fn parse_call_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        let kind: CallKind = self.current_kind().into();
        self.bump();
        self.expect(Kind::Dot)?;
        let callee = self.parse_identifier()?;
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
        Ok(CallExpression { span: self.end_span(span), kind, callee, arguments }.into())
    }

    fn parse_this_expression(&mut self) -> Result<Expression<'src>> {
        let span = self.start_span();
        self.expect(Kind::This)?;
        Ok(ThisExpression { span: self.end_span(span) }.into())
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
    fn current_src(&self) -> &'src str {
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
    fn end_span_single(&self, mut span: Span) -> Span {
        span.end = span.start + 1;
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
            let curr_token = self.current_token();
            return Err(expected_token(kind.as_str(), curr_token.kind.as_str(), curr_token.span()));
        }
        Ok(())
    }

    fn error(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    #[inline]
    fn is_in_function(&self) -> bool {
        self.function_depth > 0
    }

    #[inline]
    fn enter_function(&mut self) {
        self.function_depth += 1;
    }

    #[inline]
    fn exit_function(&mut self) {
        debug_assert!(self.function_depth != 0, "exiting a function but depth is 0");
        self.function_depth -= 1;
    }
}

#[cold]
fn invalid_number(span: Span) -> Diagnostic {
    Diagnostic::error("invalid number").with_label(span)
}

#[cold]
fn unexpected_token(span: Span) -> Diagnostic {
    Diagnostic::error("unexpected token").with_label(span)
}

#[cold]
fn expected_token(expected: &str, found: &str, span: Span) -> Diagnostic {
    Diagnostic::error(format!("expected `{expected}` but found `{found}`")).with_label(span)
}

#[cold]
fn semi_required_in_complex(span: Span) -> Diagnostic {
    Diagnostic::error("semicolons are required for complex programs (containing `=` or `;`)")
        .with_help("try inserting a semicolon here")
        .with_label(span)
}

#[cold]
fn semi_required_in_parenthesized(span: Span) -> Diagnostic {
    Diagnostic::error("statements inside parenthesized expressions must be delimited by `;` if the other statements also end with `;`")
            .with_help("try inserting a semicolon here")
            .with_label(span)
}

#[cold]
fn semi_required_in_block_expression(span: Span) -> Diagnostic {
    Diagnostic::error("statements inside block expressions must be delimited by `;`")
        .with_help("try inserting a semicolon here")
        .with_label(span)
}

#[cold]
fn unterminated_string(span: Span) -> Diagnostic {
    Diagnostic::error("unterminated string").with_label(span)
}

#[cold]
fn loop_in_expression(span: Span) -> Diagnostic {
    Diagnostic::error("`loop` statement cannot be used inside expressions")
        .with_help("try defining it in a statement")
        .with_label(span)
}

#[cold]
fn illegal_update_operation(span: Span) -> Diagnostic {
    Diagnostic::error("`++` and `--` can only be used on variables").with_label(span)
}

#[cold]
fn invalid_for_each_first_arg(span: Span) -> Diagnostic {
    Diagnostic::error("`for_each` statement first argument must be a variable").with_label(span)
}

#[cold]
fn function_variable_outside_function(span: Span) -> Diagnostic {
    Diagnostic::error(
        "parameter and local variables may only be used inside the body of a function",
    )
    .with_label(span)
}
