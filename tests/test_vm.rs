extern crate plang_rust;

use std::path::Path;

use plang_rust::parse::parse_input;
use plang_rust::parse::Bytecode;
use plang_rust::interp::run;

const LEX_PATH: &str = "grammar/lexer.l";
const YACC_PATH: &str = "grammar/grammar.y";

fn build_bytecode(source: String) -> Bytecode {
    let lex_path = Path::new(LEX_PATH);
    let yacc_path = Path::new(YACC_PATH);
    parse_input(source, &lex_path, &yacc_path).unwrap()
}

#[test]
fn main_returns() {
    let src = "
        class global() {
            def main() {
                666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    let res = run(bc);
    assert_eq!(res, "666");
}

#[test]
fn simple_main_with_call() {
    let src = "
        class global() {
            def main() {
                hello()
            };

            def hello() {
                678
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    let res = run(bc);
    assert_eq!(res, "678");
}

#[test]
fn function_call_with_args() {
    let src = "
        class global() {
            def main() {
                hello(123)
            };

            def hello(x) {
                x
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "123");
}

