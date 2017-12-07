extern crate lrpar;
extern crate lrlex;
extern crate lrtable;
extern crate cfgrammar;

use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::convert::{TryFrom, TryInto};
use std::collections::HashMap;
use std::fmt;

use self::lrpar::parser;
use self::lrpar::parser::Node;
use self::lrlex::{build_lex, Lexer};
use self::lrtable::{Minimiser, from_yacc};

use self::cfgrammar::TIdx;
use self::cfgrammar::yacc::{yacc_grm, YaccGrammar, YaccKind};


#[derive(Debug)]
pub enum ParseError {
    IO(String),
    FileNotFound(String),
    BrokenLexer,
    BrokenParser,
    LexicalError,
    SyntaxError,
    GeneratorError(String),
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


pub fn parse_file(source_path: &Path, lex_path: &Path, yacc_path: &Path) -> Result<CompilerContext,ParseError> {

    let input = read_file(source_path)?;
    let lexs = read_file(lex_path)?;
    let mut lexer_def = build_lex::<u16>(&lexs)
        .map_err(|_| ParseError::BrokenLexer)?;
    let grms = read_file(yacc_path)?;
    let grm = yacc_grm(YaccKind::Original, &grms)
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

    Ok(gen_bytecode(&pt, &grm, &input))

}

#[derive(Debug)]
enum Instr {
    PUSH_INT(i32),
    POP,
    ADD,
    SUB,
    LOAD_VAR(String),
    STORE_VAR(String),
    NEW_OBJECT,
    LOAD_FIELD(String),
    STORE_FIELD(String),
    CLASS_LABEL(String),
    METH_LABEL(String),
    SWAP,
    DUP,
    CALL(i32),
    JEQ(i32),
    RET,
}

struct Bytecode<'pt> {
    classes: HashMap<String, Vec<Instr>>,
    symbols: HashMap<String, (Vec<String>, Vec<String>)>,

    // Store the grammar and input for convenience
    grm:    &'pt YaccGrammar,
    input:  &'pt str,
}

impl<'pt> fmt::Debug for Bytecode<'pt> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ctx{{classes: {:?}, symbols: {:?}}}", self.classes, self.symbols)
    }
}

impl<'pt> Bytecode<'pt> {
    fn register_function(class_name: String, function_name: String) {
    }
}

#[derive(Debug)]
struct Class {
    methods: HashMap<String, Method>,
    code:    Vec<Instr>
}

impl Class {
    fn new() -> Class {
        Class {methods: HashMap::new()}
    }

    fn add_method(&mut self, name: String, method: Method) {
        self.methods.insert(name, method);
    }
}


#[derive(Debug)]
struct Method {
    locals: Vec<String>,
    code:   Vec<Instr>
}

impl Method {
    fn new() -> Method {
        Method{
            locals: Vec::new(),
            code: Vec::new()
        }
    }

    fn push_instr(&mut self, instr: Instr) {
        self.code.push(instr)
    }

    fn push_var(&mut self, var_name: String) {
        self.locals.push(var_name)
    }

}


#[derive(Debug)]
pub struct CompilerContext {
    classes: HashMap<String,Class>,
}

impl CompilerContext {
    fn add(&mut self, name: String, class: Class) {
        self.classes.insert(name, class);
    }
}

fn gen_bytecode(parse_tree: &Node<u16>, grm: &YaccGrammar, input: &str) -> CompilerContext {

    fn id_value (node: &Node<u16>, grm: &YaccGrammar, input: &str) -> String {
        if let &Node::Term { lexeme } = node {
            return String::from(&input[lexeme.start()..lexeme.start() + lexeme.len()])
        }
        else { panic!("Identifier not found") }
    };

    fn term_name (node: &Node<u16>, grm: &YaccGrammar, input: &str) -> String {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                grm.nonterm_name(nonterm_idx).to_string()
            }
            Node::Term { lexeme } => {
                let token_id: usize = lexeme.tok_id().try_into().ok().unwrap();
                grm.term_name(TIdx::from(token_id)).unwrap().to_string()
            }
        }
    };

    fn gen_class_body(node: &Node<u16>, grm: &YaccGrammar, input: &str, class: &mut Class) {
        if let &Node::Nonterm { nonterm_idx, ref nodes } = node {
            for child in nodes.iter() {
                if let &Node::Nonterm { nonterm_idx, ref nodes} = child {
                    let name = term_name(child, grm, input);
                    match name.as_ref() {
                        "method_def" => gen_method(child, grm, input, class),
                        "class_body" => gen_class_body(child, grm, input, class),
                        _ => (),
                    }
                }
            }
        }
    }

    // method_def : "DEF" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN" block ;
    fn gen_method(node: &Node<u16>, grm: &YaccGrammar, input: &str, class: &mut Class) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            let mut method = Method::new();
            let meth_name = id_value(&nodes[1], grm, input);
            gen_params(&nodes[3], grm, input, &mut method);
            gen_block(&nodes[5], grm, input, &mut method);
            class.add_method(meth_name, method)
        }
    }

    fn gen_params(node: &Node<u16>, grm: &YaccGrammar, input: &str, method: &mut Method) {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                for child in nodes.iter() {
                    gen_params(child, grm, input, method)
                }
            }
            Node::Term{ lexeme } => {
                if term_name(node, grm, input) == "IDENTIFIER" {
                    let var_name = id_value(node, grm, input);
                    method.push_var(var_name);
                }
            }
        }
    }

    fn gen_block(node: &Node<u16>, grm: &YaccGrammar, input: &str, method: &mut Method) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            for child in nodes {
                let name = term_name(child, grm, input);
                match name.as_ref() {
                    "statement" => gen_stmt(child, grm, input, method),
                    _ => gen_block(child, grm, input, method)
                }
            }
        }
    }

    // statement : expression
    //           | if_statement
    //           | let_statement
    //           | for_statement
    //           ;
    fn gen_stmt(node: &Node<u16>, grm: &YaccGrammar, input: &str, method: &mut Method) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            let name = term_name(&nodes[0], grm, input);
            println!("{}", name);
            match name.as_ref() {
                "expression" => gen_exp(&nodes[0], grm, input, method),
                "if_statement" => (),
                "let_statement" => gen_let(&nodes[0], grm, input, method),
                "for_statement" => (),
                _ => ()
            }
        }
    }

    //let_statement : "LET" "IDENTIFIER" "EQ" expression;
    fn gen_let(node: &Node<u16>, grm: &YaccGrammar, input: &str, method: &mut Method) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            gen_exp(&nodes[3], grm, input, method);
            let var_name = id_value(&nodes[1], grm, input);
            method.push_instr(Instr::STORE_VAR(var_name));
        }
    }

    // expression : variable
    //            | binary_expression
    //            | method_invocation
    //            | field_access
    //            | class_instance_creation
    //            | literal
    //            ;
    fn gen_exp(node: &Node<u16>, grm: &YaccGrammar, input: &str, method: &mut Method) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            let exp_type = &nodes[0];
            let name = term_name(exp_type, grm, input);
            if let &Node::Nonterm{ nonterm_idx, ref nodes } = exp_type {
                match name.as_ref() {
                    "variable" => {
                        let var_name = id_value(&nodes[0], grm, input);
                        method.push_instr(Instr::LOAD_VAR(var_name));
                    }
                    "binary_expression"       => panic!("NotYetImplemented"),
                    "method_invocation"       => panic!("NotYetImplemented"),
                    "field_access"            => panic!("NotYetImplemented"),
                    "class_instance_creation" => panic!("NotYetImplemented"),
                    "literal" => {
                        let lit_type =  term_name(&nodes[0], grm, input);
                        print!("{}", lit_type);
                        let lit_value = id_value(&nodes[0], grm, input);
                        match lit_type.as_ref(){
                            "INT_LITERAL" => {
                                let int = lit_value.parse::<i32>().unwrap();
                                method.push_instr(Instr::PUSH_INT(int))
                            }
                            _ => panic!("NotYetImplemented")
                        }

                    }
                    _ => panic!("unknown expression")
                }

            }
        }
    }


    // class_def : "CLASS" "IDENTIFIER" "LPAREN" parent_class_opt "RPAREN" "LBRACE" class_body "RBRACE";
    // parent_class_opt :
    //                  | "IDENTIFIER"
    //                  ;
    fn gen_class(node: &Node<u16>, grm: &YaccGrammar, input: &str) -> (String, Class) {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                let class_name  = id_value(&nodes[1], grm, input);
                let mut class       = Class::new();
                gen_class_body(&nodes[6], grm, input, &mut class);
                (class_name, class)
            }
            _ => panic!("Class nonterm expected")
        }
    };

    let mut ctx = CompilerContext{ classes: HashMap::new() };
    match *parse_tree {
        Node::Nonterm { nonterm_idx, ref nodes } => {
            for x in nodes.iter() {
                let (name, class) = gen_class(x, grm, input);
                ctx.add(name, class)
            }
            ctx
        }
        _ => panic!("Error")
    }
}



