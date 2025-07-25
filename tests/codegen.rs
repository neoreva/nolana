macro_rules! test_codegen {
    ($name:ident, $source:literal, @$result:literal $(,)?) => {
        #[test]
        fn $name() {
            let ret = nolana::parser::Parser::new($source).parse();
            let out = nolana::codegen::Codegen::default().build(&ret.program);
            assert!(ret.errors.is_empty());
            assert!(!ret.panicked);
            insta::assert_snapshot!(out, @$result);
        }
    };
}

test_codegen!(boolean, "false; true;", @"false;true;");
test_codegen!(string, "'foo_bar123.-$#*()'", @"'foo_bar123.-$#*()'");
test_codegen!(
    variable,
    "variable.foo; v.foo; temp.foo; t.foo; context.foo; c.foo;",
    @"variable.foo;variable.foo;temp.foo;temp.foo;context.foo;context.foo;",
);
test_codegen!(
    weird_variable_members,
    "variable.v.temp.t.context.c.query.q.math.a.b.c",
    @"variable.v.temp.t.context.c.query.q.math.a.b.c",
);

test_codegen!(
    binary_and_unary_operations,
    "1 == (((2 != 3) < 4 <= 5 > 6) >= -7 + 8 - 9 * 10 / 11 || 12) && !(13 ?? 14)",
    @"1 == (((2 != 3) < 4 <= 5 > 6) >= -7 + 8 - 9 * 10 / 11 || 12) && !(13 ?? 14)",
);

test_codegen!(conditional, "q.foo ? 1", @"query.foo ? 1");

test_codegen!(ternary, "q.foo ? 1 : 0", @"query.foo ? 1 : 0");

test_codegen!(
    assignment,
    "v.cow.location = 16;",
    @"variable.cow.location = 16;",
);

test_codegen!(parenthesis_single, "((((16))))", @"((((16))))");
test_codegen!(parenthesis_complex, "(1; 2; (3; (4; 5;);););", @"(1;2;(3;(4;5;);););");

test_codegen!(block, "{v.a = 0;};", @"{variable.a = 0;};");

test_codegen!(
    resource,
    "geometry.foo; material.foo; texture.foo;",
    @"geometry.foo;material.foo;texture.foo;",
);

test_codegen!(array_access, "array.foo[q.bar]", @"array.foo[query.bar]");

test_codegen!(arrow_access, "v.foo->v.bar", @"variable.foo->variable.bar");

test_codegen!(
    r#loop,
    "loop(10, {v.i = v.i + 1;});",
    @"loop(10, {variable.i = variable.i + 1;});",
);

test_codegen!(
    for_each,
    "for_each(v.a, q.foo, {v.b = v.a + 1;});",
    @"for_each(variable.a, query.foo, {variable.b = variable.a + 1;});",
);

test_codegen!(
    keywords,
    "return v.a; break; continue; this;",
    @"return variable.a;break;continue;this;",
);
