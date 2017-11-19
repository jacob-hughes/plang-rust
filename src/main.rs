use std::vec::Vec;

enum ExpNode {
    Integer(i32),
    Addition(Box<ExpNode>, Box<ExpNode>),
    Subtraction(Box<ExpNode>, Box<ExpNode>),
}

impl ExpNode {

    fn eval(&self, frame: &mut Frame) {
        match self {
            &ExpNode::Integer(n) => println!("Int: {}", n),
            &ExpNode::Addition(ref op0, ref op1) => {
            }
            &ExpNode::Subtraction(ref op0, ref op1) => println!("Sub"),
        }
    }
}

pub trait BoxedNumber {
    fn new <U: BoxedNumber, T> (value: T) -> U {U(value)}

    fn add <T: BoxedNumber> (&self, y: T) -> T;
}

struct BoxedInt {
    value: u32,
}

impl BoxedNumber for BoxedInt {
    fn add <T: BoxedNumber> (&self, y: T) -> T {
        return
    }
}

struct VM {
    // The VM keeps track of the current frame in the main
    // interpreter loop.
    //
    // The heap stores objects which may outlive the function
    // in which they were instantiated.
    frame:  Frame,
    prog:   ExpNode,
}

struct Frame {
    // A frame represents the layout of a function being evaluated
    // in memory. The frame has its own local stack and variable
    // namespace. A frame is de-referenced, removing both its local
    // stack and vars when it returns execution to the caller.
    caller: Box<Frame>
}

//impl Frame {
//    pub fn push(&mut self, obj: W_Obj) {
//        self.stack.push(obj);
//    }
//
//    pub fn pop(&mut self) -> W_Obj {
//        match self.stack.pop(obj) {
//            Some(x) => return x;
//            None => panic!("Popped from empty stack!");
//        }
//    }
//
//}




fn main() {
    let exp = ExpNode::Addition(Box::new(ExpNode::Integer(1)), Box::new(ExpNode::Integer(2)));
    println!("Hello, world!");
}
