use std::borrow::Cow;

use crate::{span::Span, token::Kind};

/// Represents the root of a Molang expression AST, containing all the top-level
/// information.
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'src> {
    pub span: Span,
    pub source: &'src str,
    pub body: ProgramBody<'src>,
}

/// A program is considered complex if it contains any statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramBody<'src> {
    Simple(Expression<'src>),
    Complex(Vec<Statement<'src>>),
    Empty,
}

impl ProgramBody<'_> {
    pub fn is_simple(&self) -> bool {
        matches!(self, ProgramBody::Simple(_))
    }

    pub fn is_complex(&self) -> bool {
        matches!(self, ProgramBody::Complex(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'src> {
    Expression(Box<Expression<'src>>),
    Assignment(Box<AssignmentStatement<'src>>),
    Function(Box<FunctionStatement<'src>>),
    Loop(Box<LoopStatement<'src>>),
    ForEach(Box<ForEachStatement<'src>>),
    Return(Box<ReturnStatement<'src>>),
    Break(Box<BreakStatement>),
    Continue(Box<ContinueStatement>),
    Empty(Box<EmptyStatement>),
}

impl Statement<'_> {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty(_))
    }
}

/// `v.a = 0;`
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStatement<'src> {
    pub span: Span,
    pub left: VariableExpression<'src>,
    pub operator: AssignmentOperator,
    pub right: Expression<'src>,
}

impl<'src> From<AssignmentStatement<'src>> for Statement<'src> {
    fn from(value: AssignmentStatement<'src>) -> Self {
        Self::Assignment(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentOperator {
    /// `=`
    Assign,
    /// `+=`
    Addition,
    /// `-=`
    Subtraction,
    /// `*=`
    Multiplication,
    /// `-=`
    Division,
    /// `**=`
    Exponential,
    /// `%=`
    Remainder,
    /// `||=`
    LogicalOr,
    /// `&&=`
    LogicalAnd,
    /// `<<=`
    ShiftLeft,
    /// `>>=`
    ShiftRight,
    /// `|=`
    BitwiseOr,
    /// `&=`
    BitwiseAnd,
    /// `^=`
    BitwiseXor,
}

impl AssignmentOperator {
    /// The string representation of this operator as it appears in source code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Addition => "+=",
            Self::Subtraction => "-=",
            Self::Multiplication => "*=",
            Self::Division => "/=",
            Self::Exponential => "**=",
            Self::Remainder => "%=",
            Self::LogicalOr => "||=",
            Self::LogicalAnd => "&&=",
            Self::ShiftLeft => "<<=",
            Self::ShiftRight => ">>=",
            Self::BitwiseOr => "|=",
            Self::BitwiseAnd => "&=",
            Self::BitwiseXor => "^=",
        }
    }

    pub fn is_custom(&self) -> bool {
        !matches!(self, Self::Assign)
    }
}

impl From<Kind> for AssignmentOperator {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Eq => Self::Assign,
            Kind::PlugEq => Self::Addition,
            Kind::MinusEq => Self::Subtraction,
            Kind::StarEq => Self::Multiplication,
            Kind::SlashEq => Self::Division,
            Kind::Star2Eq => Self::Exponential,
            Kind::PercentEq => Self::Remainder,
            Kind::Pipe2Eq => Self::LogicalOr,
            Kind::Amp2Eq => Self::LogicalAnd,
            Kind::ShiftLeftEq => Self::ShiftLeft,
            Kind::ShiftRightEq => Self::ShiftRight,
            Kind::PipeEq => Self::BitwiseOr,
            Kind::AmpEq => Self::BitwiseAnd,
            Kind::CaretEq => Self::BitwiseXor,
            _ => unreachable!("Assignment Operator: {kind:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionStatement<'src> {
    pub span: Span,
    pub name: Identifier<'src>,
    pub parameters: Option<Vec<StringLiteral<'src>>>,
    pub body: BlockExpression<'src>,
}

impl<'src> From<FunctionStatement<'src>> for Statement<'src> {
    fn from(value: FunctionStatement<'src>) -> Self {
        Self::Function(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#loop>
///
/// `loop(10, { v.x = v.x + 1; });`
#[derive(Debug, Clone, PartialEq)]
pub struct LoopStatement<'src> {
    pub span: Span,
    pub count: Expression<'src>,
    pub block: BlockExpression<'src>,
}

impl<'src> From<LoopStatement<'src>> for Statement<'src> {
    fn from(value: LoopStatement<'src>) -> Self {
        Self::Loop(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#for_each>
///
/// `for_each(t.foo, q.baz, { v.x = v.x + 1; });`
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachStatement<'src> {
    pub span: Span,
    pub variable: VariableExpression<'src>,
    pub array: Expression<'src>,
    pub block: BlockExpression<'src>,
}

impl<'src> From<ForEachStatement<'src>> for Statement<'src> {
    fn from(value: ForEachStatement<'src>) -> Self {
        Self::ForEach(value.into())
    }
}

/// `return` in `v.a = 1; return v.a;`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement<'src> {
    pub span: Span,
    pub argument: Expression<'src>,
}

impl<'src> From<ReturnStatement<'src>> for Statement<'src> {
    fn from(value: ReturnStatement<'src>) -> Self {
        Self::Return(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#break>
///
/// `break` in `loop(10, { v.x = v.x + 1; (v.x > 20) ? break; });`
#[derive(Debug, Clone, PartialEq)]
pub struct BreakStatement {
    pub span: Span,
}

impl From<BreakStatement> for Statement<'_> {
    fn from(value: BreakStatement) -> Self {
        Self::Break(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#continue>
///
/// `continue` in `loop(10, { (v.x > 5) ? continue; v.x = v.x + 1; });`
#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStatement {
    pub span: Span,
}

impl From<ContinueStatement> for Statement<'_> {
    fn from(value: ContinueStatement) -> Self {
        Self::Continue(value.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyStatement {
    pub span: Span,
}

impl From<EmptyStatement> for Statement<'_> {
    fn from(value: EmptyStatement) -> Self {
        Self::Empty(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Lexical%20Structure>
#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'src> {
    NumericLiteral(Box<NumericLiteral<'src>>),
    BooleanLiteral(Box<BooleanLiteral>),
    StringLiteral(Box<StringLiteral<'src>>),
    Variable(Box<VariableExpression<'src>>),
    Parenthesized(Box<ParenthesizedExpression<'src>>),
    Block(Box<BlockExpression<'src>>),
    Binary(Box<BinaryExpression<'src>>),
    Unary(Box<UnaryExpression<'src>>),
    Update(Box<UpdateExpression<'src>>),
    Ternary(Box<TernaryExpression<'src>>),
    Conditional(Box<ConditionalExpression<'src>>),
    Resource(Box<ResourceExpression<'src>>),
    ArrayAccess(Box<ArrayAccessExpression<'src>>),
    ArrowAccess(Box<ArrowAccessExpression<'src>>),
    Call(Box<CallExpression<'src>>),
    This(Box<ThisExpression>),
}

impl<'src> From<Expression<'src>> for Statement<'src> {
    fn from(value: Expression<'src>) -> Self {
        Self::Expression(value.into())
    }
}

/// `1.23` in `v.a = 1.23;`
#[derive(Debug, Clone, PartialEq)]
pub struct NumericLiteral<'src> {
    pub span: Span,
    pub value: f32,
    pub raw: &'src str,
}

impl<'src> From<NumericLiteral<'src>> for Expression<'src> {
    fn from(value: NumericLiteral<'src>) -> Self {
        Self::NumericLiteral(value.into())
    }
}

/// `true` or `false`
#[derive(Debug, Clone, PartialEq)]
pub struct BooleanLiteral {
    pub span: Span,
    pub value: bool,
}

impl From<BooleanLiteral> for Expression<'_> {
    fn from(value: BooleanLiteral) -> Self {
        Self::BooleanLiteral(value.into())
    }
}

impl BooleanLiteral {
    /// Returns `"true"` or `"false"` depending on this boolean's value.
    pub fn as_str(&self) -> &'static str {
        if self.value { "true" } else { "false" }
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Strings>
///
/// `'foo bar'` in `v.a = 'foo bar';`
#[derive(Debug, Clone, PartialEq)]
pub struct StringLiteral<'src> {
    pub span: Span,
    pub value: &'src str,
}

impl<'src> From<StringLiteral<'src>> for Expression<'src> {
    fn from(value: StringLiteral<'src>) -> Self {
        Self::StringLiteral(value.into())
    }
}

/// `foo` in `v.foo.bar`
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier<'src> {
    pub span: Span,
    pub name: Cow<'src, str>,
}

/// <https://bedrock.dev/docs/stable/Molang#Variables>
#[derive(Debug, Clone, PartialEq)]
pub struct VariableExpression<'src> {
    pub span: Span,
    pub lifetime: VariableLifetime,
    pub member: VariableMember<'src>,
}

impl VariableExpression<'_> {
    /// A struct is defined when an object or more are specified: `v.foo.bar`.
    pub fn is_struct(&self) -> bool {
        matches!(self.member, VariableMember::Object { .. })
    }
}

impl<'src> From<VariableExpression<'src>> for Expression<'src> {
    fn from(value: VariableExpression<'src>) -> Self {
        Self::Variable(value.into())
    }
}

/// The variable lifetime associated with [`VariableExpression`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VariableLifetime {
    /// `temp` in `temp.foo`
    Temporary,
    /// `variable` in `variable.foo`
    Variable,
    /// `context` in `context.foo`
    Context,
    /// `parameter` in `parameter.foo`
    Parameter,
}

impl VariableLifetime {
    pub fn as_str_long(&self) -> &'static str {
        match self {
            Self::Temporary => "temp",
            Self::Variable => "variable",
            Self::Context => "context",
            Self::Parameter => "parameter",
        }
    }

    pub fn as_str_short(&self) -> &'static str {
        match self {
            Self::Temporary => "t",
            Self::Variable => "v",
            Self::Context => "c",
            Self::Parameter => "p",
        }
    }
}

impl From<Kind> for VariableLifetime {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Temporary => Self::Temporary,
            Kind::Variable => Self::Variable,
            Kind::Context => Self::Context,
            Kind::Parameter => Self::Parameter,
            _ => unreachable!("Variable Lifetime: {kind:?}"),
        }
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Structs>
#[derive(Debug, Clone, PartialEq)]
pub enum VariableMember<'src> {
    /// `foo.bar` in `v.foo.bar`
    Object { object: Box<VariableMember<'src>>, property: Identifier<'src> },
    /// `foo` in `v.foo`
    Property { property: Identifier<'src> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParenthesizedExpression<'src> {
    pub span: Span,
    pub body: ParenthesizedBody<'src>,
}

impl<'src> From<ParenthesizedExpression<'src>> for Expression<'src> {
    fn from(value: ParenthesizedExpression<'src>) -> Self {
        Self::Parenthesized(value.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParenthesizedBody<'src> {
    /// `(1 + 1)` in `(1 + 1) * 2`
    Single(Expression<'src>),
    /// `(v.a = 1;)` in `(v.b = 'B'; v.a = 1;);`
    Multiple(Vec<Statement<'src>>),
}

/// `{ v.a = 0; }` in `loop(10, { v.a = 0; })`
#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpression<'src> {
    pub span: Span,
    pub statements: Vec<Statement<'src>>,
}

impl<'src> From<BlockExpression<'src>> for Expression<'src> {
    fn from(value: BlockExpression<'src>) -> Self {
        Self::Block(value.into())
    }
}

/// `1 + 1` in `v.a = 1 + 1;`
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpression<'src> {
    pub span: Span,
    pub left: Expression<'src>,
    pub operator: BinaryOperator,
    pub right: Expression<'src>,
}

impl<'src> From<BinaryExpression<'src>> for Expression<'src> {
    fn from(value: BinaryExpression<'src>) -> Self {
        Self::Binary(value.into())
    }
}

/// Operators used in [`BinaryExpression`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `==`
    Equality,
    /// `!=`
    Inequality,
    /// `<`
    LessThan,
    /// `<=`
    LessEqualThan,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterEqualThan,
    /// `+`
    Addition,
    /// `-`
    Subtraction,
    /// `*`
    Multiplication,
    /// `/`
    Division,
    /// `**`
    Exponential,
    /// `%`
    Remainder,
    /// `||`
    Or,
    /// `&&`
    And,
    /// `??`
    Coalesce,
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `|`
    BitwiseOr,
    /// `&`
    BitwiseAnd,
    /// `^`
    BitwiseXor,
}

impl BinaryOperator {
    /// The string representation of this operator as it appears in source code.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equality => "==",
            Self::Inequality => "!=",
            Self::LessThan => "<",
            Self::LessEqualThan => "<=",
            Self::GreaterThan => ">",
            Self::GreaterEqualThan => ">=",
            Self::Addition => "+",
            Self::Subtraction => "-",
            Self::Multiplication => "*",
            Self::Division => "/",
            Self::Or => "||",
            Self::And => "&&",
            Self::Coalesce => "??",
            Self::Exponential => "**",
            Self::Remainder => "%",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::BitwiseOr => "|",
            Self::BitwiseAnd => "&",
            Self::BitwiseXor => "^",
        }
    }

    pub fn is_custom(&self) -> bool {
        !matches!(
            self,
            Self::Equality
                | Self::Inequality
                | Self::LessThan
                | Self::LessEqualThan
                | Self::GreaterThan
                | Self::GreaterEqualThan
                | Self::Addition
                | Self::Subtraction
                | Self::Multiplication
                | Self::Division
                | Self::Or
                | Self::And
                | Self::Coalesce
        )
    }
}

impl From<Kind> for BinaryOperator {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Eq2 => Self::Equality,
            Kind::Neq => Self::Inequality,
            Kind::Lt => Self::LessThan,
            Kind::Gt => Self::GreaterThan,
            Kind::LtEq => Self::LessEqualThan,
            Kind::GtEq => Self::GreaterEqualThan,
            Kind::Pipe2 => Self::Or,
            Kind::Amp2 => Self::And,
            Kind::Question2 => Self::Coalesce,
            Kind::Minus => Self::Subtraction,
            Kind::Plus => Self::Addition,
            Kind::Star => Self::Multiplication,
            Kind::Slash => Self::Division,
            Kind::Star2 => Self::Exponential,
            Kind::Percent => Self::Remainder,
            Kind::ShiftLeft => Self::ShiftLeft,
            Kind::ShiftRight => Self::ShiftRight,
            Kind::Pipe => Self::BitwiseOr,
            Kind::Amp => Self::BitwiseAnd,
            Kind::Caret => Self::BitwiseXor,
            _ => unreachable!("Binary Operator: {kind:?}"),
        }
    }
}

impl From<AssignmentOperator> for BinaryOperator {
    fn from(op: AssignmentOperator) -> Self {
        match op {
            AssignmentOperator::Addition => Self::Addition,
            AssignmentOperator::Subtraction => Self::Subtraction,
            AssignmentOperator::Multiplication => Self::Multiplication,
            AssignmentOperator::Division => Self::Division,
            AssignmentOperator::Exponential => Self::Exponential,
            AssignmentOperator::Remainder => Self::Remainder,
            AssignmentOperator::ShiftLeft => Self::ShiftLeft,
            AssignmentOperator::ShiftRight => Self::ShiftRight,
            AssignmentOperator::BitwiseOr => Self::BitwiseOr,
            AssignmentOperator::BitwiseAnd => Self::BitwiseAnd,
            AssignmentOperator::BitwiseXor => Self::BitwiseXor,
            _ => unimplemented!("Binary Operator: {op:?}"),
        }
    }
}

/// `-1` in `q.foo(-1)`
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpression<'src> {
    pub span: Span,
    pub operator: UnaryOperator,
    pub argument: Expression<'src>,
}

impl<'src> From<UnaryExpression<'src>> for Expression<'src> {
    fn from(value: UnaryExpression<'src>) -> Self {
        Self::Unary(value.into())
    }
}

/// Operators used in [`UnaryExpression`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `-`
    Negate,
    /// `!`
    Not,
    /// `~`
    BitwiseNot,
}

impl UnaryOperator {
    /// The string representation of this operator as it appears in source code.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
            Self::BitwiseNot => "~",
        }
    }
}

impl From<Kind> for UnaryOperator {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Minus => Self::Negate,
            Kind::Bang => Self::Not,
            Kind::Tilde => Self::BitwiseNot,
            _ => unreachable!("Unary Operator: {kind:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateExpression<'src> {
    pub span: Span,
    pub variable: VariableExpression<'src>,
    pub operator: UpdateOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOperator {
    /// `++`
    Increment,
    /// `--`
    Decrement,
}

impl UpdateOperator {
    /// The string representation of this operator as it appears in source code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Increment => "++",
            Self::Decrement => "--",
        }
    }
}

impl From<Kind> for UpdateOperator {
    fn from(token: Kind) -> Self {
        match token {
            Kind::Plus2 => Self::Increment,
            Kind::Minus2 => Self::Decrement,
            _ => unreachable!("Update Operator: {token:?}"),
        }
    }
}

impl From<UpdateOperator> for BinaryOperator {
    fn from(op: UpdateOperator) -> Self {
        match op {
            UpdateOperator::Increment => BinaryOperator::Addition,
            UpdateOperator::Decrement => BinaryOperator::Subtraction,
        }
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Conditionals>
///
/// `q.foo ? 0 : 1`
#[derive(Debug, Clone, PartialEq)]
pub struct TernaryExpression<'src> {
    pub span: Span,
    pub test: Expression<'src>,
    pub consequent: Expression<'src>,
    pub alternate: Expression<'src>,
}

impl<'src> From<TernaryExpression<'src>> for Expression<'src> {
    fn from(value: TernaryExpression<'src>) -> Self {
        Self::Ternary(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Conditionals>
///
/// `q.foo ? 0`
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalExpression<'src> {
    pub span: Span,
    pub test: Expression<'src>,
    pub consequent: Expression<'src>,
}

impl<'src> From<ConditionalExpression<'src>> for Expression<'src> {
    fn from(value: ConditionalExpression<'src>) -> Self {
        Self::Conditional(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Resource%20Expression>
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceExpression<'src> {
    pub span: Span,
    pub section: ResourceSection,
    pub name: Identifier<'src>,
}

impl<'src> From<ResourceExpression<'src>> for Expression<'src> {
    fn from(value: ResourceExpression<'src>) -> Self {
        Self::Resource(value.into())
    }
}

/// The resource section in [`ResourceExpression`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSection {
    /// `geometry` in `geometry.foo`
    Geometry,
    /// `material` in `material.foo`
    Material,
    /// `texture` in `texture.foo`
    Texture,
}

impl ResourceSection {
    /// String representation of the resource section ("geometry", "material", or "texture").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Geometry => "geometry",
            Self::Material => "material",
            Self::Texture => "texture",
        }
    }
}

impl From<Kind> for ResourceSection {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Geometry => Self::Geometry,
            Kind::Material => Self::Material,
            Kind::Texture => Self::Texture,
            _ => unreachable!("Resource Section: {kind:?}"),
        }
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Array%20Expressions>
///
/// `array.foo[0]`
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayAccessExpression<'src> {
    pub span: Span,
    pub name: Identifier<'src>,
    pub index: Expression<'src>,
}

impl<'src> From<ArrayAccessExpression<'src>> for Expression<'src> {
    fn from(value: ArrayAccessExpression<'src>) -> Self {
        Self::ArrayAccess(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#-%3E%20%20Arrow%20Operator>
///
/// `v.foo->q.bar`
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowAccessExpression<'src> {
    pub span: Span,
    pub left: Expression<'src>,
    pub right: Expression<'src>,
}

impl<'src> From<ArrowAccessExpression<'src>> for Expression<'src> {
    fn from(value: ArrowAccessExpression<'src>) -> Self {
        Self::ArrowAccess(value.into())
    }
}

/// <https://bedrock.dev/docs/stable/Molang#Lexical%20Structure>
/// <https://bedrock.dev/docs/stable/Molang#Math%20Functions>
///
/// `math.random(1, 2)` or `math.random`
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpression<'src> {
    pub span: Span,
    pub kind: CallKind,
    pub callee: Identifier<'src>,
    pub arguments: Option<Vec<Expression<'src>>>,
}

impl<'src> From<CallExpression<'src>> for Expression<'src> {
    fn from(value: CallExpression<'src>) -> Self {
        Self::Call(value.into())
    }
}

/// The call kind for [`CallExpression`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// `math` in `math.foo`
    Math,
    /// `query` in `query.foo`
    Query,
    /// `function` in `function.foo`
    Function,
}

impl CallKind {
    pub fn as_str_long(&self) -> &'static str {
        match self {
            Self::Math => "math",
            Self::Query => "query",
            Self::Function => "function",
        }
    }

    pub fn as_str_short(&self) -> &'static str {
        match self {
            Self::Math => "math",
            Self::Query => "q",
            Self::Function => "f",
        }
    }
}

impl From<Kind> for CallKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Math => Self::Math,
            Kind::Query => Self::Query,
            Kind::Function => Self::Function,
            _ => unreachable!("Call Kind: {kind:?}"),
        }
    }
}

/// `this` in `q.foo(this)`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThisExpression {
    pub span: Span,
}

impl From<ThisExpression> for Expression<'_> {
    fn from(value: ThisExpression) -> Self {
        Self::This(value.into())
    }
}
