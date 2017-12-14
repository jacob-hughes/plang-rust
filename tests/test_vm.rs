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
fn main_with_call() {
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
fn call_with_args() {
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

#[test]
fn add_operator() {
    let src = "
        class global() {
            def main() {
                5 + 5
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "10");
}

#[test]
fn sub_operator() {
    let src = "
        class global() {
            def main() {
               666 - 66
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "600");
}

#[test]
fn cmp_eq() {
    let src = "
        class global() {
            def main() {
               666 == 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "true");
}

#[test]
fn cmp_eq_false() {
    let src = "
        class global() {
            def main() {
               66 == 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "false");
}

#[test]
fn cmp_lt() {
    let src = "
        class global() {
            def main() {
               6 < 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "true");
}

#[test]
fn cmp_lte() {
    let src = "
        class global() {
            def main() {
               666 <= 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "true");
}

#[test]
fn cmp_gt() {
    let src = "
        class global() {
            def main() {
               666 > 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "false");
}

#[test]
fn cmp_gte() {
    let src = "
        class global() {
            def main() {
               666 >= 666
            }
        }
    ";
    let bc = build_bytecode(src.to_string());
    println!("{:?}", bc);
    let res = run(bc);
    assert_eq!(res, "true");
}

