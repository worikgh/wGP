// The basic unit of a AST
pub type NodeBox = Box<Node>;
use std::fmt;
use std::usize;
use std::f64;
//use std::f32;
use rng;
//use rand::distributions::{Distribution, Uniform};
//use super::Operator;
use inputs::Inputs;

// The type of data that can be a terminal
#[derive(Debug, Clone)]
enum TerminalType {
    Float(f64),
    // Custom terminals for inputs

    // Input from the outside world comes as strings.  Always (at time
    // of writing) a f64. The string names the input field that
    // contains the value
    Inputf64(String),
}

// Get the data from the terminal
fn gt(tt:&TerminalType) -> String {
    match tt {
        &TerminalType::Float(f) => format!("Float {} ",f),
        &TerminalType::Inputf64(ref s) => format!("{} ",s),
    }
}

impl fmt::Display for TerminalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = gt(self);
        write!(f, "{}", n)
    }
}

// The operations that are implemented
#[derive(Debug, Clone)]
enum Operator {
    Add,  
    Log,  
    Multiply,

    // ??! Invert 0.0?  If a individual tries to use `invert` on 0.0
    // it will die... FIXME Is that right? 
    Invert,
    
    Negate,
    If,
    Gt, // >
    Lt, // <
    Remainder, // %
    Terminal(TerminalType),
}
fn _rand_f64() -> f64 {
    // Return a float that is -inf and +inf biased to numbers around 0
    let x = rng::gen_range(0.0, 1.0) as f64;
    let base = 2.0; // This is the shape of the distribution
    let scale = 10.0; // Scales results

    // With base == 2.0 and scale == 10.0 for 42,248 samples got the data described as (R):
    // Min.   :-131.53423  
    // 1st Qu.: -10.13796  
    // Median :  -0.03012  
    // Mean   :  -0.02720  
    // 3rd Qu.:  10.19232  
    // Max.   : 207.13013  
    
    if x < 0.5 {
        (2.0_f64*x).log(base)*scale
    }else{
        -1.0_f64*(2.0_f64*(-1.0_f64*(x-0.5_f64)+0.5_f64)).log(base) * scale
    }
}
#[derive(Debug, Clone)]
pub struct Node {
    // Operator and a left and right child trees, or None.  
    o:Operator,
    pub l:Option<NodeBox>,
    pub r:Option<NodeBox>,
    pub d:Option<NodeBox>, // The decision leg for if
}

impl Node {
    #[allow(dead_code)]
    pub fn new_from_string(s:&str) -> Node {
        let mut iter = s.split_whitespace();
        Node::new_from_iter(&mut iter)
    }
    #[allow(dead_code)]
    fn new_from_iter(iter:&mut std::str::SplitWhitespace) -> Node{
        
        let operator = match iter.next().unwrap() {
            "Add" => Operator::Add,
            "Log" => Operator::Log,
            "Multiply" => Operator::Multiply,
            "Invert" => Operator::Invert,
            "Negate" => Operator::Negate,
            "If" => Operator::If,
            "Gt" => Operator::Gt,
            "Remainder" => Operator::Remainder,
            "Lt" => Operator::Lt,
            "Float" =>
            {
                let s = iter.next().unwrap();
                let s = s.parse::<f64>().unwrap();
                Operator::Terminal(TerminalType::Float(s))
            },
            s => Operator::Terminal(TerminalType::Inputf64(s.to_string())),
        };

        let d = match operator {
            Operator::If =>
                Some(NodeBox::new(Node::new_from_iter(iter))),
            _ => None,
        };
        let l = match operator {
            Operator::Terminal(_) => None,
            _ => Some(NodeBox::new(Node::new_from_iter(iter))),
        };
        let r = match operator {
            Operator::Add|Operator::Multiply|Operator::If|
            Operator::Gt|Operator::Remainder|Operator::Lt =>
                Some(NodeBox::new(Node::new_from_iter(iter))),
            _ => None,
        };
        
        Node{o:operator, l:l, r:r, d:d}
    }    // Build a random tree
    /* Paramaters:
     * entropy - A source of randomness
     * names - The names of the input fields
     * level - The distance from the root node for this node
     */
    pub fn new(names:&Vec<String>, level:usize) -> Node {
        let l = level+1;

        macro_rules! NewNode {
            // Create a Node (in a Box).  The first argument is the
            // name of the node operator the second is 1,2, or 3 that
            // sets the number of children.  (FIXME The number of
            // children depends on the operator so $c depends on
            // $name)
            ($name:ident, $c:expr) => {
                {
                    let mut ret = Node{o:Operator::$name,
                                       l: None,
                                       r: None,
                                       d: None,
                    };
                    if $c > 0 {
                        ret.l = Some(Box::new(Node::new(names, l)));
                    }
                    if $c > 1 {
                        ret.r = Some(Box::new(Node::new(names, l)));
                    }
                    if $c > 2 {
                        ret.d = Some(Box::new(Node::new(names, l)));
                    }
                    ret
                }
            }
        };

        // FIXME Make this max level a configurable constant
        let maxlevel = 10;
        let a = if level > maxlevel { 
            0
        }else{
            //  This controlls how many input nodes there are
            //  (probabilistically). For a <= 9 ithe node will be a
            //  internal function (there are ten internal node types:
            //  0 => constant to 9 => if.  FIXME Make nmax
            //  configurable
            let nmax = 18;
            rng::gen_range(0, nmax)
        };
        match a {
            0 => Node{o:Operator::Terminal(TerminalType::Float(_rand_f64())), l:None, r:None, d:None},
            1 => NewNode!(Log,1),
            2 => NewNode!(Invert,1),
            3 => NewNode!(Negate,1),
            4 => NewNode!(Multiply,2),
            5 => NewNode!(Gt,2),
            6 => NewNode!(Lt,2),
            7 => NewNode!(Add,2),
            8 => NewNode!(Remainder,2),
            9 => NewNode!(If,3),
            _ => {
                // Input node
                let n = names.len();
                let b = rng::gen_range(0, n);
                let s = names[b].clone();
                Node{o:Operator::Terminal(TerminalType::Inputf64(s)), l:None, r:None, d:None}
            }
        }
    }

    /// Recursive count of child nodes
    pub fn count_nodes(&self) -> usize {
        
        let dc = match self.d {
            Some(ref n) => n.count_nodes(),
            None => 0,
        };
        let lc = match self.l {
            Some(ref n) => n.count_nodes(),
            None => 0,
        };
        let rc = match self.r {
            Some(ref n) => n.count_nodes(),
            None => 0,
        };
        dc + lc + rc + 1
    }

    /// Select a random node from the tree
    pub fn random_node(&self) -> NodeBox {
        // Choose a subtree (node) of this tree (node).  FIXME there
        // is a lot of optimisation to be done.  Paticularly if each
        // node had the number of nodes that are child nodes of this...

        // Select the node by first counting all nodes in the tree
        // then randomly selecting a number < the total
        let c = self.count_nodes();
        let mut n = rng::gen_range(0, c);
        
        // At the end of the loop `node` will be the selected Node.
        let mut node:& Node = self; 
        loop {

            // Loop invariant n >= 0 Exit when a node with no left or
            // right children is encountered or n == 0
            if n == 0 {
                break;
            }

            // Check for children
            let fl = match node.l{Some(_) => false, None => true};
            let fr = match node.r{Some(_) => false, None => true};
            if fl && fr {
                // No children (if there is a 'd' child there must be
                // 'l'and 'r') or n is one so we have arrived at the
                // node selected by the e.gen_range statement above

                // FIXME n cannot be zero.  The comment above talks of
                // n == 1.  Why?
                if n != 0 {
                    panic!("Node: {} n {}", self.to_string(), n);
                }
                break;
            }

            // Children in decision node
            let dc = match node.d {
                Some(ref d) => (*d).count_nodes(),
                None => 0,
            };

            if dc >= n {
                // Wanted node is in decision sub-tree

                if let Some(ref nd) = node.d {
                    node = &*nd;

                    // Subtract the current (consumed) node from n
                    n -= 1;
                    
                    continue // FIXME is this needed?
                }else{
                    // There is no decision node.  This code should
                    // never be reached.
                    panic!("dc {} n {} operator {:?} ",
                           dc, n, self.to_string());
                }
            } // dc >= n

            // Is node in left subtree?
            let lc = match node.l {
                Some(ref q) => (*q).count_nodes(),
                None => 0,
            };

            if dc+lc >= n {
                // Get node from left sub tree

                if let Some(ref nd) =  node.l {
                    node = &*nd;

                    // Have consumed all nodes in decision tree
                    // and this node.
                    n -= 1+dc;
                    
                    continue // FIXME is this needed?
                }else{
                    // There is no left sub-tree.  This code should
                    // never be reached.
                    panic!("lc {} n {} operator {:?} ",
                           lc, n, self.to_string())
                }
            }else{

                // Get node from right subtree
                if let Some(ref nd) = node.r {
                    node = &*nd;

                    // Have consumed all nodes in decision tree,
                    // left sub-tree, and this node.
                    n -= 1+dc+lc;

                    continue // FIXME is this needed?
                }else{
                    // There is no right sub tree.  This code should
                    // never be reached.
                    panic!("n {} operator {:?} ",
                           n, self.to_string())
                }
            }
        }
        NodeBox::new(*node.copy())
    }

    /// A recursive copy of a Node
    pub fn copy(&self) -> NodeBox {
        let ret = Node{
            // FIXME Why not: o:self.o,
            o:match self.o {
                Operator::Add => Operator::Add,
                Operator::Log => Operator::Log,
                Operator::Multiply => Operator::Multiply,
                Operator::Invert => Operator::Invert,
                Operator::Negate => Operator::Negate,
                Operator::Remainder => Operator::Remainder,
                Operator::If => Operator::If,
                Operator::Gt => Operator::Gt,
                Operator::Lt => Operator::Lt,
                Operator::Terminal(ref t) => match *t{
                    TerminalType::Float(f) => Operator::Terminal(TerminalType::Float(f)),
                    TerminalType::Inputf64(ref s) => Operator::Terminal(TerminalType::Inputf64(s.clone())),
                },
            },
            l:match self.l {
                Some(ref l) => Some(l.copy()),
                None => None,
            },
            r:match self.r {
                Some(ref r) => Some(r.copy()),
                None => None,
            },
            d:match self.d {
                Some(ref d) => Some(d.copy()),
                None => None,
            },
        };
        NodeBox::new(ret)
    }

    /// A string representation of a Node
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        let mut ret = "".to_string();

        // Macro to make the child of a node into a string
        macro_rules! child_to_string {
            ($name:ident) => {
                match self.$name {
                    Some(ref $name) => ret.push_str(&(*$name).to_string()),
                    None => panic!("{}", 1),
                };
            }
        };
        

        // Macro to make a three child  node into a string
        macro_rules! node_to_string3 {
            ($name:ident) => {
                {
                    ret.push_str(stringify!($name) );
                    ret.push_str(" ");
                    child_to_string!(d);
                    child_to_string!(l);
                    child_to_string!(r);
                }
            }
        };
        // Macro to make a two child  node into a string
        macro_rules! node_to_string2 {
            ($name:ident) => {
                {
                    //println!("node_to_string2 {}", $name);
                    ret.push_str(stringify!($name) );
                    ret.push_str(" ");
                    child_to_string!(l);
                    child_to_string!(r);
                }
            }
        };

        // Macro to make a one child  node into a string
        macro_rules! node_to_string1 {
            ($name:ident) => {
                {
                    ret.push_str(stringify!($name));
                    ret.push_str(" ");
                    child_to_string!(l);
                }
            }
        };
        
        match self.o {
            Operator::If => {
                node_to_string3!(If)
            },
            Operator::Add => node_to_string2!(Add),
            Operator::Multiply => node_to_string2!(Multiply),
            Operator::Gt => node_to_string2!(Gt),
            Operator::Lt => node_to_string2!(Lt),
            Operator::Negate => node_to_string1!(Negate),
            Operator::Remainder => node_to_string2!(Remainder),
            Operator::Log => node_to_string1!(Log),
            Operator::Invert => node_to_string1!(Invert),
            Operator::Terminal(ref f) => {
                ret.push_str(&format!("{}", f));
            },
        };
        ret
    }

    #[allow(dead_code)]
    pub fn to_pretty_string(&self, level:usize) -> String {
        let mut ret = "".to_string();
        let sp = " ";
        // Macro to cmake the child of a node into a string
        macro_rules! child_to_string {
            ($name:ident) => {
                match self.$name {
                    Some(ref $name) => ret.push_str(&(*$name).to_pretty_string(level+1)),
                    None => panic!("name invalid"),
                };
            }
        };

        // Macro to make a three child  node into a string
        macro_rules! node_to_string3 {
            ($name:ident) => {
                {
                    for _ in 0..level {
                        ret.push_str(sp);
                    }
                    ret.push_str(stringify!($name) );
                    ret.push_str("\n");
                    child_to_string!(d);
                    child_to_string!(l);
                    child_to_string!(r);
                }
            }
        };

        // Macro to make a two child  node into a string
        macro_rules! node_to_string2 {
            ($name:ident) => {
                {
                    for _ in 0..level {
                        ret.push_str(sp);
                    }
                    ret.push_str(stringify!($name) );
                    ret.push_str("\n");
                    child_to_string!(l);
                    child_to_string!(r);
                }
            }
        };

        // Macro to make a one child  node into a string
        macro_rules! node_to_string1 {
            ($name:ident) => {
                {
                    for _ in 0..level {
                        ret.push_str(sp);
                    }
                    ret.push_str(stringify!($name));
                    ret.push_str("\n");
                    child_to_string!(l);
                }
            }
        };
        
        match self.o {
            Operator::If => node_to_string3!(If),
            Operator::Add => node_to_string2!(Add),
            Operator::Multiply => node_to_string2!(Multiply),
            Operator::Gt => node_to_string2!(Gt),
            Operator::Lt => node_to_string2!(Lt),
            Operator::Negate => node_to_string1!(Negate),
            Operator::Remainder => node_to_string2!(Remainder),
            Operator::Log => node_to_string1!(Log),
            Operator::Invert => node_to_string1!(Invert),
            Operator::Terminal(ref f) => {
                for _ in 0..level {
                    ret.push_str(sp);
                }
                ret.push_str(&format!("{}\n", f));
            },
        };
        ret
    }

    /// Recursively evaluate a tree over a set of inputs.  This is
    /// where operators are defined.
    pub fn evaluate(&self, inputs:&Inputs)->Option<f64> {
        macro_rules! evaluate {
            ($a:ident) => {
                match self.$a {
                    Some(ref $a) => {
                        let n = &(*$a); // Node
                        let f = n.evaluate(inputs); // Option<f64>
                        let l = f.unwrap();
                        l
                    },
                    None => panic!("Missing child") ,
                }
            }
        }

        match self.o {
            Operator::Terminal(TerminalType::Float(f)) => Some(f),
            Operator::Terminal(TerminalType::Inputf64(ref s)) => Some(*(inputs.get(s).unwrap())),
            Operator::If => {
                let def = evaluate!(d);
                let e:f64;
                if def >= 0.0 {
                    e = evaluate!(l);
                }else{
                    e = evaluate!(r);
                }
                Some(e)
            },
            Operator::Lt => {
                let left = evaluate!(l);
                let right = evaluate!(r);
                if left < right {
                    Some(1.0)
                }else{
                    Some(-1.0)
                }
            },
            Operator::Gt => {
                let left = evaluate!(l);
                let right = evaluate!(r);
                if left > right {
                    Some(1.0)
                }else{
                    Some(-1.0)
                }
            },
            Operator::Add => {
                let left = evaluate!(l);
                let right = evaluate!(r);
                Some(left+right)
            },
            Operator::Remainder => {
                let left = evaluate!(l);
                let right = evaluate!(r);
                Some(left%right)
            },
            Operator::Multiply => {
                let left = evaluate!(l);
                let right = evaluate!(r);
                Some(left*right)
            },
            Operator::Negate => {
                let left = evaluate!(l);
                Some(-1.0*left)
            },
            Operator::Log => {
                let left = evaluate!(l);
                Some(left.ln())
            },
            Operator::Invert => {
                let left = evaluate!(l);
                // FIXME  Divide by 0.0???!
                Some(1.0/left)
            },
        }
    }
}// impl Node
