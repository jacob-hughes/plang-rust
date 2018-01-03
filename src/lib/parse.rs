extern crate lrpar;
extern crate lrlex;
extern crate lrtable;
extern crate cfgrammar;

use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::convert::{TryFrom, TryInto};
use std::collections::HashMap;

use self::lrpar::parser;
use self::lrpar::parser::Node;
use self::lrlex::{build_lex};
use self::lrtable::{Minimiser, from_yacc};

use self::cfgrammar::TIdx;
use self::cfgrammar::yacc::{yacc_grm, YaccGrammar, YaccKind};

// This can be arbitrary, ultimately it doesn't matter what the placeholder's
// value is, because it is switched out almost immediately.
const PLACEHOLDER: usize = usize::max_value();

static CONSTRUCTOR: &'static str = "construct";

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
    parse_input(input, lex_path, yacc_path)
}

pub fn parse_input(source: String, lex_path: &Path, yacc_path: &Path) -> Result<Bytecode, ParseError> {
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

    let lexer = lexer_def.lexer(&source);
    let lexemes = lexer.lexemes().map_err(|_| ParseError::LexicalError)?;
    let (sgraph, stable) = from_yacc(&grm, Minimiser::Pager)
        .map_err(|_| ParseError::BrokenParser)?;

    let pt = parser::parse::<u16>(&grm, &sgraph, &stable, &lexemes)
        .map_err(|_| ParseError::SyntaxError)?;

    Ok(gen_bytecode(&pt, &grm, &source))
}

#[derive(Debug, Clone)]
pub enum Instr {
    PushInt(i32),
    PushStr(String),
    Pop,
    Add,
    Sub,
    Lteq,
    Gteq,
    Lt,
    Gt,
    Eqeq,
    Raise,
    LoadVar(usize),
    StoreVar(usize),
    LoadGlobal(String),
    StoreGlobal(String),
    NewObject,
    LoadField(String),
    StoreField(String),
    Swap,
    Dup,
    Call(String, String),
    JumpIfTrue(usize),
    JumpIfFalse(usize),
    Jump(usize),
    Ret,
    Exit,
}

#[derive(Debug)]
pub struct Fn {
    locals: Vec<String>,
    num_params: usize,
}

impl Fn {
    fn new() -> Fn {
        Fn {
            num_params: 0,
            locals: Vec::new(),
        }
    }

    pub fn params_len(&self) -> usize {
        self.num_params
    }

    pub fn locals_len(&self) -> usize {
        self.locals.len()
    }
}

// Conversion from the CompilerContext struct, removes the helper fields
// which are used for building up the symbol table and bytecode. These
// aren't needed anymore and as they are references which require a
// lifetime, their removal makes working with the struct easier.
#[derive(Debug)]
pub struct Bytecode {
    pub bytecode: Vec<Instr>,
    pub symbols: HashMap<(String, String), Fn>,
    pub labels: HashMap<(String, String), usize>,
}

impl Bytecode {
    fn new(ctx : CompilerContext) -> Bytecode {
        Bytecode {
            bytecode: ctx.bytecode,
            symbols: ctx.symbols,
            labels: ctx.labels
        }
    }
}

struct CompilerContext<'pt> {
    symbols: HashMap<(String, String), Fn>,
    bytecode: Vec<Instr>,
    labels: HashMap<(String, String), usize>,

    // Fields for convenience when building up the Bytecode struct
    grm:        &'pt YaccGrammar,
    input:      &'pt str,
    cur_cls:    String,
    cur_fn:     String,
}

impl<'pt> CompilerContext<'pt> {
    fn new(grm: &'pt YaccGrammar, input: &'pt str) -> CompilerContext<'pt> {
        CompilerContext {
            symbols: HashMap::new(),
            bytecode: Vec::new(),
            labels: HashMap::new(),
            grm:     grm,
            input:   input,
            cur_cls: "global".to_string(),
            cur_fn:  "global".to_string(),
        }
    }

    // Used when building up conditional branches and loops, where the pos. to
    // jump to is not known until all the relevant code is generated.
    fn patch(&mut self, pos: usize) {
        let patch_value = self.bytecode.len();
        let ref mut jump_instr = self.bytecode[pos];
        match *jump_instr {
            Instr::JumpIfTrue(ref mut _i) => *_i = patch_value,
            Instr::JumpIfFalse(ref mut _i) => *_i = patch_value,
            _ => panic!("Unknown jump instruction")
        }
    }

    // Makes a note of the current class, useful for generating metadata about
    // functions in the symbol table.
    fn register_class(&mut self, class: &Node<u16>) {
        match *class {
            Node::Term { .. } => {
                let class_name = self.get_value(class);
                self.cur_cls   = class_name.clone();
            }
            _ => panic!("Can only register a class on a terminal node")
        }
    }

    fn register_function(&mut self, func: &Node<u16>) -> (String, String) {
        match *func {
            Node::Term { .. } => {
                let func_name = self.get_value(func);
                self.cur_fn = func_name.clone();
                let fn_entry_point = self.bytecode.len();
                self.labels.insert((self.cur_cls.to_string(), func_name.to_string()), fn_entry_point);
                self.symbols.insert((self.cur_cls.to_string(), func_name.to_string()), Fn::new());
                return (self.cur_cls.to_string(), func_name)
            }
            _ => panic!("Can only register a func on a terminal node")
        }
    }

    // Adds the parameter name to the param vector of the current cls + func.
    fn register_parameter(&mut self, param: &Node<u16>) -> usize {
        let param_name = self.get_value(param);
        let ref key = (self.cur_cls.to_string(), self.cur_fn.to_string());
        let ref mut fn_meta = self.symbols.get_mut(key).unwrap();
        fn_meta.num_params += 1;
        fn_meta.locals.push(param_name);
        fn_meta.locals.len() - 1
    }

    fn get_var_offset(&self, var: &Node<u16>) -> usize {
        let ref var_name = self.get_value(var);
        let ref key = (self.cur_cls.to_string(), self.cur_fn.to_string());
        let ref locals = self.symbols.get(key).unwrap().locals;
        locals.iter().position(|x| x == var_name).unwrap()
    }

    fn register_local(&mut self, var: &Node<u16>) -> usize {
        let var_name = self.get_value(var);
        let ref key = (self.cur_cls.to_string(), self.cur_fn.to_string());
        let ref mut locals = self.symbols.get_mut(key).unwrap().locals;
        match locals.iter().position(|x| x == &var_name) {
            Some(x) => x,
            None => {
                locals.push(var_name);
                locals.len() - 1
            }
        }
    }

    fn gen_bc(&mut self , instr: Instr) -> usize {
        self.bytecode.push(instr);
        self.bytecode.len() - 1
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
            Node::Nonterm { nonterm_idx, .. } => {
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
       if let &Node::Nonterm { ref nodes, .. } = node {
            match ctx.get_name(node).as_ref(){
                "class_def" => {
                    ctx.register_class(&nodes[1]);
                    gen_block(&nodes[5], ctx);
                },
                "prog" => {
                    for child in nodes {
                        gen_class(child, ctx)
                    }
                }
                _ => panic!("Unknown class def")
            }
        }
    }

    // block_statements : statement
    //                  | block_statements "SEMI" statement
    //                  ;
    fn gen_block(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
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
    //           | try_except
    //           | raise
    //           ;
    fn gen_stmt(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
            match ctx.get_name(&nodes[0]).as_ref(){
                "expression"    => gen_exp(&nodes[0], ctx),
                "if_statement"  => gen_if(&nodes[0], ctx),
                "let_statement" => gen_let(&nodes[0], ctx),
                "func_def"      => gen_func_def(&nodes[0], ctx),
                "for_statement" => gen_for(&nodes[0], ctx),
                "raise"         => gen_raise(&nodes[0], ctx),
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
        if let &Node::Nonterm{ ref nodes, .. } = node {
            let exp_type = &nodes[0];
            let name = ctx.get_name(exp_type);
            if let &Node::Nonterm{ ref nodes, .. } = exp_type {
                match name.as_ref() {
                    "variable" => {
                        let var_offset = ctx.get_var_offset(&nodes[0]);
                        ctx.gen_bc(Instr::LoadVar(var_offset));
                    }
                    "binary_expression" => {
                        gen_exp(&nodes[0], ctx);
                        gen_exp(&nodes[2], ctx);
                        let bin_op = &nodes[1];
                        if let &Node::Nonterm{ref nodes, .. } = bin_op {
                            let operator = &nodes[0];
                            match ctx.get_name(operator).as_ref() {
                                "PLUS"  => ctx.gen_bc(Instr::Add),
                                "MINUS" => ctx.gen_bc(Instr::Sub),
                                "LTEQ"  => ctx.gen_bc(Instr::Lteq),
                                "GTEQ"  => ctx.gen_bc(Instr::Gteq),
                                "LT"    => ctx.gen_bc(Instr::Lt),
                                "GT"    => ctx.gen_bc(Instr::Gt),
                                "EQEQ"  => ctx.gen_bc(Instr::Eqeq),
                                _       => panic!("Unknown operator")
                            };
                        }
                    }
                    "method_invocation" => {
                        gen_args(&nodes[4], ctx);
                        let obj_name = ctx.get_value(&nodes[0]);
                        let method_name = ctx.get_value(&nodes[2]);
                        ctx.gen_bc(Instr::Call(obj_name, method_name));
                    },
                    "method_invocation_same_class" => {
                        gen_args(&nodes[2], ctx);
                        let obj_name = ctx.cur_cls.clone();
                        let method_name = ctx.get_value(&nodes[0]);
                        ctx.gen_bc(Instr::Call(obj_name, method_name));
                    },
                    "field_access" => {
                        let obj_alias = ctx.get_var_offset(&nodes[0]);
                        let field_name = ctx.get_value(&nodes[2]);
                        ctx.gen_bc(Instr::LoadVar(obj_alias));
                        ctx.gen_bc(Instr::LoadField(field_name));
                    },
                    "field_set" => {
                        gen_exp(&nodes[4], ctx);
                        let obj_alias = ctx.get_var_offset(&nodes[0]);
                        let field_name = ctx.get_value(&nodes[2]);
                        ctx.gen_bc(Instr::LoadVar(obj_alias));
                        ctx.gen_bc(Instr::StoreField(field_name));
                    },
                    "class_instance_creation" => {
                        let cls_name = ctx.get_value(&nodes[1]);
                        ctx.gen_bc(Instr::NewObject);
                        ctx.gen_bc(Instr::Dup);
                        gen_args(&nodes[3], ctx);
                        ctx.gen_bc(Instr::Call(cls_name, CONSTRUCTOR.to_string()));
                        ctx.gen_bc(Instr::Pop); // remove returned NoneType, leaving obj instance
                    },
                    "literal" => {
                        let lit_type =  ctx.get_name(&nodes[0]);
                        let lit_value = ctx.get_value(&nodes[0]);
                        match lit_type.as_ref(){
                            "INT_LITERAL" => {
                                let int = lit_value.parse::<i32>().unwrap();
                                ctx.gen_bc(Instr::PushInt(int))
                            }
                            "STR_LITERAL" => {
                                ctx.gen_bc(Instr::PushStr(lit_value))
                            }
                            _ => panic!("NotYetImplemented")
                        };
                    }
                    _ => panic!("unknown expression")
                }
            }
        }
    }

    // arg_list_opt :
    //              | arg_list
    //              ;

    // arg_list : expression
    //          | parameter_list "COMMA" expression
    //          ;
    fn gen_args(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm { ref nodes, .. } = node {
            for child in nodes.iter() {
                match ctx.get_name(child).as_ref() {
                    "arg_list" => gen_args(child, ctx),
                    "expression" => gen_exp(child, ctx),
                    "COMMA" => (),
                    _ => panic!("Illegal node found in arg list")
                }
            }
        }
    }

    //let_statement : "LET" "IDENTIFIER" "EQ" expression;
    fn gen_let(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
            gen_exp(&nodes[3], ctx);
            let var_index = ctx.register_local(&nodes[1]);
            ctx.gen_bc(Instr::StoreVar(var_index));
        }
    }

    //raise : "RAISE";
    fn gen_raise(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{..} = node {
            ctx.gen_bc(Instr::Raise);
        }
    }

    //if_statement : "IF" expression block;
    fn gen_if(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
            gen_exp(&nodes[1], ctx);
            let pos = ctx.gen_bc(Instr::JumpIfFalse(PLACEHOLDER));
            gen_block(&nodes[2], ctx);
            ctx.patch(pos);
        }
    }

    //for_statement : "FOR" "LPAREN" statement "SEMI" expression "SEMI" statement "RPAREN" block;
    fn gen_for(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
            gen_stmt(&nodes[2], ctx);
            // Loop begins
            let loop_entry = ctx.bytecode.len();
            gen_exp(&nodes[4], ctx); // conditional
            let exit_call = ctx.gen_bc(Instr::JumpIfFalse(PLACEHOLDER));
            gen_block(&nodes[8], ctx); // loop body
            gen_stmt(&nodes[6], ctx); // step
            ctx.gen_bc(Instr::Jump(loop_entry));
            ctx.patch(exit_call);
        }
    }

    // func_def : "DEF" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN" block ;
    fn gen_func_def(node: &Node<u16>, ctx: &mut CompilerContext) {
        if let &Node::Nonterm{ ref nodes, .. } = node {
            let (cls_name, fn_name) = ctx.register_function(&nodes[1]);
            gen_params(&nodes[3], ctx);
            gen_block(&nodes[5], ctx);
            if (cls_name, fn_name) == ("global".to_string(), "main".to_string()) {
                ctx.gen_bc(Instr::Exit);
            }
            else {
                ctx.gen_bc(Instr::Ret);
            }
        }
    }

    // parameter_list : "IDENTIFIER"
    //                | parameter_list "COMMA" "IDENTIFIER"
    //                ;
    fn gen_params(node: &Node<u16>, ctx: &mut CompilerContext) {
        match *node {
            Node::Nonterm { ref nodes, ..} => {
                for child in nodes.iter() {
                    gen_params(child, ctx)
                }
            }
            Node::Term{..} => {
                if ctx.get_name(node) == "IDENTIFIER" {
                    ctx.register_parameter(node);
                }
            }
        }
    }

    let mut ctx = CompilerContext::new(grm, input);
    match *parse_tree {
        Node::Nonterm { ref nodes, .. } => {
            for cls in nodes.iter() {
                gen_class(cls, &mut ctx);
            }
        }
        _ => panic!("Error")
    }
    Bytecode::new(ctx)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use parse::{parse_input, Bytecode};
    const LEX_PATH: &str = "grammar/lexer.l";
    const YACC_PATH: &str = "grammar/grammar.y";

    fn build_bytecode(source: String) -> Bytecode {
        let lex_path = Path::new(LEX_PATH);
        let yacc_path = Path::new(YACC_PATH);
        parse_input(source, &lex_path, &yacc_path).unwrap()
    }
    // tests go here
}
