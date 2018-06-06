// Data inputs.  Text files with commer seperated values a record per
// line and the same number of fields per record.  Inputs are f64 and
// classes {1,0}.  Each line represents a record that is in one or
// more class.

// First line descripbes which columns are inputs and which are
// classes.  The input columns/fields are first followed by the class
// columns/fields.  There must be at least two classes and each record
// must be in at least one class.  Line is of form:

// 1, 1, 1, 0, 0 where three inputs are data for classifying into two
// classes

// The second line is the names of the fields.

// Data starts from third line and continues to end of file.

use File;
use std::io::Read;
use BufReader;
use super::rng;
//use score::classify_node;
// Type for class definitions
//pub type Class = f64; // {1.0,0.0}

// Hold data for training or testing.
pub struct Data {
    
    // Names of the columns
    pub input_names:Vec<String>,
    pub class_names:Vec<String>,

    // Kind of fields.  true for inputs false for classes
    pub kind:Vec<bool>,

    // Each row of inputs.  
    pub data:Vec<Vec<f64>>,

    // Indexes into rows for training data
    pub training_i:Vec<usize>,

    // Indexes into rows for testing data
    pub testing_i:Vec<usize>,

    // Indexes into rows for all data
    pub all_i:Vec<usize>,

}

impl Data {
    pub fn class_idx(&self, class:&str) -> usize {
        // Return the index of the class name in self.data
        self.input_names.len() +
            match self.class_names.iter().position(|ref x| *x == class) {
                Some(i) => i,
                None => panic!("Class {} is unknown", class)
            }
    }

    pub fn get_class(&self, i:usize) -> &String {
        let row = &self.data[i];
        let mut ret:Option<&String> = None;
        let start =  self.input_names.len();
        let end = start + self.class_names.len();
        for j in start..end {
            if row[j] == 1.0 {
                ret = Some(&self.class_names[j-start]);
                break;
            }
        }
        ret.unwrap()
    }
    #[allow(dead_code)]
    /// Return all the data as a string
    // fn to_string(&self) -> String {
    //     let mut ret = "".to_string();
    //     for r in &self.inputs {
    //         for i in 0..self.input_names.len() {
    //             ret.push_str(&r[i].to_string()[..]);
    //             ret.push_str(",");
    //         }
    //     }
    //     for r in &self.classes {
    //         for i in 0..self.class_names.len() {
    //             ret.push_str(&r[i].to_string()[..]);
    //             ret.push_str(",");
    //         }
    //         ret.push_str("\n");
    //     }
    //     ret
    // }
    pub fn new() -> Data {
        Data{
            input_names:Vec::<String>::new(),
            class_names:Vec::<String>::new(),
            kind:Vec::new(),
            data:Vec::<Vec<f64>>::new(),
            testing_i:Vec::<usize>::new(),
            training_i:Vec::<usize>::new(),
            all_i:Vec::<usize>::new(),
        }
    }
    fn reset(&mut self){
        self.input_names = Vec::<String>::new();
        self.class_names = Vec::<String>::new();
        self.data = Vec::<Vec<f64>>::new();
        self.testing_i = Vec::<usize>::new();
        self.training_i = Vec::<usize>::new();
        self.all_i = Vec::<usize>::new();
    }        
    pub fn ith_row(&self, i:usize) -> &Vec<f64> {
        &self.data[i]
    }

    fn add_data_row(&mut self, row:Vec<f64>){
        self.data.push(row);
    }
    
    fn partition(&mut self, training_percent:usize){
        // Partition the data into training and testing sets
        for i in 0..self.data.len() {
            let z = rng::gen_range(0, 100);
            if z < training_percent {
                self.training_i.push(i);
            }else{
                self.testing_i.push(i);
            }
            self.all_i.push(i);
        }
    }        
    // Read in the data from a file
    pub fn read_data(&mut self, f_name:&str,
                     training_percent:usize) {

        // Must be in file f_name.  First row is a header with names.
        // Second is a row that indicates which columns are inputs and
        // which identify classes
        self.reset();
        let file = File::open(f_name).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let mut lines = contents.lines();

        // Get first line, allocate names 
        let h = lines.nth(0);
        let l_names = match h {
            Some(l) =>   {
                l
            },
            None => panic!(""),

        };

        // Get second line for column types
        let h = lines.nth(0);
        let l_indicate = match h {
            Some(l) =>   {
                l
            },
            None => panic!(""),
        };

        // FIXME Why over two lines?
        let h_ind1:Vec<&str> = l_indicate.split(',').collect();
        let h_ind:Vec<usize> = h_ind1.iter().map(|x| {x.parse::<usize>().unwrap()}).collect();
        // let h_ind = h_ind2.map(|x| x.parse::<usize>().unwrap()).collect();

        let h_names:Vec<&str> = l_names.split(',').collect();
        // assert_eq!(h_ind.len(), h_names.len());

        // Set to false after processed all input columns.  Means the
        // case where all inputs, then all classes is violated can be
        // detected.  "You Will Respect My Authoritah!"
        let mut flag = true; 

        for i in 0..h_names.len() {

            // Get the name of input/class
            let s = h_names.iter().nth(i).unwrap();
            
            let f = *h_ind.iter().nth(i).unwrap();
            if f == 1 {
                // This is input
                
                if !flag {
                    panic!("At column {} see a input!  Already seen a class", i);
                }

                // This is where input names are owned.  Borrow them
                // from here
                self.input_names.push(s.clone().to_string());

            }else{
                // This is class
                
                // Get the class name
                let class = s.clone();

                // Transfer the class name.  This is where class names
                // are owned.  Borrow them from here
                self.class_names.push(class.to_string());
                
                flag = false;
            }
        }

        // Loop over the data storing it in the rows
        loop {
            
            let line = match lines.next() {
                Some(l) => l,
                None => break,
            };
            let d:Vec<&str> = line.split(',').collect();
            let d:Vec<f64> = d.iter().map(|x| {x.parse::<f64>().unwrap()}).collect();

            self.add_data_row(d);
            
        }
        self.partition(training_percent);
    }
}
