use parse::Bytecode;
use parse::Instr;
use std::collections::HashMap;

static GLOBAL_NSPACE: &'static str = "global";
static MAIN_FN: &'static str = "main";

#[derive(Clone)]
enum NativeType {
    Int(i32),
    Double(f32),
    Bool(bool),
    Str(String),
    ObjectRef(usize),
}

#[derive(Clone)]
struct Object {
    class_name : String,
    fields: HashMap<String, NativeType>,
}

impl Object {
    fn new(class_name: String) -> Object {
        Object {
            class_name: class_name,
            fields: HashMap::new()
        }
    }

}

pub struct VM {
    heap: Vec<Object>,
    bytecode: Bytecode,
    frames: Vec<Frame>,
    pc: usize,
}

impl VM {
    pub fn new(bytecode: Bytecode) -> VM {
        VM {
            heap: Vec::new(),
            bytecode: bytecode,
            frames: Vec::new(),
            pc: 0,
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.pc >= self.bytecode.bytecode.len() {
                break
            }
            let instr = self.bytecode.bytecode[self.pc].clone();
            match instr {
                Instr::PUSH_INT(ref x) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(NativeType::Int(x.clone()));
                    self.pc += 1
                }
                Instr::PUSH_STR(ref x) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(NativeType::Str(x.clone()));
                    self.pc += 1
                }
                Instr::POP => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.pop();
                    self.pc +=1
                }
                Instr::ADD => panic!("NotYetImplemented"),
                Instr::SUB => panic!("NotYetImplemented"),
                Instr::LTEQ => panic!("NotYetImplemented"),
                Instr::GTEQ => panic!("NotYetImplemented"),
                Instr::LT => panic!("NotYetImplemented"),
                Instr::GT => panic!("NotYetImplemented"),
                Instr::EQEQ => panic!("NotYetImplemented"),
                Instr::SWAP => panic!("NotYetImplemented"),
                Instr::DUP => panic!("NotYetImplemented"),
                Instr::LOAD_VAR(ref index) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.load_local(index);
                    self.pc += 1
                }
                Instr::STORE_VAR(ref name) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.store_local(name);
                    self.pc += 1
                }
                Instr::LOAD_GLOBAL(ref name) => panic!("NotYetImplemented"),
                Instr::STORE_GLOBAL(ref name) => panic!("NotYetImplemented"),
                Instr::NEW_OBJECT(ref class_name) => panic!("NotYetImplemented"),
                Instr::LOAD_FIELD(ref field_name) => panic!("NotYetImplemented"),
                Instr::STORE_FIELD(ref field_name) => panic!("NotYetImplemented"),
                Instr::CALL(ref class_name, ref fn_name) => panic!("NotYetImplemented"),
                Instr::JUMP_IF_TRUE(ref pos) => panic!("NotYetImplemented"),
                Instr::JUMP_IF_FALSE(ref pos) => panic!("NotYetImplemented"),
                Instr::JUMP(ref pos) => panic!("NotYetImplemented"),
                Instr::RET =>  panic!("NotYetImplemented"),
                _ => (),
            };
        }
    }

}

struct Frame {
    stack:  Vec<NativeType>,
    locals: Vec<NativeType>,
    return_address: usize
}

impl Frame {
    pub fn new(return_address: usize) -> Frame {
        Frame {
            stack: Vec::new(),
            locals: Vec::new(),
            return_address: return_address,
        }
    }

    pub fn push(&mut self, obj: NativeType) {
        self.stack.push(obj);
    }

    pub fn pop(&mut self) -> NativeType {
        match self.stack.pop() {
            Some(x) => x,
            None => panic!("Popped from empty stack!"),
        }
    }

    fn load_local(&mut self, index: &usize) {
        let value = self.locals[index.clone()].clone();
        self.push(value.clone())
    }

    fn store_local(&mut self, index: &usize) {
        let value = self.pop();
        self.locals[index.clone()] = value;
    }
}
