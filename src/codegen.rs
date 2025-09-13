use std::iter;

use crate::ast::*;

pub struct CodegenOptions {
    pub minify: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self { minify: true }
    }
}

#[derive(Default)]
pub struct Codegen {
    options: CodegenOptions,
    code: String,
    is_complex: bool,
    indent: usize,
}

impl Codegen {
    pub fn build(mut self, program: &Program) -> String {
        self.code.reserve(program.source.len());
        self.is_complex = matches!(program.body, ProgramBody::Complex(_));
        program.print(&mut self);
        self.code
    }

    pub fn with_options(mut self, options: CodegenOptions) -> Self {
        self.options = options;
        self
    }

    #[inline]
    fn indent(&mut self) {
        self.indent += 1;
    }

    #[inline]
    fn dedent(&mut self) {
        self.indent -= 1;
    }

    #[inline]
    fn print_indent(&mut self) {
        if !self.options.minify && self.is_complex {
            self.code.extend(iter::repeat_n("    ", self.indent))
        }
    }

    #[inline]
    fn print_str(&mut self, s: &str) {
        self.code.push_str(s);
    }

    #[inline]
    fn print_char(&mut self, ch: char) {
        self.code.push(ch);
    }

    #[inline]
    fn print_newline(&mut self) {
        if !self.options.minify {
            self.code.push('\n');
        }
    }

    #[inline]
    fn print_space(&mut self) {
        if !self.options.minify {
            self.code.push(' ');
        }
    }

    #[inline]
    fn print_dot(&mut self) {
        self.code.push('.');
    }

    #[inline]
    fn print_comma(&mut self) {
        self.code.push(',');
    }

    #[inline]
    fn print_colon(&mut self) {
        self.code.push(':');
    }

    #[inline]
    fn print_semi(&mut self) {
        self.code.push(';');
    }

    fn print_list<T: Print>(&mut self, items: &[T]) {
        for (index, item) in items.iter().enumerate() {
            if index != 0 {
                self.print_comma();
                self.print_space();
            }
            item.print(self);
        }
    }

    #[inline]
    fn print_wrapped(&mut self, open: char, close: char, f: impl FnOnce(&mut Self)) {
        self.print_char(open);
        f(self);
        self.print_char(close);
    }

    fn print_scope(&mut self, open: char, close: char, f: impl FnOnce(&mut Self)) {
        self.print_wrapped(open, close, |c| {
            c.print_newline();
            c.indent();
            f(c);
            c.dedent();
            c.print_indent();
        });
    }
}

/// Generate code for an AST node.
trait Print {
    fn print(&self, c: &mut Codegen);
}

impl Print for Program<'_> {
    fn print(&self, c: &mut Codegen) {
        match &self.body {
            ProgramBody::Simple(expr) => expr.print(c),
            ProgramBody::Complex(stmts) => {
                for stmt in stmts {
                    stmt.print(c);
                }
            }
            ProgramBody::Empty => (),
        }
    }
}

impl Print for Statement<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_indent();
        match self {
            Statement::Expression(stmt) => stmt.print(c),
            Statement::Assignment(stmt) => stmt.print(c),
            Statement::Function(stmt) => stmt.print(c),
            Statement::Loop(stmt) => stmt.print(c),
            Statement::ForEach(stmt) => stmt.print(c),
            Statement::Return(stmt) => stmt.print(c),
            Statement::Break(stmt) => stmt.print(c),
            Statement::Continue(stmt) => stmt.print(c),
            Statement::Empty(stmt) => stmt.print(c),
        }
        if c.is_complex && !matches!(self, Statement::Empty(_)) {
            c.print_semi();
            c.print_newline();
        }
    }
}

impl Print for AssignmentStatement<'_> {
    fn print(&self, c: &mut Codegen) {
        self.left.print(c);
        c.print_space();
        self.operator.print(c);
        c.print_space();
        self.right.print(c);
    }
}

impl Print for AssignmentOperator {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.as_str());
    }
}

impl Print for FunctionStatement<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str("f.");
        self.name.print(c);
        c.print_space();
        c.print_char('=');
        c.print_space();
        c.print_str("function");
        c.print_wrapped('(', ')', |c| {
            if let Some(params) = &self.parameters {
                for param in params {
                    param.print(c);
                    c.print_comma();
                    c.print_space();
                }
            }
            self.body.print(c);
        });
    }
}

impl Print for LoopStatement<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str("loop");
        c.print_wrapped('(', ')', |c| {
            self.count.print(c);
            c.print_comma();
            c.print_space();
            self.block.print(c);
        });
    }
}

impl Print for ForEachStatement<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str("for_each");
        c.print_scope('(', ')', |c| {
            self.variable.print(c);
            c.print_comma();
            c.print_space();
            self.array.print(c);
            c.print_comma();
            c.print_space();
            self.block.print(c);
        });
    }
}

impl Print for ReturnStatement<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str("return ");
        self.argument.print(c);
    }
}

impl Print for BreakStatement {
    fn print(&self, c: &mut Codegen) {
        c.print_str("break");
    }
}

impl Print for ContinueStatement {
    fn print(&self, c: &mut Codegen) {
        c.print_str("continue");
    }
}

impl Print for EmptyStatement {
    fn print(&self, _: &mut Codegen) {}
}

impl Print for Expression<'_> {
    fn print(&self, c: &mut Codegen) {
        match self {
            Self::BooleanLiteral(expr) => expr.print(c),
            Self::NumericLiteral(expr) => expr.print(c),
            Self::StringLiteral(expr) => expr.print(c),
            Self::Variable(expr) => expr.print(c),
            Self::Parenthesized(expr) => expr.print(c),
            Self::Block(expr) => expr.print(c),
            Self::Binary(expr) => expr.print(c),
            Self::Unary(expr) => expr.print(c),
            Self::Ternary(expr) => expr.print(c),
            Self::Update(expr) => expr.print(c),
            Self::Conditional(expr) => expr.print(c),
            Self::Resource(expr) => expr.print(c),
            Self::ArrayAccess(expr) => expr.print(c),
            Self::ArrowAccess(expr) => expr.print(c),
            Self::Call(expr) => expr.print(c),
            Self::This(expr) => expr.print(c),
        }
    }
}

impl Print for Identifier<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str(&self.name);
    }
}

impl Print for NumericLiteral<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.raw);
    }
}

impl Print for BooleanLiteral {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.as_str());
    }
}

impl Print for StringLiteral<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_wrapped('\'', '\'', |c| c.print_str(self.value));
    }
}

impl Print for VariableExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.lifetime.print(c);
        c.print_dot();
        self.member.print(c);
    }
}

impl Print for VariableLifetime {
    fn print(&self, c: &mut Codegen) {
        c.print_str(if c.options.minify { self.as_str_short() } else { self.as_str_long() });
    }
}

impl Print for VariableMember<'_> {
    fn print(&self, c: &mut Codegen) {
        match self {
            Self::Object { object, property, .. } => {
                object.print(c);
                c.print_dot();
                property.print(c);
            }
            Self::Property { property, .. } => {
                property.print(c);
            }
        }
    }
}

impl Print for ParenthesizedExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.body.print(c);
    }
}

impl Print for ParenthesizedBody<'_> {
    fn print(&self, c: &mut Codegen) {
        match self {
            Self::Single(expression) => {
                c.print_wrapped('(', ')', |c| expression.print(c));
            }
            Self::Multiple(statements) => {
                c.print_scope('(', ')', |c| {
                    for stmt in statements {
                        stmt.print(c);
                    }
                });
            }
        }
    }
}

impl Print for BlockExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_scope('{', '}', |c| {
            for stmt in &self.statements {
                stmt.print(c);
            }
        });
    }
}

impl Print for BinaryExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.left.print(c);
        c.print_space();
        self.operator.print(c);
        c.print_space();
        self.right.print(c);
    }
}

impl Print for BinaryOperator {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.as_str());
    }
}

impl Print for UnaryExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.operator.print(c);
        self.argument.print(c);
    }
}

impl Print for UnaryOperator {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.as_str());
    }
}

impl Print for UpdateExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.variable.print(c);
        self.operator.print(c);
    }
}

impl Print for UpdateOperator {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.as_str());
    }
}

impl Print for TernaryExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.test.print(c);
        c.print_space();
        c.print_char('?');
        c.print_space();
        self.consequent.print(c);
        c.print_space();
        c.print_colon();
        c.print_space();
        self.alternate.print(c);
    }
}

impl Print for ConditionalExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.test.print(c);
        c.print_space();
        c.print_char('?');
        c.print_space();
        self.consequent.print(c);
    }
}

impl Print for ResourceExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str(self.section.as_str());
        c.print_dot();
        self.name.print(c);
    }
}

impl Print for ArrayAccessExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        c.print_str("array");
        c.print_dot();
        self.name.print(c);
        c.print_char('[');
        self.index.print(c);
        c.print_char(']');
    }
}

impl Print for ArrowAccessExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.left.print(c);
        c.print_str("->");
        self.right.print(c);
    }
}

impl Print for CallExpression<'_> {
    fn print(&self, c: &mut Codegen) {
        self.kind.print(c);
        c.print_dot();
        self.callee.print(c);
        if let Some(args) = &self.arguments {
            c.print_wrapped('(', ')', |c| c.print_list(args));
        }
    }
}

impl Print for CallKind {
    fn print(&self, c: &mut Codegen) {
        c.print_str(if c.options.minify { self.as_str_short() } else { self.as_str_long() });
    }
}

impl Print for ThisExpression {
    fn print(&self, c: &mut Codegen) {
        c.print_str("this");
    }
}
