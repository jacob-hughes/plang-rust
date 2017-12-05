
extern crate lrpar;
extern crate lrlex;
extern crate lrtable;
extern crate cfgrammar;

use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::convert::TryFrom;

use self::lrpar::parser;
use self::lrpar::parser::Node;
use self::lrlex::{build_lex, Lexer};
use self::lrtable::{Minimiser, from_yacc};
use self::cfgrammar::yacc::{yacc_grm, YaccGrammar, YaccKind};


#[derive(Debug)]
pub enum ParseError {
    IO(String),
    FileNotFound(String),
    BrokenLexer,
    BrokenParser,
    LexicalError,
    SyntaxError,
}

pub fn read_file(path: &Path) -> Result<String, ParseError> {
    if !Path::new(path).exists() {
        Err(ParseError::FileNotFound(path.to_str().unwrap().into()))
    }
    else {
        let mut f = File::open(path).map_err(|e| ParseError::IO(e.to_string()))?;
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        Ok(s)
    }
}


pub fn parse_file(source_path: &Path, lex_path: &Path, yacc_path: &Path) -> Result<Node<u16>,ParseError> {

    let input = read_file(source_path)?;
    let lexs = read_file(lex_path)?;
    let mut lexer_def = build_lex::<u16>(&lexs)
        .map_err(|_| ParseError::BrokenLexer)?;
    let grms = read_file(yacc_path)?;
    let grm = yacc_grm(YaccKind::Eco, &grms)
        .map_err(|_| ParseError::BrokenParser)?;

    // Sync up the IDs of terminals in the lexer and parser.
    let rule_ids = grm.terms_map()
         .iter()
         .map(|(&n, &i)| (n, u16::try_from(usize::from(i)).unwrap()))
         .collect();
    lexer_def.set_rule_ids(&rule_ids);

    let lexer = lexer_def.lexer(&input);
    let lexemes = lexer.lexemes().map_err(|_| ParseError::LexicalError)?;
    let (sgraph, stable) = from_yacc(&grm, Minimiser::Pager)
        .map_err(|_| ParseError::BrokenParser)?;

    let pt = parser::parse::<u16>(&grm, &sgraph, &stable, &lexemes)
        .map_err(|_| ParseError::SyntaxError)?;

    Ok(pt)

}


