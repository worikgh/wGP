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
use node::Node;
use node::NodeBox;
use rng;
use score::Score;
use std::collections::BTreeMap;    
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Vacant;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
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
    /// Insert a Tree into the Forrest

    /// # Panics

    /// If the tree is already in the forrest this will panic.
    /// Duplicate trees are not allowed.
    fn insert(&mut self, tree:Tree) {
        // Check for duplicates
        let string_rep = tree.tree.to_string();
        let str_rep:&str = string_rep.as_str();
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
    /// Make a deep copy of the Forest.  FIXME Should this be `clone`?
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
        // let data = self.data.clone();

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
        let bnd_fname = self._bnd_file_name();
        let mut bnd_rec = Recorder::new(bnd_fname.as_str());
        let  generations_file = format!("{}/Data/{}/{}",
                                        self.config.get_string("root_dir").expect("Config: root_dir"),
                                        self.config.get_string("name").expect("Config: name"),
                                        self.config.get_string("generations_file").unwrap());
        let mut generation_recorder = Recorder::new(&generations_file[..]);
        let save_file = self._save_file_name();
        
        // Write the header for the generation file
        let s = "generation, best_id, Best Score, Individual".to_string();
        generation_recorder.write_line(&s[..]);
        generation_recorder.buffer.flush().unwrap();

        rng::reseed(seed.as_slice());

        let mut generation = 0;

        if self.config.get_string("reload").unwrap() == "true" {
            // Restore state from the last run
            self.restore_state().unwrap();
        }else{
            // Initialise a random population

            if self.forest.trees.len() == 0 {
                _initialise_rand(&mut self.forest,
                                 &self.data,
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
                                          &self.data,
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
    fn best_idx(&self) -> usize {
        0
    }    
    #[allow(dead_code)]
    pub fn best_id(&self) -> usize {
        panic!("Function that needs refactoring for new location of forests")
    }
    
    #[allow(dead_code)]
    pub fn best_score(&self) -> & Score {
        panic!("Function that needs refactoring for new location of forests")
    }

    pub fn report(&self) -> String {
        "".to_string()
    }

    fn _save_file_name(&self) -> String {
        format!("{}/Data/{}/{}",
                self.config.get_string("root_dir").expect("Config: root_dir"),
                self.config.get_string("name").expect("Config: name"),
                self.config.get_string("save_file").unwrap())
    }
    fn _bnd_file_name(&self) -> String {
        format!("{}/Data/{}/{}",
                self.config.get_string("root_dir").expect("Config: root_dir"),
                self.config.get_string("name").expect("Config: name"),
                self.config.get_string("birthsanddeaths_filename").unwrap())
    }

    /// Restore state from a save file
    fn restore_state(&mut self) -> std::io::Result<()>{

        let file_name = self._save_file_name();
        let file = File::open(file_name)?;
        let mut buf_reader = BufReader::new(file);

        self.forest.clear();

        // Need to record each individual that is recreated.
        let bnd_fname = self._bnd_file_name();
        let mut bnd = Recorder::new(bnd_fname.as_str());
        
        // Get first line, allocate names 
        let mut line:String = String::new();
        while buf_reader.read_line(&mut line)? != 0 {
            let  split = line.as_str().split(" ");
            let v:Vec<&str> = split.collect();
            let mut f = false; // Set this when we get to node
            let mut tree = String::new();
            for w in v {
                if f {
                    tree = tree + " " + w;
                }
                if w == "Node:" {
                    f = true;
                }
            }
            // tree is string representation of a tree
            let n = Box::new(Node::new_from_string(tree.as_str()));
            match  score_individual(&n, &self.data, true) {
                Ok(sc) => {
                    if sc.is_finite() {
                        let id = self.forest.maxid + 1;
                        {
                            bnd.write_line(&format!("Recreate {}/(Sc: {}) {}",
                                                    id, &sc.quality, n.to_string()));
                        }
                        self.forest.insert(Tree{id:id, score:sc, tree:n});
                        self.forest.maxid = id;
                    }
                },
                Err(e) => {
                    let s = format!("Recreate Failed {:?}  {}",
                                    e, n.to_string());
                    bnd.write_line(&s);
                },
            };
        }
        Ok(())
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

    fn _cull_sort(forest:&Forest, bnd_rec:& mut Recorder) -> Forest {
        // Remove individuals that we can no longer let live.  Eugenics!
        // Individuals with score NAN or 0

        // NOTE: The tree with id == maxid may be culled so it is not
        // guaranteed that a tree ith id == maxid exists

        let mut ret = Forest::new();
        ret.maxid = forest.maxid;
        for (_, v) in forest.trees.iter() {
            if v.score.is_finite() {
                ret.insert(v.clone());
            }else{
                bnd_rec.write_line(
                    &format!("RIP {} culled", v.id)
                );
            }
        }
        ret
    }

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
    
    //===============================================================
    //
    // Selection algorithms.
    //
    fn roulette_selection(forest:&Forest) -> usize {

        // https://en.wikipedia.org/wiki/Fitness_proportionate_selection
        // Return the id of a individual selected using roulette wheel
        // selection:
        // FIXME Return a reference to a tree not a id
        // FIXME What part should class play in selection?

        
        let total_score:f64 =
            forest.score_trees.iter().
            fold(0.0,
                 //  `a` is the accumulator and (b,v) is element from
                 //  score_trees. `b` is a `Score` and v a vector of
                 //  individuals. This fold returns total score over
                 //  all individuals
                 | a, (ref b, ref v)| {
                     b.quality() * v.len() as f64 + a
                 });
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
                    acc += s.quality();
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

            match  score_individual(&n, d_all, true) {
                Ok(sc) => {
                    bnd_rec.write_line(&format!("Create {}/(Sc: {}) {}", id, sc.quality, n.to_string())); 
                    forest.insert(Tree{id:id, score:sc, tree:n});
                    forest.maxid = id;
                    true
                },
                Err(_) => false,
            }
        }else{
            false
        }
    }

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

    fn _do_crossover(forest:&Forest)  -> (NodeBox, usize, usize){
        let i0 = Population::roulette_selection(forest);
        // FIXME Here is a possible place to take account of class.
        // Could apply some sort of "class prejudice" as a probability
        // that `i1` will not be accepted if it is of a different class
        let i1 = Population::roulette_selection(forest);
        (Population::_crossover(forest, i0, i1), i0, i1)
    }

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
            state += &format!("Score: {} Node: {}\n", t.score.quality,s);
        }

        let mut file = File::create(save_file).unwrap();
        file.lock_exclusive().expect("Failed to lock save file");
        file.write_all(state.as_bytes()).unwrap();
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
            match score_individual(&nb, d_all, true) {
                Ok(sc) => {
                    let id = new_forest.maxid+1;
                    new_forest.insert(Tree{id:id, score:sc.clone(), tree:nb});
                    new_forest.maxid = id;
                    bnd_rec.write_line(&format!("Cross {} + {} --> {}/(Sc:{}): {}",
                                                l, r, id, &sc.quality, st));
                },
                Err(e) => {
                    bnd_rec.write_line(&format!("Cross Failed {:?} Cross {} + {} ",
                                                e, l, r));
                    
                },
            };
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

                match  score_individual(&nb, d_all, true) {
                    Ok(sc) => {
                        new_forest.maxid += 1;
                        let id = new_forest.maxid;
                        new_forest.insert(Tree{id:id, score:sc.clone(), tree:nb});
                        bnd_rec.write_line(format!("Mutate {} --> {}: {}/(Sc: {})",
                                                   id0, new_forest.maxid, st, &sc.quality).as_str());
                    },
                    Err(e) => bnd_rec.write_line(format!("Failed Mutate {:?}", e).as_str()),
                };
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
                new_forest.insert(t.clone());
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
