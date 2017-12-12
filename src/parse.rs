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


pub fn parse_file(source_path: &Path, lex_path: &Path, yacc_path: &Path) -> Result<Bytecode,ParseError> {
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
    LTEQ,
    GTEQ,
    LT,
    GT,
    LOAD_VAR(String),
    STORE_VAR(String),
    LOAD_GLOBAL(String),
    STORE_GLOBAL(String),
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

#[derive(Debug)]
struct Fn {
    params:   Vec<String>,
    locals: Vec<String>
}

impl Fn {
    fn new() -> Fn {
        Fn {
            params: Vec::new(),
            locals: Vec::new(),
        }
    }
    fn param_size(&self) -> usize {
        self.params.len()
    }

    fn push_param(&mut self, param: String) {
        self.params.push(param);
    }

    fn push_local(&mut self, local: String) {
        self.locals.push(local);
    }

    fn locals_size(&self) -> usize {
        self.locals.len()
    }

    fn size(&self) -> usize {
        self.locals.len() + self.params.len()
    }
}

// Conversion from the CompilerContext struct, removes the helper fields
// which are used for building up the symbol table and bytecode. These
// aren't needed anymore and as they are references which require a
// lifetime, their removal makes working with the struct easier.
#[derive(Debug)]
pub struct Bytecode {
    classes: HashMap<String, Vec<Instr>>,
    symbols: HashMap<(String, String), Fn>,
}

impl Bytecode {
    fn new(ctx : CompilerContext) -> Bytecode {
        Bytecode {
            classes: ctx.classes,
            symbols: ctx.symbols
        }
    }
}

struct CompilerContext<'pt> {
    classes: HashMap<String, Vec<Instr>>,
    symbols: HashMap<(String, String), Fn>,

    // Fields for convenience when building up the Bytecode struct
    grm:        &'pt YaccGrammar,
    input:      &'pt str,
    cur_cls:    String,
    cur_fn:     String,
}

impl<'pt> CompilerContext<'pt> {
    fn new(grm: &'pt YaccGrammar, input: &'pt str) -> CompilerContext<'pt> {
        CompilerContext {
            classes: HashMap::new(),
            symbols: HashMap::new(),
            grm:     grm,
            input:   input,
            cur_cls: "global".to_string(),
            cur_fn:  "global".to_string(),
        }
    }

    fn register_class(&mut self, class: &Node<u16>) {
        match *class {
            Node::Term { lexeme } => {
                let class_name = self.get_value(class);
                self.cur_cls   = class_name.clone();
                self.classes.insert(class_name, Vec::new());
            }
            _ => panic!("Can only register a class on a terminal node")
        }
    }

    fn register_function(&mut self, func: &Node<u16>) -> String {
        match *func {
            Node::Term { lexeme } => {
                let func_name = self.get_value(func);
                self.cur_fn = func_name.clone();
                self.symbols.insert((self.cur_cls.to_string(), func_name.to_string()), Fn::new());
                return func_name
            }
            _ => panic!("Can only register a func on a terminal node")
        }
    }

    // Adds the parameter name to the param vector of the current cls + func.
    fn register_parameter(&mut self, param: &Node<u16>) {
        let param_name = self.get_value(param);
        let ref key = (self.cur_cls.to_string(), self.cur_fn.to_string());
        self.symbols.get_mut(key).unwrap().push_param(param_name);
    }

    fn gen_bc(&mut self , instr: Instr) {
        self.classes.get_mut(&self.cur_cls).unwrap().push(instr);
    }

    fn get_value(&self, node: &Node<u16>) -> String {
        match *node {
            Node::Term { lexeme } => self.input[lexeme.start()..lexeme.start() + lexeme.len()]
                                        .to_string(),
            _ => panic!("Cannot determine name of non-terminal node")
        }
    }

    fn get_name(&self, node: &Node<u16>) -> String {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                self.grm.nonterm_name(nonterm_idx).to_string()
            }
            Node::Term { lexeme } => {
                let token_id: usize = lexeme.tok_id().try_into().ok().unwrap();
                self.grm.term_name(TIdx::from(token_id)).unwrap().to_string()
            }
        }
    }
}

fn gen_bytecode(parse_tree: &Node<u16>, grm: &YaccGrammar, input: &str) -> Bytecode {
    // class_def : "CLASS" "IDENTIFIER" "LPAREN" parent_class_opt "RPAREN" "LBRACE" class_body "RBRACE";
    // parent_class_opt :
    //                  | "IDENTIFIER"
    //                  ;
    fn gen_class(node: &Node<u16>, ctx: &mut CompilerContext) {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                ctx.register_class(&nodes[1]);
                gen_block(&nodes[5], ctx);
            }
            _ => panic!("Class nonterm expected")
        }
    }

    // block_statements : statement
    //                  | block_statements "SEMI" statement
    //                  ;
    fn gen_block(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            for child in nodes {
                match ctx.get_name(child).as_ref(){
                    "statement" => gen_stmt(child, ctx),
                    _ => gen_block(child, ctx)
                }
            }
        }
    }

    // statement : expression
    //           | if_statement
    //           | let_statement
    //           | for_statement
    //           ;
    fn gen_stmt(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            match ctx.get_name(&nodes[0]).as_ref(){
                "expression"    => gen_exp(&nodes[0], ctx),
                "if_statement"  => gen_if(&nodes[0], ctx),
                "let_statement" => gen_let(&nodes[0], ctx),
                "func_def"      => gen_func_def(&nodes[0], ctx),
                "for_statement" => gen_for(&nodes[0], ctx),
                _ => panic!("unknown nonterminal node")
            }
        }
    }

    // expression : variable
    //            | binary_expression
    //            | method_invocation
    //            | field_access
    //            | class_instance_creation
    //            | literal
    //            ;
    fn gen_exp(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            let exp_type = &nodes[0];
            let name = ctx.get_name(exp_type);
            if let &Node::Nonterm{ nonterm_idx, ref nodes } = exp_type {
                match name.as_ref() {
                    "variable" => {
                        let var_name = ctx.get_value(&nodes[0]);
                        ctx.gen_bc(Instr::LOAD_VAR(var_name));
                    }
                    "binary_expression"       => {
                        gen_exp(&nodes[0], ctx);
                        gen_exp(&nodes[2], ctx);
                        let bin_op = &nodes[1];
                        if let &Node::Nonterm{ nonterm_idx, ref nodes } = bin_op {
                            let operator = &nodes[0];
                            match ctx.get_name(operator).as_ref() {
                                "PLUS"  => ctx.gen_bc(Instr::ADD),
                                "MINUS" => ctx.gen_bc(Instr::SUB),
                                "LTEQ"  => ctx.gen_bc(Instr::LTEQ),
                                "GTEQ"  => ctx.gen_bc(Instr::GTEQ),
                                "LT"    => ctx.gen_bc(Instr::LT),
                                "GT"    => ctx.gen_bc(Instr::GT),
                                _       => panic!("Unknown operator")
                            }
                        }
                    }
                    "method_invocation"       => panic!("NotYetImplemented"),
                    "field_access"            => panic!("NotYetImplemented"),
                    "class_instance_creation" => panic!("NotYetImplemented"),
                    "literal" => {
                        let lit_type =  ctx.get_name(&nodes[0]);
                        let lit_value = ctx.get_value(&nodes[0]);
                        match lit_type.as_ref(){
                            "INT_LITERAL" => {
                                let int = lit_value.parse::<i32>().unwrap();
                                ctx.gen_bc(Instr::PUSH_INT(int))
                            }
                            _ => panic!("NotYetImplemented")
                        }
                    }
                    _ => panic!("unknown expression")
                }
            }
        }
    }

    //let_statement : "LET" "IDENTIFIER" "EQ" expression;
    fn gen_let(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            gen_exp(&nodes[3], ctx);
            let var_name = ctx.get_value(&nodes[1]);
            ctx.gen_bc(Instr::STORE_VAR(var_name));
        }
    }

    //if_statement : "IF" expression block;
    fn gen_if(node: &Node<u16>, ctx: &mut CompilerContext) {
        panic!("NotYetImplemented");
    }

    fn gen_for(node: &Node<u16>, ctx: &mut CompilerContext) {
        panic!("NotYetImplemented");
    }

    // func_def : "DEF" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN" block ;
    fn gen_func_def(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ nonterm_idx, ref nodes } = node {
            ctx.register_function(&nodes[1]);
            gen_params(&nodes[3], ctx);
            gen_block(&nodes[5], ctx);
        }
    }

    // parameter_list : "IDENTIFIER"
    //                | parameter_list "COMMA" "IDENTIFIER"
    //                ;
    fn gen_params(node: &Node<u16>, ctx: &mut CompilerContext) {
        match *node {
            Node::Nonterm { nonterm_idx, ref nodes } => {
                for child in nodes.iter() {
                    gen_params(child, ctx)
                }
            }
            Node::Term{ lexeme } => {
                if ctx.get_name(node) == "IDENTIFIER" {
                    ctx.register_parameter(node);
                }
            }
        }
    }

    let mut ctx = CompilerContext::new(grm, input);
    match *parse_tree {
        Node::Nonterm { nonterm_idx, ref nodes } => {
            for cls in nodes.iter() {
                gen_class(cls, &mut ctx);
            }
        }
        _ => panic!("Error")
    }
    Bytecode::new(ctx)
}

