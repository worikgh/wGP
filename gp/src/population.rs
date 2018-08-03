use config::Config;
use node::Node;
use node::NodeBox;
use rng;

use controller::SimulationCommand;
use std::collections::hash_map::Entry::Vacant;
use controller::SimulationStatus;
use std::thread;
use std::sync::{Mutex, Arc};
use fs2::FileExt;
use inputs::Inputs;
use score::Score;
use std::cmp::Ordering;
use std::collections::BTreeMap;    
use std::collections::HashMap;    
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use super::Data;
use super::Recorder;
use super::score_individual;

// Define a individual.  Consists of a node, a id, and a score.  Called
// a Tree because it is not a sub-tree...
#[derive(Clone)]
struct Tree {
    id:usize,
    score:Score,
    tree:NodeBox,
} 


type ScoreTreeMap = BTreeMap<Score, Vec<String>>;

#[derive(Clone)]
struct Forest {
    // Store trees in a Hash keyed by the string representation of the
    // tree
    trees:HashMap<String, Tree>,

    
    // Map score to trees so it is easy to find best and worst.  Store
    // the string representation and beware of trees with same
    // score...  Hence the vector
    score_trees:ScoreTreeMap,

    // Each tree in the forest has a unique id.
    maxid:usize,

    // // State for the iterator
    // current:Option<<BTreeMap<Score, Vec<String>> as Iterator>::Item>,
}

// impl Iterator for Forest {
//     // Iterate over all trees in score order
//     type Item = Tree;
//     fn next(&mut self) -> Option<Self::Item> {
//         match self.current {
//             None => {
//                 // First call
//                 self.current = self.score_trees.iter().first();
//                 match self.current {
//                     None => // No trees...
//                         None,
//                     Some(_, t) => {
//                         self.currrent = self.current.next();
//                         t
//                     },
//                 }
//             },
//             Some(_, t) => {
//                 self.currrent = self.current.next();
//                 t
//             },
//         }
//     }
// }

impl Forest {
    fn new() -> Forest {
        Forest{
            maxid:0,
            trees:HashMap::new(),
            score_trees:BTreeMap::new(),
        }
    }
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.trees.clear();
        self.score_trees.clear();
        self.maxid = 0;
    }
    fn insert(&mut self, str_rep:&str, tree:Tree) {
        self.trees.insert(str_rep.to_string(), tree.clone());
        let v = self.score_trees.entry(tree.score).or_insert(Vec::new());
        v.push(str_rep.to_string());
        if tree.id > self.maxid {
            self.maxid = tree.id;
        }
    }
    fn has_tree_str(&self, t:&str) -> bool {
        self.trees.contains_key(t)
    }
    fn has_tree_nb(&self, t:&NodeBox) -> bool {
        self.has_tree_str(t.to_string().as_str())
    }

    fn delete_str(&mut self, str_rep:&str) -> usize{

        // Delete a tree using the string rep
        
        let tree = self.trees.remove(&str_rep.to_string()).unwrap();
        let ret = tree.id;
        
        // Get the vector holding the string representation of the
        // tree and delete it
        let v:Vec<String>;
        {
            let _v = self.score_trees.get(&tree.score).unwrap();
            v = _v.iter().filter(|s| **s == str_rep).
                map(|x| x.to_string()).collect();
        }
        if v.len() != 0 {
            self.score_trees.insert(tree.score.clone(), v);
        }else{
            // If that is the last tree with this score
            self.score_trees.remove(&tree.score).unwrap();
        }
        ret
    }
    #[allow(dead_code)]
    fn delete(&mut self, tree:&Tree, str_rep:Option<&str>) -> usize{
        // Delete a tree.  For callers that have a string
        // representation of the tree already pass it in in Some(..),
        // others pass None in second argument

        // Return id of tree deleted
        let str_rep = match str_rep {
            Some(s) => s.to_string(),
            None => tree.tree.to_string(),
        };
        let ret = self.trees.remove(&str_rep).unwrap().id;

        // Get the vector holding the string representation of the
        // tree and delete it
        let v:Vec<String>;
        {
            let _v = self.score_trees.get(&tree.score).unwrap();
            v = _v.iter().filter(|s| **s == str_rep).map(|x| x.to_string()).collect();
        }
        if v.len() != 0 {
            self.score_trees.insert(tree.score.clone(), v);
        }else{
            // If that is the last tree with this score
            self.score_trees.remove(&tree.score).unwrap();
        }
        ret
    }
    
    #[allow(dead_code)]
    fn copy(&self) -> Forest {
        Forest {
            maxid:self.maxid,

            // FIXME  Can I mix copy and clone like this?
            trees:self.trees.clone(),
            score_trees:self.score_trees.clone(),
            
        }
    }
    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.trees.len()
    }
}


pub struct Population {

    // The programme representations
    forest:Forest,

    d_all:Data,

    pop_config:PopulationConfig,
}

// The configuration data for a Population.
#[derive(Clone)]
struct PopulationConfig {

    // For random number generator to make simulations repeatable
    seed:Vec<u32>,
    num_generations:usize,

    // File name here log of each generation goes
    generations_file:String,

    crossover_percent:usize,
    mutate_prob:usize,
    copy_prob:usize,

    save_file:String,
    birthsanddeaths_file:String,
    max_population:usize,

    // If non 0 pick best 'filter' rules in restore trees
    filter:usize, 

    // If set then then reclassify trees with score_individual in
    // restore_trees
    rescore:bool, 
    
}

impl PopulationConfig {
    fn new(config:&Config) -> PopulationConfig {

        PopulationConfig {
            seed:config.get_string("seed").unwrap().split_whitespace().map(|x| x.parse::<u32>().unwrap()).collect(),
            num_generations:config.get_usize("num_generations").unwrap(),
            crossover_percent:config.get_usize("crossover_percent").unwrap(),
            mutate_prob:config.get_usize("mutate_prob").unwrap(),
            copy_prob:config.get_usize("copy_prob").unwrap(),
            max_population:config.get_usize("max_population").unwrap(),
            filter:config.get_usize("filter").unwrap(),

            birthsanddeaths_file:config.get_string("birthsanddeaths_filename").unwrap(),
            generations_file:config.get_string("generations_file").unwrap(),
            save_file:config.get_string("save_file").unwrap(),

            rescore:match config.get_usize("filter") {
                Some(r) => match r {
                    0 => false,
                    1 => true,
                    _ => panic!("Invalid value for {}: {}", "filter", r)
                },
                None => false, // default
            },
        }
    }
}

pub struct PopulationAnalysis<'a> {
    // Stores a analysis of the population

    // Over all count of miss-classifications
    incorrect:usize,    

    // Over all count of correct classifications
    correct:usize,    

    // Number classified (Total 
    classified:usize,

    // Total population
    population:usize,
    
    // The names of classes are owned by the Data object owned by the
    // Population object

    // False positives by class
    false_positives:HashMap<&'a str, usize>,

    // False negatives by class
    false_negatives:HashMap<&'a str, usize>,

    // Count examples of a class to normalise the other parameters
    counts:HashMap<&'a str, usize>,

}    

impl Population {

    pub fn new(config:&Config) -> Population {
        // Load the data
        
        let pop_config = PopulationConfig::new(&config);

        let forest = Forest::new();

        let data_file = config.get_string("data_file").unwrap() ;
        let training_percent = config.get_usize("training_percent").unwrap();
        let d_all = Data::new(&data_file, training_percent);


        // Write the header for the generaion file
        let s = format!("generation, best_id, Best Score General, Best Score Special, Population, Best");
        let  generations_file = pop_config.generations_file.clone();
        let mut generation_recorder = Recorder::new(&generations_file[..]);
        generation_recorder.write_line(&s[..]);
        generation_recorder.buffer.flush().unwrap();

        Population{
            forest,

            d_all,
            pop_config,
        }
        
    }

    // pub fn config(&mut self, config:Config) {
    //     // First get the mode of the run
    //     self.copy_prob = config.get_usize("copy_prob").unwrap();
    //     self.crossover_percent = config.get_usize("crossover_percent").unwrap();
    //     self.max_population = config.get_usize("max_population").unwrap();
    //     self.mutate_prob = config.get_usize("mutate_prob").unwrap();

    //     self.filter = config.get_usize("filter").unwrap_or(0);
    //     self.rescore =  match config.get_string("rescore") {
    //         Some(x) =>
    //         // A string.  Must be valid usize.  If it is 0 then false 
    //             if x.parse::<usize>().unwrap() == 1 {
    //                 true
    //             }else if x.parse::<usize>().unwrap() == 0 {
    //                 false
    //             }else{
    //                 panic!("Invalid rescore: {}", x)
    //             },
    //         None => false // Default
    //     }
    // }

    pub fn initialise_rand(&mut self){
        // Initialise with a random tree
        loop {

            // Random individual.  'add_individual' returns true when
            // a unique individual is created.
            let mut bnd_rec = Recorder::new(self.pop_config.birthsanddeaths_file.as_str());
            while !self.add_individual(&mut bnd_rec) {} 

            if self.len() == self.pop_config.max_population {
                break;
            }
        }
    }        

    #[allow(dead_code)]
    pub fn restore(&mut self){
        // FIXME This could have a file name argument so Population
        // does not need to know
        // FIXME Restore trees must return a forest
        self.forest = self.restore_trees();
    }
    
    pub fn run_in_thread(&mut self, a:Arc<Mutex<SimulationStatus>>) -> thread::JoinHandle<()>{

        // Initialise a random population
        self.initialise_rand();
        
        // FIXME There should be a idiomatic Rust way to do this
        let mut forest = self.forest.clone(); 
        
        let mutate_prob = self.pop_config.mutate_prob;
        let copy_prob = self.pop_config.copy_prob;
        let crossover_percent = self.pop_config.crossover_percent; 
        let max_population = self.pop_config.max_population;
        let d_all = self.d_all.clone();
        let mut bnd_rec = Recorder::new(self.pop_config.birthsanddeaths_file.as_str());
        let save_file = self.pop_config.save_file.clone();


        let seed:Vec<u32> = self.pop_config.seed.clone();
        let num_generations = self.pop_config.num_generations;
        // Write the header for the generaion file
        let s = format!("generation, best_id, Best Score General, Best Score Special, Population, Best");
        let  generations_file = self.pop_config.generations_file.clone();
        let mut generation_recorder = Recorder::new(&generations_file[..]);
        generation_recorder.write_line(&s[..]);
        generation_recorder.buffer.flush().unwrap();
        
        thread::spawn( move ||  {

            // The source of entropy.  
            rng::reseed(seed.as_slice());


            {
                // Update status.  We are running
                let mut ps = a.lock().unwrap();
                (*ps).running = true;
            }
            let mut command:SimulationCommand;
            let mut generation = 0;
            loop {

                // Main loop
                {
                    // Update status.  We are running
                    let mut ps = a.lock().unwrap();
                    (*ps).generation = generation;
                    command = (*ps).command.clone();
                    (*ps).command = SimulationCommand::Empty;
                }
                
                match  command {
                    SimulationCommand::Analyse => {
                        command = SimulationCommand::Empty;
                    },
                    SimulationCommand::Empty => {
                        forest = 
                            Population::_new_generation(&forest, mutate_prob, copy_prob,
                                                        crossover_percent,
                                                        max_population, &d_all,
                                                        &mut bnd_rec,
                                                        save_file.as_str());
                        generation = generation + 1;
                    },
                };
                // Update the status structure and check if caller has
                // decided to shut this thread down
                let mut ps = a.lock().unwrap();
                (*ps).generation = generation;
                
                if ps.cleared == false ||
                    generation == num_generations  {
                        
                        // Caller wants us to stop
                        ps.running = false;
                        break;
                    }
            }
            
            {
                // Update status.  We are not running
                let mut ps = a.lock().unwrap();
                (*ps).running = false;
            }

            
        })
    }



    
    #[allow(dead_code)]
    fn best_idx(&self) -> usize {
        0
    }    
    pub fn best_id(&self) -> usize {

        // Get trees associated with lowest score
        let (_, vt) = self.forest.score_trees.iter().next().unwrap();
        // Get a tree from that vector
        let st = vt.iter().next().unwrap();
        // Get the tree labled and return its id
        self.forest.trees.get(st).unwrap().id
            
    }
    
    pub fn best_score(&self) -> & Score {
        // Get trees associated with lowest score
        let (_, vt) = self.forest.score_trees.iter().next().unwrap();
        // Get a tree from that vector
        let st = vt.iter().next().unwrap();
        // Get the tree labled and return its id
        &self.forest.trees.get(st).unwrap().score
    }
    pub fn len(&self) -> usize {
        self.forest.trees.len()
    }

    fn classify(&self, case:&Vec<f64>) -> Option<(&str, String)> {
        // Classify a case using the population.  @param `case` is the
        // case to classify. The first &str in the pair returned is
        // the class from self.d_all and the second part lists the
        // classes in desending order of estimated liklihood along
        // with the calculated liklihood

        // Create the input structure
        let mut input = Inputs::new();
        for j in 0..self.d_all.input_names.len() {
            let v:f64 = case[j];
            input.insert(&self.d_all.input_names[j], v);
        }

        // Store the results of each classifier.  The class of the
        // classifier is used as the key and keep each result and the
        // score/quality.  FIXME Make key &str
        let mut results:HashMap<&String, Vec<(f64,f64)>> = HashMap::new();
        for c in self.d_all.class_names.iter() {
            results.insert(c, Vec::new());
        }

        // Ask each classifier what it thinks of the case.  Each one
        // is specialised to detect a particular class.  If a
        // classifier thinks the case is of the class it is
        // specialised for it returns 1.0.  Else -1.0.  The values are
        // stored in the results hash.  If the classifier cannot make
        // a decision it will not return a finite score
        for (_, t) in self.forest.trees.iter() {
            // Using each classifier
            let class = &t.score.class;

            // Given a input of class C and a tree (t) whose class is
            // D if C == D then score should be 1.0.  Else -1.0.  
            let score = t.tree.evaluate(&input).unwrap();
            if score.is_finite() {
                // Score::special is from training and is how well this
                // rule performed over all training cases.
                let quality = t.score.special;
                results.get_mut(class).unwrap().push((quality, score));
            }
        }

        // Interpretation of results.  The class with the highest
        // score, weighted by the quality (score.special in
        // results{<class>}[<index>].0) and divided by the count of
        // classifiers, is the class to choose.  The magnitude of the
        // score relative to how many classifiers contributed is a
        // measure of quality of classification.  As is the score for
        // other classes.  FIXME for clarity!

        let mut scores:Vec<(&str, f64)> = Vec::new();

        // Set this if at leaset one class had some finite results
        let mut flag = false; 

        for k in self.d_all.class_names.iter() {

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
            let mut ret = (scores.first().unwrap().0, String::new());
            for s in scores {
                ret.1 += format!("{} {} ", s.0, // Class
                                 s.1.to_string() // Score
                ).as_str();
            }
            Some(ret)
        }
    }

    pub fn analyse(&self) -> PopulationAnalysis {

        // Indexes for  data that is used for testing
        let ref index = self.d_all.testing_i;
        

        // Build a object that describes the quality of the
        // classifiers, as a set, over the test data
        let mut pa = PopulationAnalysis {
            incorrect:0, // Proportion of incorrect classifications
            correct:0,   // Proportion of correct classifications

            // Per class proportion of classifications incorrrectly
            // into that class
            false_positives:HashMap::new(), 

            // Per class proportion of classifications incorrrectly
            // into another class
            false_negatives:HashMap::new(),
            counts:HashMap::new(),

            // The proportion of cases classified
            classified:0,

            // The total number of cases
            population:index.len(),
        };            

        // Initialise counts 
        self.get_classes().iter().map(|x| pa.counts.insert(x, 0));
        
        // Over the testing data clasify each record and compare with true
        // class
        for i in index {
            let ref r = self.d_all.data[*i];
            if let Some((s, _)) = self.classify(r){ 
                // s is estimated class. 

                // The actual class
                let c = self.d_all.get_class(*i);

                // Record how many instances of this class are seen
                let _c = *pa.counts.get(c).unwrap();
                pa.counts.insert(c, _c + 1);
                
                // Check if estimated class is correct.
                if s != c {
                    pa.incorrect = pa.incorrect + 1;
                    let fp = *pa.false_positives.get_mut(s).unwrap();
                    let nn = *pa.false_negatives.get_mut(c).unwrap();
                    pa.false_positives.
                        insert(s, fp + 1).
                        unwrap();
                    pa.false_negatives.
                        insert(&c, nn + 1).
                        unwrap();
                }else{
                    pa.correct = pa.correct + 1;
                }
            }//老虎
        }
        
        pa
    }

    // FIXME Refactor this so it calls a standalone function which can
    // do a new generation referencing only the Forest so it can be put
    // into a thread.  Having it a method of Population means that
    // cannot access 'self'
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
            if !forest.has_tree_nb(&nb) {

                // A unique child in next generation
                let sc = score_individual(&nb, d_all, true);
                let id = new_forest.maxid+1;
                new_forest.insert(&st, Tree{id:id, score:sc.clone(), tree:nb});
                new_forest.maxid = id;
                bnd_rec.write_line(&format!("Cross: {} + {} --> {}/(Sc:{}): {}",
                                            l, r, id, &sc.special, st));
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
                    bnd_rec.write_line(format!("Mutate: {} --> {}: {}/(Sc: {})",
                                               id0, new_forest.maxid, st, &sc.special).as_str());
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
                    new_forest.trees.insert(st.to_string(), t.clone()).unwrap();
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
            //n1 += 1;
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
        new_forest 
    }
    
    pub fn new_generation(&mut self, generation:usize){

        // Call every generation
        let mut bnd_rec = Recorder::new(self.pop_config.birthsanddeaths_file.as_str());
        let new_forest = Population::_new_generation(&mut self.forest,
                                                     self.pop_config.mutate_prob,
                                                     self.pop_config.copy_prob,
                                                     self.pop_config.crossover_percent, 
                                                     self.pop_config.max_population,
                                                     &self.d_all,
                                                     &mut bnd_rec,
                                                     self.pop_config.save_file.as_str());
        

        let s = format!("{} {} {} {} {}",
                        generation,
                        self.best_id(),
                        self.best_score().special,
                        self.len(),
                        self.get_tree_id(self.best_id()).tree.to_string());
        // Set up output file to record each generation:  FIXME move this to population
        let generations_file = &self.pop_config.generations_file;
        let mut generation_recorder = Recorder::new(generations_file.as_str());
        generation_recorder.write_line(&s[..]);
        generation_recorder.buffer.flush().unwrap();
        self.forest = new_forest;
    }
    
    fn _check(forest:&Forest) -> bool {
        let mut ret = true;
        for (_, v) in forest.trees.iter() {
            if !v.score.special.is_finite() {
                ret = false;
                break;
            }
        }
        ret
    }
    fn check(&self) -> bool {
        Population::_check(&self.forest)
    }
    
    

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
                ret.trees.insert(k.to_string(), v.clone()).unwrap();
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
    pub fn cull_sort(&mut self, bnd_rec:&mut Recorder) {
        Population::_cull_sort(&self.forest, bnd_rec);
    }

    fn _get_tree_id<'a>(forest:&'a Forest, id:usize) -> &'a Tree {

        // Get a tree based on its id
        // FIXME Use iterator on Forest or ScoreTreeMap
        for (_, t) in forest.trees.iter() {
            if t.id == id {
                return &t;
            }
        }
        panic!("Cannot get node with id: {}", id);
    }
    fn get_tree_id(&self, id:usize) -> &Tree {    
        Population::_get_tree_id(&self.forest, id)
    }
    

    // #[allow(dead_code)]
    // pub fn get_tree(&self, id:usize) -> &Tree {
    //     // Get a tree based on its order in self.forest.trees. Used to
    //     // inumerate all trees.  FIXME Use a iterator
    //     &self.trees[id]
    // }

    #[allow(dead_code)]
    fn get_trees_of_class(&self, class:&String) -> Vec<&Tree> {
        // FIXME: Ho do e do string comparison better in rust?  Is
        // there a problem ith this?
        let test = class.clone();
        self.forest.trees.iter().filter(|(_,t)| t.score.class == test).map(|(_, x)| x).collect()
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
        Population::roulette_selection(&self.forest)
    }

    fn roulette_selection(forest:&Forest) -> usize {

        // https://en.wikipedia.org/wiki/Fitness_proportionate_selection
        // Return the id of a individual selected using roulette wheel
        // selection:
        // FIXME Return a reference to a tree not a id

        let total_score:f64 = forest.score_trees.iter().fold(0.0, | a, (ref b, ref v)| b.evaluate() * v.len() as f64 + a);

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
                        // actual tree's id
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
        
        // Add a individuall.  If the individual is already in the
        // population do not add it and return false
        let n = Box::new(Node::new(&d_all.input_names, 0));

        let st = n.to_string();
        if !forest.trees.contains_key(&st.clone()) {

            // This node is unique
            let id = forest.maxid + 1;

            let sc = score_individual(&n, d_all, true);
            {
                bnd_rec.write_line(&format!("Create {}/(Sc: {}) {}", id, sc.special, n.to_string()));
            }
            forest.insert(&st, Tree{id:id, score:sc, tree:n});
            forest.maxid = id;
            true
        }else{
            false
        }
    }
    fn add_individual(&mut self, bnd_rec:&mut Recorder) -> bool {
        Population::_add_individual(&self.d_all, bnd_rec, &mut self.forest)
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

        // Get the id so e can rite a record
        let id = forest.trees.get(&t).unwrap().id;
        bnd_rec.write_line(&format!("RIP {} culled", id));
        
        // Delete it
        forest.delete_str(t.as_str());
                                
        
    }
    pub fn delete_worst(&mut self, bnd_rec:&mut Recorder) {
        Population::_delete_worst(&mut self.forest, bnd_rec);
    }

    fn _do_crossover(forest:&Forest)  -> (NodeBox, usize, usize){
        let i0 = Population::roulette_selection(forest);
        let i1 = Population::roulette_selection(forest);
        (Population::_crossover(forest, i0, i1), i0, i1)
    }
    pub fn do_crossover(&mut self) -> (NodeBox, usize, usize){

        // Crossover to breed individuals better at generalisation

        // Choose a node from population to participate in crossover.
        // The higher the score the node got last generation the
        // higher the probability it will be selected to be
        // participate in crossover

        Population::_do_crossover(&self.forest)
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
    fn mutate_tree(&mut self,i:NodeBox) -> NodeBox {
        Population::_mutate_tree(i, &self.d_all)
    }
    
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

    fn crossover(&mut self, lidx:usize, ridx:usize) -> NodeBox {
        Population::_crossover(&self.forest, lidx, ridx)
    }

    
    fn restore_trees(&self) -> Forest {
        // Read in saved state from a file, build a population from
        // it, and return the population

         let mut trees:Forest = Forest::new();

        let file = File::open(&self.pop_config.save_file).expect(format!("Cannot open {}", &self.pop_config.save_file).as_str());
        file.lock_exclusive().expect("Failed to lock save file");
        let buf = BufReader::new(file);

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
                            let strep = &l[m+4..];
                            let node = NodeBox::new(Node::new_from_string(strep));
                            let sc = if self.pop_config.rescore {
                                // Reevaluate the trees against the
                                // test part of the data
                                let _sc = score_individual(&node, &self.d_all, true);
                                //println!("class {} -> {}\tScore {} -> {}", &class, &_sc.class, score, _sc.special);
                                _sc
                                    
                            }else{
                                // Use the saved scores
                                Score{class:class, special:score}
                            };
                            let id = trees.maxid + 1;
                            trees.insert(strep, Tree{id:id, tree:node, score:sc});
                        }
                    }
                },
                Err(e) => panic!(e)
            }
        }
        if self.pop_config.filter > 0 {
            // Only have best self.filter trees of each class

            // // First sort trees by score and length
            // trees.sort_by(|a, b|{
            //     let a1 = &a.score.special;
            //     let b1 = &b.score.special;
            //     match b1.partial_cmp(a1) {
            //         Some(x) => match x {
            //             Ordering::Equal => a.tree.count_nodes().cmp(&b.tree.count_nodes()),
            //             Ordering::Greater => Ordering::Greater,
            //             Ordering::Less => Ordering::Less,
            //         },
            //         None => panic!("Cannot compare {} and {}", a1, b1)
            //     }
            // });

            // Build the structure that will hold the trees
            let mut class_trees:HashMap<String,  Vec<Tree>> = HashMap::new();
            for z in self.get_classes() {
                class_trees.insert(z.clone(), Vec::new());
            }

            // Put trees into classes
            for (_,t) in trees.trees.iter() {
                let c = t.score.class.clone();
                if class_trees.get(&c).unwrap().len() < self.pop_config.filter {
                    class_trees.get_mut(&c).unwrap().push(t.clone());
                }
            }

            // Reinitialise trees and check that there are enough
            // trees on the way
            trees.clear();
            for ct in class_trees.keys() {
                if class_trees.get(ct).unwrap().len() < self.pop_config.filter {
                    eprintln!("Too few trees for class {} Have {} want {}",
                              ct, class_trees.get(ct).unwrap().len(), self.pop_config.filter);
                }
                for t in class_trees.get(ct).unwrap() {
                    // FIXME How do I avoid all this copying
                    trees.insert(&t.tree.to_string(), t.clone());
                }
            }
        }                
        trees
    }

    fn _save_trees(forest:&Forest, save_file:&str){
        
        let mut state = String::new();
        for (s, t) in forest.trees.iter() {
            state += &format!("Class{}Score{}Node{}\n", t.score.class, t.score.special,s);
        }

        let file = File::create(save_file).unwrap();
        file.lock_exclusive().expect("Failed to lock save file");

        let mut buf = BufWriter::new(file);
        buf.write(&state.into_bytes()[..]).unwrap();
    }
    
    pub fn save_trees(&self){
        Population::_save_trees(&self.forest, self.pop_config.save_file.as_str())
        
    }
}    


// pub fn run_in_thread(config:&Config, a:Arc<Mutex<SimulationStatus>>) -> thread::JoinHandle<()> {
//     // Load the data
    
//     let pop_config = PopulationConfig::new(&config);

//     let forest = Forest::new();

//     let data_file = config.get_string("data_file").unwrap() ;
//     let training_percent = config.get_usize("training_percent").unwrap();
//     let d_all = Data::new(&data_file, training_percent);


//     // Write the header for the generaion file
//     let s = format!("generation, best_id, Best Score General, Best Score Special, Population, Best");
//     let  generations_file = pop_config.generations_file.clone();
//     let mut generation_recorder = Recorder::new(&generations_file[..]);
//     generation_recorder.write_line(&s[..]);
//     generation_recorder.buffer.flush().unwrap();

// }
