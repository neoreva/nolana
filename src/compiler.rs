use std::mem;

use crate::{
    ast::*,
    span::SPAN,
    traverse::{traverse, Traverse},
};

#[derive(Default)]
pub struct Compiler<'a> {
    scopes: Vec<Scope<'a>>,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self, program: &mut Program<'a>) {
        traverse(self, program);
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn exit_scope(&mut self) -> Scope<'a> {
        self.scopes.pop().unwrap()
    }

    fn scope(&mut self) -> &mut Scope<'a> {
        self.scopes.last_mut().unwrap()
    }

    fn compile_binary_expression(&self, expr: &mut Expression<'a>) {
        let Expression::Binary(bin_expr) = expr else { return };
        if !matches!(
            bin_expr.operator,
            BinaryOperator::Remainder
                | BinaryOperator::Exponential
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
        ) {
            return;
        }

        let operator = bin_expr.operator;
        let bin_expr = mem::take(bin_expr);
        *expr = match operator {
            BinaryOperator::Remainder => math_mod_expression(bin_expr.left, bin_expr.right),
            BinaryOperator::Exponential => math_pow_expression(bin_expr.left, bin_expr.right),
            BinaryOperator::ShiftLeft => shift_left_expression(bin_expr.left, bin_expr.right),
            BinaryOperator::ShiftRight => shift_right_expression(bin_expr.left, bin_expr.right),
            _ => unreachable!(),
        };
    }

    fn compile_assignment_statement(&self, stmt: &mut Statement<'a>) {
        let Statement::Assignment(assign_stmt) = stmt else { return };
        if assign_stmt.operator == AssignmentOperator::Assign {
            return;
        }

        let operator = assign_stmt.operator;
        let left = Expression::Variable(assign_stmt.left.clone().into());
        let right = mem::take(&mut assign_stmt.right);
        assign_stmt.operator = AssignmentOperator::Assign;
        assign_stmt.right = match operator {
            AssignmentOperator::Remainder => math_mod_expression(left, right),
            AssignmentOperator::Exponential => math_pow_expression(left, right),
            AssignmentOperator::Addition => {
                basic_arithmetic_expression(left, BinaryOperator::Addition, right)
            }
            AssignmentOperator::Subtraction => {
                basic_arithmetic_expression(left, BinaryOperator::Subtraction, right)
            }
            AssignmentOperator::Multiplication => {
                basic_arithmetic_expression(left, BinaryOperator::Multiplication, right)
            }
            AssignmentOperator::Division => {
                basic_arithmetic_expression(left, BinaryOperator::Division, right)
            }
            AssignmentOperator::ShiftLeft => shift_left_expression(left, right),
            AssignmentOperator::ShiftRight => shift_right_expression(left, right),
            _ => unreachable!(),
        };
    }

    fn compile_update_to_variable_expression(&mut self, expr: &mut Expression<'a>) {
        let Expression::Update(update_expr) = expr else { return };
        *expr = Expression::Variable(mem::take(&mut update_expr.variable).into());
    }

    fn compile_update_expression_to_statement(&mut self, stmt: &mut Statement<'a>) {
        let (expr, has_side_effect) = match stmt {
            Statement::Expression(v) => (&mut **v, false),
            Statement::Assignment(v) => (&mut v.right, true),
            _ => return,
        };
        let Expression::Update(update_expr) = expr else { return };

        let new_stmt = Statement::Assignment(
            AssignmentStatement {
                span: SPAN,
                left: update_expr.variable.clone(),
                operator: AssignmentOperator::Assign,
                right: Expression::Binary(
                    BinaryExpression {
                        span: SPAN,
                        left: Expression::Variable(update_expr.variable.clone().into()),
                        operator: match update_expr.operator {
                            UpdateOperator::Increment => BinaryOperator::Addition,
                            UpdateOperator::Decrement => BinaryOperator::Subtraction,
                        },
                        right: Expression::NumericLiteral(
                            NumericLiteral { span: SPAN, value: 1.0, raw: "1" }.into(),
                        ),
                    }
                    .into(),
                ),
            }
            .into(),
        );

        if has_side_effect {
            let scope = self.scope();
            let index = scope.update_statements.len() + scope.statement_count - 1;
            scope.update_statements.push((index, new_stmt));
        } else {
            *stmt = new_stmt;
        }
    }
}

impl<'a> Traverse<'a> for Compiler<'a> {
    fn enter_statements(&mut self, _: &mut Vec<Statement<'a>>) {
        self.enter_scope();
    }

    fn exit_statements(&mut self, it: &mut Vec<Statement<'a>>) {
        let scope = self.exit_scope();
        for (index, stmt) in scope.update_statements {
            it.insert(index, stmt);
        }
    }

    fn enter_statement(&mut self, it: &mut Statement<'a>) {
        self.scope().statement_count += 1;

        self.compile_update_expression_to_statement(it);
        self.compile_assignment_statement(it);
    }

    fn enter_expression(&mut self, it: &mut Expression<'a>) {
        self.compile_update_to_variable_expression(it);
        self.compile_binary_expression(it)
    }
}

/// Contextual info about the current scope.
///
/// Mainly stores extra statements to be added to the statement list upon
/// exiting the scope.
#[derive(Default)]
struct Scope<'a> {
    statement_count: usize,
    update_statements: Vec<(usize, Statement<'a>)>,
}

#[inline]
fn basic_arithmetic_expression<'a>(
    left: Expression<'a>,
    operator: BinaryOperator,
    right: Expression<'a>,
) -> Expression<'a> {
    Expression::Binary(BinaryExpression { span: SPAN, left, operator, right }.into())
}

/// `v.x * math.pow(2, math.y)`
#[inline]
fn shift_left_expression<'a>(left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
    BinaryExpression {
        span: SPAN,
        left,
        operator: BinaryOperator::Multiplication,
        right: math_pow_expression(
            NumericLiteral { span: SPAN, value: 2.0, raw: "2" }.into(),
            right,
        ),
    }
    .into()
}

/// `math.floor(v.x / math.pow(2, math.y))`
#[inline]
fn shift_right_expression<'a>(left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
    math_floor_expression(
        BinaryExpression {
            span: SPAN,
            left,
            operator: BinaryOperator::Division,
            right: math_pow_expression(
                NumericLiteral { span: SPAN, value: 2.0, raw: "2" }.into(),
                right,
            ),
        }
        .into(),
    )
}

#[inline]
fn math_pow_expression<'a>(left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
    CallExpression {
        span: SPAN,
        kind: CallKind::Math,
        callee: IdentifierReference { span: SPAN, name: "pow" },
        arguments: Some(vec![left, right]),
    }
    .into()
}

#[inline]
fn math_mod_expression<'a>(left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
    CallExpression {
        span: SPAN,
        kind: CallKind::Math,
        callee: IdentifierReference { span: SPAN, name: "mod" },
        arguments: Some(vec![left, right]),
    }
    .into()
}

#[inline]
fn math_floor_expression<'a>(x: Expression<'a>) -> Expression<'a> {
    CallExpression {
        span: SPAN,
        kind: CallKind::Math,
        callee: IdentifierReference { span: SPAN, name: "floor" },
        arguments: Some(vec![x]),
    }
    .into()
}
