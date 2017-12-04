#![allow(dead_code)]
use std::vec::Vec;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone)]
pub enum Node {
    Integer(i32),
    Bool(bool),
    Str(String),
    Var(String),
    Let(String, Box<Node>),
    Addition(Box<Node>, Box<Node>),
    Subtraction(Box<Node>, Box<Node>),
    Block(Vec<Node>),
    If { cond: Box<Node>, body: Box<Node> },
    For { start: Box<Node>, cond: Box<Node>, step: Box<Node>, body: Box<Node> },
    Class { name: String, methods: HashMap<String,Node> },
    NewObj { class_name: String, fields: Option<HashMap<String, Box<Node>>> },
    SetObjField { var: Box<Node>, field: String, value: Box<Node> },
    GetObjField { var: Box<Node>, field: String },
    Function { name: String, args: Vec<Node>, body: Box<Node>},
}

impl Node {
    fn eval(&self, frame: &mut Frame) {
        match self {
            &Node::Integer(x)   => frame.push_const(NativeType::Int(x)),
            &Node::Bool(x)      => frame.push_const(NativeType::Bool(x)),
            &Node::Str(ref x)   => frame.push_const(NativeType::Str(x.clone())),
            &Node::Var(ref x)   => frame.load_var(x.clone()),
            &Node::Let(ref x, ref exp)   =>  {
                exp.eval(frame);
                frame.store_var(x.clone());
            }
            &Node::Addition(ref op0, ref op1) => {
                op0.eval(frame);
                op1.eval(frame);
                frame.add();
            }
            &Node::Subtraction(ref op0, ref op1) => {
                op0.eval(frame);
                op1.eval(frame);
                frame.sub();
            }
            &Node::Block(ref stmts) => {
                for stmt in stmts {
                    stmt.eval(frame)
                }
            }
            &Node::If{ref cond, ref body} => {
                cond.eval(frame);
                match frame.pop() {
                    NativeType::Bool(true) => body.eval(frame),
                    NativeType::Bool(false) => {},
                    _ => panic!("If conditional expected Bool"),
                }
            }
            &Node::For{ref start, ref cond, ref step, ref body} => {
                start.eval(frame);
                loop {
                    cond.eval(frame);
                    match frame.pop() {
                        NativeType::Bool(true) => {
                            body.eval(frame);
                            step.eval(frame);
                        }
                        NativeType::Bool(false) => break,
                        _ => panic!("Loop conditional expected Bool"),
                    }
                }
            }
            &Node::NewObj{ref class_name, ref fields} => {
                frame.create_object(class_name);
            }
            &Node::SetObjField{ref var, ref field, ref value} => {
                value.eval(frame);
                var.eval(frame);
                frame.set_field(field.clone());
            }
            &Node::GetObjField{ref var, ref field} => {
                var.eval(frame);
                frame.get_field(field.clone());
            }
            &Node::Class{ref name, ref methods} => {
                frame.register_class(name.clone(), methods.to_owned());
            }
            _ => panic!("Not yet implemented"),
        }
    }
}


// OBJECT REPRESENTATION IN MEMORY
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
    fields: HashMap<String, NativeType>,
}

impl Object {
    pub fn new() -> Object {
        Object {
            fields: HashMap::new(),
        }
    }
}

impl fmt::Display for NativeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &NativeType::Int(ref x) => write!(f, "{}", x),
            &NativeType::Str(ref x) => write!(f, "{}", x),
            &NativeType::Double(ref x) => write!(f, "{}", x),
            &NativeType::Bool(ref x) => write!(f, "{}", x),
            _ => write!(f, "Object"),
        }
    }
}

#[derive(Clone)]
struct Class {
    methods: HashMap<String, Node>,
}

impl Class {
    pub fn new(methods: HashMap<String, Node>) -> Class {
        Class {
            methods: methods
        }
    }
}

struct VM {
    heap: Vec<Object>,
    classes: HashMap<String, Class>
}

impl VM {
    pub fn interp(&mut self, instr: &Node) -> NativeType {
        let mut frame = Frame::new(self);
        instr.eval(&mut frame);
        frame.pop()
    }

    pub fn get_mut_obj(&mut self, heap_ref: &NativeType) -> &mut Object {
        match heap_ref {
            &NativeType::ObjectRef(x) => self.heap.get_mut(x)
                .expect("Obj not found"),
            _ => panic!("Invalid heap reference.")
        }
    }

    pub fn get_obj(&mut self, heap_ref: &NativeType) -> &Object {
        match heap_ref {
            &NativeType::ObjectRef(x) => self.heap.get(x)
                .expect("Obj not found"),
            _ => panic!("Invalid heap reference.")
        }
    }

    pub fn alloc(&mut self, obj: Object) -> NativeType {
        self.heap.push(obj);
        return NativeType::ObjectRef(self.heap.len() - 1);
    }

    pub fn new() -> VM {
        VM {
            heap: Vec::new(),
            classes: HashMap::new()
        }
    }


}

struct Frame<'a> {
    // A frame represents the layout of a function being evaluated
    // in memory. The frame has its own local stack and variable
    // namespace. A frame is de-referenced, removing both its local
    // stack and vars when it returns execution to the caller.
    stack:  Vec<NativeType>,
    next:   Option<Box<Frame<'a>>>,
    locals: HashMap<String, NativeType>,

    vm: &'a mut VM

}

impl<'a> Frame<'a> {
    pub fn new(vm: &'a mut VM) -> Frame<'a> {
        Frame {
            next: None,
            stack: Vec::new(),
            locals: HashMap::new(),
            vm: vm
        }
    }
    pub fn create_next(&'a mut self) {
        self.next = Some(Box::new(Frame::new(self.vm)));
    }

    pub fn push(&mut self, obj: NativeType) {
        self.stack.push(obj);
    }

    pub fn push_const(&mut self, const_val: NativeType) {
        self.stack.push(const_val)
    }

    pub fn pop(&mut self) -> NativeType {
        match self.stack.pop() {
            Some(x) => x,
            None => panic!("Popped from empty stack!"),
        }
    }

    pub fn load_var(&mut self, var_name: String) {
        let value = self.locals.get(&var_name)
            .expect("Variable undefined").clone(); //FIXME Re-evaluate use of clone
        self.push(value)
    }

    pub fn store_var(&mut self, var_name: String) {
        let value = self.pop();
        self.locals.insert(var_name, value);
    }

    pub fn create_object(&mut self, class_name: &String) {
        let obj_ref = self.vm.alloc(Object::new());
        self.push(obj_ref)
    }

    pub fn set_field(&mut self, field: String) {
        let obj_ref = self.pop();
        let value = self.pop();
        let obj = self.vm.get_mut_obj(&obj_ref);
        obj.fields.insert(field, value);
    }

    pub fn get_field(&mut self, field: String) {
        let obj_ref = self.pop();
        let value = self.vm.get_mut_obj(&obj_ref).fields.get(&field)
            .expect("Field Not Found").clone();
        self.push(value)
    }

    pub fn add(&mut self) {
        let lhs = self.pop();
        let rhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Int(x+y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Double(x as f32 + y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Double(x + y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Double(x+y)),
            _ => panic!("TypeError"),
        }
    }

    pub fn sub(&mut self) {
        let lhs = self.pop();
        let rhs = self.pop();
        match (lhs, rhs) {
            (NativeType::Int(x), NativeType::Int(y))        => self.push(NativeType::Int(x-y)),
            (NativeType::Int(x), NativeType::Double(y))     => self.push(NativeType::Double(x as f32 - y)),
            (NativeType::Double(x), NativeType::Int(y))     => self.push(NativeType::Double(x - y as f32)),
            (NativeType::Double(x), NativeType::Double(y))  => self.push(NativeType::Double(x-y)),
            _ => panic!("TypeError"),
        }

    }

    pub fn register_class(&mut self, name: String, methods: HashMap<String,Node>) {
        self.vm.classes.insert(name, Class::new(methods));
    }


}


fn main() {
    let exp = Node::Addition(Box::new(Node::Integer(7)), Box::new(Node::Integer(2)));
    let if_exp = Node::If{cond:Box::new(Node::Bool(false)), body: Box::new(Node::Integer(3))};

    // Test block with assignment
    let mut stmts = Vec::new();

    let new_obj = Node::NewObj{class_name: String::from("Obj"), fields: None};
    let setter = Node::SetObjField{
        var: Box::new(Node::Var(String::from("x"))),
        field: String::from("hello"),
        value: Box::new(Node::Integer(10))
    };
    let getter = Node::GetObjField{
        var: Box::new(Node::Var(String::from("x"))),
        field: String::from("hello"),
    };
    stmts.push(Node::Let(String::from("x"), Box::new(new_obj)));
    stmts.push(setter);
    stmts.push(getter);
    let block = Node::Block(stmts);
    let mut vm = VM::new();
    let res = vm.interp(&block);
    println!("{}", res)
}

