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
    const SCOPE_ERR: &'static str = "at least one scope will always exist";

    pub fn compile(&mut self, program: &mut Program<'a>) {
        traverse(self, program);
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn exit_scope(&mut self) -> Scope<'a> {
        self.scopes.pop().expect(Self::SCOPE_ERR)
    }

    fn scope(&mut self) -> &mut Scope<'a> {
        self.scopes.last_mut().expect(Self::SCOPE_ERR)
    }

    /// `v.left ** v.right;` -> `math.pow(v.left, v.right);`
    /// `v.left % v.right;` -> `math.mod(v.left, v.right);`
    #[inline]
    fn compile_binary_expression(&self, expr: &mut Expression<'a>) {
        let Expression::Binary(bin_expr) = expr else { return };
        let name = match bin_expr.operator {
            BinaryOperator::Remainder => "mod",
            BinaryOperator::Exponential => "pow",
            _ => return,
        };
        let bin_expr = mem::take(bin_expr);
        let left = bin_expr.left;
        let right = bin_expr.right;
        *expr = Expression::Call(
            CallExpression {
                span: SPAN,
                kind: CallKind::Math,
                callee: IdentifierReference { span: SPAN, name },
                arguments: Some(vec![left, right]),
            }
            .into(),
        )
    }

    /// `v.left += v.right;` -> `v.left = v.left + v.right;`
    /// `v.left -= v.right;` -> `v.left = v.left - v.right;`
    /// `v.left *= v.right;` -> `v.left = v.left * v.right;`
    /// `v.left /= v.right;` -> `v.left = v.left / v.right;`
    /// `v.left **= v.right;` -> `v.left = math.pow(v.left, v.right);`
    /// `v.left %= v.right;` -> `v.left = math.mod(v.left, v.right);`
    #[inline]
    fn compile_assignment_statement(&self, stmt: &mut Statement<'a>) {
        enum MathOrOp {
            Math(&'static str),
            Op(BinaryOperator),
        }

        let Statement::Assignment(assign_stmt) = stmt else { return };
        let math_or_op = match assign_stmt.operator {
            AssignmentOperator::Remainder => MathOrOp::Math("mod"),
            AssignmentOperator::Exponential => MathOrOp::Math("pow"),
            AssignmentOperator::Addition => MathOrOp::Op(BinaryOperator::Addition),
            AssignmentOperator::Subtraction => MathOrOp::Op(BinaryOperator::Subtraction),
            AssignmentOperator::Multiplication => MathOrOp::Op(BinaryOperator::Multiplication),
            AssignmentOperator::Division => MathOrOp::Op(BinaryOperator::Division),
            AssignmentOperator::Assign => return,
        };
        assign_stmt.operator = AssignmentOperator::Assign;
        match math_or_op {
            MathOrOp::Math(name) => {
                let left = Expression::Variable(assign_stmt.left.clone().into());
                let right = mem::take(&mut assign_stmt.right);
                assign_stmt.right = Expression::Call(
                    CallExpression {
                        span: SPAN,
                        kind: CallKind::Math,
                        callee: IdentifierReference { span: SPAN, name },
                        arguments: Some(vec![left, right]),
                    }
                    .into(),
                );
            }
            MathOrOp::Op(bin_op) => {
                assign_stmt.right = Expression::Binary(
                    BinaryExpression {
                        span: SPAN,
                        left: Expression::Variable(assign_stmt.left.clone().into()),
                        operator: bin_op,
                        right: mem::take(&mut assign_stmt.right),
                    }
                    .into(),
                );
            }
        }
    }

    /// `v.i++` -> `v.i`
    fn compile_update_to_variable_expression(&mut self, expr: &mut Expression<'a>) {
        let Expression::Update(update_expr) = expr else { return };
        *expr = Expression::Variable(mem::take(&mut update_expr.variable).into());
    }

    /// `v.i++;` -> `v.i = v.i + 1;`
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

#[derive(Default)]
struct Scope<'a> {
    statement_count: usize,
    update_statements: Vec<(usize, Statement<'a>)>,
}
