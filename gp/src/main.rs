extern crate rand;

use rand::Rng;
use rand::SeedableRng;
use rand::StdRng;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
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
        &TerminalType::Float(f) => format!("Float {}",f),
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
    Terminal(TerminalType),
}

// The basic unit of aAST
struct Node {
    // Operator and a left and right child trees, or None.  
    o:Operator,
    l:Option<Box<Node>>,
    r:Option<Box<Node>>,
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
}

impl Node {

    // Build a random tree
    fn new(e:&mut Entropy, names:&Vec<String>, level:usize) -> Node {
        let l = level+1;
        let a = if level > 10 {
            4
        }else{
            e.gen_range(0, 6)
        };
        //print!("level {} ", l);
        macro_rules! NewNode {
            ($name:ident) => {
                Node{o:Operator::$name,
                     l: Some(Box::new(Node::new(e , names, l))),
                     r: Some(Box::new(Node::new(e, names, l)))
                }
            }
        };
        match a {
            0 => NewNode!(Add),
            1 => NewNode!(Multiply),
            2 => NewNode!(Invert),
            3 => NewNode!(Negate),
            4 => {
                // Input node
                let n = names.len() - 1; // -1 as last name/column is solution
                let b = e.gen_range(0, n);
                let s = names[b].clone();
                Node{o:Operator::Terminal(TerminalType::Inputf64(s)), l:None, r:None}
            }
            _ => Node{o:Operator::Terminal(TerminalType::Float(e.gen())), l:None, r:None}
        }
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
                    ret.push_str("$name ");
                    child_to_string!(l);
                }
            }
        };
        
        match self.o {
            Operator::Add => node_to_string2!(Add),
            Operator::Multiply => node_to_string2!(Multiply),
            Operator::Negate => node_to_string1!(Negate),
            Operator::Invert => node_to_string1!(Invert),
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
                    None => panic!("Missing child") ,
                }
            }
        }
        match self.o {
            Operator::Terminal(TerminalType::Float(f)) => Some(f),
            Operator::Terminal(TerminalType::Inputf64(ref s)) => Some(*(inputs.dataf.get(s).unwrap())),
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
}

// Hold data for training or testing.
 struct Data {
    
    // Names of the columns
    names:Vec<String>, 

    // Each row is a hash keyed by names FIXME Inefficient(?) use of memory
    rows:Vec<HashMap<String, f64>>, 
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
fn read_data() -> std::io::Result<Data> {
    // Must be in file 'data.in'.  First row is header

    let mut ret = Data{names:Vec::<String>::new(), rows:Vec::<HashMap<String, f64>>::new()};

    let file = File::open("Data.in")?;
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
            let v = d[i].parse::<f64>().unwrap();
            ret.rows[ln].insert(k, v);
        }
        ln += 1; 
    }

    Ok(ret)
}

fn main() {
    println!("Start");
    let mut e = Entropy::new(&[11,2,3,422, 195]);
     // let mut e = Entropy::new(&[11,2,3,4]);

    let d:Data = read_data().unwrap();

    // Create a population
    let mut population:Vec<(Box<Node>, f64)> = Vec::new();
    for _ in  1..25 {
        let n = Box::new(Node::new(&mut e, &d.names, 0));
        population.push((n, 0.0));
    }
    
    for r in d.rows.iter() {
        // For each row of data Create a input
        let mut inputs = Inputs{
            dataf:HashMap::new(),
        };
        for h in d.names.iter() {
            let k = h.clone();
            let v1 = r.get(&k);
            let v:f64 = *v1.unwrap();
            inputs.dataf.insert(k, v);
        }

        // For each member of the population calculate a evaluation
        for i in  0..24 { // FIXME Use a iterator over population
            let e = population[i].0.evaluate(&inputs).unwrap();
            population[i].1 = e;
        }            
    }
}
