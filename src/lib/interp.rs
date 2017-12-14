use parse::Bytecode;
use parse::Instr;
use std::collections::HashMap;

static GLOBAL_NSPACE: &'static str = "global";
static MAIN_FN: &'static str = "main";

#[derive(Debug, Clone)]
pub enum NativeType {
    Int(i32),
    Double(f32),
    Bool(bool),
    Str(String),
    ObjectRef(usize),
}

impl NativeType {
    fn pretty(&self) -> String {
        match *self {
            NativeType::Int(ref x) => x.to_string(),
            NativeType::Double(ref x) => x.to_string(),
            NativeType::Bool(ref x) => x.to_string(),
            NativeType::Str(ref x) => x.to_string(),
            NativeType::ObjectRef(ref x) => x.to_string(),
        }
    }
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

    pub fn run(&mut self) -> Option<NativeType> {
        self.enter_main();
        let mut result = None;
        loop {
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
                Instr::LOAD_VAR(index) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.load_local(index);
                    self.pc += 1
                }
                Instr::STORE_VAR(name) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.store_local(name);
                    self.pc += 1
                }
                Instr::LOAD_GLOBAL(ref name) => panic!("NotYetImplemented"),
                Instr::STORE_GLOBAL(ref name) => panic!("NotYetImplemented"),
                Instr::NEW_OBJECT(ref class_name) => panic!("NotYetImplemented"),
                Instr::LOAD_FIELD(ref field_name) => panic!("NotYetImplemented"),
                Instr::STORE_FIELD(ref field_name) => panic!("NotYetImplemented"),
                Instr::CALL(ref class_name, ref fn_name) => {
                    let ref key = (class_name.to_string(), fn_name.to_string());
                    let fn_metadata = self.bytecode.symbols.get(&key.clone())
                        .expect("Function not found");
                    let mut new_frame = Frame::new(self.pc + 1);
                    {
                        let frame = self.frames.last_mut().unwrap();
                        for i in 0..fn_metadata.params_len() {
                            new_frame.locals.push(frame.pop())
                        }
                    }
                    self.frames.push(new_frame);
                    self.pc = self.bytecode.labels.get(key).unwrap().clone();
                },
                Instr::JUMP_IF_TRUE(ref pos) => panic!("NotYetImplemented"),
                Instr::JUMP_IF_FALSE(ref pos) => panic!("NotYetImplemented"),
                Instr::JUMP(ref pos) => panic!("NotYetImplemented"),
                Instr::RET => {
                    let (return_value, return_address) =  {
                        let frame = self.frames.last_mut().unwrap();
                        (frame.pop(), frame.return_address)
                    };
                    self.frames.pop();
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(return_value);
                    self.pc = return_address;
                },
                Instr::EXIT => {
                    let frame = self.frames.last_mut().unwrap();
                    result = match frame.peek() {
                        Some(x) => Some(x.clone()),
                        None => None
                    };
                    break
                }
                _ => (),
            };
        }
        result
    }

    fn enter_main(&mut self) {
        self.pc = self.bytecode.labels.get(
            &(GLOBAL_NSPACE.to_string(), MAIN_FN.to_string()))
            .expect("Main method not found").clone();
        self.frames.push(Frame::new(self.bytecode.bytecode.len()))
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

    pub fn peek(&mut self) -> Option<&NativeType> {
         self.stack.last()
    }

    fn load_local(&mut self, index: usize) {
        let value = self.locals[index].clone();
        self.push(value)
    }

    fn store_local(&mut self, index: usize) {
        let value = self.pop();
        self.locals[index] = value;
    }
}

pub fn run(bytecode: Bytecode) -> String {
    let mut vm = VM::new(bytecode);
    let res = vm.run();
    match res {
        Some(ref x) => x.pretty(),
        None => "".to_string(),
    }
}
