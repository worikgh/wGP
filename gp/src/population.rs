
use std::collections::HashMap;    

use super::Recorder;
use super::entropy::Randomness;
use super::Tree;
use super::Node;
//use super::NodeBox;
use super::Data;
use super::Mutator;
use super::score_individual;
use super::simulate;
use super::add_simulation;
use super::crossover;
use std::cmp::Ordering;

pub struct Population<'a> {
    trees:Vec<Tree>,
    str_rep:HashMap<String, bool>,
    maxid:usize,
    mutator:&'a Mutator,
    bnd_rec:&'a mut Recorder,
    d_all:&'a mut Data,
    mutate_prob:usize,
    best_id:usize,
    best_individual:String,
    model_data_file:String,
}
impl<'a> Population<'a> {
    pub fn new(bnd_rec:&'a mut Recorder, mutator:&'a  Mutator, d_all:&'a mut Data, mutate_prob:usize, model_data_file:String) ->
        Population<'a> {
            Population{trees:Vec::new(), str_rep:HashMap::new(), maxid:0, bnd_rec:bnd_rec, d_all:d_all,
                       mutator:mutator, mutate_prob:mutate_prob, best_id:0, best_individual:"".to_string(),
                       model_data_file:model_data_file}
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
    pub fn new_generation(&mut self, generation:usize){

        // Call every generation

        self.cull();
        //self.bnd_rec.buffer.flush().unwrap(); FIXME  Why can this not work???

        // Sort population by score, ascending so the best are
        // earliest.  Allows the less good individuals to be easilly
        // pop'd off the end
        &self.trees[..].sort_by(|b, a| {
            let a2 = a.2;
            let b2 = b.2;
            b2.partial_cmp(&a2).unwrap_or(Ordering::Equal)
        });

        // If the best individual has changed display it
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
                let simulation = simulate(&n, self.d_all);
                add_simulation(simulation, lable,
                               self.model_data_file.as_str());
            }
        }
        
    }
    pub fn cull(&mut self) {
        // Remove individuals that we can no longer let live.  Eugenics!
        let mut z:Vec<Tree> = Vec::new();
        for i in 0..self.trees.len(){
            let (id, e) = (self.trees[i].0, self.trees[i].2);
            if e.is_finite() {
                let t:Tree = (self.trees[i].0, self.trees[i].1.copy(), e);
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

    pub fn add_individual(&mut self, e:&mut Randomness) -> bool {
        // Add a individuall.  If the individual is already in the
        // population do not add it and return false
        let n = Box::new(Node::new(e, &self.d_all.names, 0));
        let st = n.to_string();
        self.str_rep.entry(st.clone()).or_insert(false);

        if !self.str_rep.get(&st).unwrap() {
            // This node is unique
            self.maxid += 1;
            self.str_rep.insert(st, true);
            let sc = score_individual(&n, self.d_all, false);
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
    pub fn do_crossover(&mut self, e:&mut Randomness) {
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
                    let s = e.gen_rangef64(0.000001, ts);
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

        let mut s = 0.0; // Score
        let  pc; // Node resulting from crossover
        let p0_id:usize;
        let p1_id:usize;
        {
            // Block to limit scope of p0 and p1
            let ref p0 = &self.get_tree(i0);
            let ref p1 = &self.get_tree(i1);
            p0_id = p0.0;
            p1_id = p1.0;                
            pc = crossover(&p0.1, &p1.1, e);
        }
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
            s = score_individual(&pc, self.d_all, false);
            self.bnd_rec.write_line(&format!("{} + {} = {}/{}: {}", p0_id, p1_id, self.maxid, s, pc.to_string()));
        }

        if flag {
            let str_pc = pc.to_string();
            self.trees.push((self.maxid, pc, s));
            if s == 0.0 {
                // Found the perfect individual.  Quit
                println!("Found perfect Node! {}", str_pc);
            }
        }
    }
    pub fn mutate(&mut self, e:&mut Randomness) {
        for i in 0..self.trees.len() {
            if e.gen_range(0, 100) < self.mutate_prob {
                // Choosen this individual
                let old_individual = self.trees[i].1.copy();
                let olds = old_individual.to_string();
                self.str_rep.remove(&old_individual.to_string());
                let new_individual = self.mutator.mutate_tree(old_individual, e);
                self.trees[i].2 = score_individual(&new_individual, self.d_all, false);
                self.trees[i].1 = new_individual;
                let news = self.trees[i].1.to_string();
                self.str_rep.insert(news.clone(), true);
                self.bnd_rec.write_line(format!("Mutate: {} {} --> {}",
                                                self.trees[i].0, olds, news).as_str());
            }
        }
    }
}

