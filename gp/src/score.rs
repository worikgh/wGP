use super::NodeBox;
use super::Data;
//use data::Class;
use inputs::Inputs;
use std::cmp::Ordering;
//use std;
use super::rng;
/// Scoring a individual is key to evolving a good population of
/// individuals.

/// The object of a simulation, in the general non-linear
/// classification case, is to evolve a good population of classifiers.

/// Each classifier is specialised for a class, the class is stored in
/// the Score struct along with a measure of quality.

/// Much of the evolution process is driven by scoring.  When a
/// individual is selected for crossover or for reproduction the
/// selection is weighted by probability.

#[derive(PartialEq, Debug, Clone)]
pub struct Score {

    // FIXME Need differentiation also
    
    // Fitness calculated when classifying to self.class.unwrap()
    pub quality:f64,

    //  The name of the class this score is specialised for. Obtained
    //  from the objective data
    pub class:String,
}

impl Score {

    pub fn evaluate(&self) -> f64 {
        self.quality
    }
    
    pub fn partial_cmp(&self, other:&Score) -> Option<Ordering> {
        // For ordering array of scores
        self.evaluate().partial_cmp(&other.evaluate())
    }
    pub fn is_finite(&self) -> bool {
        self.quality.is_finite()
    }

    #[allow(dead_code)]
    pub fn copy(&self) -> Score {
        let class = self.class.clone();
        Score{quality:self.quality, class:class}
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Score) -> Ordering {
        let s1 = self.evaluate();
        let s2 = other.evaluate();
        let o = s1 - s2;
        if o > 0.0 {
            Ordering::Less
        }else if o < 0.0 {
            Ordering::Greater
        }else{
            Ordering::Equal
        }
    }
}
// https://doc.rust-lang.org/std/cmp/trait.Eq.html
impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Score) -> Option<Ordering> {
        self.partial_cmp(other)
    }
}

#[inline]
fn loss_function(t:f64, y:f64) -> f64 {
    // y is the estimate, t the true value 

    // True value is -1.0 if this the input is not a member of the class
    // the individual tests for, and 1.0 if it is
    
    // Hinge loss
    let loss = 1.0-t*y;
    if loss < 0.0 {
        0.0
    }else{
        loss
    }
}

pub fn score_individual(
    node:&NodeBox,
    d:&Data,
    use_testing:bool) -> Score {

    // Score individual is called once per node.  Classifies it
    // (decides what class it is for) and gives it a rating.

    // Using data for which the classes are known, foreach class C run
    // the Node for each example.  If the example is in Class C the
    // desired result from the simulation is 1.0, if not it is -1.0.
    // Using a loss function collect the results for each example in
    // y_d.  The final score (score.quality) is 1.0/(1.0+mean(y_d)).
    // The class is the class that gets the highest score.

    // FIXME Score could have a third part that is a measure of
    // specificity.  Unsure how to represent that in a scalar.
    // Perhaps the difference between the best and the second best
    // score (for two classes)?

    // Get the data to do the evaluation on
    let index:&Vec<usize>;
    if use_testing {
        index = &d.testing_i;
    }else{
        index = &d.training_i;
    }

    let mut c:Option<String> = None;
    let mut best_s = 0.0;// std::f64::MIN;

    let mut inputs = Inputs::new();

    // FIXME To enable differentiation here store each Score for each
    // class and store the difference between the Score::quality for
    // the best and the second best
    for  class in d.class_names.iter() {

        // Store each distance from the estimate to the actual value
        // to calculate best and mean estimate

        let mut y_d:Vec<f64> = Vec::new(); // Distances

        for i in index {

            // Examine each example

            // The data for this example
            let ref r = d.ith_row(*i);

            // Prepare the inputs to the simulation
            for j in 0..d.input_names.len() {
                let v:f64 = r[j];
                let h = d.input_names[j].clone();
                inputs.insert(h.as_str(), v);
            }

            // Is this example in the class of this node?  True value
            // is -1.0 if this the input is not a member of the class
            // the individual tests for, and 1.0 if it is for.

            let t = if r[d.class_idx(class)] > 0.0 {
                1.0
            }else{
                -1.0
            };

            // Get the estimate
            let e = node.evaluate(&inputs).unwrap();
            

            // Score: See ExperimentalResults.tex for explanation of
            // the loss function
            let l = loss_function(t, e);
            y_d.push(l);
        }

        let mut s = (1.0/(index.len() as f64))*
            y_d.iter().fold(0.0, |mut sum, &x| {sum += x; sum});
        if s.is_finite() {
            // The score must finite and be increasing with quality.
            // The above is a quantity to minimise.  Cannot use a
            // direct inverse as a zero will cause a exception.  Zero
            // is the floor of s
            s = 1.0/(1.0+s);
            
            if s >= best_s {
                c = Some(class.clone());
                best_s = s;
            }
        }
    }
    let _c = match c{
        Some(c) => c,
        None => {
            // This only happens if there as no finite score for the
            // Node for any class. 
            let n = rng::gen_range(0, d.class_names.len());
            d.class_names.iter().nth(n).unwrap().to_string()
        }
    };

    //println!("Node: {} Score: {} Class: {}", node.to_string(), best_s, _c);
    Score{quality:best_s,
          // FIXME This should be a reference with a life time.  This
          // string should be in population.class_names only
          class:_c,
    }
}

