extern crate rand;
extern crate statistical;

use rand::Rng;
use rand::SeedableRng;
use rand::StdRng;
use statistical::mean;
use std::collections::HashMap;
//use std::env;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::cmp::Ordering;

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
    Terminal(TerminalType),
}

// The basic unit of aAST
type NodeBox = Box<Node>;
struct Node {
    // Operator and a left and right child trees, or None.  
    o:Operator,
    l:Option<NodeBox>,
    r:Option<NodeBox>,
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
            Operator::Add|Operator::Multiply =>
                Some(NodeBox::new(Node::new_from_iter(iter))),
            _ => None,
        };
                
        Node{o:operator, l:l, r:r}
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
            Operator::Add => node_to_string2!(Add),
            Operator::Multiply => node_to_string2!(Multiply),
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
        

        // Macro to make a two child  node into a string
        macro_rules! node_to_string2 {
            ($name:ident) => {
                {
                    for i in 0..level {
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
                    for i in 0..level {
                        ret.push_str(sp);
                    }
                    ret.push_str(stringify!($name));
                    ret.push_str("\n");
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
                for i in 0..level {
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
}// impl Node

#[cfg(test)]
mod tests {
    use super::*;
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

// Calculate the score of a indvidual against the data
// Param n: The individual
// Param d: The data to use
fn score_individual(n:&NodeBox, d:&Data) -> f64 {
    let mut scorev:Vec<f64> = vec![];
    let mut inputs = Inputs{
        dataf:HashMap::new(),
    };
    for r in d.rows.iter() {
        for h in d.names.iter() {
            let k = h.clone();
            let v1 = r.get(&k);
            let v:f64 = *v1.unwrap();
            inputs.dataf.insert(k, v);
        }
        let e = n.evaluate(&inputs).unwrap();
        // Get the target
        let t = inputs.dataf.get(d.names.last().unwrap()).unwrap();
        // Compare
        scorev.push( 1.0/(e-t).abs());
    }
    // Take the mean value of the score
    let ret = mean(&scorev[..]);
    ret
}

fn main() {
    println!("Start");

    // The source of entropy.  This is done this way so the same seed
    // can be used to produce repeatable results
    // let mut e = Entropy::new(&[11,2,3,422, 195]);
    let mut e = Entropy::new(&[11,2,3,4]);

    // Load the data
    let d:Data = read_data().unwrap();

    // Create a population
    // This is the root of a tree
    type Tree = (u64, NodeBox, f64);
    let mut population:Vec<(Tree)> = Vec::new();
    let mut maxid = 0; // Each tree as a ID
    for _ in  1..25 {
        maxid += 1;
        let n = Box::new(Node::new(&mut e, &d.names, 0));
        let s = score_individual(&n, &d);
        population.push((maxid, n, s));
    }
    // For each member of the population calculate a evaluation
    let num_generations = 1500; // FIXME A configurable num_generations
    let mut best_id = 0;
    let mut best_individual = "".to_string();
    for generation in 0..num_generations {
        print!("\r{}\t{}     ", generation, population.len()-1);
        // Filter out members of population that have no valid score (arithmetic error)
        population = population.into_iter().filter(|x| {
            x.2.is_finite()
        }).collect();
        
        // Sort population by score
        &population[..].sort_by(|a, b| {
            let a2 = a.2;
            let b2 = b.2;
            a2.partial_cmp(&b2).unwrap_or(Ordering::Equal)
        });

        // If the best individual has changed display it
        let best_idx = population.len()-1;
        let _best_id = population[best_idx].0;
        if _best_id != best_id {
            best_id = _best_id;
            let this_individual = population[best_idx].1.to_string().clone();
            if this_individual != best_individual {
                best_individual = this_individual.clone();
                println!("ID: {} Sc:{}\n{}\n",
                         population[best_idx].0, population[best_idx].2, population[best_idx].1.to_pretty_string(0));
            }
        }
        
        let mut total_score = 0.0;
        for x in population.iter() {
            total_score += x.2;
        }
        let num_crossovers = 10; // FIXME A configurable num_crossovers

        // Choose a node from population to participate in crossover.
        // The higher the score the node got last generation the
        // higher the probability it will be selected to be
        // participate in crossover
        macro_rules! get_node {
            () => {
                {
                    let mut p:Option<&NodeBox> = None;

                    // The selector.  By setting the floor to more
                    // than 0 nodes with 0.0 score will not get
                    // selected.  
                    let s = e.gen_rangef64(0.000001, total_score);
                    
                    let mut cum_score = 0.0;
                    for i in 0..population.len() {
                        let t:&Tree = &population[i];
                        cum_score += t.2;
                        if cum_score >= s {
                            let ref n1 = t.1;
                            p = Some(n1);
                            break;
                        }
                    }
                    p
                }
            }
        };
        for _ in 0..num_crossovers {
            // Choose two trees to cross over
            let pc; // Node resulting from crossover
            {
                let p0 = get_node!().unwrap();
                let p1 = get_node!().unwrap();

                pc = crossover(&p0, &p1, &mut e);
                // println!("p0: {}", p0.to_string());
                // println!("p1: {}", p1.to_string());
                // println!("pc: {}", pc.to_string());
            }
            let s = score_individual(&pc, &d);
            maxid += 1;
            population.push((maxid, pc, s));
        }
    }
    println!("");
}

