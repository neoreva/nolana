use std::{fs, path::Path};

use insta::Settings;
use nolana::{Codegen, CodegenOptions, MolangTransformer, Parser, semantic::SemanticChecker};

fn with_settings(f: impl FnOnce()) {
    let mut settings = Settings::clone_current();
    settings.set_omit_expression(true);
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(f)
}

fn read_and_parse(path: &Path) -> String {
    let source = fs::read_to_string(path).unwrap();
    let result = Parser::new(&source).parse();
    format!("{result:#?}")
}

fn read_and_codegen(path: &Path) -> String {
    let source = fs::read_to_string(path).unwrap();
    let result = Parser::new(&source).parse();
    assert!(result.errors.is_empty());
    Codegen::default().build(&result.program)
}

fn read_and_semantic(path: &Path) -> String {
    let source = fs::read_to_string(path).unwrap();
    let mut result = Parser::new(&source).parse();
    let diagnostics = SemanticChecker::default().check(&mut result.program);
    format!("{diagnostics:#?}")
}

fn read_and_transform(path: &Path) -> String {
    let source = fs::read_to_string(path).unwrap();
    let mut result = Parser::new(&source).parse();
    MolangTransformer::new(&mut result.program).transform();
    Codegen::default().with_options(CodegenOptions { minify: false }).build(&result.program)
}

#[test]
fn test_parser() {
    with_settings(|| {
        insta::glob!("parser/*.nolana", |path| {
            insta::assert_snapshot!(read_and_parse(path));
        });
    });
}

#[test]
fn test_codegen() {
    with_settings(|| {
        insta::glob!("codegen/*.nolana", |path| {
            insta::assert_snapshot!(read_and_codegen(path));
        });
    });
}

#[test]
fn test_semantic() {
    with_settings(|| {
        insta::glob!("semantic/*.nolana", |path| {
            insta::assert_snapshot!(read_and_semantic(path));
        });
    });
}

#[test]
fn test_transformer() {
    with_settings(|| {
        insta::glob!("transformer/*.nolana", |path| {
            insta::assert_snapshot!(read_and_transform(path));
        });
    });
}
