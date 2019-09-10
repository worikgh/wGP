// Data inputs.  Text files with commer seperated values a record per
// line and the same number of fields per record.  All but the last
// column arer inputs.  Teh last column is the true value of the
// function.  Inputs are all f64

// First line describes names the  columns

// Data starts from second line and continues to end of file.

use File;
use std::io::BufReader;
use std::io::BufRead;
use super::rng;

#[derive(Debug, Clone)]
/// Hold data for training or testing.  Data is columnated where each
/// row is a case.  Training and testing data have two sorts of
/// columns: input columns where each column contains the value for a
/// particular parameter and case, and class columns where each column
/// represents a class and the value is `1` if that case is of that
/// class and `0` if it is not.  The cases (rows) are divided into
/// training and testing cases using seperate indexes.
pub struct Data {
    
    /// Names of the columns that are inputs
    pub names:Vec<String>,

    // Reference into names for the columns that are inputs (all but
    // last column)
    // pub input_names:Vec<&'a str>,
    pub input_names:Vec<String>,
    
    /// Each row of inputs.  
    pub data:Vec<Vec<f64>>,

    /// Indexes into rows for training data
    pub training_i:Vec<usize>,

    /// Indexes into rows for testing data
    pub testing_i:Vec<usize>,

}

impl Data {
    pub fn new(data_file:&str, training_percent:usize) -> Data {
        let mut ret = Data{
            names:Vec::<String>::new(),
            //input_names:Vec::<&'a str>::new(),
            input_names:Vec::<String>::new(),
            data:Vec::<Vec<f64>>::new(),
            testing_i:Vec::<usize>::new(),
            training_i:Vec::<usize>::new(),
        };
        ret.read_data(data_file, training_percent).unwrap();
        ret
    }

    fn reset(&mut self){
        self.names = Vec::<String>::new();
        // self.input_names = Vec::<&'a str>::new();
        self.input_names = Vec::<String>::new();
        self.data = Vec::<Vec<f64>>::new();
        self.testing_i = Vec::<usize>::new();
        self.training_i = Vec::<usize>::new();
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
        }
    }        

    /// Read in the data from a file
    fn read_data(&mut self, f_name:&str,
                 training_percent:usize)  -> std::io::Result<()>{

        // Must be in file f_name.  First row is a header with names.
        self.reset();
        let file = File::open(f_name)?;
        let mut buf_reader = BufReader::new(file);

        // Get first line, allocate names 
        let mut l_names:String = String::new();
        buf_reader.read_line(&mut l_names)?;

        let h_names:Vec<&str> = l_names.split(',').collect();


        for i in 0..h_names.len() {

            // Get the name of input/class
            let s = h_names.iter().nth(i).unwrap();
            self.names.push(s.to_string());
        }
        for i in 0..self.names.len() - 1 {
            self.input_names.push(self.names[i].clone());
        }
        eprintln!("Names: {:?}", self.names);
        eprintln!("Input Names: {:?}", self.input_names);
        
        // Loop over the data storing it in the rows
        loop {
            let mut line:String = String::new();
            match buf_reader.read_line(&mut line)? {
                0 => break, // EOF
                _ => {
                    let d:Vec<&str> = line.split(',').collect();
                    let d:Vec<f64> = d.iter().map(|x| {
                        x.trim_end().parse::<f64>().unwrap()
                    }).collect();
                    self.add_data_row(d);
                },
            };
        }
        self.partition(training_percent);
        Ok(())
    }
}
