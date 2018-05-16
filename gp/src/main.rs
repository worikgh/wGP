extern crate rand;
extern crate statistical;

//use statistical::mean;
mod config;
mod entropy;
mod population;
use config::Config;
use entropy::Randomness;
use population::Population;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::time::SystemTime;
use std::io::BufWriter;

// The type of data that can be a terminal
#[derive(Debug)]
enum TerminalType {
    Float(f64),
    // Custom terminals for inputs
    Inputf64(String),
}

// Passed to Node::evaluate.  Matches custom terminals in TerminalType
struct Inputs {
    dataf:HashMap<String, f64>,
}
impl Inputs {
    fn new() -> Inputs {
        Inputs{
            dataf:HashMap::new(),
        }
    }
    fn  insert(&mut self, k:&str, v:f64) {
        self.dataf.insert(k.to_string(), v);
    }
    fn get(&self, k:&str) -> Option<&f64> {
        self.dataf.get(k)
    }
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
#[derive(Debug)]
enum Operator {
    Add,  
    Log,  
    Multiply,
    Invert, // ??! Invert 0.0?
    Negate,
    If,
    Gt, // >
    Lt, // <
    Terminal(TerminalType),
}

// The basic unit of aAST
type NodeBox = Box<Node>;
pub struct Node {
    // Operator and a left and right child trees, or None.  
    o:Operator,
    l:Option<NodeBox>,
    r:Option<NodeBox>,
    d:Option<NodeBox>, // The decision leg for if
}

// The source of entropy that is passed to trees to create themselves.
// With.  FIXME Why not pass around a StdRng?
impl Node {
    fn new_from_string(s:&str) -> Node {
        let mut iter = s.split_whitespace();
        Node::new_from_iter(&mut iter)
    }
    fn new_from_iter(iter:&mut std::str::SplitWhitespace) -> Node{
        
        let operator = match iter.next().unwrap() {
            "Add" => Operator::Add,
            "Log" => Operator::Log,
            "Multiply" => Operator::Multiply,
            "Invert" => Operator::Invert,
            "Negate" => Operator::Negate,
            "If" => Operator::If,
            "Gt" => Operator::Gt,
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
            Operator::Gt|Operator::Lt =>
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
    fn new(e:&mut Randomness, names:&Vec<String>, level:usize) -> Node {
        let l = level+1;

        // FIXME Make this max levela configurable constant
        let maxlevel = 10;
        let a = if level > maxlevel { 
            0
        }else{
            e.gen_range(0, 18)
        };

        macro_rules! NewNode {
            ($name:ident, $c:expr) => {
                {
                    let mut ret = Node{o:Operator::$name,
                                       l: None,
                                       r: None,
                                       d: None,
                    };
                    if $c > 0 {
                        ret.l = Some(Box::new(Node::new(e , names, l)));
                    }
                    if $c > 1 {
                        ret.r = Some(Box::new(Node::new(e , names, l)));
                    }
                    if $c > 2 {
                        ret.d = Some(Box::new(Node::new(e , names, l)));
                    }
                    ret
                }
            }
        };
        match a {
            0 => Node{o:Operator::Terminal(TerminalType::Float(e.gen())), l:None, r:None, d:None},
            1 => NewNode!(Log,1),
            2 => NewNode!(Invert,1),
            3 => NewNode!(Negate,1),
            4 => NewNode!(Multiply,2),
            5 => NewNode!(Gt,2),
            6 => NewNode!(Lt,2),
            7 => NewNode!(Add,2),
            8 => NewNode!(If,3),
            _ => {
                // Input node
                let n = names.len() - 1; // -1 as last name/column is solution
                let b = e.gen_range(0, n);
                let s = names[b].clone();
                Node{o:Operator::Terminal(TerminalType::Inputf64(s)), l:None, r:None, d:None}
            }
        }
    }
    fn count_nodes(&self) -> usize {
        // Recursive count of child nodes
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
    fn random_node(&self, e:& mut Randomness) -> NodeBox {
        // Choose a subtree (node) of this tree (node).  FIXME there
        // is a lot of optimisation to be done.  Paticularly if each
        // node had the number of nodes that are child nodes of this...
        let c = self.count_nodes();
        let mut n = e.gen_range(0, c);
        let mut node:& Node = self;
        loop {
            // println!("Node: {} n {}", self.to_string(), n);
            // Loop invariant n >= 0 Exit when a node with no left or
            // right children is encountered or n == 0
            if n == 0 {
                break;
            }
            let fl = match node.l{Some(_) => false, None => true};
            let fr = match node.r{Some(_) => false, None => true};
            if fl && fr {
                // No children (if there is a 'd' child there must be
                // 'l'and 'r') or n is one so we have arrived at the
                // node selected by the e.gen_range statement above
                if n != 0 {
                    panic!("Node: {} n {}", self.to_string(), n);
                }
                break;
            }

            // Children in decision node
            let dc = match node.d {
                Some(ref q) => (*q).count_nodes(),
                None => 0,
            };

            if dc >= n {
                // Wanted node is in decision sub-tree
                // println!("Go d: dc {}", dc);
                if let Some(ref nd) = node.d {
                    node = &*nd;

                    // Subtract the current (consumed) node from n
                    n -= 1;
                        
                    continue // FIXME is this needed?
                }else{
                    panic!("dc {} n {} operator {:?} ",
                           dc, n, self.to_string());
                }
            }

            // Is node in left subtree?
            let lc = match node.l {
                Some(ref q) => (*q).count_nodes(),
                None => 0,
            };

            if dc+lc >= n {
                // Get node from left sub tree
                // println!("Go l: dc {} lc {}", dc, lc);
                if let Some(ref nd) =  node.l {
                    node = &*nd;

                    // Have consumed all nodes in decision tree
                    // and this node.
                    n -= 1+dc;
                    
                    continue // FIXME is this needed?
                }else{
                    panic!("lc {} n {} operator {:?} ",
                           lc, n, self.to_string())
                }
            }else{

                // println!("Go r: dc {} lc {}", dc, lc);
                // Get node from right subtree
                if let Some(ref nd) = node.r {
                    node = &*nd;

                    // Have consumed all nodes in decision tree,
                    // left sub-tree, and this node.
                    n -= 1+dc+lc;

                    continue // FIXME is this needed?
                }else{
                    panic!("n {} operator {:?} ",
                           n, self.to_string())
                }
            }
        }
        NodeBox::new(*node.copy())
    }
    fn copy(&self) -> NodeBox {
        // Recursive copy
        let ret = Node{
            // FIXME Why not: o:self.o,
            o:match self.o {
                Operator::Add => Operator::Add,
                Operator::Log => Operator::Log,
                Operator::Multiply => Operator::Multiply,
                Operator::Invert => Operator::Invert,
                Operator::Negate => Operator::Negate,
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
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        let mut ret = "".to_string();

        // Macro to cmake the child of a node into a string
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
            Operator::Log => node_to_string1!(Log),
            Operator::Invert => node_to_string1!(Invert),
            Operator::Terminal(ref f) => {
                ret.push_str(&format!("{}", f));
            },
        };
        ret
    }

    #[allow(dead_code)]
    fn to_pretty_string(&self, level:usize) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_data_partition() {
        let config = Config::new("config");
        let data_file = config.get_string("data_file").unwrap();

        // The source of entropy.  This is done this way so the same seed
        // can be used to produce repeatable results
        // let mut e = Randomness::new(&[11,2,3,422, 195]);
        let mut e = Randomness::new(&[11,2,3,4]);

        // Load the data
        let mut d_all:Data = Data::new();
        d_all.read_data(data_file.as_str(), 0, &mut e);
        assert_eq!(d_all.training_i.len(), 0);

        
        d_all.read_data(data_file.as_str(), 100, &mut e);
        assert_eq!(d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 50, &mut e);
        assert_ne!(d_all.testing_i.len(), 0);
        assert_ne!(d_all.training_i.len(), 0);

        d_all.read_data(data_file.as_str(), 10, &mut e);
        assert!(d_all.training_i.len()< d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 90, &mut e);
        assert!(d_all.training_i.len() > d_all.testing_i.len(), 0);

    }
    #[test]
    fn test_node_eval(){
        let mut inputs = Inputs::new();
        let s = "Multiply Float 0.06 Negate Diameter";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, -0.60);

        let mut inputs = Inputs::new();
        let s = "Invert Float 0.1";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 10.0);

        let inputs = Inputs::new();
        let s = "Add Float 2000.0 Invert Float 0.1";
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 2010.0);

        let mut inputs = Inputs::new();
        let s = "Multiply Height Add Float 10.0 Invert Float 0.1";
        inputs.insert("Height", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 200.0);

        let s = "If Lt x Float 0.0 Float -1.0 Float 1.0";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", 0.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", -0.01);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Gt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Lt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.2);
        inputs.insert("y", 1.1);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.1);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", -9.0);
        inputs.insert("y", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -9.0);        

    }
    #[test]
    fn test_node_from_string(){
        let s = "Add Add Add Invert Height Diameter Add Negate Float 0.03049337449511591 Add Multiply Negate Invert Float 0.40090461861005733 Negate Diameter Negate Float 0.06321754406175395 Length";
        let n = Node::new_from_string(s);
        let ns = &n.to_string()[..];
        println!("");
        println!("1: {}", s);
        println!("2: {}", ns.trim());
        assert_eq!(ns.trim(), s.to_string());        
    }
}

// Hold data for training or testing.
pub struct Data {
    
    // Names of the columns
    names:Vec<String>, 

    // Each row is a hash keyed by names FIXME Inefficient(?) use of memory
    rows:Vec<Vec<f64>>, 
    
    // Indexes into rows for training data
    training_i:Vec<usize>,

    // Indexes into rows for testing data
    testing_i:Vec<usize>,

    // Indexes into rows for all data
    all_i:Vec<usize>,

}

impl Data {
    #[allow(dead_code)]
    /// Return all the data as a string
    fn to_string(&self) -> String {
        let mut ret = "".to_string();
        for r in &self.rows {
            for i in 0..self.names.len() {
                ret.push_str(&r[i].to_string()[..]);
                ret.push_str(",");
            }
            ret.push_str("\n");
        }
        ret
    }
    fn new() -> Data {
        Data{
            names:Vec::<String>::new(),
            rows:Vec::<Vec<f64>>::new(),
            testing_i:Vec::<usize>::new(),
            training_i:Vec::<usize>::new(),
            all_i:Vec::<usize>::new(),
        }
    }
    fn reset(&mut self){
        self.names = Vec::<String>::new();
        self.rows = Vec::<Vec<f64>>::new();
        self.testing_i = Vec::<usize>::new();
        self.training_i = Vec::<usize>::new();
        self.all_i = Vec::<usize>::new();
    }        
    fn ith_row(&self, i:usize) -> &Vec<f64> {
        &self.rows[i]
    }
    fn add_name(& mut self, k:&str) {
        self.names.push(k.to_string())
    }
    fn partition(&mut self, training_percent:usize, e:&mut Randomness){
        // Partition the data into training and testing sets
        for i in 0..self.rows.len() {
            let z = e.gen_range(0, 100);
            if z < training_percent {
                self.training_i.push(i);
            }else{
                self.testing_i.push(i);
            }
            self.all_i.push(i);
        }
    }        
    fn add_row(&mut self, row:Vec<f64>){
        self.rows.push(row);
    }
    // Read in the data from a file
    fn read_data(&mut self, f_name:&str,
                 training_percent:usize, e:&mut Randomness) {

        // Must be in file f_name.  First row is header
        self.reset();
        let file = File::open(f_name).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let mut lines = contents.lines();

        // Get first line, allocate names 
        let h = lines.nth(0);
        let l = match h {
            Some(l) =>   {
                l
            },
            None => panic!(""),
        };

        let h:Vec<&str> = l.split(',').collect();
        for s in h {
            // s is a header
            self.add_name(s);
        }

        // Loop over the data storing it in the rows
        loop {
            
            let line = match lines.next() {
                Some(l) => l,
                None => break,
            };
            let d:Vec<&str> = line.split(',').collect();
            let d:Vec<f64> = d.iter().map(|x| {x.parse::<f64>().unwrap()}).collect();

            self.add_row(d);
            
        }
        self.partition(training_percent, e);
    }
}



// Calculate the score of a indvidual against the data Param n: The
// individual Param d: The data to use.  'use_testing' is true if the
// individual is to be scored on the testing set
fn score_individual(n:&NodeBox, d:&Data, use_testing:bool) -> f64 {
    let mut inputs = Inputs::new();

    let index:&Vec<usize>;
    if use_testing {
        index = &d.testing_i;
    }else{
        index = &d.training_i;
    }
    let mut sum_square = 0.0;
    for i in index {
        let ref r = d.ith_row(*i);
        for j in 0..d.names.len() {
            let v:f64 = r[j];
            let h = d.names[j].clone();
            inputs.insert(h.as_str(), v);
        }
        let e = n.evaluate(&inputs).unwrap();
        // Get the target
        let t = inputs.get(d.names.last().unwrap()).unwrap();
        sum_square += (e-t)*(e-t);
    }
    // This transformation is a puzzle.  Take the square root to make
    // it a Euclidian distance (in the dimensionality of the number of
    // samples) and divide by the number of samples to scale it.  It
    // should not be too critical (??? Test that ???) so long as the
    // evaluation is monotonic.
    sum_square.sqrt() / (index.len() as f64)
        
}

// Do a simulation to evaluate a model.  Returns a vector of pairs.
// The first element is true value the second is simulation result
fn simulate(n:&NodeBox, d:&Data) -> Vec<(f64, f64)> {
    let mut ret:Vec<(f64, f64)> = vec![];
    let mut inputs = Inputs::new();
    let ref index = d.all_i;
    for i in index {
        let ref r = d.rows[*i];
        for j in 0..d.names.len() {
            let v = r[j];
            let h = d.names[j].clone();
            inputs.insert(h.as_str(), v);
        }
        let e = n.evaluate(&inputs).unwrap();
        // Get the target
        let t = inputs.get(d.names.last().unwrap()).unwrap();
        ret.push((*t, e));
    }
    ret
}

// Add a simulation to Simlations.txt.  First line is the IDs of the
// data.  First column (labled ID 0) is the actual value.  Each column
// is the matching data from the model with the ID at the top (FIXME
// Comment!)
fn add_simulation(data:Vec<(f64, f64)>, id:usize, fname:&str){
    let mut contents = String::new();
    // Create a string to hold final results    
    let mut results = "".to_string();

    {
        
        // Test if file exists.  If not create it
        if ! Path::new(fname).exists() {
            OpenOptions::new().write(true).create(true).open(fname).unwrap();
        }
        let  file = OpenOptions::new()
            .read(true)
            .open(fname).unwrap();
        
        // Get the data in to read
        let mut buf_reader = BufReader::new(file);
        buf_reader.read_to_string(&mut contents).unwrap();
        if contents.len() == 0 {
            // The file was empty Create the first column
            contents.push_str("0 \n");
            for ref d in data.clone() {
                contents.push_str(format!("{} \n", d.0).as_str());
            }
        }
        // Contents is now ready to have another data set appended

        let mut lines = contents.lines();

        // First test: The number of lines is the numebr of cases plus
        // one for header.  This is the same for all simulations so
        // data.len() == lines.count()-1
        assert_eq!(data.len(), lines.count()-1);

        // The test above consumed lines so reinitialise it
        lines = contents.lines();
        
        // Set up the header
        let mut header = lines.next().unwrap().to_string();

        // Append the ID of this model and initialise results with it

        header.push_str(format!("{}", id).as_str());
        header.push(' ');
        header.push('\n');

        results.push_str(header.as_str());
        // Going to loop over all the data from the file and through
        // the data supplied in the 'data' parameter simultaneously
        let mut i = 0; // Index into data
        for l in lines {
            // Get the data members of this row
            let mut d = l.split_whitespace();
            let d0 = d.next().unwrap(); // The actual value as string
            let d0u = d0.parse::<f64>().unwrap(); // The actual value as a number

            // Test: The actual value here must be the same as the
            // actual value in data[i].0
            assert_eq!(d0u, data[i].0);

            // Add the actual to this line
            results.push_str(format!("{} ", d0).as_str());

            // Put the rest of the line in results. FIXME There must
            // be a variadic way to do this
            loop {
                match d.next() {
                    Some(v) => results.push_str(format!("{} ", v).as_str()),
                    None => break,
                };
            }
            // Add in the new data
            results.push_str(format!("{} \n", data[i].1).as_str());

            i += 1;
        }
    }
    // results now holds the new contents of the file
    let mut file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(fname).unwrap();
    file.write(&results.into_bytes()[..]).unwrap();
}
    

/// rite out R script to generate plit of results
fn write_plotting_script(input_data:&str, xlab:&str,
                         outfile:&str, r_script_file:&str,
                         generations_file:&str,
                         id:&str) {
    let  script ="
data <- read.table('SIMULATIONS', header=TRUE)
names <- names(data)

objective <- data[,1]
best.estimate <- data[,names[length(names)]]
png(filename=\"OUTFILE\", width=210, height=297, units=\"mm\", res=600)
oldpar <- par(mfrow=c(2,2))
ratio <- 100*(objective-best.estimate)/objective

plot(x=data[,\"X0\"], y=best.estimate, cex=.2, ylab=\"Estimate\", xlab=\"Actual\", main=\"Best Model\")
hist(ratio, main=\"Error Ratio\", density=10, xlab=\"Percent Error\", freq=FALSE)
hist(objective, main=\"Objective Data\", density=10, breaks=30, xlab=\"XLAB\")
hist(objective-best.estimate, main=\"Differences\", density=10, freq=FALSE, breaks=30)
dev.off()

png(\"OUTFILE_Solutions.png\", width=210, height=297, units=\"mm\", res=600)
c <- ceiling(sqrt(dim(plot.data)[2]-1))
par(mfrow=c(c,c))
plot.data <- data[order(data[,1]),]
x <- plot.data[,1]
y <- plot.data[, dim(plot.data)[2]]

plot(x=x, y=y,
     xlab='Objective',
     ylab='Best Estimate',
     main='Comparison of Models', t='p', cex=.75, pch='x')
for(i in 2:(dim(plot.data)[2]-1)){
    plot(x=x, y=plot.data[,i],
         xlab='Objective',
         ylab='Estimate',
         main='Comparison of Models', t='p', cex=.75, pch='x')
}

dev.off()


png(filename=\"IDGeneration.png\",
    width=210, height=297, units=\"mm\", res=600)
## Read the first four columns from the file as numeric
q <- scan(\"GENERATIONS_FILE\", what = list(1,1,1,1,1), flush = TRUE)
data <- cbind(c(0, diff(q[[1]])),q[[2]], q[[3]], q[[4]], q[[5]])
## First row has invalid time data (no diff at time 0) so get rid of it?
## data <- data[-1,] Na!

colnames(data) <- c(\"Sec\", \"Gen\", \"ID\", \"Eval\", \"Pop\")


gen <- data[,\"Gen\"]
sec <- data[,\"Sec\"]
pop <- data[,\"Pop\"]
eval <- data[,'Eval']

## Normalise eval and pop to same scale as sec
eval.2 <- (eval - min(eval))*max(sec)
pop.2 <-  (pop - min(pop))*max(sec)


## Define Margins. The trick is to use give as much space possible on
## the left margin (second value)
par(mar=c(5, 12, 4, 4) + 0.1)

## Plot the first time series. Notice that you don’t have to draw the
## axis nor the labels


plot(gen, sec, axes=F, ylim=c(0,max(sec)), xlab=\"\", ylab=\"\",type=\"l\",col=\"black\", main=\"ID\",xlim=range(gen))

axis(2, ylim=c(0,max(sec)),col=\"black\",lwd=2)
mtext(2,text=\"Sec\",line=2)

par(new=T)
plot(gen, eval.2, axes=F, ylim=range(eval.2), xlab=\"\", ylab=\"\", type=\"l\",lty=2, main=\"\",xlim=range(gen),lwd=2, col=2)

labels <- signif(seq(from=min(eval), to=max(eval), length.out=8),  4)
at <- seq(from=min(eval.2), to=max(eval.2), length.out=8)
axis(2, at=at, labels=labels, lwd=2,line=3.5)
mtext(2,text=\"Eval\",line=5.5)

## Plot the third time series. Again the line parameter are both
## further increased.

par(new=T)
plot(gen, pop.2, axes=F, ylim=range(pop.2), xlab=\"\", ylab=\"\", type=\"l\",lty=3, main=\"\",xlim=range(gen),lwd=2, col=3)
axis(2, ylim=range(pop),lwd=2,line=7)
mtext(2,text=\"Population\",line=9)

##We can now draw the X-axis, which is of course shared by all the
##three time-series.

axis(1,pretty(range(gen),10))
mtext(\"Generation\",side=1,col=\"black\",line=2)

##And then plot the legend.
legend(x=\"topleft\", legend=c(\"Sec\",\"Eval\",\"Pop\"),lty=c(1,2,3), col=c(1,2,3), bty='n') 
dev.off()
";
    let script = script.replace("SIMULATIONS", input_data);
    let script = script.as_str().replace("XLAB", xlab).to_string();
    let script = script.as_str().replace("GENERATIONS_FILE",
                                         generations_file).to_string();
    let script = script.as_str().replace("OUTFILE", outfile).to_string();
    let script = script.as_str().replace("ID", id).to_string();
    let  file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(r_script_file).unwrap();
    
    let mut writer = BufWriter::new(&file);
    writer.write_all(script.to_string().as_bytes()).unwrap();
}


pub struct Recorder {
    // Manage writing data to a file.  The constructor takes a file
    // name and creates a buffered writer to the file.  Ha a "write"
    // withod that sends data to that buffer for writing to the file.
    // The SystemTime object is used to prefix the number of elapsed
    // seconds to each record
    buffer:BufWriter<File>,
    created:SystemTime,
}
impl Recorder {
    fn new(file_name:&str) -> Recorder {
        Recorder{
            buffer:BufWriter::new(OpenOptions::new()
                                  .append(true)
                                  .create(true)
                                  .open(file_name).unwrap()),
            created:SystemTime::now(),
        }
    }
    fn write_line(&mut self, line:&str) {
        let now = format!("{} ", self.created.elapsed().unwrap().as_secs());
        self.buffer.write(&now.into_bytes()[..]).unwrap();
        self.buffer.write(&line.to_string().into_bytes()[..]).unwrap();
        self.buffer.write(&"\n".to_string().into_bytes()[..]).unwrap();
    }
}


fn main() {
    println!("Start");

    let args: Vec<_> = env::args().collect();
    let cfg_file:String;
    if args.len() > 1 {
        cfg_file = args[1].clone();
    }else{
        cfg_file = "config".to_string();
    }
    

    let config = Config::new(cfg_file.as_str());
    let data_file = config.get_string("data_file").unwrap();
    let generations_file = config.get_string("generations_file").unwrap();
    let initial_population =  config.get_usize("initial_population").unwrap();
    let model_data_file = config.get_string("model_data_file").unwrap();
    let num_generations = config.get_usize("num_generations").unwrap();
    let plot_file = config.get_string("plot_file").unwrap();
    let plot_xlab = config.get_string("plot_xlab").unwrap();
    let r_script_file = config.get_string("r_script_file").unwrap();
    let seed = config.get_string("seed").unwrap(); // The seed is a string of usize numbers
    let seed:Vec<usize> = seed.split_whitespace().map(|x| x.parse::<usize>().unwrap()).collect();
    let sim_id = config.get_string("id").unwrap();
    let training_percent = config.get_usize("training_percent").unwrap(); // The percentage of data to use as trainng
    

    // Set up output file to record each generation
    let mut generation_recorder = Recorder::new(generations_file.as_str());

    // Write out the R script to plot the simulation
    write_plotting_script(model_data_file.as_str(),
                          plot_xlab.as_str(),
                          plot_file.as_str(),
                          r_script_file.as_str(),
                          generations_file.as_str(),
                          sim_id.as_str(),
    );

    // The source of entropy.  This is done this way so the same seed
    // can be used to produce repeatable results
    // let mut e = Randomness::new(&[11,2,3,422, 195]);
    let mut e = Randomness::new(&seed);


    if let Some(ns) = config.get_string("eval") {
        // Load the data
        let mut d_all = Data::new();
        d_all.read_data(data_file.as_str(), training_percent, &mut e);
        let n = NodeBox::new(Node::new_from_string(ns.as_str()));
        let s = (*n).to_string();
        println!("{} {}", s, score_individual(&n, &d_all, true));
        
    }else{
        

        // Create a population. The first part of the tuple is the set of
        // trees that is the population.  The second part stores the
        // string representation of every individual (Node::to_string())
        // to keep duplicates out of the population
        println!("Population start");

        let mut population = Population::new(&config, &mut e);

        loop {
            while !population.add_individual() {}
            if population.len() == initial_population {
                break;
            }
        }
        println!("Created initial population");
        // For each member of the population calculate a evaluation


        for generation in 0..num_generations {
            let s = format!("{} {} {} {} {}", generation,
                            population.best_id(), population.best_score(),
                            population.len(),
                            population.get_tree_id(population.best_id()).1.to_string());
            generation_recorder.write_line(&s[..]);
            generation_recorder.buffer.flush().unwrap();

            population.new_generation(generation);
            
            //println!("Best pop sc: {} Worst: {}", population.0[0].2, population.0[population.0.len()-1].2);
            
        }
        println!("Bye!");
    }
}
