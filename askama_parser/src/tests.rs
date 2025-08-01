use winnow::Parser;

use crate::node::{Lit, Raw, Whitespace, Ws};
use crate::{
    Ast, Expr, Filter, InnerSyntax, Node, Num, PathComponent, PathOrIdentifier, Span, StrLit,
    Syntax, SyntaxBuilder, WithSpan,
};

impl<T> WithSpan<'static, T> {
    fn no_span(inner: T) -> Self {
        Self {
            inner,
            span: Span::default(),
        }
    }
}

fn as_path<'a>(path: &'a [&'a str]) -> Vec<WithSpan<'a, PathComponent<'a>>> {
    path.iter()
        .map(|name| {
            WithSpan::no_span(PathComponent {
                name,
                generics: Vec::new(),
            })
        })
        .collect::<Vec<_>>()
}

fn check_ws_split(s: &str, res: &(&str, &str, &str)) {
    let Lit { lws, val, rws } = Lit::split_ws_parts(s);
    assert_eq!(lws, res.0);
    assert_eq!(val, res.1);
    assert_eq!(rws, res.2);
}

#[test]
fn test_ws_splitter() {
    check_ws_split("", &("", "", ""));
    check_ws_split("a", &("", "a", ""));
    check_ws_split("\ta", &("\t", "a", ""));
    check_ws_split("b\n", &("", "b", "\n"));
    check_ws_split(" \t\r\n", &(" \t\r\n", "", ""));
}

#[test]
#[should_panic]
fn test_invalid_block() {
    Ast::from_str("{% extend \"blah\" %}", None, &Syntax::default()).unwrap();
}

fn int_lit<'a>(i: &'a str) -> WithSpan<'a, Box<Expr<'a>>> {
    WithSpan::new_without_span(Box::new(Expr::NumLit(i, Num::Int(i, None))))
}

fn bin_op<'a>(
    op: &'a str,
    lhs: WithSpan<'a, Box<Expr<'a>>>,
    rhs: WithSpan<'a, Box<Expr<'a>>>,
) -> WithSpan<'a, Box<Expr<'a>>> {
    WithSpan::new_without_span(Box::new(Expr::BinOp(crate::expr::BinOp { op, lhs, rhs })))
}

fn call<'a>(
    path: WithSpan<'a, Box<Expr<'a>>>,
    args: Vec<WithSpan<'a, Box<Expr<'a>>>>,
) -> WithSpan<'a, Box<Expr<'a>>> {
    WithSpan::new_without_span(Box::new(Expr::Call(crate::expr::Call { path, args })))
}

#[test]
fn test_parse_filter() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ strvar|e }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("e"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Var("strvar")))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ 2|abs }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![int_lit("2")],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ -2|abs }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Unary("-", int_lit("2"))))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1 - 2)|abs }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Group(bin_op(
                    "-",
                    int_lit("1"),
                    int_lit("2"),
                ))))],
            }))),
        ))],
    );
}

#[test]
fn test_parse_numbers() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ 2 }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(Ws(None, None), int_lit("2")))],
    );
    assert_eq!(
        Ast::from_str("{{ 2.5 }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::NumLit("2.5", Num::Float("2.5", None))))
        ))],
    );
}

#[test]
fn test_parse_var() {
    let s = Syntax::default();

    assert_eq!(
        Ast::from_str("{{ foo }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Var("foo")))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{ foo_bar }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Var("foo_bar")))
        ))],
    );

    assert_eq!(
        Ast::from_str("{{ none }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Var("none")))
        ))]
    );
}

#[test]
fn test_parse_const() {
    let s = Syntax::default();

    assert_eq!(
        Ast::from_str("{{ FOO }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Path(as_path(&["FOO"]))))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{ FOO_BAR }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Path(as_path(&["FOO_BAR"]))))
        ))],
    );

    assert_eq!(
        Ast::from_str("{{ NONE }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Path(as_path(&["NONE"]))))
        ))]
    );
}

#[test]
fn test_parse_path() {
    let s = Syntax::default();

    assert_eq!(
        Ast::from_str("{{ None }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Path(as_path(&["None"])))),
        ))]
    );
    assert_eq!(
        Ast::from_str("{{ Some(123) }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&["Some"])))),
                vec![int_lit("123")],
            ),
        ))],
    );

    assert_eq!(
        Ast::from_str("{{ Ok(123) }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&["Ok"])))),
                vec![int_lit("123")],
            ),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ Err(123) }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&["Err"])))),
                vec![int_lit("123")],
            ),
        ))],
    );
}

#[test]
fn test_parse_var_call() {
    assert_eq!(
        Ast::from_str("{{ function(\"123\", 3) }}", None, &Syntax::default())
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Var("function"))),
                vec![
                    WithSpan::no_span(Box::new(Expr::StrLit(StrLit {
                        content: "123",
                        prefix: None,
                        contains_null: false,
                        contains_unicode_character: false,
                        contains_unicode_escape: false,
                        contains_high_ascii: false,
                    }))),
                    int_lit("3")
                ],
            ),
        ))],
    );
}

#[test]
fn test_parse_path_call() {
    let s = Syntax::default();

    assert_eq!(
        Ast::from_str("{{ Option::None }}", None, &s).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Path(as_path(&["Option", "None"])))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ Option::Some(123) }}", None, &s)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&["Option", "Some"])))),
                vec![int_lit("123")],
            )
        ))],
    );

    assert_eq!(
        Ast::from_str("{{ self::function(\"123\", 3) }}", None, &s)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&["self", "function"])))),
                vec![
                    WithSpan::no_span(Box::new(Expr::StrLit(StrLit {
                        content: "123",
                        prefix: None,
                        contains_null: false,
                        contains_unicode_character: false,
                        contains_unicode_escape: false,
                        contains_high_ascii: false,
                    }))),
                    int_lit("3")
                ],
            )
        ))],
    );
}

#[test]
fn test_parse_root_path() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ std::string::String::new() }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&[
                    "std", "string", "String", "new"
                ])))),
                vec![],
            ),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ ::std::string::String::new() }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Path(as_path(&[
                    "", "std", "string", "String", "new"
                ])))),
                vec![],
            ),
        ))],
    );
}

#[test]
fn test_rust_macro() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ vec!(1, 2, 3) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["vec"], "1, 2, 3"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ alloc::vec!(1, 2, 3) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["alloc", "vec"], "1, 2, 3"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{a!()}}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["a"], "")))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{a !()}}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["a"], "")))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{a! ()}}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["a"], "")))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{a ! ()}}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["a"], "")))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{A!()}}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["A"], "")))
        ))]
    );
    assert_eq!(
        &*Ast::from_str("{{a.b.c!( hello )}}", None, &syntax)
            .unwrap_err()
            .to_string(),
        "failed to parse template source near offset 7",
    );
}

#[test]
fn change_delimiters_parse_filter() {
    let syntax = Syntax(InnerSyntax {
        expr_start: "{=",
        expr_end: "=}",
        ..InnerSyntax::default()
    });
    Ast::from_str("{= strvar|e =}", None, &syntax).unwrap();
}

#[test]
fn unicode_delimiters_in_syntax() {
    let syntax = Syntax(InnerSyntax {
        expr_start: "🖎", // U+1F58E == b"\xf0\x9f\x96\x8e"
        expr_end: "✍",   // U+270D = b'\xe2\x9c\x8d'
        ..InnerSyntax::default()
    });
    assert_eq!(
        Ast::from_str("Here comes the expression: 🖎 e ✍.", None, &syntax)
            .unwrap()
            .nodes(),
        [
            Box::new(Node::Lit(WithSpan::no_span(Lit {
                lws: "",
                val: "Here comes the expression:",
                rws: " ",
            }))),
            Box::new(Node::Expr(
                Ws(None, None),
                WithSpan::no_span(Box::new(Expr::Var("e")))
            )),
            Box::new(Node::Lit(WithSpan::no_span(Lit {
                lws: "",
                val: ".",
                rws: "",
            }))),
        ],
    );
}

#[test]
fn test_precedence() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ a + b == c }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "==",
                bin_op(
                    "+",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                ),
                WithSpan::no_span(Box::new(Expr::Var("c")))
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a + b * c - d / e }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "-",
                bin_op(
                    "+",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    bin_op(
                        "*",
                        WithSpan::no_span(Box::new(Expr::Var("b"))),
                        WithSpan::no_span(Box::new(Expr::Var("c")))
                    )
                ),
                bin_op(
                    "/",
                    WithSpan::no_span(Box::new(Expr::Var("d"))),
                    WithSpan::no_span(Box::new(Expr::Var("e")))
                ),
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a * (b + c) / -d }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "/",
                bin_op(
                    "*",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Group(bin_op(
                        "+",
                        WithSpan::no_span(Box::new(Expr::Var("b"))),
                        WithSpan::no_span(Box::new(Expr::Var("c")))
                    ))))
                ),
                WithSpan::no_span(Box::new(Expr::Unary(
                    "-",
                    WithSpan::no_span(Box::new(Expr::Var("d")))
                )))
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a || b && c || d && e }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "||",
                bin_op(
                    "||",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    bin_op(
                        "&&",
                        WithSpan::no_span(Box::new(Expr::Var("b"))),
                        WithSpan::no_span(Box::new(Expr::Var("c")))
                    ),
                ),
                bin_op(
                    "&&",
                    WithSpan::no_span(Box::new(Expr::Var("d"))),
                    WithSpan::no_span(Box::new(Expr::Var("e")))
                ),
            )
        ))],
    );
}

#[test]
fn test_associativity() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ a + b + c }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "+",
                bin_op(
                    "+",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                ),
                WithSpan::no_span(Box::new(Expr::Var("c")))
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a * b * c }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "*",
                bin_op(
                    "*",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                ),
                WithSpan::no_span(Box::new(Expr::Var("c")))
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a && b && c }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "&&",
                bin_op(
                    "&&",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                ),
                WithSpan::no_span(Box::new(Expr::Var("c")))
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a + b - c + d }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "+",
                bin_op(
                    "-",
                    bin_op(
                        "+",
                        WithSpan::no_span(Box::new(Expr::Var("a"))),
                        WithSpan::no_span(Box::new(Expr::Var("b")))
                    ),
                    WithSpan::no_span(Box::new(Expr::Var("c")))
                ),
                WithSpan::no_span(Box::new(Expr::Var("d")))
            )
        ))],
    );
}

#[test]
fn test_odd_calls() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ a[b](c) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Index(
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                ))),
                vec![WithSpan::no_span(Box::new(Expr::Var("c")))],
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (a + b)(c) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Group(bin_op(
                    "+",
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    WithSpan::no_span(Box::new(Expr::Var("b")))
                )))),
                vec![WithSpan::no_span(Box::new(Expr::Var("c")))],
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a + b(c) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            bin_op(
                "+",
                WithSpan::no_span(Box::new(Expr::Var("a"))),
                call(
                    WithSpan::no_span(Box::new(Expr::Var("b"))),
                    vec![WithSpan::no_span(Box::new(Expr::Var("c")))],
                ),
            ),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (-a)(b) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            call(
                WithSpan::no_span(Box::new(Expr::Group(WithSpan::no_span(Box::new(
                    Expr::Unary("-", WithSpan::no_span(Box::new(Expr::Var("a"))))
                ))))),
                vec![WithSpan::no_span(Box::new(Expr::Var("b")))],
            )
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ -a(b) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Unary(
                "-",
                call(
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    vec![WithSpan::no_span(Box::new(Expr::Var("b")))],
                )
            )))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ a(b)|c }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("c"),
                arguments: vec![call(
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    vec![WithSpan::no_span(Box::new(Expr::Var("b")))],
                )],
            })))
        ))]
    );
    assert_eq!(
        Ast::from_str("{{ a(b)| c }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("c"),
                arguments: vec![call(
                    WithSpan::no_span(Box::new(Expr::Var("a"))),
                    vec![WithSpan::no_span(Box::new(Expr::Var("b")))],
                )],
            }))),
        ))]
    );
}

#[test]
fn test_parse_comments() {
    #[track_caller]
    fn one_comment_ws(source: &str, ws: Ws) {
        let s = &Syntax::default();
        let mut nodes = Ast::from_str(source, None, s).unwrap().nodes;
        assert_eq!(nodes.len(), 1, "expected to parse one node");
        match *nodes.pop().unwrap() {
            Node::Comment(comment) => assert_eq!(comment.ws, ws),
            node => panic!("expected a comment not, but parsed {node:?}"),
        }
    }

    one_comment_ws("{##}", Ws(None, None));
    one_comment_ws("{#- #}", Ws(Some(Whitespace::Suppress), None));
    one_comment_ws("{# -#}", Ws(None, Some(Whitespace::Suppress)));
    one_comment_ws(
        "{#--#}",
        Ws(Some(Whitespace::Suppress), Some(Whitespace::Suppress)),
    );
    one_comment_ws(
        "{#- foo\n bar -#}",
        Ws(Some(Whitespace::Suppress), Some(Whitespace::Suppress)),
    );
    one_comment_ws(
        "{#- foo\n {#- bar\n -#} baz -#}",
        Ws(Some(Whitespace::Suppress), Some(Whitespace::Suppress)),
    );
    one_comment_ws("{#+ #}", Ws(Some(Whitespace::Preserve), None));
    one_comment_ws("{# +#}", Ws(None, Some(Whitespace::Preserve)));
    one_comment_ws(
        "{#++#}",
        Ws(Some(Whitespace::Preserve), Some(Whitespace::Preserve)),
    );
    one_comment_ws(
        "{#+ foo\n bar +#}",
        Ws(Some(Whitespace::Preserve), Some(Whitespace::Preserve)),
    );
    one_comment_ws(
        "{#+ foo\n {#+ bar\n +#} baz -+#}",
        Ws(Some(Whitespace::Preserve), Some(Whitespace::Preserve)),
    );
    one_comment_ws("{#~ #}", Ws(Some(Whitespace::Minimize), None));
    one_comment_ws("{# ~#}", Ws(None, Some(Whitespace::Minimize)));
    one_comment_ws(
        "{#~~#}",
        Ws(Some(Whitespace::Minimize), Some(Whitespace::Minimize)),
    );
    one_comment_ws(
        "{#~ foo\n bar ~#}",
        Ws(Some(Whitespace::Minimize), Some(Whitespace::Minimize)),
    );
    one_comment_ws(
        "{#~ foo\n {#~ bar\n ~#} baz -~#}",
        Ws(Some(Whitespace::Minimize), Some(Whitespace::Minimize)),
    );

    one_comment_ws("{# foo {# bar #} {# {# baz #} qux #} #}", Ws(None, None));
}

#[test]
fn test_parse_tuple() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ () }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Group(int_lit("1"))))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1,) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1, ) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1 ,) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1 , ) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1, 2) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1"), int_lit("2")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1, 2,) }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1"), int_lit("2")]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1, 2, 3) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Tuple(vec![
                int_lit("1"),
                int_lit("2"),
                int_lit("3")
            ]))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ ()|abs }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Tuple(vec![])))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1)|abs }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Group(int_lit("1"))))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1,)|abs }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Tuple(vec![int_lit("1")])))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ (1, 2)|abs }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("abs"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Tuple(vec![
                    int_lit("1"),
                    int_lit("2")
                ])))],
            }))),
        ))],
    );
}

#[test]
fn test_missing_space_after_kw() {
    let syntax = Syntax::default();
    let err = Ast::from_str("{%leta=b%}", None, &syntax).unwrap_err();
    assert_eq!(
        err.to_string(),
        "unknown node `leta`\nfailed to parse template source near offset 2",
    );
}

#[test]
fn test_parse_array() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ [] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [ 1] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1 ] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1,2] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1"), int_lit("2")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1 ,2] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1"), int_lit("2")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1, 2] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1"), int_lit("2")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ [1,2 ] }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![int_lit("1"), int_lit("2")])))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ []|foo }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("foo"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Array(vec![])))],
            })))
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ []| foo }}", None, &syntax).unwrap().nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Identifier("foo"),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Array(vec![])))],
            })))
        ))],
    );

    let n = || {
        Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Array(vec![WithSpan::no_span(Box::new(
                Expr::NumLit("1", Num::Int("1", None)),
            ))]))),
        ))
    };
    assert_eq!(
        Ast::from_str(
            "{{ [1,] }}{{ [1 ,] }}{{ [1, ] }}{{ [1 , ] }}",
            None,
            &syntax
        )
        .unwrap()
        .nodes,
        [n(), n(), n(), n()],
    );
}

#[test]
fn fuzzed_unicode_slice() {
    let d = "{eeuuu{b&{!!&{!!11{{
            0!(!1q҄א!)!!!!!!n!";
    assert!(Ast::from_str(d, None, &Syntax::default()).is_err());
}

#[test]
fn fuzzed_macro_no_end() {
    let s = "{%macro super%}{%endmacro";
    assert!(Ast::from_str(s, None, &Syntax::default()).is_err());
}

#[test]
fn fuzzed_target_recursion() {
    const TEMPLATE: &str = include_str!("../tests/target-recursion.txt");
    assert!(Ast::from_str(TEMPLATE, None, &Syntax::default()).is_err());
}

#[test]
fn fuzzed_unary_recursion() {
    const TEMPLATE: &str = include_str!("../tests/unary-recursion.txt");
    assert!(Ast::from_str(TEMPLATE, None, &Syntax::default()).is_err());
}

#[test]
fn fuzzed_comment_depth() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let test = std::thread::spawn(move || {
        const TEMPLATE: &str = include_str!("../tests/comment-depth.txt");
        assert!(Ast::from_str(TEMPLATE, None, &Syntax::default()).is_ok());
        sender.send(()).unwrap();
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(3))
        .expect("timeout");
    test.join().unwrap();
}

#[test]
fn let_set() {
    assert_eq!(
        Ast::from_str("{% let a %}", None, &Syntax::default())
            .unwrap()
            .nodes(),
        Ast::from_str("{% set a %}", None, &Syntax::default())
            .unwrap()
            .nodes(),
    );
}

#[test]
fn fuzzed_filter_recursion() {
    const TEMPLATE: &str = include_str!("../tests/filter-recursion.txt");
    assert!(Ast::from_str(TEMPLATE, None, &Syntax::default()).is_err());
}

#[test]
fn fuzzed_excessive_syntax_lengths() {
    const LONG_DELIM: Option<&str> =
        Some("\0]***NEWFILE\u{1f}***:7/v/.-3/\u{1b}/~~~~z~0/*:7/v/./t/t/.p//NEWVILE**::7/v");

    for (kind, syntax_builder) in [
        (
            "opening block",
            SyntaxBuilder {
                block_start: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
        (
            "closing block",
            SyntaxBuilder {
                block_end: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
        (
            "opening expression",
            SyntaxBuilder {
                expr_start: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
        (
            "closing expression",
            SyntaxBuilder {
                expr_end: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
        (
            "opening comment",
            SyntaxBuilder {
                comment_start: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
        (
            "closing comment",
            SyntaxBuilder {
                comment_end: LONG_DELIM,
                ..SyntaxBuilder::default()
            },
        ),
    ] {
        let err = syntax_builder.to_syntax().unwrap_err();
        assert_eq!(
            err,
            format!(
                "delimiters must be at most 32 characters long. The {kind} delimiter \
                 (\"\\0]***NEWFILE\\u{{1f}}***\"...) is too long"
            ),
        );
    }
}

#[test]
fn extends_with_whitespace_control() {
    const CONTROL: &[&str] = &["", "\t", "-", "+", "~"];

    let syntax = Syntax::default();
    let expected = Ast::from_str(r#"front {% extends "nothing" %} back"#, None, &syntax).unwrap();
    for front in CONTROL {
        for back in CONTROL {
            let src = format!(r#"front {{%{front} extends "nothing" {back}%}} back"#);
            let actual = Ast::from_str(&src, None, &syntax).unwrap();
            assert_eq!(expected.nodes(), actual.nodes(), "source: {src:?}");
        }
    }
}

#[test]
fn fuzzed_span_is_not_substring_of_source() {
    let _: Result<Ast<'_>, crate::ParseError> = Ast::from_str(
        include_str!("../tests/fuzzed_span_is_not_substring_of_source.bin"),
        None,
        &Syntax::default(),
    );
}

#[test]
fn fuzzed_excessive_filter_block() {
    let src = include_str!("../tests/excessive_filter_block.txt");
    let err = Ast::from_str(src, None, &Syntax::default()).unwrap_err();
    assert_eq!(
        err.to_string().lines().next(),
        Some("your template code is too deeply nested, or the last expression is too complex"),
    );
}

#[test]
fn test_generics_parsing() {
    // Method call.
    Ast::from_str("{{ a.b::<&str, H<B<C>>>() }}", None, &Syntax::default()).unwrap();
    Ast::from_str(
        "{{ a.b::<&str, H<B<C> , &u32>>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();

    // Call.
    Ast::from_str(
        "{{ a::<&str, H<B<C> , &u32>>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();

    // Filter.
    Ast::from_str("{{ 12 | a::<&str> }}", None, &Syntax::default()).unwrap();
    Ast::from_str("{{ 12 | a::<&str, u32>('a') }}", None, &Syntax::default()).unwrap();

    // Unclosed `<`.
    assert!(
        Ast::from_str(
            "{{ a.b::<&str, H<B<C> , &u32>() }}",
            None,
            &Syntax::default()
        )
        .is_err()
    );

    // With path and spaces
    Ast::from_str(
        "{{ a.b::<&&core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b ::<&&core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b:: <&&core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::< &&core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<& &core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&& core::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core ::primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core:: primitive::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core::primitive ::str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core::primitive:: str>() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core::primitive::str >() }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
    Ast::from_str(
        "{{ a.b::<&&core::primitive::str> () }}",
        None,
        &Syntax::default(),
    )
    .unwrap();
}

#[test]
fn fuzzed_deeply_tested_if_let() {
    let src = include_str!("../tests/fuzzed-deeply-tested-if-let.txt");
    let syntax = Syntax::default();
    let err = Ast::from_str(src, None, &syntax).unwrap_err();
    assert_eq!(
        err.to_string().lines().next(),
        Some("your template code is too deeply nested, or the last expression is too complex"),
    );
}

#[test]
fn test_filter_with_path() {
    let syntax = Syntax::default();
    assert_eq!(
        Ast::from_str("{{ strvar|::e }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Path(as_path(&["", "e"])),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Var("strvar")))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ strvar|::e::f }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Path(as_path(&["", "e", "f"])),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Var("strvar")))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ strvar|e::f }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Path(as_path(&["e", "f"])),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Var("strvar")))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ strvar|e::f() }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::Filter(Filter {
                name: PathOrIdentifier::Path(as_path(&["e", "f"])),
                arguments: vec![WithSpan::no_span(Box::new(Expr::Var("strvar")))],
            }))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ strvar|e()::f }}", None, &syntax)
            .unwrap_err()
            .to_string(),
        "failed to parse template source near offset 13",
    );
    assert_eq!(
        Ast::from_str("{{ strvar|e::f()::g }}", None, &syntax)
            .unwrap_err()
            .to_string(),
        "failed to parse template source near offset 16",
    );
}

#[test]
fn underscore_is_an_identifier() {
    let mut input = "_";
    let result = crate::identifier.parse_next(&mut input);
    assert_eq!(result.unwrap(), "_");
    assert_eq!(input, "");
}

#[test]
fn there_is_no_digit_two_in_a_binary_integer() {
    let syntax = Syntax::default();
    assert!(Ast::from_str("{{ 0b2 }}", None, &syntax).is_err());
    assert!(Ast::from_str("{{ 0o9 }}", None, &syntax).is_err());
    assert!(Ast::from_str("{{ 0xg }}", None, &syntax).is_err());
}

#[test]
fn comparison_operators_cannot_be_chained() {
    const OPS: &[&str] = &["==", "!=", ">=", ">", "<=", "<"];

    let syntax = Syntax::default();
    for op1 in OPS {
        assert!(Ast::from_str(&format!("{{{{ a {op1} b }}}}"), None, &syntax).is_ok());
        for op2 in OPS {
            assert!(Ast::from_str(&format!("{{{{ a {op1} b {op2} c }}}}"), None, &syntax).is_err());
            for op3 in OPS {
                assert!(
                    Ast::from_str(
                        &format!("{{{{ a {op1} b {op2} c {op3} d }}}}"),
                        None,
                        &syntax,
                    )
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn macro_calls_can_have_raw_prefixes() {
    // Related to issue <https://github.com/askama-rs/askama/issues/475>.
    let syntax = Syntax::default();
    let inner = r####"r#"test"# r##"test"## r###"test"### r#loop"####;
    assert_eq!(
        Ast::from_str(&format!("{{{{ z!{{{inner}}} }}}}"), None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["z"], inner))),
        ))],
    );
}

#[test]
fn macro_comments_in_macro_calls() {
    // Related to <https://issues.oss-fuzz.com/issues/425145246>.
    let syntax = Syntax::default();

    assert!(Ast::from_str("{{ e!(// hello) }}", None, &syntax).is_err());
    assert!(Ast::from_str("{{ e!(/// hello) }}", None, &syntax).is_err());
    assert!(Ast::from_str("{{ e!(// hello)\n }}", None, &syntax).is_err());
    assert!(Ast::from_str("{{ e!(/// hello)\n }}", None, &syntax).is_err());

    assert_eq!(
        Ast::from_str("{{ e!(// hello\n) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "// hello\n"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ e!(/// hello\n) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "/// hello\n"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ e!(//! hello\n) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "//! hello\n"))),
        ))],
    );

    assert_eq!(
        Ast::from_str("{{ e!(/* hello */) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "/* hello */"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ e!(/** hello */) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "/** hello */"))),
        ))],
    );
    assert_eq!(
        Ast::from_str("{{ e!(/*! hello */) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["e"], "/*! hello */"))),
        ))],
    );
}

#[test]
fn test_raw() {
    let syntax = Syntax::default();
    let val = "hello {{ endraw %} my {%* endraw %} green {% endraw }} world";
    assert_eq!(
        Ast::from_str(
            &format!("{{%+ raw -%}} {val} {{%~ endraw ~%}}"),
            None,
            &syntax
        )
        .unwrap()
        .nodes,
        [Box::new(Node::Raw(WithSpan::no_span(Raw {
            ws1: Ws(Some(Whitespace::Preserve), Some(Whitespace::Suppress)),
            lit: Lit {
                lws: " ",
                val,
                rws: " ",
            },
            ws2: Ws(Some(Whitespace::Minimize), Some(Whitespace::Minimize)),
        })))],
    );

    // We must make sure that the character for whitespace handling, e.g. `-` is not consumed,
    // unless `{% endraw %}` was actually found. Otherwise opening block delimiters that begin with
    // `-`, `~` or `+` may break.
    let syntax = SyntaxBuilder {
        block_start: Some("-$"),
        block_end: Some("$-"),
        ..SyntaxBuilder::default()
    };
    let syntax = syntax.to_syntax().unwrap();
    assert_eq!(
        Ast::from_str("-$- raw -$- -$- endraw -$ endraw -$-", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Raw(WithSpan::no_span(Raw {
            ws1: Ws(Some(Whitespace::Suppress), Some(Whitespace::Suppress)),
            lit: Lit {
                lws: " ",
                val: "-$- endraw",
                rws: " ",
            },
            ws2: Ws(None, Some(Whitespace::Suppress)),
        })))],
    );
}

#[test]
fn test_macro_call_nested_comments() {
    // Regression test for <https://issues.oss-fuzz.com/issues/427825995>.
    let syntax = Syntax::default();

    assert_eq!(
        Ast::from_str("{{ x!(/*/*/*)*/*/*/) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["x"], "/*/*/*)*/*/*/"))),
        ))],
    );

    let msg = Ast::from_str("{{ x!(/*/*/) }}", None, &syntax)
        .unwrap_err()
        .to_string();
    assert!(msg.contains("missing `*/` to close block comment"));

    assert_eq!(
        Ast::from_str("{{ x!(/**/) }}", None, &syntax)
            .unwrap()
            .nodes,
        [Box::new(Node::Expr(
            Ws(None, None),
            WithSpan::no_span(Box::new(Expr::RustMacro(vec!["x"], "/**/"))),
        ))],
    );
}

#[test]
fn test_try_reserved_raw_identifier() {
    // Regression test for <https://issues.oss-fuzz.com/issues/429130577>.
    let syntax = Syntax::default();

    for id in ["crate", "super", "Self"] {
        let msg = format!("`{id}` cannot be used as an identifier");
        assert!(
            Ast::from_str(&format!("{{{{ {id}? }}}}"), None, &syntax)
                .unwrap_err()
                .to_string()
                .contains(&msg),
        );
        assert!(
            Ast::from_str(&format!("{{{{ {id}|filter }}}}"), None, &syntax)
                .unwrap_err()
                .to_string()
                .contains(&msg),
        );
        assert!(
            Ast::from_str(
                &format!("{{{{ var|filter(arg1, {id}, arg3) }}}}"),
                None,
                &syntax
            )
            .unwrap_err()
            .to_string()
            .contains(&msg),
        );
        assert!(
            Ast::from_str(
                &format!("{{{{ var|filter(arg1=arg1, arg2={id}, arg3=arg3) }}}}"),
                None,
                &syntax
            )
            .unwrap_err()
            .to_string()
            .contains(&msg),
        );
    }
}

#[test]
fn test_isolated_cr_in_raw_string() {
    // Regression test for <https://issues.oss-fuzz.com/issues/429645376>.
    let syntax = Syntax::default();

    assert!(
        Ast::from_str("{{ x!(\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
    assert!(
        Ast::from_str("{{ x!(c\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
    assert!(
        Ast::from_str("{{ x!(b\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
    assert!(
        Ast::from_str("{{ x!(r\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
    assert!(
        Ast::from_str("{{ x!(cr\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
    assert!(
        Ast::from_str("{{ x!(br\"hello\rworld\") }}", None, &syntax)
            .unwrap_err()
            .to_string()
            .contains("a bare CR (Mac linebreak) is not allowed in string literals"),
    );
}
