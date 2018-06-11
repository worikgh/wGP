
// The basic unit of aAST
pub type NodeBox = Box<Node>;
use rng;
use std;
use super::Operator;
use super::TerminalType;
use inputs::Inputs;

#[derive(Debug)]
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
    pub fn new(names:&Vec<String>, level:usize) -> Node {
        let l = level+1;

        // FIXME Make this max levela configurable constant
        let maxlevel = 10;
        let a = if level > maxlevel { 
            0
        }else{
            rng::gen_range(0, 18)
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
        match a {
            0 => Node{o:Operator::Terminal(TerminalType::Float(rng::random())), l:None, r:None, d:None},
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
                let b = rng::gen_range(0, n);
                let s = names[b].clone();
                Node{o:Operator::Terminal(TerminalType::Inputf64(s)), l:None, r:None, d:None}
            }
        }
    }
    pub fn count_nodes(&self) -> usize {
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
    pub fn random_node(&self) -> NodeBox {
        // Choose a subtree (node) of this tree (node).  FIXME there
        // is a lot of optimisation to be done.  Paticularly if each
        // node had the number of nodes that are child nodes of this...
        let c = self.count_nodes();
        let mut n = rng::gen_range(0, c);
        let mut node:& Node = self;
        loop {

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
    pub fn copy(&self) -> NodeBox {
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
    pub fn to_string(&self) -> String {
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
