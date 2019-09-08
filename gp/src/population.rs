//! # Collection of Programmes.

//! All the code to generate, evolve and utilise a population of
//! programme trees.</br>

//! API: new, start, classify, save_state, resume_state

//! * new Constructor. Pass a [configuration](../config/index.html) object

//! * start  Sets up and starts a simulation creating a population of trees

//! * classify: Passed a example from the domain.  If it can be
//! classified return Ok((String, Vec<(String, f64))) where the first
//! string in the result is the class name that the example is
//! classified as and the vector is all the classes and the associated
//! probability (estimated by the population of trees) of the example
//! being in that class

//! * report: Display the best classifiers for each class.

//! * save_state restore_state Save or restore the population from
//! disc. Unimplemented.


use config::Config;
use fs2::FileExt;
use inputs::Inputs;
use node::Node;
use node::NodeBox;
use rng;
use score::Score;
use std::cmp::Ordering;
use std::collections::BTreeMap;    
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Vacant;
use std::fs::File;
use std::io::Write;
//use std::sync::{Arc, RwLock};
use std::thread;
use super::Data;
use super::Recorder;
use super::score_individual;

/// Define a individual.  Consists of a node, a id, and a score.
#[derive(Clone)]
struct Tree {
    // FIXME Should this be in node.rs?
    id:usize,
    score:Score,
    tree:NodeBox,
} 


#[derive(Clone)]
/// A collection of [Trees](struct.Tree.html)
pub struct Forest {

    /// Store trees in a Hash keyed by the string representation of
    /// the tree. Indexes the HashMap with the string representation
    /// so duplicates aerf easy to detect
    trees:HashMap<String, Tree>,
    
    /// Map score to trees so it is easy to find best and worst.
    /// Store the string representation and beware of trees with same
    /// score...  Hence the vector
    score_trees:BTreeMap<Score, Vec<String>>,

    /// Each tree in the forest has a unique id.  This maintains the
    /// maximum assigned so adding another tree is a matter of
    /// incrementing this value
    maxid:usize,

}

impl Forest {
    /// Initialise and return a empty Forest
    pub fn new() -> Forest {
        Forest{
            maxid:0,
            trees:HashMap::new(),
            score_trees:BTreeMap::new(),
        }
    }
    #[allow(dead_code)]
    /// Reset the Forest to empty
    pub fn clear(&mut self) {
        self.trees.clear();
        self.score_trees.clear();
        self.maxid = 0;
    }

    // Return number of trees in self.trees - number in score_trees.
    // If the forrest is consistent then this will return 0
    fn _check_sz(&self) -> i32 {
        self.trees.len() as i32 - self.score_trees.iter().fold(0, |mut sum, x| {sum += x.1.len(); sum})  as i32
    }

    /// Overwrite this Forrest with another one 
    // fn replace(&mut self, forest:&Forest) {
    //     self.trees = forest.trees.clone();
    //     self.score_trees = forest.score_trees.clone();
    //     self.maxid = forest.maxid;
    // }
    
    /// Insert a Tree into the Forrest

    /// # Panics

    /// If the tree is already in the forrest this will panic.
    /// Duplicate trees are not allowed.
    fn insert(&mut self, str_rep:&str, tree:Tree) {
        // Check for duplicates
        match self.trees.get(str_rep){
            Some(_) => {
                panic!("Inserting a duplicate tree");
            },
            None => {
                self.trees.insert(str_rep.to_string(), tree.clone());
                let v = self.score_trees.entry(tree.score).or_insert(Vec::new());
                v.push(str_rep.to_string());

                
                if tree.id > self.maxid {
                    self.maxid = tree.id;
                }
            },
        };
        assert!(self._check_sz() == 0);
    }

    // Check if a Tree is in this Forrest by string
    fn has_tree_str(&self, t:&str) -> bool {
        self.trees.contains_key(t)
    }

    // Check if a Tree is in this Forrest using the NodeBox
    fn has_tree_nb(&self, t:&NodeBox) -> bool {
        self.has_tree_str(t.to_string().as_str())
    }

    /// Delete a tree using the string rep.  Returns the ID of the
    /// tree deleted
    fn delete_str(&mut self, str_rep:&str) -> usize{
        // FIXME If passed a invalid string (either nonsense of a
        // string representation of a tree not in the forest) this
        // will panic
        let tree = self.trees.remove(&str_rep.to_string()).unwrap();
        let ret = tree.id;
        
        // Get the vector holding the string representation of the
        // tree and delete it
        let v:Vec<String>;
        {
            // Get the vector holding trees with this score
            let _v = self.score_trees.get(&tree.score).unwrap();

            // Remove the tree from the array 
            v = _v.iter().filter(|s| **s != str_rep).
                map(|x| x.to_string()).collect();
        }

        // Update the vector of trees in score_trees
        if v.len() != 0 {
            self.score_trees.insert(tree.score.clone(), v);
        }else{
            // If that is the last tree with this score
            self.score_trees.remove(&tree.score).unwrap();
        }
        
        ret
    }
    
    #[allow(dead_code)]
    /// Make a deep copy of the Forest.  FIXME Should thi be `clone`?
    /// Yes.
    fn copy(&self) -> Forest {
        Forest {
            maxid:self.maxid,

            // FIXME  Can I mix copy and clone like this?
            trees:self.trees.clone(),
            score_trees:self.score_trees.clone(),
            
        }
    }
    #[allow(dead_code)]
    /// How many trees are in the forest?
    fn count(&self) -> usize {
        self.trees.len()
    }
}

/// The collection of trees representing programmes and the genetic
/// algorithms to work on them
pub struct Population {

    // There is one Population
    
    pub handle:Option<thread::JoinHandle<()>>,
    forest:Forest,
    config:Config,
    data:Data,
}

impl Population {

    //==============================
    //
    // API Implementation
    //

    /// Initialise a population
    pub fn new(config:&Config) ->  Population {
        
        // Get the data.  FIXME Document some (other) place where the
        // data files reside and how they are found
        let data_path = format!("{}/Data/{}/{}",
                                config.get_string("root_dir").expect("Config: root_dir"),
                                config.get_string("name").expect("Config: name"),
                                config.get_string("data_file").expect("Config: data_file"));
        let data = Data::new(&data_path, config.get_usize("training_percent").expect("Config: training_percent"));
        
        Population {
            forest:Forest::new(),
            handle:None,
            data:data, 
            config:config.clone(),
        }
    }

    pub fn start(&mut self) -> Result<bool, String> {
        
        // Get all the variables the simulation will need
        //let forest_lock = self.forest.clone();
        let data = self.data.clone();

        // FIXME Make these probabilities f64s instead of
        // 'percentages' in [0..100]!!
        let mutate_prob = self.config.get_usize("mutate_prob").unwrap(); 
        let copy_prob = self.config.get_usize("copy_prob").unwrap();
        let crossover_percent = self.config.get_usize("crossover_percent").unwrap();

        let max_population = self.config.get_usize("max_population").unwrap();
        // The random number generator I use has its seed as a vector
        // of usize.  Why?  Just use one number...
        let seed:Vec<u32> = vec![self.config.get_u32("seed").unwrap()];
        let num_generations = self.config.get_usize("num_generations").unwrap();
        let bnd_fname = format!("{}/Data/{}/{}",
                                self.config.get_string("root_dir").expect("Config: root_dir"),
                                self.config.get_string("name").expect("Config: name"),
                                self.config.get_string("birthsanddeaths_filename").expect("Config: birthsanddeaths_filename"));
        let mut bnd_rec = Recorder::new(bnd_fname.as_str());
        let  generations_file = format!("{}/Data/{}/{}",
                                        self.config.get_string("root_dir").expect("Config: root_dir"),
                                        self.config.get_string("name").expect("Config: name"),
                                        self.config.get_string("generations_file").unwrap());
        let mut generation_recorder = Recorder::new(&generations_file[..]);
        let save_file = format!("{}/Data/{}/{}",
                                self.config.get_string("root_dir").expect("Config: root_dir"),
                                self.config.get_string("name").expect("Config: name"),
                                self.config.get_string("save_file").unwrap());

        // Write the header for the generation file
        let s = "generation, best_id, Best Score, Individual".to_string();
        generation_recorder.write_line(&s[..]);
        generation_recorder.buffer.flush().unwrap();

        // Start the thread

        rng::reseed(seed.as_slice());

        let mut generation = 0;

        // Initialise a random population
        {
            // Block for accessing forest with a mutex FIXME: Get
            // rid of the lock and do not check length of tree
            if self.forest.trees.len() == 0 {
                _initialise_rand(&mut self.forest,
                                 &data,
                                 &mut bnd_rec, max_population);
            }else{
                eprintln!("population Not calling _initialise_rand");
            }
        }

        loop {
            
            generation = generation + 1;

            // If we have done as many generations as we
            // plan to, quit
            if generation > num_generations {
                break;
            }
            // FIXME Are there other criterion for ending a
            // simulation?
            
            // Advance simulation by generating a new forest
            self.forest = _new_generation(&self.forest,
                                          mutate_prob, copy_prob,
                                          crossover_percent,
                                          max_population,
                                          &data,
                                          &mut bnd_rec,
                                          save_file.as_str());

            // Write a report of this old generation
            // Not saying much yet....

            let s = format!("{}",// {}, {}, {}",
                            generation,
                            // id,
                            // score,
                            // tree.tree.to_string()
            );
            generation_recorder.write_line(&s[..]);
            generation_recorder.buffer.flush().unwrap(); 
        }

        Ok(true)
    }
    #[allow(dead_code)]
    pub fn restore(&mut self){
        // FIXME This could have a file name argument so Population
        // does not need to know
        // FIXME Restore trees must return a forest
        // self.forest = self.restore_trees();
    }
    
    
    #[allow(dead_code)]
    fn best_idx(&self) -> usize {
        0
    }    
    #[allow(dead_code)]
    pub fn best_id(&self) -> usize {

        // // Get trees associated with lowest score
        // let (_, vt) = self.forest.score_trees.iter().next().unwrap();
        // // Get a tree from that vector
        // let st = vt.iter().next().unwrap();
        // // Get the tree labled and return its id
        // self.forest.trees.get(st).unwrap().id
        panic!("Function that needs refactoring for new location of forests")
    }
    
    #[allow(dead_code)]
    pub fn best_score(&self) -> & Score {
        // // Get trees associated with lowest score
        // let (_, vt) = self.forest.score_trees.iter().next().unwrap();
        // // Get a tree from that vector
        // let st = vt.iter().next().unwrap();
        // // Get the tree labled and return its id
        // &self.forest.trees.get(st).unwrap().score
        panic!("Function that needs refactoring for new location of forests")
    }
    // pub fn len(&self) -> usize {
    //     // let f = &self.controller.forests;
    //     // let f = f.get(&self.name).unwrap().read().unwrap();
    //     // (*f).trees.len()
        
    // }

    #[allow(dead_code)]
    pub fn classify(&self, case:&Vec<f64>) -> Option<(String, String)> {
        let input_names = &self.data.input_names;
        let class_names = &self.data.class_names;
        classify(case, input_names, class_names, &self.forest)
    }

    /// For each class report the best classifier that has the best
    /// score.  In case of ties report all classifiers that have the
    /// best score.
    pub fn report(&self) -> String {

        // Map class names to 2-tupple (Best score for that class,  string representations of
        // classifiers)
        let mut class_score:HashMap<String, (f64, Vec<String>)> = HashMap::new();

        for (score, trees) in &self.forest.score_trees {
            let class = &score.class;

            // Ensure class_score has a record for this class
            if !class_score.contains_key(class.as_str()) {
                class_score.insert(class.clone(), (0.0, Vec::new()));
            }

            let q = score.evaluate();
            if class_score.get(class).unwrap().0 < q {
                // For this class there are better trees
                class_score.insert(class.clone(), (q, trees.clone()));
            }else if class_score.get(class).unwrap().0 <= q {
                // These are trees that are equal in quality to those
                // in class_score.  Do not replace the trees, add to
                // them
                for v in trees {
                    class_score.get_mut(class.as_str()).unwrap().1.push(v.clone());
                }
            }
        }
        
        let mut ret = String::new();
        for (c, (q, vt)) in class_score {
            for v in vt {
                ret =ret +  format!("{} {} {}\n",
                                    c, q, v.to_string()).as_str();
                
            }
        }
        ret
    }


    fn _check(forest:&Forest) -> bool {
        let mut ret = true;
        for (_, v) in forest.trees.iter() {
            if !v.score.quality.is_finite() {
                ret = false;
                break;
            }
        }
        ret
    }
    // fn check(&self) -> bool {
    //     true //Population::_check(&self.forests)
    // }
    
    

    // fn _unique_node(&self) -> NodeBox {
    //     // Generate a node and check if it is unique
    
    //     let n:NodeBox;
    //     loop {

    //         // Make a node
    //         let _n = Box::new(Node::new(&self.d_all.input_names, 0));

    //         // Check for uniqueness
    //         let st = _n.to_string();
    //         if !self.str_rep.contains_key(st.as_str()) {
    //             // Is unique
    //             n = _n;
    //             break;
    //         }
    //     }
    //     n
    // }        
    
    // pub fn new_individual(&self) -> Tree {
    //     // Create a random tree and return it
    //     let n = self._unique_node();
    //     // Find the class of the new node
    // }

    fn _cull_sort(forest:&Forest, bnd_rec:& mut Recorder) -> Forest {
        // Remove individuals that we can no longer let live.  Eugenics!
        // Individuals with score NAN or 0

        // NOTE: The tree with id == maxid may be culled so it is not
        // guaranteed that a tree ith id == maxid exists

        let mut ret = Forest::new();
        ret.maxid = forest.maxid;
        for (k, v) in forest.trees.iter() {
            if v.score.is_finite() {
                ret.insert(k, v.clone());
            }else{
                bnd_rec.write_line(
                    &format!("RIP {} culled", v.id)
                );
            }
        }
        
        // // Sort population by score then length, descending so the best are
        // // earliest.  Allows the worst individuals to be easilly
        // // pop'd off the end
        // ret.trees.sort_by(|a, b|{
        //     let a1 = &a.score;
        //     let b1 = &b.score;
        //     match b1.partial_cmp(a1) {
        //         Some(x) => match x {
        //             Ordering::Equal => a.tree.count_nodes().cmp(&b.tree.count_nodes()),
        //             Ordering::Greater => Ordering::Greater,
        //             Ordering::Less => Ordering::Less,
        //         },
        //         None => panic!("Cannot compare {:?} and {:?}", a1, b1)
        //     }
        // });
        ret
    }
    // pub fn cull_sort(&mut self, bnd_rec:&mut Recorder) {
    //     Population::_cull_sort(&*self.controller.forests.get(&self.name).unwrap().read().unwrap(), bnd_rec);
    // }

    fn _get_tree_id<'b>(forest:&'b Forest, id:usize) -> &'b Tree {

        // Get a tree based on its id
        // FIXME Use iterator on Forest or ScoreTreeMap
        for (_, t) in forest.trees.iter() {
            if t.id == id {
                return &t;
            }
        }
        panic!("Cannot get node with id: {}", id);
    }
    // fn get_tree_id(&self, id:usize) -> &Tree {    
    //     //        //*(self.controller.forests.get(&self.name).unwrap().read().unwrap()).trees.len()
    //     Population::_get_tree_id(&*self.controller.forests.get(&self.name).unwrap().read().unwrap(), id)
    // }
    

    // #[allow(dead_code)]
    // pub fn get_tree(&self, id:usize) -> &Tree {
    //     // Get a tree based on its order in self.forest.trees. Used to
    //     // inumerate all trees.  FIXME Use a iterator
    //     &self.trees[id]
    // }

    // #[allow(dead_code)]
    // fn get_trees_of_class(& self, class:&String) -> Vec<& Tree> {
    //     // FIXME: Ho do e do string comparison better in rust?  Is
    //     // there a problem ith this?
    //     let test = class.clone();
    //     self.controller.forests.get(&self.name).unwrap().
    //         read().unwrap().trees.iter().
    //         filter(|(_,t)| t.score.class == test).map(|(_, x)| x).collect()
    // }
    
    #[allow(dead_code)]
    // fn get_classes(&self) -> &Vec<String>{
    //     // Return all known class lables
    //     &self.d_all.class_names
    // }

    
    //===============================================================
    //
    // Selection algorithms.
    //

    // fn select(&self) -> usize {
    //     // FIXME Implement choice of selection algorithm
    //     Population::roulette_selection(&*self.controller.forests.get(&self.name).unwrap().read().unwrap())
    // }

    fn roulette_selection(forest:&Forest) -> usize {

        // https://en.wikipedia.org/wiki/Fitness_proportionate_selection
        // Return the id of a individual selected using roulette wheel
        // selection:
        // FIXME Return a reference to a tree not a id
        // FIXME What part should class play in selection?

        
        let total_score:f64 =
            forest.score_trees.iter().
            fold(0.0,
                 //  `a` is the accululator and (b,v) is element from
                 //  score_trees. `b` is a `Score` and v a vector of
                 //  individuals. This fold returns total score over
                 //  all individuals
                 | a, (ref b, ref v)| b.evaluate() * v.len() as f64 + a);

        if total_score == 0.0 {
            // Return a random key
            let key_count = forest.trees.keys().count();
            forest.trees.get(forest.trees.keys().nth(rng::gen_range(0, key_count - 1)).unwrap()).unwrap().id
        }else{

            let sel = rng::gen_range(0.0, total_score); 
            let mut  acc = 0.0;
            let mut ret:Option<usize> = None;  // Index of selected individual
            'lable:
            for (s, v) in forest.score_trees.iter() {
                for t in v {
                    acc += s.evaluate();
                    if acc > sel {
                        // Have the tree's string rep in t.  Get the
                        // actual tree's id.  
                        ret = Some(forest.trees.get(t).unwrap().id);
                        break 'lable;
                    }
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

    fn _add_individual(d_all:&Data,
                       bnd_rec:&mut Recorder,
                       forest:&mut Forest) -> bool {
        
        // Add a random individuall.  If the individual is already in
        // the population do not add it and return false
        let n = Box::new(Node::new(&d_all.input_names, 0));

        let st = n.to_string();
        if !forest.trees.contains_key(&st.clone()) {

            // This node is unique
            let id = forest.maxid + 1;

            let sc = score_individual(&n, d_all, true);
            {
                bnd_rec.write_line(&format!("Create {}/(Sc: {}) {}", id, sc.quality, n.to_string()));
            }
            forest.insert(&st, Tree{id:id, score:sc, tree:n});
            forest.maxid = id;
            true
        }else{
            false
        }
    }
    // fn add_individual(&mut self, bnd_rec:&mut Recorder) -> bool {
    //     Population::_add_individual(&self.d_all, bnd_rec, &mut*self.controller.forests.get(&self.name).unwrap().write().unwrap())
    // }
    fn _delete_worst(forest:&mut Forest, bnd_rec:&mut Recorder) {

        // Delete a tree from the forest that has the worst score
        let t:String; // String rep of tree to delete
        {
            // Get the worst score
            let s = forest.score_trees.iter().last().unwrap().0;

            // Get the trees that have that score
            let v = forest.score_trees.get(&s).unwrap();

            // Choose one
            t = v.iter().next().unwrap().clone();
        }
        // t is the string representation of the tree

        // Get the id so e can write a record
        let id = forest.trees.get(&t).unwrap().id;
        bnd_rec.write_line(&format!("RIP {} culled", id));
        
        // Delete it
        forest.delete_str(t.as_str());
        assert!(forest._check_sz() == 0);
                                
        
    }
    // pub fn delete_worst(&mut self, bnd_rec:&mut Recorder) {
    //     Population::_delete_worst(&mut*self.controller.forests.get(&self.name).unwrap().write().unwrap(), bnd_rec);
    // }

    fn _do_crossover(forest:&Forest)  -> (NodeBox, usize, usize){
        let i0 = Population::roulette_selection(forest);
        // FIXME Here is a possible place to take account of class.
        // Could apply some sort of "class prejudice" as a probability
        // that i1 will not be accepted if it is of a different class
        let i1 = Population::roulette_selection(forest);
        (Population::_crossover(forest, i0, i1), i0, i1)
    }
    // pub fn do_crossover(&mut self) -> (NodeBox, usize, usize){

    //     // Crossover to breed individuals better at generalisation

    //     // Choose a node from population to participate in crossover.
    //     // The higher the score the node got last generation the
    //     // higher the probability it will be selected to be
    //     // participate in crossover

    //     Population::_do_crossover(&*self.controller.forests.get(&self.name).unwrap().read().unwrap())
    // }

    fn _mutate_tree(i:NodeBox, d_all:&Data) -> NodeBox {
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
            Population::_mutate_tree(i.d.unwrap(), d_all)
        }else if selector < dnc + lnc {
            Population::_mutate_tree(i.l.unwrap(), d_all)
        }else if selector < dnc + lnc + rnc {
            Population::_mutate_tree(i.r.unwrap(), d_all)
        }else{
            // Mutate i
            // Two cases: This is a terminal, this is not terminal
            if nc == 1 {
                // i is a terminal.  FIXME  Mutate this!
                i.copy()
            }else{
                // i is not terminal
                let mut ret = i.copy();
                let child = Node::new(&d_all.input_names, 0);
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
    // fn mutate_tree(&mut self,i:NodeBox) -> NodeBox {
    //     Population::_mutate_tree(i, &self.d_all)
    // }
    
    fn _crossover(forest:&Forest, lidx:usize, ridx:usize) -> NodeBox {
        // FIXME Use references to nodes (and lifetimes?) insted of
        // indexes.  Save on lookups

        // Given the indexes of the a left and a right tree combine
        // the two trees to make a third individual
        let p:NodeBox;
        let c:NodeBox;
        if rng::random::<f64>() > 0.0 {
            p = Population::_get_tree_id(&forest, lidx).tree.random_node();
            c = Population::_get_tree_id(&forest, ridx).tree.random_node()
        }else{
            c = Population::_get_tree_id(&forest, lidx).tree.random_node();
            p = Population::_get_tree_id(&forest, ridx).tree.random_node()
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


    fn _save_trees(forest:&Forest, save_file:&str){
        
        let mut state = String::new();
        for (s, t) in forest.trees.iter() {
            state += &format!("Class: {} Score: {} Node: {}\n", t.score.class, t.score.quality,s);
        }

        // FIXME This is not in correct directory
        let mut file = File::create(save_file).unwrap();
        file.lock_exclusive().expect("Failed to lock save file");
        file.write_all(state.as_bytes()).unwrap();
    }
}

#[allow(dead_code)]
pub fn classify(case:&Vec<f64>, input_names:&Vec<String>,
                class_names:&Vec<String>, forest:&Forest) ->
    Option<(String, String)> {
        // Classify a case using the population.  @param `case` is the
        // case to classify. @param `input_names` is names of the
        // independant variables.  @param `class_names` is names of
        // classes

        // The first String in the pair returned is the class and the
        // second part lists the classes in desending order of
        // estimated liklihood along with the calculated liklihood
        // Create the input structure
        let mut input = Inputs::new();
        for j in 0..input_names.len() {
            let v:f64 = case[j];
            input.insert(&input_names[j], v);
        }

        // Store the results of each classifier.  The class of the
        // classifier is used as the key and keep each result and the
        // score/quality. 
        let mut results:HashMap<&String, Vec<(f64,f64)>> = HashMap::new();
        for c in class_names.iter() {
            results.insert(c, Vec::new());
        }

        // Ask each classifier what it thinks of the case.  Each one
        // is specialised to detect a particular class.  If a
        // classifier thinks the case is of the class it is
        // specialised for it returns 1.0.  Else -1.0.  The values are
        // stored in the results hash.  If the classifier cannot make
        // a decision it will not return a finite score
        for (_, t) in forest.trees.iter() {
            // Using each classifier

            // Given a input of class C and a tree (t) whose class is
            // D if C == D then score should be 1.0.  Else -1.0.  
            let score = t.tree.evaluate(&input).unwrap();
            if score.is_finite() {
                // Score::special is from training and is how well this
                // rule performed over all training cases.
                let quality = t.score.quality;
                results.get_mut(&t.score.class).unwrap().push((quality, score));
            }
        }

        // Interpretation of results.  The class with the highest
        // score, weighted by the quality (score.quality in
        // results{<class>}[<index>].0) and divided by the count of
        // classifiers, is the class to choose.  The magnitude of the
        // score relative to how many classifiers contributed is a
        // measure of quality of classification.  As is the score for
        // other classes.  FIXME for clarity!

        let mut scores:Vec<(&str, f64)> = Vec::new();

        // Set this if at leaset one class had some finite results
        let mut flag = false; 

        for k in class_names.iter() {

            let count = results.get(k).unwrap().len();
            let score = match count {
                0 => 0.0, // No finite results
                _ => {
                    flag = true; 
                    results.get(k).unwrap().
                        iter().fold(0.0, |mut sum, &x| {
                            // x.0 is the quality from training for
                            // this rule.  x.2 is ideally -1.0 if the
                            // case is not of the rules class and 1.0
                            // if it is of the rules class.
                            sum += x.0*x.1; sum
                        })  / (count  as f64)
                },
            };
            // For each class...
            scores.push((k.as_str(), score));
        }


        // Check case of all scores being 0.0
        if !flag {
            // This case cannot be classified
            None
        }else{
            scores.sort_by(|a,b| {
                let a1 = &a.1;
                let b1 = &b.1;
                b1.partial_cmp(a1).unwrap_or(Ordering::Equal)
            });

            // The return value.  ret.0 is the predicted class ret.1
            // stores information for all classes in a string.
            // "<class> <score>..." in descending score order
            let mut ret = (scores.first().unwrap().0.to_string(), String::new());
            for s in scores {
                ret.1 += format!("{} {} ", s.0, // Class
                                 s.1.to_string() // Score
                ).as_str();
            }
            Some(ret)
        }
    }


pub fn _initialise_rand(forest:&mut Forest, d_all:&Data, bnd_rec:&mut Recorder, max_population:usize){
    // Initialise with a random tree
    loop {

        // Random individual.  'add_individual' returns true when a
        // unique individual is created.  FIXME FIXTHAT!
        // _add_individual should be much more deterministic, pseudo
        // random
        while !Population::_add_individual(d_all, bnd_rec, forest) {} 

        if forest.trees.len() == max_population {
            break;
        }
    }
}        
fn _new_generation(forest:&Forest,
                   mutate_prob:usize,
                   copy_prob:usize,
                   crossover_percent:usize, 
                   max_population:usize,
                   d_all:&Data,
                   bnd_rec:&mut Recorder,
                   save_file:&str) -> Forest // New trees
{


    let mut new_forest = Forest::new();

    // The unique id given to each tree
    new_forest.maxid = forest.maxid + 1;

    // Generate some of new population from the old population. The
    // number of crossovers to do is (naturally) population.len()
    // * crossover_percent/100
    let ncross = (forest.trees.len() * crossover_percent)/100;
    let mut nc = 0;

    while nc < ncross  {

        let (nb, l, r) = Population::_do_crossover(&forest);

        let st = (*nb).to_string();
        if !new_forest.has_tree_nb(&nb) {

            // A unique child in next generation
            let sc = score_individual(&nb, d_all, true);
            let id = new_forest.maxid+1;
            new_forest.insert(&st, Tree{id:id, score:sc.clone(), tree:nb});
            new_forest.maxid = id;
            bnd_rec.write_line(&format!("Cross {} + {} --> {}/(Sc:{}): {}",
                                        l, r, id, &sc.quality, st));
        }
        nc += 1;
    }
    
    // Do mutation.  Take mut_probab % of trees, mutate them, add
    // them to the new population
    for (_, t) in forest.trees.iter() {
        if rng::gen_range(0, 100) < mutate_prob {

            // The id of the tree being mutated
            let id0 = t.id;

            // Copy the tree and mutate it.  Loose interest in
            // original tree now
            let t = t.tree.copy();

            let nb = Population::_mutate_tree(t, d_all);

            // Convert to a string to check for duplicates and for
            // the record 
            let st = (*nb).to_string();
            if let Vacant(_) = new_forest.trees.entry(st.clone()) {

                // Unique in the new population

                let sc = score_individual(&nb, d_all, true);
                new_forest.maxid += 1;
                let id = new_forest.maxid;
                new_forest.insert(&st, Tree{id:id, score:sc.clone(), tree:nb});
                bnd_rec.write_line(format!("Mutate {} --> {}: {}/(Sc: {})",
                                           id0, new_forest.maxid, st, &sc.quality).as_str());
            }                
        }
    }

    // Copy the best trees.
    let mut cp = 0; // Number copied
    let ncp = (forest.trees.len()*100)/copy_prob; // Number to copy
    for (_, vt) in forest.score_trees.iter() {
        //let it = forest.iter();
        // FIXME This could be probabilistic with roulette wheel
        // selection.
        for st in vt.iter() {

            if let Vacant(_) = new_forest.trees.entry(st.clone()) {

                // Unique in the new population
                let t = forest.trees.get(st).unwrap();
                new_forest.insert(st, t.clone());
                cp += 1;
                if cp == ncp  {
                    break;
                }
            }
            // FIXME The previous break should use a label or some thing
            if cp == ncp  {
                break;
            }
        }
    }    

    // New population is created in new_forest;
    
    // Eliminate all trees with no valid score and sort them 
    new_forest = Population::_cull_sort(&new_forest, bnd_rec);
    // Adjust population
    // let mut n1 = 0; // Number of individuals deleted
    // let mut n2 = 0; // Number of individuals added
    while new_forest.trees.len() > max_population {
        Population::_delete_worst(&mut new_forest, bnd_rec);
    }
    let flag =  new_forest.trees.len() < max_population; // Set if new individuals  to be added
    while new_forest.trees.len() < max_population {
        while Population::_add_individual(d_all, bnd_rec, &mut new_forest){}
    }
    if flag {
        // Sort again as we added new individuals. FIXME cull_sort
        // must be independant of Population for thread safety
        new_forest = Population::_cull_sort(&new_forest, bnd_rec ); 
    }

    bnd_rec.buffer.flush().unwrap(); 

    // FIXME check must be independent of Population for thread
    // safety

    if !Population::_check(&new_forest) {
        panic!("Check failed");
    }

    // FIXME save_trees must be independant of Population for
    // thread safety
    Population::_save_trees(&new_forest, save_file);
    assert!(new_forest._check_sz() == 0);

    new_forest 
}
