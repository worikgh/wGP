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
    mutate_prob:usize,
    best_id:usize,
    best_individual:String,
    model_data_file:String,
    e:&'b mut Randomness,
    input_names:Vec<String>,
}
impl<'a> Population<'a> {
    pub fn new(config:&Config, e:&'a mut Randomness) ->
        Population<'a> {
            // Load the data
            let mut d_all = Data::new();
            d_all.read_data(config.get_string("data_file").unwrap().as_str(),
                            config.get_usize("training_percent").unwrap(), e);
            Population{trees:Vec::new(), str_rep:HashMap::new(),
                       input_names:d_all.names.clone(),
                       maxid:0, e:e,
                       bnd_rec:Recorder::new(config.get_string("birthsanddeaths_file").unwrap().as_str()),
                       d_all:d_all,
                       mutate_prob:config.get_usize("mutate_prob").unwrap(),
                       best_id:0, best_individual:"".to_string(),
                       model_data_file:config.get_string("model_data_file").unwrap()}
        }
    pub fn best_id(&self) -> usize {
        self.trees[0].0
    }
    pub fn best_score(&self) -> f64 {
        self.trees[0].2
    }
    pub fn len(&self) -> usize {
        self.trees.len()
    }
    fn check_peek(&mut self, generation:usize){
        let best_idx = 0;
        let _best_id = self.trees[best_idx].0;
        if _best_id != self.best_id {
            self.best_id = _best_id;
            let this_individual = self.trees[best_idx].1.to_string().clone();
            if this_individual != self.best_individual {
                self.best_individual = this_individual.clone();
                println!("G {} ID: {} Sc:{}\n{}\n",
                         generation, self.trees[best_idx].0, self.trees[best_idx].2, self.trees[best_idx].1.to_pretty_string(0));

                // Best tree
                let ref n = self.trees[best_idx].1;

                // ID to lable it
                let lable = self.trees[best_idx].0;

                // Store its data
                let simulation = simulate(&n, &self.d_all);
                add_simulation(simulation, lable,
                               self.model_data_file.as_str());
            }
        }
    }
    pub fn new_generation(&mut self, generation:usize){

        // Call every generation

        // Do mutation
        self.mutate();
        self.cull();
        self.bnd_rec.buffer.flush().unwrap(); 

        // Sort population by score, ascending so the best are
        // earliest.  Allows the less good individuals to be easilly
        // pop'd off the end
        &self.trees[..].sort_by(|b, a| {
            let a2 = a.2;
            let b2 = b.2;
            b2.partial_cmp(&a2).unwrap_or(Ordering::Equal)
        });

        // If the best individual has changed display it
        self.check_peek(generation);
        
    }
    pub fn cull(&mut self) {
        // Remove individuals that we can no longer let live.  Eugenics!
        let mut z:Vec<Tree> = Vec::new();
        for i in 0..self.trees.len(){
            let (id, ev) = (self.trees[i].0, self.trees[i].2);
            if ev.is_finite() {
                let t:Tree = (self.trees[i].0, self.trees[i].1.copy(), ev);
                z.push(t);
            }else{
                self.bnd_rec.write_line(
                    &format!("RIP {} culled", id)
                );
            }
            
        }
        self.trees = z;
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

    pub fn add_individual(&mut self) -> bool {
        // Add a individuall.  If the individual is already in the
        // population do not add it and return false
        let n = Box::new(Node::new(self.e, &self.d_all.names, 0));
        let st = n.to_string();
        self.str_rep.entry(st.clone()).or_insert(false);

        if !self.str_rep.get(&st).unwrap() {
            // This node is unique
            self.maxid += 1;
            self.str_rep.insert(st, true);
            let sc = score_individual(&n, &self.d_all, false);
            {
                self.bnd_rec.write_line(&format!("{}/{}: {}", self.maxid, sc, n.to_string()));
            }
            if sc == 0.0 {
                // Found the perfect individual.  
                println!("Found perfect Node! {}", n.to_string());
            }
            self.trees.push((self.maxid, n, sc));
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
            // Minimising score so we use inverse for selecting
            // crossover roulette selection
            if x.2.is_finite() {
                ret += 1.0/x.2;
            }
        }
        ret
    }
    pub fn do_crossover(&mut self) {
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
                    let ts = self.total_score();
                    let s = self.e.gen_rangef64(0.000001, ts);
                    let mut cum_score = 0.0;
                    for i in 0..self.len() {
                        let t:&Tree = self.get_tree(i);
                        // Inverse as scores are being minimised
                        if t.2.is_finite() {
                            cum_score += 1.0/t.2; 
                            if cum_score >= s {
                                p = Some(i);
                                break;
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

        let mut flag = false;  // Set to true if pc is unique

        let  pc; // Node resulting from crossover
        pc = self.crossover(i0, i1);

        let st = pc.to_string();
        self.str_rep.entry(st.clone()).or_insert(false);
        if !self.str_rep.get(&st).unwrap() {
            // This node is unique
            self.str_rep.insert(st, true);
            flag =  true;
        }else{
        }
        
        if flag {
            self.maxid += 1;  // Done here so it can be passed to record_birth
            let s = score_individual(&pc, &self.d_all, false);
            self.bnd_rec.write_line(&format!("{} + {} = {}/{}: {}", i0, i1, self.maxid, s, pc.to_string()));
        // }

        // if flag {
            let str_pc = pc.to_string();
            self.trees.push((self.maxid, pc, s));
            if s == 0.0 {
                // Found the perfect individual.  Quit
                println!("Found perfect Node! {}", str_pc);
            }
        }
    }
    pub fn mutate(&mut self) {
        for i in 0..self.trees.len() {
            if self.e.gen_range(0, 100) < self.mutate_prob {
                // Choosen this individual
                let old_individual = self.trees[i].1.copy();
                let olds = old_individual.to_string();
                self.str_rep.remove(&old_individual.to_string());
                let new_individual = self.mutate_tree(old_individual);
                self.trees[i].2 = score_individual(&new_individual, &self.d_all, false);
                self.trees[i].1 = new_individual;
                let news = self.trees[i].1.to_string();
                self.str_rep.insert(news.clone(), true);
                self.bnd_rec.write_line(format!("Mutate: {} {} --> {}",
                                                self.trees[i].0, olds, news).as_str());
            }
        }
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

