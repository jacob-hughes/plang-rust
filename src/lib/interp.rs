use parse::Bytecode;
use parse::Instr;
use std::collections::HashMap;

static GLOBAL_NSPACE: &'static str = "global";
static MAIN_FN: &'static str = "main";
const EXCEPTION_PTR: usize = 0;

#[derive(Debug, Clone)]
pub enum NativeType {
    Int(i32),
    Double(f32),
    Bool(bool),
    Str(String),
    ObjectRef(usize),
    NoneType,
}

impl NativeType {
    fn pretty(&self) -> String {
        match *self {
            NativeType::Int(ref x) => x.to_string(),
            NativeType::Double(ref x) => x.to_string(),
            NativeType::Bool(ref x) => x.to_string(),
            NativeType::Str(ref x) => x.to_string(),
            NativeType::ObjectRef(ref x) => format!("&{}",x.to_string()),
            NativeType::NoneType => "None".to_string()
        }
    }
}

#[derive(Clone)]
struct Object {
    fields: HashMap<String, NativeType>,
}

impl Object {
    fn new() -> Object {
        Object {
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
            let bytecode_size = self.bytecode.bytecode.len();
            if self.pc >= bytecode_size {
                let frame = self.frames.last_mut();
                match frame {
                    Some(x) => {
                        match x.peek() {
                            Some(y) => result = Some(y.clone()),
                            None => (),
                        }
                    }
                    None => (),
                }
                break
            }
            match *&self.bytecode.bytecode[self.pc] {
                Instr::PushInt(ref x) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(NativeType::Int(x.clone()));
                    self.pc += 1
                }
                Instr::PushStr(ref x) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(NativeType::Str(x.clone()));
                    self.pc += 1
                }
                Instr::Pop => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.pop();
                    self.pc +=1
                }
                Instr::Dup => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.dup();
                    self.pc += 1
                }
                Instr::Add => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.add();
                    self.pc +=1
                }
                Instr::Sub => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.sub();
                    self.pc +=1
                }
                Instr::Lteq => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.lteq();
                    self.pc +=1
                }
                Instr::Gteq =>{
                    let frame = self.frames.last_mut().unwrap();
                    frame.gteq();
                    self.pc +=1
                }
                Instr::Lt =>{
                    let frame = self.frames.last_mut().unwrap();
                    frame.lt();
                    self.pc +=1
                }
                Instr::Gt =>{
                    let frame = self.frames.last_mut().unwrap();
                    frame.gt();
                    self.pc +=1
                }
                Instr::Eqeq =>{
                    let frame = self.frames.last_mut().unwrap();
                    frame.eq();
                    self.pc +=1
                }
                Instr::LoadVar(index) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.load_local(index);
                    self.pc += 1
                }
                Instr::StoreVar(name) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.store_local(name);
                    self.pc += 1
                }
                Instr::Raise => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.raise("Exception");
                }
                Instr::LoadGlobal(ref _name) => panic!("NotYetImplemented"),
                Instr::StoreGlobal(ref _name) => panic!("NotYetImplemented"),
                Instr::NewObject => {
                    let obj = Object::new();
                    self.heap.push(obj);
                    let obj_ref = self.heap.len() - 1;
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(NativeType::ObjectRef(obj_ref));
                    self.pc += 1
                },
                Instr::LoadField(ref field_name) => {
                    let frame = self.frames.last_mut().unwrap();
                    let obj_ref = frame.pop();
                    let obj = match obj_ref {
                        NativeType::ObjectRef(x) => self.heap.get(x).unwrap(),
                        _ => panic!("Not a valid object")
                    };
                    let field = obj.fields.get(field_name)
                        .expect("Field not found");
                    frame.push(field.clone());
                    self.pc += 1
                }
                Instr::StoreField(ref field_name) => {
                    let frame = self.frames.last_mut().unwrap();
                    let obj_ref = frame.pop();
                    let value = frame.pop();
                    match obj_ref {
                        NativeType::ObjectRef(x) => {
                            let obj = self.heap.get_mut(x).unwrap();
                            obj.fields.insert(field_name.to_string(), value);
                        }
                        _ => panic!("Not a valid object")
                    };
                    self.pc += 1
                },
                Instr::JumpIfTrue(pos) => {
                    let frame = self.frames.last_mut().unwrap();
                    if let NativeType::Bool(true) = frame.pop() {
                        self.pc = pos
                    }
                    else {
                        self.pc += 1
                    }
                },
                Instr::JumpIfFalse(pos) => {
                    let frame = self.frames.last_mut().unwrap();
                    if let NativeType::Bool(false) = frame.pop() {
                        self.pc = pos
                    }
                    else {
                        self.pc += 1
                    }
                },
                Instr::Jump(pos) => self.pc = pos,
                Instr::Call(ref class_name, ref fn_name) => {
                    let ref key = (class_name.to_string(), fn_name.to_string());
                    let fn_metadata = self.bytecode.symbols.get(&key.clone())
                        .expect("Function not found");
                    let mut locals = {
                        let frame = self.frames.last_mut().unwrap();
                        let mut locals = Vec::new();
                        for _ in 0..fn_metadata.params_len() {
                            locals.push(frame.pop())
                        }
                        locals
                    };
                    locals.reverse(); // TODO: This can be more efficient if we rework
                                    // this to add args in reverse order in place
                    let new_frame = Frame::new(fn_name.to_string(), locals, self.pc + 1);
                    self.frames.push(new_frame);
                    self.pc = self.bytecode.labels.get(key).unwrap().clone();
                },
                Instr::Ret => {
                    let (return_value, return_address) =  {
                        let frame = self.frames.last_mut().unwrap();
                        let ret_val = if frame.stack.len() > 0 {
                            frame.pop()
                        }
                        else {
                            NativeType::NoneType
                        };
                        (ret_val, frame.return_address)
                    };
                    self.frames.pop();
                    let frame = self.frames.last_mut().unwrap();
                    frame.push(return_value);
                    self.pc = return_address;
                },
                Instr::Exit => {
                    let frame = self.frames.last_mut().unwrap();
                    result = match frame.peek() {
                        Some(x) => Some(x.clone()),
                        None => None
                    };
                    break
                }
                _ => panic!("InstrNotImplemented"),
            };
            self.unwind_stack_on_raise();
        }
        result
    }

    fn enter_main(&mut self) {
        self.pc = self.bytecode.labels.get(
            &(GLOBAL_NSPACE.to_string(), MAIN_FN.to_string()))
            .expect("Main method not found").clone();
        self.frames.push(Frame::new("main".to_string(), Vec::new(), self.bytecode.bytecode.len()))
    }

    fn unwind_stack_on_raise(&mut self) {
        if self.frames.last().unwrap().raise {
            let mut backtrace: Vec<NativeType> = Vec::new();
            let mut try_index: usize = self.frames.len() - 1;
            for (i, f) in self.frames.iter().rev().enumerate() {
                if f.in_try {
                    try_index = i;
                    break
                }
                else {
                    backtrace.push(NativeType::Str(f.name.to_string()));
                }
            }
            let try_index = self.frames.len() - try_index - 1;
            self.frames.drain(try_index..);
            match self.frames.last() {
                Some(ref x) => self.pc = x.return_address, //FIXME: WRONG
                None => {
                    eprintln!("Exception raised. Backtrace:");
                    eprintln!("{:?}", backtrace);
                    self.pc = usize::max_value()
                }
            }
        }
    }
}

struct Frame {
    stack:  Vec<NativeType>,
    locals: Vec<NativeType>,
    return_address: usize,
    raise: bool,
    in_try: bool,
    name: String
}

impl Frame {
    fn new(name: String, locals: Vec<NativeType>, return_address: usize) -> Frame {
        Frame {
            stack: Vec::new(),
            locals: locals,
            return_address: return_address,
            raise: false,
            in_try: false,
            name: name
        }
    }

    fn push(&mut self, obj: NativeType) {
        self.stack.push(obj);
    }

    fn pop(&mut self) -> NativeType {
        match self.stack.pop() {
            Some(x) => x,
            None => panic!("Popped from empty stack!"),
        }
    }

    fn dup(&mut self) {
        let tos = self.peek().unwrap().clone();
        self.push(tos)
    }

    fn peek(&mut self) -> Option<&NativeType> {
         self.stack.last()
    }

    fn load_local(&mut self, index: usize) {
        let value = self.locals[index].clone();
        self.push(value)
    }

    fn store_local(&mut self, index: usize) {
        let value = self.pop();
        let len = self.locals.len();
        if index < len {
            self.locals[index] = value;
        }
        else {
            assert_eq!(index, len);
            self.locals.push(value)
        }
    }

    fn raise(&mut self, msg: &str) {
        self.push(NativeType::ObjectRef(EXCEPTION_PTR));
        self.push(NativeType::Str(msg.to_string()));
        self.raise = true
    }

    fn add(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Int(x+y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Double(x as f32 + y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Double(x + y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Double(x+y)),
            _ => self.raise("TypeError"),
        }
    }

    fn sub(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Int(x-y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Double(x as f32 - y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Double(x - y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Double(x-y)),
            _ => self.raise("TypeError"),
        }
    }

    fn lteq(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Bool(x<=y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Bool(x as f32 <= y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Bool(x <= y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Bool(x<=y)),
            _ => self.raise("TypeError"),
        }
    }

    fn lt(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Bool(x<y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Bool((x as f32) < y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Bool(x < (y as f32))),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Bool(x<y)),
            _ => self.raise("TypeError"),
        }
    }

    fn gt(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Bool(x>y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Bool((x as f32) > y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Bool(x > (y as f32))),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Bool(x>y)),
            _ => self.raise("TypeError"),
        }
    }

    fn gteq(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Bool(x>=y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Bool(x as f32 >= y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Bool(x >= y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Bool(x>=y)),
            _ => self.raise("TypeError"),
        }
    }

    pub fn eq(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Bool(x==y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Bool(x as f32 == y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Bool(x == (y as f32))),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Bool(x==y)),
            _ => self.raise("TypeError"),
        }
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
