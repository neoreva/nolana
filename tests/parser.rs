macro_rules! test_parser {
    ($name:ident, $source:literal) => {
        #[test]
        fn $name() {
            let ret = nolana::parser::Parser::new($source).parse();
            insta::with_settings!({ omit_expression => true }, {
                insta::assert_debug_snapshot!(ret);
            });
        }
    };
}

test_parser!(boolean_false, "false");
test_parser!(boolean_true, "true");

test_parser!(string, "'foo_bar123.-$#*()'");
test_parser!(unterminated_string, "'hello wor-");

test_parser!(variable_variable, "variable.foo");
test_parser!(variable_v, "v.foo");
test_parser!(variable_temp, "temp.foo");
test_parser!(variable_t, "t.foo");
test_parser!(variable_context, "context.foo");
test_parser!(variable_c, "c.foo");
test_parser!(weird_variable_members, "variable.v.temp.t.context.c.query.q.math.a.b.c");

test_parser!(binary_operation, "1 + 2 * 3");
test_parser!(parenthesized_binary_operation, "(1 + 1) * (1 + 1)");
test_parser!(parenthesized_binary_operation_alt, "((2 * 3) + 1) / 2");
test_parser!(update_operation, "v.foo++ - 1; (v.bar-- / 2) * 2;");

test_parser!(negate_operation, "-(1 + 1)");
test_parser!(not_operation, "!(1 && 0)");

test_parser!(null_operation, "v.a ?? 1.2");

test_parser!(ternary_double_left, "q.foo ? v.bar == 13 ? 1 : 2 : 3");
test_parser!(ternary_double_right, "q.foo ? 1 : v.bar == 13 ? 2 : 3");

test_parser!(conditional, "q.foo ? 1");

test_parser!(
    assignment,
    "
    v.cow.a = 204.31;
    v.cow.b += 87;
    v.cow.c -= 48.933;
    v.cow.c *= 3233.23;
    v.cow.c /= 1290;
    v.cow.c **= 32.2;
    v.cow.c %= 32;
    "
);

test_parser!(complex_expression, "0; 0; 0;");

test_parser!(complex_parenthesized_expression, "(v.a = 1; v.b = 2;);");
test_parser!(empty_parenthesized_expression, "()");
test_parser!(nested_parenthesis, "((((16))))");

test_parser!(block, "{1;};");
test_parser!(block_undelimited, "{1}");

test_parser!(unclosed_parenthesis_in_call, "q.a(1");
test_parser!(unclosed_parenthesis_in_parenthesized_expression, "(1+1");

test_parser!(resource_geometry, "geometry.foo");
test_parser!(resource_material, "material.bar");
test_parser!(resource_texture, "texture.baz");

test_parser!(array_access, "array.foo[q.bar]");

test_parser!(arrow_access, "v.foo->v.bar");

test_parser!(r#loop, "loop(10, {v.i = v.i + 1;});");

test_parser!(for_each, "for_each(v.a, q.foo, {v.b = v.a + 1;});");
test_parser!(for_each_wrong_first_arg, "for_each(1, q.foo, {v.b = v.a + 1;});");

test_parser!(r#return, "return v.a");

test_parser!(r#break, "break");

test_parser!(r#continue, "continue");

test_parser!(this, "this");

test_parser!(missing_semi_with_semi, "0; 0");
test_parser!(missing_semi_with_assignment, "v.a = 0; v.a");
test_parser!(illegal_update_operation_with_query, "q.random()++");
test_parser!(illegal_update_operation_with_context, "context.foo++");

test_parser!(
    semisemisemisemi,
    "
    ;;;;;;; ;;;;;;; ;;;    ;;; ;;
    ;;      ;;      ;;;;  ;;;; ;;
    ;;;;;;; ;;;;;   ;; ;;;; ;; ;;
         ;; ;;      ;;  ;;  ;; ;;
    ;;;;;;; ;;;;;;; ;;      ;; ;;
    "
);
