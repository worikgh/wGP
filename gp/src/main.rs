#[macro_use] extern crate lazy_static;
extern crate rand;
extern crate statistical;

mod config;
mod data;
mod inputs;
mod node;
mod population;
mod rng;
mod score;
use config::Config;
use data::Data;
use inputs::Inputs;
use population::Population;
use score::score_individual;
use std::env;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;
use std::path::Path;
use std::time::SystemTime;

// The type of data that can be a terminal
#[derive(Debug)]
enum TerminalType {
    Float(f64),
    // Custom terminals for inputs
    Inputf64(String),
}

// Get the data from the terminal
fn gt(tt:&TerminalType) -> String {
    match tt {
        &TerminalType::Float(f) => format!("Float {} ",f),
        &TerminalType::Inputf64(ref s) => format!("{} ",s),
    }
}

impl fmt::Display for TerminalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = gt(self);
        write!(f, "{}", n)
    }
}

// The operations that are implemented
#[derive(Debug)]
enum Operator {
    Add,  
    Log,  
    Multiply,
    Invert, // ??! Invert 0.0?
    Negate,
    If,
    Gt, // >
    Lt, // <
    Terminal(TerminalType),
}
use node::NodeBox;
//use node::Node;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_data_partition() {
        let config = Config::new("config");
        let data_file = config.get_string("data_file").unwrap();

        // Load the data
        let mut d_all:Data = Data::new();
        d_all.read_data(data_file.as_str(), 0);
        assert_eq!(d_all.training_i.len(), 0);

        d_all.read_data(data_file.as_str(), 100);
        assert_eq!(d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 50);
        assert_ne!(d_all.testing_i.len(), 0);
        assert_ne!(d_all.training_i.len(), 0);

        d_all.read_data(data_file.as_str(), 10);
        assert!(d_all.training_i.len()< d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 90);
        assert!(d_all.training_i.len() > d_all.testing_i.len(), 0);

    }
    #[test]
    fn test_node_eval(){
        let mut inputs = Inputs::new();
        let s = "Multiply Float 0.06 Negate Diameter";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, -0.60);

        let mut inputs = Inputs::new();
        let s = "Invert Float 0.1";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 10.0);

        let inputs = Inputs::new();
        let s = "Add Float 2000.0 Invert Float 0.1";
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 2010.0);

        let mut inputs = Inputs::new();
        let s = "Multiply Height Add Float 10.0 Invert Float 0.1";
        inputs.insert("Height", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 200.0);

        let s = "If Lt x Float 0.0 Float -1.0 Float 1.0";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", 0.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", -0.01);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Gt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Lt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.2);
        inputs.insert("y", 1.1);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.1);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", -9.0);
        inputs.insert("y", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -9.0);        

    }
    #[test]
    fn test_node_from_string(){
        let s = "Add Add Add Invert Height Diameter Add Negate Float 0.03049337449511591 Add Multiply Negate Invert Float 0.40090461861005733 Negate Diameter Negate Float 0.06321754406175395 Length";
        let n = Node::new_from_string(s);
        let ns = &n.to_string()[..];
        println!("");
        println!("1: {}", s);
        println!("2: {}", ns.trim());
        assert_eq!(ns.trim(), s.to_string());        
    }
}


// Do a simulation to evaluate a model.  Pass a node, the class of the
// node and some data.  Returns a vector of pairs.  The first element
// is true value the second is simulation result
fn simulate(n:&NodeBox, class:&str, d:&Data) -> Vec<(f64, f64)> {
    let mut ret:Vec<(f64, f64)> = vec![];
    let mut inputs = Inputs::new();
    let ref index = d.all_i;
    for i in index {
        let ref r = d.data[*i];
        for j in 0..d.input_names.len() {
            let v = r[j];
            let h = d.input_names[j].clone();
            inputs.insert(h.as_str(), v);
        }
        let e = n.evaluate(&inputs).unwrap();

        // Get the target
        let t = d.data[*i][d.class_idx(class)];
        ret.push((t as f64, e));
    }
    ret
}

// Add a simulation to Simlations.txt.  First line is the IDs of the
// data.  First column (labled ID 0) is the actual value.  Each column
// is the matching data from the model with the ID at the top (FIXME
// Comment!)
fn add_simulation(data:Vec<(f64, f64)>, id:usize, fname:&str){
    let mut contents = String::new();
    // Create a string to hold final results    
    let mut results = "".to_string();

    {
        
        // Test if file exists.  If not create it
        if ! Path::new(fname).exists() {
            OpenOptions::new().write(true).create(true).open(fname).unwrap();
        }
        let  file = OpenOptions::new()
            .read(true)
            .open(fname).unwrap();
        
        // Get the data in to read
        let mut buf_reader = BufReader::new(file);
        buf_reader.read_to_string(&mut contents).unwrap();
        if contents.len() == 0 {
            // The file was empty Create the first column
            contents.push_str("0 \n");
            for ref d in data.clone() {
                contents.push_str(format!("{} \n", d.0).as_str());
            }
        }
        // Contents is now ready to have another data set appended

        let mut lines = contents.lines();

        // First test: The number of lines is the numebr of cases plus
        // one for header.  This is the same for all simulations so
        // data.len() == lines.count()-1
        assert_eq!(data.len(), lines.count()-1);

        // The test above consumed lines so reinitialise it
        lines = contents.lines();
        
        // Set up the header
        let mut header = lines.next().unwrap().to_string();

        // Append the ID of this model and initialise results with it

        header.push_str(format!("{}", id).as_str());
        header.push(' ');
        header.push('\n');

        results.push_str(header.as_str());
        // Going to loop over all the data from the file and through
        // the data supplied in the 'data' parameter simultaneously
        let mut i = 0; // Index into data
        for l in lines {
            // Get the data members of this row
            let mut d = l.split_whitespace();
            let d0 = d.next().unwrap(); // The actual value as string
            let d0u = d0.parse::<f64>().unwrap(); // The actual value as a number

            // Test: The actual value here must be the same as the
            // actual value in data[i].0
            assert_eq!(d0u, data[i].0);

            // Add the actual to this line
            results.push_str(format!("{} ", d0).as_str());

            // Put the rest of the line in results. FIXME There must
            // be a variadic way to do this
            loop {
                match d.next() {
                    Some(v) => results.push_str(format!("{} ", v).as_str()),
                    None => break,
                };
            }
            // Add in the new data
            results.push_str(format!("{} \n", data[i].1).as_str());

            i += 1;
        }
    }
    // results now holds the new contents of the file
    let mut file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(fname).unwrap();
    file.write(&results.into_bytes()[..]).unwrap();
}
    

/// rite out R script to generate plit of results
fn write_plotting_script(input_data:&str, xlab:&str,
                         outfile:&str, r_script_file:&str,
                         generations_file:&str,
                         id:&str) {
    let  script ="
data <- read.table('SIMULATIONS', header=TRUE)
names <- names(data)

objective <- data[,1]
best.estimate <- data[,names[length(names)]]
png(filename=\"OUTFILE\", width=210, height=297, units=\"mm\", res=600)
oldpar <- par(mfrow=c(2,2))
ratio <- 100*(objective-best.estimate)/objective

plot(x=data[,\"X0\"], y=best.estimate, cex=.2, ylab=\"Estimate\", xlab=\"Actual\", main=\"Best Model\")
hist(ratio, main=\"Error Ratio\", density=10, xlab=\"Percent Error\", freq=FALSE)
hist(objective, main=\"Objective Data\", density=10, breaks=30, xlab=\"XLAB\")
hist(objective-best.estimate, main=\"Differences\", density=10, freq=FALSE, breaks=30)
dev.off()

png(\"OUTFILE_Solutions.png\", width=210, height=297, units=\"mm\", res=600)
plot.data <- data[order(data[,1]),]
c <- ceiling(sqrt(dim(plot.data)[2]-1))
par(mfrow=c(c,c))
x <- plot.data[,1]
y <- plot.data[, dim(plot.data)[2]]

plot(x=x, y=y,
     xlab='Objective',
     ylab='Best Estimate',
     main='Comparison of Models', t='p', cex=.75, pch='x')
for(i in 2:(dim(plot.data)[2]-1)){
    plot(x=x, y=plot.data[,i],
         xlab='Objective',
         ylab='Estimate',
         main='Comparison of Models', t='p', cex=.75, pch='x')
}

dev.off()


png(filename=\"IDGeneration.png\",
    width=210, height=297, units=\"mm\", res=600)
## Read the first four columns from the file as numeric
q <- scan(\"GENERATIONS_FILE\", what = list(1,1,1,1,1,1), flush = TRUE, skip=1)
data <- cbind(c(0, diff(q[[1]])),q[[2]], q[[3]], q[[4]], q[[5]], q[[6]])
## First row has invalid time data (no diff at time 0) so get rid of it?
## data <- data[-1,] Na!

colnames(data) <- c('Sec', 'Gen', 'Best Gen', 'Best Spec', 'Eval', 'Pop')

gen <- data[,'Gen']
pop <- data[,'Pop']
eval <- data[,'Eval']
eval.2 <- eval-min(eval)
pop.2 <- (pop-min(pop))
pop.2 <- pop.2*(max(eval.2)/max(pop.2))
## Normalise pop to same scale as eval



## Define Margins. The trick is to use give as much space possible on
## the left margin (second value)
par(mar=c(5, 12, 4, 4) + 0.1)

## Plot the first time series. Notice that you don’t have to draw the
## axis nor the labels

plot(gen, eval.2, axes=F, ylim=range(eval.2), xlab='', ylab='', type='l',lty=2, main='',xlim=range(gen),lwd=2, col=2)

labels <- signif(seq(from=min(eval), to=max(eval), length.out=8),  4)
at <- seq(from=min(eval.2), to=max(eval.2), length.out=8)
axis(2, at=at, labels=labels, lwd=2,line=3.5)
mtext(2,text='Eval',line=5.5)

## Plot the third time series. Again the line parameter are both
## further increased.

par(new=T)
plot(gen, pop.2, axes=F, ylim=range(pop.2), xlab='', ylab='', type='l',lty=3, main='',xlim=range(gen),lwd=2, col=3)
labels <- signif(seq(from=min(pop), to=max(pop), length.out=8),  4)
at <- seq(from=min(pop.2), to=max(pop.2), length.out=8)
axis(2, at=at, labels=labels, lwd=2, line=7)
##axis(2, ylim=range(pop),lwd=2,line=7)
mtext(2,text='Population',line=9)

##We can now draw the X-axis, which is of course shared by all the
##three time-series.

axis(1,pretty(range(gen),10))
mtext('Generation',side=1,col='black',line=2)

##And then plot the legend.
legend(x='topleft', legend=c('Eval','Pop'),lty=c(2,3), col=c(2,3), bty='n') 
dev.off()
";
    let script = script.replace("SIMULATIONS", input_data);
    let script = script.as_str().replace("XLAB", xlab).to_string();
    let script = script.as_str().replace("GENERATIONS_FILE",
                                         generations_file).to_string();
    let script = script.as_str().replace("OUTFILE", outfile).to_string();
    let script = script.as_str().replace("ID", id).to_string();
    let  file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(r_script_file).unwrap();
    
    let mut writer = BufWriter::new(&file);
    writer.write_all(script.to_string().as_bytes()).unwrap();
}



pub struct Recorder {
    // Manage writing data to a file.  The constructor takes a file
    // name and creates a buffered writer to the file.  Ha a "write"
    // withod that sends data to that buffer for writing to the file.
    // The SystemTime object is used to prefix the number of elapsed
    // seconds to each record
    buffer:BufWriter<File>,
    created:SystemTime,
}
impl Recorder {
    fn new(file_name:&str) -> Recorder {
        Recorder{
            buffer:BufWriter::new(OpenOptions::new()
                                  .append(true)
                                  .create(true)
                                  .open(file_name).unwrap()),
            created:SystemTime::now(),
        }
    }
    fn write_line(&mut self, line:&str) {
        let now = format!("{} ", self.created.elapsed().unwrap().as_secs());
        self.buffer.write(&now.into_bytes()[..]).unwrap();
        self.buffer.write(&line.to_string().into_bytes()[..]).unwrap();
        self.buffer.write(&"\n".to_string().into_bytes()[..]).unwrap();
    }
}

fn main() {
    println!("Start");

    // Get configuration file.  Will use file names 'config' if no
    // file name provided as a argument FIXME:  Don't have a default!!!
    let args: Vec<_> = env::args().collect();
    let cfg_file:String;
    if args.len() > 1 {
        cfg_file = args[1].clone();
    }else{
        cfg_file = "config".to_string();
    }
    let config = Config::new(cfg_file.as_str());

    // Load configuration data
    let generations_file = config.get_string("generations_file").unwrap();
    let model_data_file = config.get_string("model_data_file").unwrap();
    let birthsanddeaths_file = config.get_string("birthsanddeaths_file").unwrap();
    let num_generations = config.get_usize("num_generations").unwrap();
    let plot_file = config.get_string("plot_file").unwrap();
    let plot_xlab = config.get_string("plot_xlab").unwrap();
    let r_script_file = config.get_string("r_script_file").unwrap();
    let seed = config.get_string("seed").unwrap(); // The seed is a string of usize numbers
    let seed:Vec<u32> = seed.split_whitespace().map(|x| x.parse::<u32>().unwrap()).collect();
    let sim_id = config.get_string("id").unwrap();
    let data_file = config.get_string("data_file").unwrap() ;
    let training_percent = config.get_usize("training_percent").unwrap();
    
    // Set up output file to record each generation:  FIXME move this to population
    let mut generation_recorder = Recorder::new(generations_file.as_str());

    // Write out the R script to plot the simulation.  Do this first
    // as it can be used as a long simulation is proceeding
    write_plotting_script(model_data_file.as_str(),
                          plot_xlab.as_str(),
                          plot_file.as_str(),
                          r_script_file.as_str(),
                          generations_file.as_str(),
                          sim_id.as_str(),
    );

    // The source of entropy.  
    rng::reseed(seed.as_slice());

    // Create a population. 
    println!("Population start");
    let data = Data::new(&data_file, training_percent);
    let bnd_recorder = Recorder::new(birthsanddeaths_file.as_str());
    let mut population = Population::new(&config, &data, bnd_recorder);
    population.initialise();
    println!("Created initial population {}", population.len());

    // Write the header for the generaion file
    let s = format!("generation, best_id, Best Score General, Best Score Special, Population, Best");
    generation_recorder.write_line(&s[..]);
    generation_recorder.buffer.flush().unwrap();

    if population.do_train() {



        for generation in 0..num_generations {
            // Main loop
            
            population.new_generation(generation);
            let s = format!("{} {} {} {} {}",
                            generation,
                            population.best_id(),
                            population.best_score().special,
                            population.len(),
                            population.get_tree_id(population.best_id()).1.to_string());
            generation_recorder.write_line(&s[..]);
            generation_recorder.buffer.flush().unwrap();
        }
    }

    if population.do_classify() {
        // Do classification
        population.classify_test();
    }
    println!("Bye!");
        
}

