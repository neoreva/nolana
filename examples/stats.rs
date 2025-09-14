use std::fs;

use nolana::{
    ast::{CallExpression, CallKind, Program},
    traverse::{Traverse, traverse},
    {ParseResult, Parser},
};

#[derive(Debug)]
struct MolangStats {
    pub math_functions: u32,
    pub queries: u32,
}

impl MolangStats {
    pub fn new(program: &mut Program) -> Self {
        let mut stats = Self { math_functions: 0, queries: 0 };
        traverse(&mut stats, program);
        stats
    }
}

impl<'a> Traverse<'a> for MolangStats {
    fn enter_call_expression(&mut self, it: &mut CallExpression<'a>) {
        match it.kind {
            CallKind::Math => self.math_functions += 1,
            CallKind::Query => self.queries += 1,
            CallKind::Function => (),
        }
    }
}

fn main() {
    let source_text = fs::read_to_string("examples/sample.molang").unwrap();

    let ParseResult { mut program, errors } = Parser::new(&source_text).parse();

    if !errors.is_empty() {
        for error in errors {
            let error = error.with_source_code(source_text.clone());
            print!("{error:?}");
        }
        return;
    }

    let molang_stats = MolangStats::new(&mut program);
    println!("{molang_stats:?}");
}
