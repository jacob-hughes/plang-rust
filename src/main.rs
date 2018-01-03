#![feature(try_from)]
use std::vec::Vec;
use std::path::Path;
use std::env;

extern crate plang_rust;
use plang_rust::parse::parse_file;
use plang_rust::interp::run;


fn main() {
    let args: Vec<String> = env::args().collect();
    let ref source = &args[1];
    let lex_path    = Path::new("grammar/lexer.l");
    let yacc_path   = Path::new("grammar/grammar.y");
    let source_path = Path::new(source);
    let bytecode = parse_file(source_path, lex_path, yacc_path).unwrap();
    run(bytecode);
}

