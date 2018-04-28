extern crate rand;
extern crate statistical;

use std::env;
use std::path::Path;
use std::time::SystemTime;
use rand::Rng;
use rand::SeedableRng;
use rand::StdRng;
use statistical::mean;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;

// The type of data that can be a terminal
enum TerminalType {
    Float(f64),
    // Custom terminals for inputs
    Inputf64(String),
}

// Passed to Node::evaluate.  Matches custom terminals in TerminalType
struct Inputs {
    dataf:HashMap<String, f64>,
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
enum Operator {
    Add,  
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
struct Node {
    // Operator and a left and right child trees, or None.  
    o:Operator,
    l:Option<NodeBox>,
    r:Option<NodeBox>,
    d:Option<NodeBox>, // The decision leg for if
}

// The source of entropy that is passed to trees to create themselves.
// With.  FIXME Why not pass around a StdRng?
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
    fn gen_rangef64(& mut self, a:f64, b:f64)->f64{
        self.rng.gen_range(a,b)
    }
}

impl Node {
    fn new_from_string(s:&str) -> Node {
        let mut iter = s.split_whitespace();
        Node::new_from_iter(&mut iter)
    }
    fn new_from_iter(iter:&mut std::str::SplitWhitespace) -> Node{
        
        let operator = match iter.next().unwrap() {
            "Add" => Operator::Add,
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
        let d = match operator {
            Operator::If =>
                Some(NodeBox::new(Node::new_from_iter(iter))),
            _ => None,
        };
                
        Node{o:operator, l:l, r:r, d:d}
    }    // Build a random tree
    /* Paramaters:
     * Entropy - A source of randomness
     * names - The names of the input fields
     * level - The distance from the root node for this node
     */
    fn new(e:&mut Entropy, names:&Vec<String>, level:usize) -> Node {
        let l = level+1;

        // FIXME Make this max levela configurable constant
        let maxlevel = 10;
        let a = if level > maxlevel { 
            0
        }else{
            e.gen_range(0, 18)
        };

        //print!("level {} ", l);
        macro_rules! NewNode {
            ($name:ident) => {
                Node{o:Operator::$name,
                     l: Some(Box::new(Node::new(e , names, l))),
                     r: Some(Box::new(Node::new(e, names, l))),
                     d: Some(Box::new(Node::new(e, names, l))),
                }
            }
        };
        match a {
            0 => Node{o:Operator::Terminal(TerminalType::Float(e.gen())), l:None, r:None, d:None},
            1 => NewNode!(Multiply),
            2 => {
                let n = NewNode!(Invert);
                n
            },
            3 => NewNode!(Negate),
            4 => NewNode!(If),
            5 => NewNode!(Gt),
            6 => NewNode!(Lt),
            7 => NewNode!(Add),
            _ => {
                // Input node
                let n = names.len() - 1; // -1 as last name/column is solution
                let b = e.gen_range(0, n);
                let s = names[b].clone();
                Node{o:Operator::Terminal(TerminalType::Inputf64(s)), l:None, r:None, d:None}
            }
        }
    }
    fn count_child_nodes(&self) -> usize {
        // Recursive count of child nodes
        let lc = match self.l {
            Some(ref n) => n.count_child_nodes(),
            None => 0,
        };
        let rc = match self.r {
            Some(ref n) => n.count_child_nodes(),
            None => 0,
        };
        lc + rc + 1
    }
    fn random_node(&self, e:& mut Entropy) -> NodeBox {
        // Choose a subtree (node) of this tree (node).  FIXME there
        // is a lot of optimisation to be done.  Paticularly if each
        // node had the number of nodes that are child nodes of this...
        let c = self.count_child_nodes();
        let mut n = e.gen_range(0, c);
        let mut node:& Node = self;
        loop {
            // Loop invariant n >= 1
            // Exit when n == 1
            if match node.l{Some(_) => false, None => true} &&
                match node.r{Some(_) => false, None => true} {
                    break;
                }
            let lc = match node.l {
                Some(ref q) => (*q).count_child_nodes(),
                None => 0,
            };

            if lc < n {
                // Get node from right sub tree
                node = match node.r {
                    Some(ref r) => &*r,
                    None => panic!(""),
                };
                n -= lc;
            }else if lc > n {
                // Get node from left subtree
                node = match node.l {
                    Some(ref l) => &*l,
                    None => panic!(""),
                };
            }else{
                assert_eq!(lc, n);
                break;
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
                    child_to_string!(l);
                    child_to_string!(r);
                    child_to_string!(d);
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
                    None => panic!("{}", 1),
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
                    child_to_string!(l);
                    child_to_string!(r);
                    child_to_string!(d);
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
            Operator::Terminal(TerminalType::Inputf64(ref s)) => 
                Some(*(inputs.dataf.get(s).unwrap())),
            Operator::If => {
                let d = evaluate!(d);
                let e:f64;
                if d <= 0.0 {
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
        let config = Config::new();
        let training_percent = config.get_usize("training_percent").unwrap();
        let data_file = config.get_string("data_file").unwrap();

        // The source of entropy.  This is done this way so the same seed
        // can be used to produce repeatable results
        // let mut e = Entropy::new(&[11,2,3,422, 195]);
        let mut e = Entropy::new(&[11,2,3,4]);

        // Load the data
        let d_all:Data =
            read_data(data_file.as_str(), 0, &mut e).unwrap();
        assert_eq!(d_all.training_i.len(), 0);

        let d_all:Data =
            read_data(data_file.as_str(), 100, &mut e).unwrap();
        assert_eq!(d_all.testing_i.len(), 0);

        let d_all:Data =
            read_data(data_file.as_str(), 50, &mut e).unwrap();
        assert_ne!(d_all.testing_i.len(), 0);
        assert_ne!(d_all.training_i.len(), 0);

        let d_all:Data =
            read_data(data_file.as_str(), 10, &mut e).unwrap();
        assert!(d_all.training_i.len()< d_all.testing_i.len(), 0);

        let d_all:Data =
            read_data(data_file.as_str(), 90, &mut e).unwrap();
        assert!(d_all.training_i.len() > d_all.testing_i.len(), 0);

    }
    #[test]
    fn test_node_eval(){
        let mut inputs = Inputs {
            dataf:HashMap::new(),
        };
        let s = "Multiply Float 0.06 Negate Diameter";
        inputs.dataf.insert("Diameter".to_string(), 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, -0.60);
        let s = "Invert Float 0.1";
        inputs.dataf.insert("Diameter".to_string(), 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 10.0);

        let s = "Add Float 2000.0 Invert Float 0.1";
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 2010.0);
        let s = "Multiply Height Add Float 10.0 Invert Float 0.1";
        inputs.dataf.insert("Height".to_string(), 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 200.0);
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
struct Data {
    
    // Names of the columns
    names:Vec<String>, 

    // Each row is a hash keyed by names FIXME Inefficient(?) use of memory
    rows:Vec<HashMap<String, f64>>, 
    
    // Indexes into rows for training data
    training_i:Vec<usize>,

    // Indexes into rows for testing data
    testing_i:Vec<usize>,
}

impl Data {
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        let mut ret = "".to_string();
        for r in &self.rows {
            for v in &self.names {
                ret.push_str(&r.get(v).unwrap().to_string());
                ret.push_str(",");
            }
            ret.push_str("\n");
        }
        ret
    }
}
// Read in the data from a file
fn read_data(f_name:&str , training_percent:usize, e:&mut Entropy) -> std::io::Result<Data> {

    // Must be in file f_name.  First row is header

    let mut ret = Data{
        names:Vec::<String>::new(),
        rows:Vec::<HashMap<String, f64>>::new(),
        testing_i:Vec::<usize>::new(),
        training_i:Vec::<usize>::new(),
    };

    let file = File::open(f_name)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
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
        ret.names.push(s.to_string());
    }

    // Loop over the data storing it in the rows
    let mut ln = 0; // Current line number
    loop {
        
        let line = match lines.next() {
            Some(l) => l,
            None => break,
        };
        let d:Vec<&str> = line.split(',').collect();
        let hv = HashMap::<String, f64>::new();
        ret.rows.push(hv);
        for i in 0..d.len() {
            let k = ret.names[i].clone();
            //println!("d[{}] {}", i, d[i]);
            let v = d[i].parse::<f64>().unwrap();
            ret.rows[ln].insert(k, v);
        }
        ln += 1; 
    }

    // Partition the data into training and testing sets
    for i in 0..ret.rows.len() {
        let z = e.gen_range(0, 100);
        if z < training_percent {
            ret.training_i.push(i);
        }else{
            ret.testing_i.push(i);
        }
    }
    Ok(ret)
}


fn crossover(l:&NodeBox, r:&NodeBox, e:& mut Entropy) -> NodeBox {

    
    let p:NodeBox;// Parent
    let c:NodeBox;// Child
    if e.gen() > 0.0 {
        p = (*l).random_node(e);
        c = (*r).random_node(e);
    }else{
        c = (*l).random_node( e);
        p = (*r).random_node( e);
    }
    // Make the new tree by copying c
    let c = c.copy();

    // The tree to return
    let mut ret = p.copy();
    
    // Choose a branch off p to copy c to
    match (*ret).r {
        Some(_) => {
            // p has two children.  Choose one randomly
            if e.gen() > 0.0 {
                // Left
                (*ret).l = Some(c);
            }else{
                // Right
                (*ret).r = Some(c);
            }
        },
        None => (*ret).l = Some(c),
    };
    ret
}


// Calculate the score of a indvidual against the data Param n: The
// individual Param d: The data to use.  'use_testing' is true if the
// individual is to be scored on the testing set
fn score_individual(n:&NodeBox, d:&Data, use_testing:bool) -> f64 {
    let mut scorev:Vec<f64> = vec![];
    let mut inputs = Inputs{
        dataf:HashMap::new(),
    };

    //println!("Evaluate {}", stt);
    let index:&Vec<usize>;
    if use_testing {
        index = &d.testing_i;
    }else{
        index = &d.training_i;
    }
    for i in index {
        let ref r = d.rows[*i];
        for h in d.names.iter() {
            let k = h.clone();
            let v1 = r.get(&k);
            let v:f64 = *v1.unwrap();
            inputs.dataf.insert(k, v);
        }
        let e = n.evaluate(&inputs).unwrap();
        // Get the target
        let t = inputs.dataf.get(d.names.last().unwrap()).unwrap();
        
        scorev.push((e-t).abs());
    }

    // Take the inverse of the mean value of the score
    let ret = 1.0/mean(&scorev[..]);

    // Take the maximum - The worst result
    // &scorev[..].sort_by(|a, b| {b.partial_cmp(&a).unwrap_or(Ordering::Equal)});
    // let ret = scorev[0];

    //println!("Sc! {} {}", scorev[0], scorev[scorev.len()-1]);
    ret.ln_1p()
}

// Do a simulation to evaluate a model.  Returns a vector of pairs.
// The first element is true value the second is simulation result
fn simulate(n:&NodeBox, d:&Data) -> Vec<(f64, f64)> {
    let mut ret:Vec<(f64, f64)> = vec![];
    let mut inputs = Inputs{
        dataf:HashMap::new(),
    };
    let ref index = d.testing_i;
    for i in index {
        let ref r = d.rows[*i];
        for h in d.names.iter() {
            let k = h.clone();
            let v1 = r.get(&k);
            let v:f64 = *v1.unwrap();
            inputs.dataf.insert(k, v);
        }
        let e = n.evaluate(&inputs).unwrap();
        // Get the target
        let t = inputs.dataf.get(d.names.last().unwrap()).unwrap();
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
        // the data supplied in data simultaneously
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
    

struct Config {
    // max_generations: usize,
    // max_population: usize,
    // cull_size: usize,
    // crossover_percent: usize
    data:HashMap<String, String>,
}

impl Config {
    fn new(cfg_file:&str)-> Config {
        let file = File::open(cfg_file).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let lines = contents.lines();
        let mut config_hm = HashMap::new();
        for line in lines {
            let mut iter = line.split_whitespace();
            let k = iter.next().unwrap();
            let v = iter.map(|x| format!("{} ", x)).collect::<String>();
            config_hm.insert(k.to_string(), v.trim().to_string());
        }
        Config{data:config_hm}
    }
    fn get_usize(&self, k:&str) -> Option<usize> {
        let v = match self.data.get(k) {
            Some(v) => v,
            _ => panic!(""),
        };
        let ret = match v.parse::<usize>() {
            Ok(v) => Some(v),
            _ => {
                None
            },
                
        };
        ret
    }
    fn get_string(&self, k:&str) -> Option<String> {
        match self.data.get(k) {
            Some(v) => Some(v.clone()),
            _ => None,
        }
    }        
}

/// rite out R script to generate plit of results
fn write_plotting_script(input_data:&str, xlab:&str,
                         outfile:&str, r_script_file:&str) {
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
";
    let script = script.replace("SIMULATIONS", input_data);
    let script = script.as_str().replace("XLAB", xlab).to_string();
    let script = script.as_str().replace("OUTFILE", outfile).to_string();
    let  file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(r_script_file).unwrap();
    
    let mut writer = BufWriter::new(&file);
    writer.write_all(script.to_string().as_bytes()).unwrap();
}

// Define a individual.  Consists of a node, a id, and a score.  Called
// a Tree because it is not a sub-tree...
type Tree = (usize, NodeBox, f64); 

struct Recorder {
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
    let num_generations = config.get_usize("num_generations").unwrap();
    let max_population =  config.get_usize("max_population").unwrap();
    let initial_population =  config.get_usize("initial_population").unwrap();
    let cull_size = config.get_usize("cull_size").unwrap();
    let training_percent = config.get_usize("training_percent").unwrap(); // The percentage of data to use as trainng
    let crossover_percent = config.get_usize("crossover_percent").unwrap();
    let data_file = config.get_string("data_file").unwrap();
    let model_data_file = config.get_string("model_data_file").unwrap();
    let plot_xlab = config.get_string("plot_xlab").unwrap();
    let plot_file = config.get_string("plot_file").unwrap();
    let r_script_file = config.get_string("r_script_file").unwrap();
    let generations_file = config.get_string("generations_file").unwrap();
    let birthsanddeaths_file =
        config.get_string("birthsanddeaths_file").unwrap();

    // Set up output files
    let mut generation_recorder = Recorder::new(generations_file.as_str());
    let mut birth_death_recorder =
        Recorder::new(birthsanddeaths_file.as_str());

    // The source of entropy.  This is done this way so the same seed
    // can be used to produce repeatable results
    // let mut e = Entropy::new(&[11,2,3,422, 195]);
    let mut e = Entropy::new(&[11,2,3,4]);

    // Load the data
    let d_all:Data = read_data(data_file.as_str(), training_percent, &mut e).unwrap();

    if let Some(ns) = config.get_string("eval") {
        let n = NodeBox::new(Node::new_from_string(ns.as_str()));
        let s = (*n).to_string();
        println!("{} {}", s, score_individual(&n, &d_all, false));
        
    }else{
        

        // Create a population. The first part of the tuple is the set of
        // trees that is the population.  The second part stores the
        // string representation of every individual (Node::to_string())
        // to keep duplicates out of the population
        println!("Population start");
        let mut population:(Vec<Tree>, HashMap<String, bool>) = (Vec::new(), HashMap::new());

        let mut maxid = 0; // Each tree as a ID
        loop {
            let n = Box::new(Node::new(&mut e, &d_all.names, 0));
            let st = n.to_string();
            population.1.entry(st.clone()).or_insert(false);
            if !population.1.get(&st).unwrap() {
                // This node is unique
                maxid += 1;
                population.1.insert(st, true);
                let sc = score_individual(&n, &d_all, false);
                {
                    birth_death_recorder.write_line(&format!("{}/{}: {}", maxid, sc, n.to_string()));
                }
                population.0.push((maxid, n, sc));
            }
            if population.0.len() == initial_population {
                break;
            }
        }
        println!("Created initial population");
        // For each member of the population calculate a evaluation

        let mut best_id = 0;
        let mut best_individual = "".to_string();
        for generation in 0..num_generations {
            let s = format!("{} {} {} {} {}", generation,
                            population.0[0].0, population.0[0].2,
                            population.0.len(),
                            population.0[0].1.to_string());
            generation_recorder.write_line(&s[..]);
            generation_recorder.buffer.flush().unwrap();
            birth_death_recorder.buffer.flush().unwrap();
            // Filter out members of population that have no valid score (arithmetic error)
            population.0 = population.0.into_iter().filter(|x| {
                if !x.2.is_finite() {
                    birth_death_recorder.write_line(
                        &format!("Individual died natural cuses: {}", x.0)
                    );
                }
                x.2.is_finite()
            }).collect();
            
            // Sort population by score, descending so the best are
            // earliest.  Allows the less good individuals to be easilly
            // pop'd off the end
            &population.0[..].sort_by(|a, b| {
                let a2 = a.2;
                let b2 = b.2;
                b2.partial_cmp(&a2).unwrap_or(Ordering::Equal)
            });

            // If the best individual has changed display it
            let best_idx = 0;
            let _best_id = population.0[best_idx].0;
            if _best_id != best_id {
                best_id = _best_id;
                let this_individual = population.0[best_idx].1.to_string().clone();
                if this_individual != best_individual {
                    best_individual = this_individual.clone();
                    println!("G {} ID: {} Sc:{}\n{}\n",
                             generation, population.0[best_idx].0, population.0[best_idx].2, population.0[best_idx].1.to_pretty_string(0));

                    // Best tree
                    let ref n = population.0[best_idx].1;

                    // ID to lable it
                    let lable = population.0[best_idx].0;

                    // Store its data
                    add_simulation(simulate(&n, &d_all), lable,
                                   model_data_file.as_str());
                }
            }
            
            let mut total_score = 0.0;
            for x in population.0.iter() {
                total_score += x.2;
            }

            // Choose a node from population to participate in crossover.
            // The higher the score the node got last generation the
            // higher the probability it will be selected to be
            // participate in crossover
            macro_rules! get_node {
                () => {
                    {
                        let mut p:Option<usize> = None;

                        // The selector.  By setting the floor to more
                        // than 0 nodes with 0.0 score will not get
                        // selected.  
                        let s = e.gen_rangef64(0.000001, total_score);
                        
                        let mut cum_score = 0.0;
                        for i in 0..population.0.len() {
                            let t:&Tree = &population.0[i];
                            cum_score += t.2;
                            if cum_score >= s {
                                p = Some(i);
                                break;
                            }
                        }
                        p
                    }
                }
            };

            // The number of crossovers to do is (naturally)
            // population.len() * crossover_percent/100
            let ncross = population.0.len() * crossover_percent/100;
            for _ in 0..ncross {
                // Choose two trees to cross over
                let  pc; // Node resulting from crossover
                let i0 = get_node!().unwrap();
                let i1 = get_node!().unwrap();
                let mut flag = false;  // Set to true if pc is unique

                let mut s = 0.0; // Score
                {
                    // Block to limit scope of p0 and p1
                    let ref p0 = &population.0[i0];
                    let ref p1 = &population.0[i1];
                    pc = crossover(&p0.1, &p1.1, &mut e);
                    let st = pc.to_string();
                    population.1.entry(st.clone()).or_insert(false);
                    if !population.1.get(&st).unwrap() {
                        // This node is unique
                        population.1.insert(st, true);
                        flag =  true;
                    }else{
                    }
                    
                    if flag {
                        maxid += 1;  // Done here so it can be passed to record_birth
                        s = score_individual(&pc, &d_all, false);
                        birth_death_recorder.write_line(&format!("{} + {} = {}/{}: {}", p0.0, p1.0, maxid, s, pc.to_string()));
                    }
                }
                if flag {
                    //println!("Befoe score: {}  ", maxid);
                    population.0.push((maxid, pc, s));
                }
            }
            // Adjust population
            if population.0.len() > max_population {
                while population.0.len() > max_population {
                    for _ in 1..cull_size {
                        let p = population.0.pop().unwrap();
                        birth_death_recorder.write_line(&format!("RIP {}", p.0)[..]);
                    }
                }
                while population.0.len() < max_population {
                    let n = Box::new(Node::new(&mut e, &d_all.names, 0));
                    let st = n.to_string();
                    population.1.entry(st.clone()).or_insert(false);
                    if !population.1.get(&st).unwrap() {
                        // This node is unique
                        maxid += 1;
                        population.1.insert(st, true);
                        let sc = score_individual(&n, &d_all, false);
                        {
                            birth_death_recorder.write_line(&format!("{}/{}: {}", maxid, s, n.to_string()));
                        }
                        population.0.push((maxid, n, sc));
                    }
                }                
            }
            let mut hh:HashMap<String, usize> = HashMap::new();
            for i in 0..population.0.len() {
                let k = population.0[i].1.to_string();
                let n = hh.entry(k).or_insert(0);
                *n += 1;
            }
            
            // for h in hh.keys() {
            //     println!("TEST {} {}", hh.get(h.as_str()).unwrap(), h);
            // }
            //println!("Population size is {} with {} unique individuals", population.0.len(), hh.keys().len())
        }
        write_plotting_script(model_data_file.as_str(),
                              plot_xlab.as_str(),
                              plot_file.as_str(),
                              r_script_file.as_str());
        println!("Bye!");
    }

}
