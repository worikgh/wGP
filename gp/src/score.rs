use super::NodeBox;
use super::Data;
use inputs::Inputs;
use std::cmp::Ordering;

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

}

impl Score {

    pub fn quality(&self) -> f64 {
        self.quality
    }
    
    pub fn partial_cmp(&self, other:&Score) -> Option<Ordering> {
        // For ordering array of scores
        self.quality().partial_cmp(&other.quality())
    }
    pub fn is_finite(&self) -> bool {
        self.quality.is_finite()
    }

    #[allow(dead_code)]
    pub fn copy(&self) -> Score {
        Score{quality:self.quality, }
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Score) -> Ordering {
        let s1 = self.quality();
        let s2 = other.quality();
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

#[derive(Debug)]
pub enum ScoreError {
    FailedEvaluation,
    NonFiniteSummation,
}
pub fn score_individual(
    node:&NodeBox,
    d:&Data,
    use_testing:bool) -> Result<Score, ScoreError> {

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

    let mut inputs = Inputs::new();


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

        let t = r[d.names.len()-1];
        
        // Get the estimate
        match node.evaluate(&inputs) {
            Some(e) => {        
                let l = (t-e).powi(2);
                y_d.push(l);
            },
            None => return Err(ScoreError::FailedEvaluation),
        };
    }


    let rss = y_d.iter().fold(0.0, |mut sum, &x| {sum += x; sum/(y_d.len().pow(2) as f64)}).sqrt();
    // Must be increasing.  In this case maximum is 1, minimum aproaches 0
    let s = 1.0/(rss + 1.0); 
    //println!("Node: {} Score: {}", node.to_string(), s);
    match s.is_finite() {
        true => Ok(Score{quality:s,}),
        false => Err(ScoreError::NonFiniteSummation),
    }
}

