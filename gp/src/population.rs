use config::Config;
use node::Node;
use node::NodeBox;
use rng;

use inputs::Inputs;
use score::Score;
use std::cmp::Ordering;
use std::collections::HashMap;    
use std::fs::File;
use std::io::prelude::*;
//use std::from_str::from_str;
//use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
//use std::io::File;
use std::io::Write;
use std;
use super::Data;
use super::Recorder;
use super::score_individual;

// Define a individual.  Consists of a node, a id, and a score.  Called
// a Tree because it is not a sub-tree...
type Tree = (usize, NodeBox, Score); 

enum Mode {
    Create, // Build a tree from scratch
    Resume, // Read a tree from a file and contine evolving it
    Run, // Read a tree from a file and use it as a classifier
}
pub struct Population<'a> {
    trees:Vec<Tree>,
    str_rep:HashMap<String, bool>,
    maxid:usize,

    bnd_rec: Recorder,

    d_all:&'a Data,
    crossover_percent:usize,
    mutate_prob:usize,
    copy_prob:usize,
    save_file:String,
    classification_file:String,
    input_names:Vec<String>,
    max_population:usize,
    mode:Mode,

    // If non 0 pick best 'filter' rules
    filter:usize, 

    // If set then then when reading in trees reclassify them with
    // score_individual
    rescore:bool, 
}

impl<'a> Population<'a> {
    pub fn new(config:&Config, d_all:&'a Data, bnd_rec:Recorder) -> Population<'a> {
        // Load the data

        let ret = Population{trees:Vec::new(),
                             str_rep:HashMap::new(),

                             rescore:match config.get_string("rescore") {
                                 Some(x) =>
                                 // A string.  Must be valid usize.  If it is 0 then false 
                                     if x.parse::<usize>().unwrap() == 1 {
                                         true
                                     }else if x.parse::<usize>().unwrap() == 0 {
                                         false
                                     }else{
                                         panic!("Invalid rescore: {}", x)
                                     },
                                 // Default
                                 None => false,
                             },
                                 
                             filter:config.get_usize("filter").unwrap_or(0),
                             mode:match config.get_string("mode").unwrap().as_str(){
                                 "Create" => Mode::Create,
                                 "Resume" => Mode::Resume,
                                 "Run" => Mode::Run,
                                 x => panic!("{}", x),
                             },
                             input_names:d_all.input_names.clone(),
                             maxid:0,
                             save_file:config.get_string("save_file").unwrap(),
                             bnd_rec:bnd_rec,
                             crossover_percent:config.get_usize("crossover_percent").unwrap(),
                             d_all:d_all,
                             max_population:config.get_usize("max_population").unwrap(),
                             mutate_prob:config.get_usize("mutate_prob").unwrap(),
                             copy_prob:config.get_usize("copy_prob").unwrap(),
                             classification_file:config.get_string("classification_file").unwrap(),
                             
        };
        ret
    }

    fn initialise_rand(&mut self){
        // Initialise with a random tree
        loop {
            // Random individual.  Returns true when a unique
            // individual is created.
            while !self.add_individual() {} 
            if self.len() == self.max_population {
                break;
            }
        }
    }        
    pub fn initialise(&mut self){
        match  self.mode {
             Mode::Create => {
                 loop {
                     self.initialise_rand();
                     self.cull_sort();
                     if self.len() > 0 {
                         break;
                     }
                 }
             },
            _ => self.trees = self.restore_trees(),
        }
    }

    // FIXME The mode should be stored outside the population
    pub fn do_classify(&self) -> bool {
        match self.mode {
            Mode::Run => true,
            _ => false,
        }
    }

    pub fn do_train(&self) -> bool {
        match self.mode {
            Mode::Run => false,
            _ => true,
        }
    }
    
    fn best_idx(&self) -> usize {
        0
    }    
    pub fn best_id(&self) -> usize {
        self.trees[self.best_idx()].0
    }
    pub fn best_score(&self) -> Score {
        self.trees[self.best_idx()].2.copy()
    }
    pub fn len(&self) -> usize {
        self.trees.len()
    }

    fn classify(&self, case:&Vec<f64>) -> String {
        // Classify a case using the population.  @param `case` is
        // the case to classify.  

        // Create the input structure
        let mut input = Inputs::new();
        for j in 0..self.d_all.input_names.len() {
            let v:f64 = case[j];
            input.insert(&self.d_all.input_names[j], v);
        }

        // Store the results of each classifier.  The class of the
        // classifier is used as the key and keep each result and the
        // score/quality
        let mut results:HashMap<&String, Vec<(f64,f64)>> = HashMap::new();
        for c in self.d_all.class_names.iter() {
            results.insert(c, Vec::new());
        }
        for t in self.trees.iter() {
            // Using each classifier
            let class = &t.2.class;
            let score = t.1.evaluate(&input).unwrap();
            if score.is_finite() {
                let quality = t.2.special;
                results.get_mut(class).unwrap().push((quality, score));
            }
        }

        // Interpretation.  The class with the highest score, weighted
        // by the quality (score.special) and divided by the count of
        // classifiers, is the class to choose.  The magnitude of the
        // score relative to ho many classifiers contributed is a
        // measure of quality of classification.  As is the score for
        // other classes

        let mut scores:Vec<(&String, f64)> = Vec::new();
        for k in self.d_all.class_names.iter() {
            let count = results.get(k).unwrap().len();
            let score = match count {
                0 => 0.0,
                _ => results.get(k).unwrap().iter().fold(0.0, |mut sum, &x| {sum += x.0*x.1; sum})  / (count  as f64)
            };
            scores.push((&k, score));
        }

        scores.sort_by(|a,b| {
            let a1 = &a.1;
            let b1 = &b.1;
            b1.partial_cmp(a1).unwrap_or(Ordering::Equal)
        });

        let mut ret = String::new();
        for s in scores {
            ret += s.0;
            ret += " ";
            ret += s.1.to_string().as_str();
            ret += " ";
        }
        ret

    }
    
    fn check(&self) -> bool {
        let mut ret = true;
        for i in self.trees.iter() {
            if !i.2.special.is_finite() {
                ret = false;
                break;
            }
        }
        ret
    }
    
    fn stats(&self) -> String {

        let mut nns = 0;  // Count individuals with special score > not a number
        let mut cums = 0.0; // Cumulative special score
        let mut maxs = 0.0; // Max special score
        let mut mins = std::f64::MAX; // Minimum special score

        for i in 0..self.len() {
            let sc = &self.trees[i].2;

            if sc.special.is_finite() {
                cums += sc.special;
                if sc.special < mins {
                    mins = sc.special;
                }
                if sc.special > maxs {
                    maxs = sc.special;
                }
            }else{
                nns += 1;
            }

        }
        format!("Population {} Total score: {} Mean score: {} Min score: {} Max score: {} Invalid Score: {}",
                self.len(),
                cums,
                cums/(self.len() as f64),
                mins,
                maxs,
                nns)
    }
    
    pub fn new_generation(&mut self, generation:usize){

        // Call every generation

        println!("New generation: {} {}", generation, self.stats());

        // Create a new population of trees to replace the old one
        let mut new_trees:Vec<Tree> = Vec::new();

        // To be ready for a new population reset self.str_rep which
        // checks for duplicates
        self.str_rep.clear();       
        
        // Generate some of new population from the old population. The
        // number of crossovers to do is (naturally) population.len()
        // * crossover_percent/100
        let ncross = self.len() * self.crossover_percent/100;
        
        let mut nc = 0;
        while nc < ncross  {

            // 
            let (nb, l, r) = self.do_crossover();

            let st = (*nb).to_string();
            self.str_rep.entry(st.clone()).or_insert(false);
            if !self.str_rep.get(&st).unwrap() {
                // A unique child
                let sc = score_individual(&nb, &self.d_all, true);
                self.maxid += 1;
                let id = self.maxid;
                self.str_rep.insert(st, true);
                let nbstr = (*nb).to_string();
                self.bnd_rec.write_line(&format!("Cross: {} + {} --> {}/(Sc:{}): {}",
                                                l, r, id, &sc.special, nbstr));
                self.trees.push((id, nb, sc));
            }else{
                // Child was not unique
            }
            nc += 1;
        }

        // Do mutation.  Take mut_probab % of trees, mutate them, add
        // them to the new population
        for i in 0..self.trees.len() {
            if rng::gen_range(0, 100) < self.mutate_prob {

                // The id of the tree being mutated
                let id0 = self.trees[i].0;

                // Copy the tree adn mutate it
                let t = self.trees[i].1.copy();
                let nb = self.mutate_tree(t);

                // Convert to a string to check for duplicates and for
                // the record
                let sc_1:String = (*nb).to_string();
                self.str_rep.entry(sc_1.clone()).or_insert(false);
                
                if !self.str_rep.get(&sc_1).unwrap() {
                    // Unique in the new population

                    let sc = score_individual(&nb, &self.d_all, true);
                    self.maxid += 1;
                    let id = self.maxid;

                    self.bnd_rec.write_line(format!("Mutate: {} --> {}: {}/(Sc: {})",
                                                    id0, id, sc_1, &sc.special).as_str());
                    self.str_rep.insert(sc_1, true);
                    new_trees.push((id, nb, sc));
                }                
            }
        }

        // Copy the best trees.
        let mut cp = 0; // Number copied
        let mut cx = 0; // Index into self.trees
        let ncp = self.len()*100/self.copy_prob;
        while cp < ncp && cx < self.len() {
            // FIXME This should be probabilistic with roulette wheel
            // selection
            let st = (*self.trees[cx].1).to_string();
            self.str_rep.entry(st.clone()).or_insert(false);
            if !self.str_rep.get(st.as_str()).unwrap() {
                new_trees.push((self.trees[cx].0, self.trees[cx].1.copy(), self.trees[cx].2.copy()));
                self.str_rep.insert(st.clone(), true).unwrap();
                cp += 1;
            }
            cx += 1;
        }    

        // New population is created
        self.trees = new_trees;
        // Eliminate all trees with no valid score and sort them
        self.cull_sort();

        // Adjust population
        let mut n1 = 0; // Number of individuals deleted
        let mut n2 = 0; // Number of individuals added
        if self.len() > self.max_population {
            while self.len() > self.max_population {
                let _ = self.delete_worst();
                n1 += 1;
            }
        }
        let flag =  self.len() < self.max_population; // Set if new individuals  to be added
        while self.len() < self.max_population {
            while !self.add_individual() {}
            n2 += 1;
        }
        if flag {
            // Sort again as we added new individuals
            self.cull_sort(); // Fixme.  'cull' could be a pipe line.  Take a vector of trees and return a vector of trees
        }
        println!("Population B: {} deleted: {} Added {}", self.len(), n1, n2);
        self.bnd_rec.buffer.flush().unwrap(); 

        // Write out a record of all scores to do statistical analysis to help with debugging
        let mut scores:Vec<f64> = Vec::new();
        for i in 0..self.trees.len() {
            let s = self.trees[i].2.special;
            scores.push(s)
        }

        let mut r = Recorder::new("fitness.csv");
        let mut l1 = "".to_string();
        let mut l2 = "".to_string();
        l1 += &generation.to_string();
        l2 += &generation.to_string();
        l1 += ",G,";
        l2 += ",S,";
        for i in scores {
            l2 += &i.to_string();
            l2 += ",";
        }
        r.write_line(&l1);
        r.write_line(&l2);
        if ! self.check() {
            panic!("Check failed");
        }
        self.save_trees();
    }

    fn _unique_node(&self) -> NodeBox {
        // Generate a node and check if it is unique
        
        let n:NodeBox;
        loop {

            // Make a node
            let _n = Box::new(Node::new(&self.d_all.input_names, 0));

            // Check for uniqueness
            let st = _n.to_string();
            if !self.str_rep.contains_key(st.as_str()) {
                // Is unique
                n = _n;
                break;
            }
        }
        n
    }        
    
    // pub fn new_individual(&self) -> Tree {
    //     // Create a random tree and return it
    //     let n = self._unique_node();
    //     // Find the class of the new node
    // }

    pub fn cull_sort(&mut self) {
        // Remove individuals that we can no longer let live.  Eugenics!
        // Individuals with score NAN or 0
        let mut z:Vec<Tree> = Vec::new();
        for i in 0..self.trees.len(){
            let (id, ev) = (self.trees[i].0, self.trees[i].2.copy());
            if ev.special.is_finite(){
                let t:Tree = (self.trees[i].0, self.trees[i].1.copy(), ev);
                z.push(t);
            }else{
                self.bnd_rec.write_line(
                    &format!("RIP {} culled", id)
                );
            }
        }
            
        self.trees = z;
        // Sort population by score, descending so the best are
        // earliest.  Allows the worst individuals to be easilly
        // pop'd off the end
        &self.trees[..].sort_by(|a,b| {
            let a2 = &a.2;
            let b2 = &b.2;
            b2.partial_cmp(a2).unwrap_or(Ordering::Equal)
        });
        // self.trees =
        //  self.trees.into_iter().filter(|x| {
        //     if !x.2.is_finite() {
        //         self.bnd_rec.write_line(
        //             &format!("RIP {} culled", x.0)
        //         );
        //     }
        //     x.2.is_finite()
        // }).collect();
    }            
    pub fn get_tree_id(&self, id:usize) -> &Tree {
        // Get a tree based on its id
        let mut idx:Option<usize> = None;
        for i in 0..self.trees.len() {
            if self.trees[i].0 == id {
                idx = Some(i);
                break;
            }
        }
        match idx {
            Some(idx) => &self.trees[idx],
            None => panic!("Cannot get node with id: {}", id),
        }
    }

    #[allow(dead_code)]
    pub fn get_tree(&self, id:usize) -> &Tree {
        // Get a tree based on its order in self.trees. Used to
        // inumerate all trees.  FIXME Use a iterator
        &self.trees[id]
    }

    #[allow(dead_code)]
    fn get_trees_of_class(&self, class:&String) -> Vec<&Tree> {
        // FIXME: Ho do e do string comparison better in rust?  Is
        // there a problem ith this?
        let test = class.clone();
        self.trees.iter().filter(|t| t.2.class == test).collect()
    }
        
    #[allow(dead_code)]
    fn get_classes(&self) -> &Vec<String>{
        // Return all known class lables
        &self.d_all.class_names
    }

    
    //===============================================================
    //
    // Selection algorithms.
    //

    fn select(&self) -> usize {
        // FIXME Implement choice of selection algorithm
        Population::roulette_selection(&self.trees)
    }

    fn roulette_selection(trees:&Vec<Tree>) -> usize {

        // Return the id of a individual selected using roulette wheel
        // selection:
        // https://en.wikipedia.org/wiki/Fitness_proportionate_selection

        let total_score:f64 = trees.iter().fold(0.0, | a, ref b| if b.2.is_finite() {
            a+b.2.evaluate()
        }else{
            0.0
        });

        if total_score == 0.0 {
            trees[rng::gen_range(0, trees.len() - 1)].0
        }else{

            let sel = rng::gen_range(0.0, total_score); 
            let mut  acc = 0.0;
            let mut ret:Option<usize> = None;  // Index of selected individual
            for i in trees.iter() {
                acc += i.2.evaluate();
                if acc > sel {
                    ret = Some(i.0);
                    break;
                }
            }
            match ret {
                Some(r) => r,
                None => {
                    // This should not happen
                    panic!("Could not select individual acc: {} sel: {} total_score: {}",
                             acc, sel, total_score)
                },
            }
        }
    }

    // End of selection algorithms
    //
    // ========================================

    fn add_individual(&mut self) -> bool {
        // Add a individuall.  If the individual is already in the
        // population do not add it and return false
        let n = Box::new(Node::new(&self.d_all.input_names, 0));

        let st = n.to_string();
        self.str_rep.entry(st.clone()).or_insert(false);

        if !self.str_rep.get(&st).unwrap() {
            // This node is unique
            self.maxid += 1;
            self.str_rep.insert(st, true);

            let sc = score_individual(&n, &self.d_all, true);
            {
                self.bnd_rec.write_line(&format!("Create {}/(Sc: {}) {}", self.maxid, sc.special, n.to_string()));
                self.trees.push((self.maxid, n, sc));
            }
            true
        }else{
            false
        }
    }
    pub fn delete_worst(&mut self) -> Tree {
        let expired = self.trees.pop().unwrap();
        self.bnd_rec.write_line(&format!("RIP {}", expired.0)[..]);
        expired
    }
    pub fn do_crossover(&mut self) -> (NodeBox, usize, usize){

        // Crossover to breed individuals better at generalisation

        // Choose a node from population to participate in crossover.
        // The higher the score the node got last generation the
        // higher the probability it will be selected to be
        // participate in crossover

        // Choose two trees to cross over.  
        let i0 = self.select();
        let i1 = self.select();
        
        (self.crossover(i0, i1), i0, i1)
    }

    fn mutate_tree(&mut self,i:NodeBox) -> NodeBox {
        //let names = &self.names;
        // How many nodes are there?
        let nc = i.count_nodes();
        // In decision branch?
        let dnc = match i.d {
            Some(ref d) => {
                d.count_nodes()
            },
            None => 0,
        };
        // In left child?
        let lnc = match i.l {
            Some(ref l) => l.count_nodes(),
            None => 0,
        };
        // In right child
        let rnc = match i.r {
            Some(ref r) => r.count_nodes(),
            None => 0,
        };

        assert_eq!(dnc+lnc+rnc, nc-1);

        // Choose which tree to mutate
        let selector = rng::gen_range(0, nc+1);
        if selector < dnc {
            self.mutate_tree(i.d.unwrap())
        }else if selector < dnc + lnc {
            self.mutate_tree(i.l.unwrap())
        }else if selector < dnc + lnc + rnc {
            self.mutate_tree(i.r.unwrap())
        }else{
            // Mutate i
            // Two cases: This is a terminal, this is not terminal
            if nc == 1 {
                // i is a terminal.  FIXME  Mutate this!
                i.copy()
            }else{
                // i is not terminal
                let mut ret = i.copy();
                let child = Node::new(&self.input_names, 0);
                // Select which branch
                let selector = rng::gen_range(0, nc-1);
                if selector < lnc {
                    ret.l = Some(NodeBox::new(child));
                }else if selector < rnc + lnc {
                    ret.r = Some(NodeBox::new(child));
                }else if selector < dnc + lnc + rnc {
                    ret.d = Some(NodeBox::new(child));
                }else{
                    panic!("selector {} is invalid lnc {} rnc {} dnc {} nc {}",
                           selector, lnc, rnc, dnc, nc);
                }
                ret
            }
        }

    }

    
    fn crossover(&mut self, lidx:usize, ridx:usize) -> NodeBox {

        // FIXME Use references to nodes (and lifetimes?) insted of
        // indexes.  Save on lookups

        // Given the indexes of the a left and a right tree combine
        // the two trees to make a third individual
        let p:NodeBox;// Parent
        let c:NodeBox;// Child
        if rng::random::<f64>() > 0.0 {
            p = self.get_tree_id(lidx).1.random_node();
            c = self.get_tree_id(ridx).1.random_node()
        }else{
            c = self.get_tree_id(lidx).1.random_node();
            p = self.get_tree_id(ridx).1.random_node()
        }
        // Make the new tree by copying c
        let c = c.copy();

        // The tree to return
        let mut ret = p.copy();
        
        // Choose a branch off p to copy c to
        match p.d {
            // This node has 3 legs

            Some(_) =>
                match rng::gen_range(0,3) {
                    0 => (*ret).d = Some(c),
                    1 => (*ret).l = Some(c),
                    2 => (*ret).r = Some(c),
                    _ => panic!("Impossible to get here"),
                },
            None =>
                match p.r {
                    Some(_) => 
                    // This node has two legs
                        if rng::random::<f64>()> 0.0 {
                            (*ret).l = Some(c)
                        }else{
                            (*ret).r = Some(c)
                        },
                    None => (*ret).l = Some(c),
                }
        }
        ret
    }
    pub fn classify_test(&self){
        let mut classification_recorder = Recorder::new(&self.classification_file);
        let ref index = self.d_all.testing_i;
        for i in index {
            let ref r = self.d_all.data[*i];
            let s = self.classify(r);
            let c = self.d_all.get_class(*i);
            classification_recorder.write_line(format!("Class: {}  Classification: {}", c, s).as_str());
        }        
    }
        
        pub fn restore_trees(&self) -> Vec<Tree> {
        // Read in saved state from a file, build a population from
        // it, and return the population
        let f = File::open(&self.save_file).unwrap();
        let buf = BufReader::new(f);
        let mut max_id = 1;

        let mut trees:Vec<Tree> = Vec::new();
        for line in buf.lines() {
            match line {
                Ok(l) => {
                    let start = &l[0..5]; // Class
                    let class:String;
                    if start == "Class" {
                        let n = l.find("Score").unwrap();//expect(panic!("Badly formatted line: {}", l));
                        class = l[5..n].to_string();
                        if &l[n..(n+5)] == "Score" {
                            
                            let m = l.find("Node").unwrap();
                            let score_str = &l[(n+5)..m];
                            let score = score_str.parse::<f64>().unwrap();

                            let node = NodeBox::new(Node::new_from_string(&l[m+4..]));
                            let sc = if self.rescore {
                                // Reevaluate the trees against the
                                // test part of the data
                                let _sc = score_individual(&node, &self.d_all, true);
                                println!("class {} -> {}\tScore {} -> {}", &class, &_sc.class, score, _sc.special);
                                _sc
                                    
                            }else{
                                // Use the saved scores
                                Score{class:class, special:score}
                            };
                            let tree = (max_id, node, sc);
                            trees.push(tree);
                            max_id += 1;
                        }
                    }
                },
                Err(e) => panic!(e)
            }
        }
        if self.filter > 0 {
            // Only have best self.filter trees of each class

            // First sort trees by score
            trees.sort_by(|a, b|{
                let a1 = &a.2;
                let b1 = &b.2;
                b1.partial_cmp(a1).unwrap_or(Ordering::Equal)
            });

            // Build the structure that will hold the trees
            let mut class_trees:HashMap<String,  Vec<Tree>> = HashMap::new();
            for z in self.get_classes() {
                class_trees.insert(z.clone(), Vec::new());
            }

            // Put trees into classes
            for t in trees.iter() {
                let c = t.2.class.clone();
                if class_trees.get(&c).unwrap().len() < self.filter {
                    // FIXME How do I avoid all this copying
                    class_trees.get_mut(&c).unwrap().push((t.0, t.1.copy(), t.2.copy()));
                }
            }

            // Reinitialise trees and check that there are enough
            // trees on the way
            trees.clear();
            for ct in class_trees.keys() {
                if class_trees.get(ct).unwrap().len() < self.filter {
                    eprintln!("Too few trees for class {} Have {} want {}",
                              ct, class_trees.get(ct).unwrap().len(), self.filter);
                }
                for t in class_trees.get(ct).unwrap() {
                    // FIXME How do I avoid all this copying
                    trees.push((t.0, t.1.copy(), t.2.copy()));
                }
            }
        }                
        trees
    }
    pub fn save_trees(&self){
        let mut state = String::new();
        for i in 0..self.trees.len() {
            let t = &self.trees[i];
            state += &format!("Class{}Score{}Node{}\n", t.2.class, t.2.special,t.1.to_string());
        }
        
        let mut buf = BufWriter::new(File::create(&self.save_file).unwrap());
        buf.write(&state.into_bytes()[..]).unwrap();

        
    }
}    

