use std::mem;

use crate::{
    ast::*,
    span::SPAN,
    traverse::{traverse, Traverse},
};

#[derive(Default)]
pub struct Compiler {}

impl Compiler {
    pub fn compile(&mut self, program: &mut Program<'_>) {
        traverse(self, program);
    }

    /// `v.left ** v.right;` -> `math.pow(v.left, v.right);`
    /// `v.left % v.right;` -> `math.mod(v.left, v.right);`
    #[inline]
    fn compile_binary_expression(&self, expr: &mut Expression<'_>) {
        let Expression::Binary(bin_expr) = expr else { unreachable!() };
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
    fn convert_assignment_statement(&self, stmt: &mut Statement<'_>) {
        enum MathOrOp {
            Math(&'static str),
            Op(BinaryOperator),
        }

        let Statement::Assignment(assign_stmt) = stmt else { unreachable!() };
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
}

impl Traverse<'_> for Compiler {
    fn enter_statement(&mut self, it: &mut Statement<'_>) {
        if let Statement::Assignment(_) = it {
            self.convert_assignment_statement(it);
        }
    }

    fn enter_expression(&mut self, it: &mut Expression<'_>) {
        if let Expression::Binary(_) = it {
            self.compile_binary_expression(it)
        }
    }
}
