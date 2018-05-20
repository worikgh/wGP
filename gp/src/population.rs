use config::Config;
use std::collections::HashMap;    

use super::Data;
use super::NodeBox;
use super::Recorder;
use super::entropy::Randomness;
use super::Node;
use super::score_individual;
use super::simulate;
use super::add_simulation;
use std::cmp::Ordering;
use std::io::Write;

// Define a individual.  Consists of a node, a id, and a score.  Called
// a Tree because it is not a sub-tree...
type Tree = (usize, NodeBox, f64); 

pub struct Population<'b> {
    trees:Vec<Tree>,
    str_rep:HashMap<String, bool>,
    maxid:usize,
    bnd_rec: Recorder,
    d_all:Data,
    crossover_percent:usize,
    mutate_prob:usize,
    copy_prob:usize,
    best_id:usize,
    best_individual:String,
    model_data_file:String,
    e:&'b mut Randomness,
    input_names:Vec<String>,
    max_population:usize,
}
impl<'a> Population<'a> {
    pub fn new(config:&Config, e:&'a mut Randomness) -> Population<'a> {
        // Load the data
        let mut d_all = Data::new();
        let df = config.get_string("data_file").unwrap() ;
        let dp = config.get_usize("training_percent").unwrap();
        d_all.read_data(df.as_str(),
                        dp, e);
        let ret = Population{trees:Vec::new(), str_rep:HashMap::new(),
                             input_names:d_all.names.clone(),
                             maxid:0, e:e,
                             bnd_rec:Recorder::new(config.get_string("birthsanddeaths_file").unwrap().as_str()),
                             crossover_percent:config.get_usize("crossover_percent").unwrap(),
                             d_all:d_all,
                             max_population:config.get_usize("max_population").unwrap(),
                             mutate_prob:config.get_usize("mutate_prob").unwrap(),
                             copy_prob:config.get_usize("copy_prob").unwrap(),
                             best_id:0, best_individual:"".to_string(),
                             model_data_file:config.get_string("model_data_file").unwrap(),
        };
        ret
    }


    fn best_idx(&self) -> usize {
        0
    }    
    pub fn best_id(&self) -> usize {
        self.trees[self.best_idx()].0
    }
    pub fn best_score(&self) -> f64 {
        self.trees[self.best_idx()].2
    }
    pub fn len(&self) -> usize {
        self.trees.len()
    }
    fn check_peek(&mut self, generation:usize){
        let _best_id = self.trees[self.best_idx()].0;
        if _best_id != self.best_id {
            self.best_id = _best_id;
            let this_individual = self.get_tree_id(self.best_id).1.to_string().clone();
            if this_individual != self.best_individual {
                let sc = score_individual(&self.trees[self.best_idx()].1, &self.d_all, false);
                self.best_individual = this_individual.clone();
                println!("G {} ID: {} Sc:{}\n{}\n",
                         generation,
                         self.trees[self.best_idx()].0,
                         sc,
                         self.trees[self.best_idx()].1.to_pretty_string(0));

                // Best tree
                let ref n = self.trees[self.best_idx()].1;

                // ID to lable it
                let lable = self.trees[self.best_idx()].0;

                // Store its data
                let simulation = simulate(&n, &self.d_all);
                add_simulation(simulation, lable,
                               self.model_data_file.as_str());
            }
        }
    }
    fn stats(&self) -> String {
        let mut n0 = 0; // Cound individuals with score 0
        let mut n1 = 0; // Cound individuals with score > 0
        let mut nn = 0;  // Cound individuals with score > not a number
        let mut cum = 0.0; // Cumulative score
        let mut max = 0.0; // Max score
        let mut min = 1.0; // Minimum score
        
        for i in 0..self.len() {
            let sc = self.trees[i].2;
            if sc.is_finite() {
                cum += sc;
                if sc < min {
                    min = sc;
                }
                if sc > max {
                    max = sc;
                }
                if sc == 0.0 {
                    n0 += 1;
                }else{
                    n1 += 1;
                }
            }else{
                nn += 1;
            }
        }
        format!("Total score: {} mean score {} population {} min {} max {} n0 {} n1 {} nn {} Zero?  {}",
                cum, cum/(self.len() as f64), self.len(), min, max, n0, n1, nn, n0+n1+nn-self.len())
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
                self.trees.push((id, nb, sc));
                self.bnd_rec.write_line(&format!("Cross: {} + {} --> {}/{}: {}",
                                                l, r, id, sc, nbstr));
            }else{
                //println!("Not unique l {} r {} new {}",l,r,st);
            }
            nc += 1;
        }
    
        // Do mutation.  Take mut_probab % of trees, mutate them, add
        // them to the new population
        for i in 0..self.trees.len() {
            if self.e.gen_range(0, 100) < self.mutate_prob {
                let sc_0:String = (*self.trees[i].1).to_string();
                let id0 = self.trees[i].0;
                let t = self.trees[i].1.copy();
                let nb = self.mutate_tree(t);
                let sc_1:String = (*nb).to_string();
                self.str_rep.entry(sc_1.clone()).or_insert(false);
                if !self.str_rep.get(&sc_1).unwrap() {
                    // Unique in the new population
                    let sc = score_individual(&nb, &self.d_all, true);
                    self.maxid += 1;
                    let id = self.maxid;
                    new_trees.push((id, nb, sc));
                    self.str_rep.insert(sc_1.clone(), true);
                    self.bnd_rec.write_line(format!("Mutate: {} --> {}: {} --> {}/{}",
                                                    id0, id, sc_0, sc_1, sc).as_str());
                    //println!("Mutate {} ", id);
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
                new_trees.push((self.trees[cx].0, self.trees[cx].1.copy(), self.trees[cx].2));
                self.str_rep.insert(st.clone(), true).unwrap();
                cp += 1;
                //println!("Gen {} Insert individual {} {}", generation, self.trees[cx].0, self.trees[cx].2);
            }else{
                //println!("Gen {} Abandon individual {} {}", generation, self.trees[cx].0, self.trees[cx].2);
            }
            cx += 1;
        }    

        // New population is created
        self.trees = new_trees;
        // Eliminate all trees with no valid score and sort them
        self.cull_sort();

        // println!("G {} <Sorted {} Best {} or {}",
        //          generation,
        //          self.trees[0].2>self.trees[self.trees.len()-1].2,
        //          self.get_tree_id(self.best_id()).2,
        //          self.trees[self.trees.len()-1].2);

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

        self.check_peek(generation);
        
    }
    pub fn cull_sort(&mut self) {
        // Remove individuals that we can no longer let live.  Eugenics!
        // Individuals with score NAN or 0
        let mut z:Vec<Tree> = Vec::new();
        for i in 0..self.trees.len(){
            let (id, ev) = (self.trees[i].0, self.trees[i].2);
            if ev.is_finite() && ev > 0.0{
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
            let a2 = a.2;
            let b2 = b.2;
            b2.partial_cmp(&a2).unwrap_or(Ordering::Equal)
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

    pub fn get_tree(&self, id:usize) -> &Tree {
        // Get a tree based on its order in self.trees. Used to
        // inumerate all trees.  FIXME Use a iterator
        &self.trees[id]
    }
    fn _initialise(&mut self){
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
        loop {
            self._initialise();
            self.cull_sort();
            println!("Population initial size : {}", self.len());
            if self.len() > 0 {
                break;
            }
        }
    }
    fn add_individual(&mut self) -> bool {
        // Add a individuall.  If the individual is already in the
        // population do not add it and return false
        let n = Box::new(Node::new(self.e, &self.d_all.names, 0));
        let st = n.to_string();
        self.str_rep.entry(st.clone()).or_insert(false);

        if !self.str_rep.get(&st).unwrap() {
            // This node is unique
            self.maxid += 1;
            self.str_rep.insert(st, true);
            let sc = score_individual(&n, &self.d_all, true);
            {
                self.bnd_rec.write_line(&format!("Create {}/{} {}", self.maxid, sc, n.to_string()));
                if sc == 1.0 {
                    // Found the perfect individual.  
                    println!("Found perfect Node! {}", n.to_string());
                }
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
    pub fn total_score(&self) -> f64 {
        let mut ret:f64 = 0.0;
        for x in self.trees.iter() {
            if x.2.is_finite() {
                ret += x.2;
            }
        }
        ret
    }
    pub fn do_crossover(&mut self) -> (NodeBox, usize, usize){
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
                    let ts = self.total_score(); // FIXME  Cache this!
                    if ts == 0.0 {
                        // No model is better than the mean so just pick at random
                        let len = self.len();
                        let i = self.e.gen_range(0, len);
                        //println!("No model better than mean: len: {} i {}", len, i);
                        p = Some(i);
                    }else{
                        let s = self.e.gen_rangef64(0.000001, ts);
                        //println!("Better than mean available: s: {}", s);
                        let mut cum_score = 0.0;
                        let mut ms = 0.0; // Max score so far
                        for i in 0..self.len() {
                            let t:&Tree = self.get_tree(i);
                            
                            if t.2.is_finite() {
                                if t.2 > ms {
                                    ms = t.2;
                                    //println!("Max score exceeded s {} cum_score {} i {} ms {}", s, cum_score, i, ms);
                                }
                                cum_score += t.2; 
                                if cum_score >= s {
                                    //println!("Got tree: i: {} cum_score {} s {} len {} ts {}", i, cum_score, s, self.len(), ts);
                                    p = Some(i);
                                    break;
                                }
                            }
                        }
                    }
                    p
                }
            }
        };

        // Choose two trees to cross over
        let i0 = get_node!().unwrap();
        let i1 = get_node!().unwrap();
        
        (self.crossover(i0, i1), i0, i1)
    }

    fn mutate_tree(&mut self,i:NodeBox) -> NodeBox {
        //let names = &self.names;
        // How many nodes are there?
        let nc = i.count_nodes();
        // In decision branch?
        let dnc = match i.d {
            Some(ref d) => {
                // println!("Got d: {}", d.to_string());
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
        // println!("dnc {} lnc {} rnc {} nc {}\n{}\n", dnc, lnc, rnc, nc, i.to_pretty_string(0));
        assert_eq!(dnc+lnc+rnc, nc-1);

        // Choose which tree to mutate
        let selector = self.e.gen_range(0, nc+1);
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
                let child = Node::new(self.e, &self.input_names, 0);
                // Select which branch
                let selector = self.e.gen_range(0, nc-1);
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

        
        let p:NodeBox;// Parent
        let c:NodeBox;// Child
        if self.e.gen() > 0.0 {
            p = self.trees[lidx].1.random_node(self.e);
            c = self.trees[ridx].1.random_node(self.e);
        }else{
            c = self.trees[lidx].1.random_node(self.e);
            p = self.trees[ridx].1.random_node(self.e);
        }
        // Make the new tree by copying c
        let c = c.copy();

        // The tree to return
        let mut ret = p.copy();
        
        // Choose a branch off p to copy c to
        match (*ret).r {
            Some(_) => {
                // p has two children.  Choose one randomly
                if self.e.gen() > 0.0 {
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
}    

