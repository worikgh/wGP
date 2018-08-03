use population::PopulationAnalysis;
//use std::collections::HashMap;    
use config::Config;
use config_default::ConfigDefault;
use controller::Controller;
use ncurses::*;
use std::env;
use std::fs::File;
use std::fs;
use std::path::Path;

pub struct FrontEnd {

    // The directory structure of simulations is constant belo this
    // root directory
    root_dir:String,

    // controller is in charge of the simulations
    controller:Controller,

    main_window:WINDOW,
    menu_window:WINDOW,
    status_window:WINDOW,
}

impl FrontEnd {
    pub fn new() -> FrontEnd {

        // The root directory of the process.  FIXME This should be
        // passed in on the command line, optionally
        let root_dir = format!("{}/",
                               env::current_dir().unwrap().
                               to_str().unwrap().
                               to_string());
        
        initscr();
        raw();
        start_color();
        cbreak();
        noecho();
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        keypad(stdscr(), true);
        init_pair(1, COLOR_RED, COLOR_BLACK);
        
        /* Get the screen bounds. */
        let (max_x, max_y, x, _y) = (COLS(), LINES(), 0, 0);

        // Calculate where the menu, main and status panes are in
        // terms of max_y.  y=0 is top of screen

        // The number of lines the menu and status indows use.  This
        // includes border FIXME make these cofigurable
        let status_lines = 3;
        let menu_lines = 3;

        let (menu_h, main_h, status_h) =
            (menu_lines, max_y - (menu_lines+status_lines), status_lines);
        let (menu_y, main_y, status_y) =
            (0, menu_lines, max_y - status_lines);
        let menu_window = newwin(menu_h, max_x, menu_y, x);    
        box_(menu_window, 0, 0);
        let main_window = newwin(main_h, max_x, main_y, x);    
        box_(main_window, 0, 0);
        let status_window = newwin(status_h, max_x, status_y, x);    
        box_(status_window, 0, 0);

        redraw(main_window);
        redraw(status_window);
        redraw(menu_window);
        
        FrontEnd{
            //config:config,
            controller:Controller::new(root_dir.clone()),
            root_dir:root_dir,

            
            main_window:main_window,
            menu_window:menu_window,
            status_window:status_window,
        }
    }
    pub fn fe_start(&mut self) {
        // Entry point
        loop {
            let c = self.make_menu(&vec!["Choose Project", "Display Config"] );
            match c {
                0 => break,
                1 => self.do_choose_object(),
                2 => self.do_display_config("<PROJECT NAME>"),
                _ => panic!(),
            }
        }
        fe_shut();
    }

    fn do_choose_object(&mut self){

        // Level one

        // Get the root directory of the projects
        let proj_dir = Path::new("./Data/");
        self.status_line(&format!("Current Directory {:?}", env::current_dir()));

        // Get the sub-directories that are projects
        let mut projects:Vec<String> = Vec::new();
        let entries = match fs::read_dir(proj_dir) {
            Ok(v) => v,
            Err(e) => {
                let s = format!("Failed reading {:?} e: {} cd: {:?}", proj_dir.to_str(), e, env::current_dir());
                fe_shut();
                panic!(s);
            }
        };
        
        for e in entries {
            // For ever entry in the directory...
            let p = e.unwrap().path();
            let md = p.metadata().expect("metadata call failed");
            if md.file_type().is_dir() {
                // ...if a sub directories it is a project
                let sp = p.file_name().unwrap().to_str().unwrap().to_string();
                projects.push(sp);
            }
        }

        // Display the projects for user to select one
        let menu_vec:Vec<_> = projects.iter().zip(1..(1+projects.len())).map(|x| format!("{} {}", x.1, x.0)).collect();
        loop {
            self.fe_main(&menu_vec);
            let c = self.make_menu(&vec!["Enter Choice", "Display Config", "Quit"] );
            if c == 0 {
                break;
            }else if c <= menu_vec.len() {
                let project = projects.iter().nth(c-1).unwrap().clone();
                self.status_line(&format!("Choose index {} project {}", c-1, &project));
                self.do_project(&project);
            }else if c == 0{
                break;
            }else{
                self.status_line(&format!("Option {} not valid", c));
            }
        }    
        
    }

    fn do_display_config(&self, name:&str)  {
        // Level One
        let config = self.default_config(name);

        let mut keys = config.data.keys();
        let mut cfg_strings:Vec<String> = Vec::new();
        for _ in 1..config.data.len() {
            if let Some(key) = keys.next() {
                let v = config.data.get(key).unwrap();
                let item = format!("{}:\t{}", key, v);
                cfg_strings.push(item);
            }
        }
        self.fe_main(&cfg_strings);
    }

    // End of level one
    //------------------------------------------
    // Level two

    fn do_project(& mut self, name:&str) {
        // Level two

        // Display a project's status and options on
        // starting/stopping/resuming/analysing it or using it to
        // classify a example
        loop {
            werase(self.main_window);

            let status = self.controller.get_status(name);
            let projects:Vec<_> = vec!["Create", "Refresh Status",
                                       "Analyse", "Configuration", "Utilise"];        

            // Display status
            // Name:Sting
            // cleared:bool,
            // running:bool,
            // generation:usize,
            // path:String,
            let x_status = 1; // Start column for status
            let mut y_status = 1; // Status lines
            if wattron(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattron");
            }
            if mvwprintw(self.main_window, y_status, x_status, "Status") != 0 {
                panic!("Failed mvwprintw");
            }
            y_status = y_status+2; // Leave a blank line
            if wattroff(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattroff");
            }

            if mvwprintw(self.main_window, y_status, x_status,
                         &format!("Name: {}", name)) != 0 {
                panic!("Failed mvprint");
            }
            y_status = y_status+1;
            if mvwprintw(self.main_window, y_status, x_status, 
                         &format!("Cleared: {:?}", status.cleared)) != 0 {
                panic!("Failed mvprint");
            }
            y_status = y_status+1;
            if mvwprintw(self.main_window, y_status, x_status, 
                         &format!("Running: {:?}", status.running)) != 0 {      
                panic!("Failed mvprint");
            }
            y_status = y_status+1;
            if mvwprintw(self.main_window, y_status, x_status, 
                         &format!("Generation: {:?}", status.generation)) != 0 {
                panic!("Failed mvprint");
            }
            y_status = y_status+1;
            if mvwprintw(self.main_window, y_status, x_status, 
                         &format!("Path: {}", status.path)) != 0 {
                panic!("Failed mvprint");
            }
            //y_status = y_status+1;

            // Get configuration object (if running) to write a
            // section on the files in the project's directory
            let x_files = 30; // Starting column for files section
            let mut y_files = 1;  // Each line...
            if wattron(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattron");
            }
            if mvwprintw(self.main_window, y_files, x_files, "Files") != 0 {
                panic!("Failed mvwprintw");
            }
            if wattroff(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattroff");
            }
            y_files = y_files+2; // Leave a blank line
            let config = match self.controller.get_config(name) {
                Some(c) => // Got a handle to a thread running simulation
                    c, 
                None => {
                    eprintln!("do_project default config");
                    self.default_config(name)
                },
            };

            
            // Check if the file that describes a population is there
            // save_file
            let save_file = match config.get_string("save_file") {
                Some(f) => f,
                None => "<NONE>".to_string(),
            };
            let save_file_path = format!("{}Data/{}/{}",
                                         self.root_dir, name, save_file);
            let save_file_exists = Path::new(save_file_path.as_str()).is_file();
            let save_file_exists = if save_file_exists {
                eprintln!("Existing");
                "Exists"
            }else{
                eprintln!("Not Existing {}", save_file_path);
                ""
            };
            
            let msg = format!("Trees: {} {}", save_file_exists, save_file);
            if mvwprintw(self.main_window, y_files, x_files, msg.as_str()) != 0 {
                panic!("Failed mvwprintw");
            }

            redraw(self.main_window);

            let c = self.make_menu(&projects);
            // Got a key.
            
            match c {
                // 0 allways sends us back 
                0 => break,
                
                1 => {
                    // Run a simulation.  Start by build a
                    // configuration object.  This is top level
                    // configuration that can be overridden by project
                    // configuration First load default data.  Either
                    // in the root_dir/.gp_config or hard coded
                    // DefaultConfig
                    let config = self.default_config(name);

                    // Use the controller o run the simulation.  If it
                    // fails to launch the status is set appropriately
                    let status = match self.controller.run_simulation(&config) {
                        Ok(_) => "Ok".to_string(), 
                        Err(s) => s.to_string(),
                    };
                    self.status_line(&status);
                },
                3 => {
                    // Analyse
                },
                4 => {
                    // Display config
                }
                
                _ => (),
            }
            
            
        }
    }

    // End of rational ordering of functions....
    //---------------------------------------------------------
    
    fn default_config(&self, name:&str) -> Config {

        // Get the default configuration file for all simulatuons.  It
        // is iin the root of the directory structure in file named
        // ".gp_config".  Or not.  If not use a hard coded default
        let mut config =
            match File::open(format!("{}.gp_config", self.root_dir)){
                Ok(f) => Config::new_file(f),
                Err(_) => ConfigDefault::new(name),
            };
        
        // Update the configuration file.  Find the project <name> and
        // if it exists check for a ".gp_config" file.  If it is found
        // use that to update the config.  If it does not then adjust
        // some fields in the default: The root_dir may not be
        // accurate as the file may have been moved, and the "name" is
        // particular to a simulation
        match self.find_project_config(name) {
            Some(cfg) => {
                eprintln!("default_config Found one");
                for k in cfg.data.keys() {
                    config.data.insert(k.to_string(),
                                       cfg.data.get(k).unwrap().to_string());
                }
            },
            None => {
                eprintln!("default_config Found none");
                
                config.data.insert("root_dir".to_string(),
                                   self.root_dir.clone());
                config.data.insert("name".to_string(), name.to_string());
            },
        };
        config
    }

    fn find_project_config(&self, name:&str) -> Option<Config> {
        // If the named project exists and has a configuration file
        // build a Config object and return it
        let path = format!("{}Data/{}/.gp_config", self.root_dir, name);
        let cfg_file = Path::new(path.as_str());
        eprintln!("find_project_config {:?}", cfg_file);
        if cfg_file.is_file() {
            // A project configuration file exists
            Some(Config::new(path.as_str()))
        }else{
            None
        }
    }


    // fn edit_config(&self, config:&Config) -> Config {
    //     // This is waiting on developing technology to edit data 
        
    // }



    fn fe_main(&self, list:&Vec<String>) {
        // Display a list in the main pane
        let x = 1;
        let mut y = 1;
        werase(self.main_window);
        for s in list.iter() {
            mvwprintw(self.main_window, y, x, &*s);
            y = y + 1;
        }
        redraw(self.main_window);
    }

    
    fn make_menu(&self, menu_items:&Vec<&str>) -> usize {
        werase(self.menu_window);
        //redraw(self.status_window);
        // eprintln!("make_menu");
        let mut x = 1; // Start of menu
        // werase(self.menu_window);
        if menu_items.len() > 0 {
            for i in 1..menu_items.len()+1 {
                let s = menu_items.iter().nth(i-1).unwrap();
                if mvwprintw(self.menu_window, 1, x,
                             &format!("{} {}", i, &s)) != 0{
                    panic!("Failed mvwprint");
                }
                eprintln!("make_menu {}", format!("{} {}", x, s));
                x = x + s.len() as i32 + 3;
            }
        }
        if mvwprintw(self.menu_window, 1, x, &format!("{} {}", 0, "Back")) != 0{
            panic!("Failed mvwprint");
        }
        redraw(self.menu_window);
        let ret = wgetch(self.menu_window) as usize - 48;
        eprintln!("make_menu Got {} ", ret);
        ret
    }

    fn status_line(& self, msg:&str) {
        werase(self.status_window);
        if mvwprintw(self.status_window, 1, 1, msg) != 0{
            panic!("Failed mvwprint");
        }
        redraw(self.status_window);
    }

    fn do_analysis_window(&self, pa:&PopulationAnalysis ){

        // Normalise
        // let total = self.population as f64;
        // let total_classified = pa.correct as f64 + pa.incorrect as f64;
        // let classified = (total_classified)/(total as f64);
        // let correct = pa.correct/(total);
        // let incorrect = pa.incorrect/(total);
        // let false_positives:HashMap<String, f64> = HashMap::new();
        // let false_negatives:HashMap<String, f64> = HashMap::new();
        // for c in self.get_classes().iter() {
        //     let count = *pa.counts.get(c.as_str()).unwrap() as f64;

        //     let f_p = *pa.false_positives.get(c.as_str()).unwrap() as f64;
        //     false_positives.insert(c, f_p / count);

        //     let f_n = *pa.false_negatives.get(c.as_str()).unwrap();
        //     false_negatives.insert(c, f_n / count);
        // }
    }
}

fn fe_shut(){
    /* Terminate ncurses. */
    endwin();    
}







fn redraw(win:WINDOW) {
    box_(win, 0, 0);
    wrefresh(win);
}    
    
