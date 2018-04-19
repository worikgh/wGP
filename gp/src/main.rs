extern crate rand;
use rand::Rng;
use rand::SeedableRng;
use rand::StdRng;
use std::fmt;
use std::collections::HashMap;

// The type of data that can be a terminal
enum TerminalType {
    Float(f64),
    Bool(bool),
    // Custom terminals for inputs
    Inputf64(String),
    InputBool(String),
}

// Passed to Node::evaluate.  Matches custom terminals in TerminalType
struct Inputs {
    length:f64,
    diameter:f64,
    height:f64,
    dataf:HashMap<String, f64>,
    datab:HashMap<String, bool>,
}
// Get the data from the terminal
fn gt(tt:&TerminalType) -> String {
    match tt {
        &TerminalType::Float(f) => format!("Float {}",f),
        &TerminalType::Inputf64(s) => format!("{} ",s),
        &TerminalType::InputBool(s) => format!(" {} ",s),
    }
}
impl fmt::Display for TerminalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = gt(self);
        write!(f, "{}", n)
    }
}

// The arithmetic operations that are allowed
enum Operator {
    Add,
    Multiply,
    Invert,
    Negate,
    Terminal(TerminalType),
}

// The basic unit of aAST
struct Node {
    //
    o:Operator,
    l:Option<Box<Node>>,
    r:Option<Box<Node>>,
}

// The source of entropy that is passed to trees to create themselves.  With 
struct Entropy
{
    rng:StdRng,
}
impl Entropy{
    fn new(seed:&[usize]) -> Entropy {
        Entropy{rng:SeedableRng::from_seed(seed)}
    }
    fn gen(& mut self) -> f64 {
        self.rng.gen()
    }
    fn gen_range(& mut self, a:usize, b:usize)->usize{
        self.rng.gen_range(a,b)
    }
}

impl Node {

    // Build a random tree
    fn new(e:&mut Entropy) -> Node {
        
        let a = e.gen_range(0, 10);
        print!("{} ", a);
        match a {
            0 => Node{o:Operator::Add,
                      l: Some(Box::new(Node::new(e))),
                      r: Some(Box::new(Node::new(e)))},
            1 => Node{o:Operator::Multiply,
                      l: Some(Box::new(Node::new(e))),
                      r: Some(Box::new(Node::new(e)))},
            2 => Node{o:Operator::Invert,
                      l: Some(Box::new(Node::new(e))),
                      r: None},
            3 => Node{o:Operator::Negate,
                      l: Some(Box::new(Node::new(e))),
                      r: None},
            _ => {
                // A terminal type.  FIXME the types of inputs are
                // still hard coded in several places
                let n = 
                4 => Node{o:Operator::Terminal(TerminalType::dataf()),
                      l:None,
                     r:None},
            5 => Node{o:Operator::Terminal(TerminalType::Diameter(0.0)),
                      l:None,
                     r:None},
            6 => Node{o:Operator::Terminal(TerminalType::Length(0.0)),
                      l:None,
                     r:None},
            _ => Node{o:Operator::Terminal(TerminalType::Float(e.gen())),
                      l:None,
                     r:None},
        }
    }

    fn to_string(&self) -> String {
        let mut ret = "".to_string();
        match self.o {
            Operator::Add => {
                ret.push_str("Add ");
                match self.l {
                    Some(ref l) => ret.push_str(&(*l).to_string()),
                    None => panic!("{}", 1),
                };
                match self.r {
                    Some(ref r) => ret.push_str(&(*r).to_string()),
                    None => panic!("{}", 1),
                };
            },
            Operator::Multiply => {
                ret.push_str("Multiply ");
                match self.l {
                    Some(ref l) => ret.push_str(&(*l).to_string()),
                    None => panic!("{}", 1),
                };
                match self.r {
                    Some(ref r) => ret.push_str(&(*r).to_string()),
                    None => panic!("{}", 1),
                };
            },
            Operator::Negate => {
                ret.push_str("Negate ");
                match self.l {
                    Some(ref l) => ret.push_str(&(*l).to_string()),
                    None => panic!("{}", 1),
                };
            },
            Operator::Invert => {
                ret.push_str("Invert ");
                match self.l {
                    Some(ref l) => ret.push_str(&(*l).to_string()),
                    None => panic!("{}", 1),
                };
            },
            Operator::Terminal(ref f) => {
                ret.push_str(&format!("{} ", f));
            },
        };
        ret
    }

    fn evaluate(&self, inputs:&Inputs)->Option<f64> {
        macro_rules! evaluate {
            ($a:ident) => {
                match self.$a {
                    Some(ref $a) => {
                        let n = &(*$a); // Node
                        let f = n.evaluate(inputs); // Option<f64>
                        let l = f.unwrap();
                        l
                    },
                    None => panic!("Add must have left and right children") ,
                }
            }
        }
        match self.o {
            Operator::Terminal(TerminalType::Float(f)) => Some(f),
            Operator::Terminal(TerminalType::Height(_)) => Some(inputs.height),
            Operator::Terminal(TerminalType::Diameter(_)) => Some(inputs.diameter),
            Operator::Terminal(TerminalType::Length(_)) => Some(inputs.length),
            Operator::Add => {
                // FIXME Make this a macro
                let left = evaluate!(l);
                let right = evaluate!(r);
                Some(left+right)
            },
            Operator::Multiply => {
                // FIXME Make this a macro
                let left = evaluate!(l);
                let right = evaluate!(r);
                Some(left*right)
            },
            Operator::Negate => {
                // FIXME Make this a macro
                let left =
                    match self.l {
                        Some(ref l) =>{
                            let n = &(*l); // Node
                            let f = n.evaluate(inputs); // Option<f64>
                            let l = f.unwrap();
                            l
                        },
                        None => panic!("Add must have left and right children") ,
                    };
                Some(-1.0*left)
            },
            Operator::Invert => {
                // FIXME Make this a macro
                let left =
                    match self.l {
                        Some(ref l) =>{
                            let n = &(*l); // Node
                            let f = n.evaluate(inputs); // Option<f64>
                            let l = f.unwrap();
                            l
                        },
                        None => panic!("Add must have left and right children") ,
                    };
                Some(1.0/left)
            },
            }
    }
}
impl Operator {
    // Get the aritys of the operators
    // fn arity(&self) -> u8 {
    //     match &self {
    //         Add => 2,
    //         // Multiply => 2,
    //         // Invert => 1,
    //         // Negate => 1,
    //         Terminal => 0,
    //     }
    // }    
}

fn main() {
    println!("Start");
    let mut e = Entropy::new(&[11,2,3,422]);
     // let mut e = Entropy::new(&[11,2,3,4]);
    let i = Node::new(&mut e);
    println!(" Built Node");
    let inputs = Inputs{
        // Age 15
        dataf:HashMap::new(),
        datab:HashMap::new(),
    };
    inputs.dataf.insert("length".to_string(), 0.455);
    inputs.dataf.insert("diameter".to_string(), 0.365);
    inputs.dataf.insert("height".to_string(), 0.095);

    let eval = i.evaluate(&inputs).unwrap();
    println!("{}\n{}\nBye!", i.to_string(), eval);
    
}
