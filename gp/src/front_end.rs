use config::Config;
use config_default::ConfigDefault;
use controller::Controller;
use controller::SimulationCommand;
use ncurses::*;
use population::PopulationAnalysis;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use rng;

#[derive(PartialEq, Debug, Clone, Copy)]
enum State {
    Stopped,
    Starting,
    // Started,
    DisplayConfig,
    ChooseProject,
    GotProject,
    Refresh,
}
pub struct FrontEnd {

    // The directory structure of simulations is constant belo this
    // root directory
    root_dir:String,

    // controller is in charge of the simulations
    controller:Controller,

    main_window:WINDOW,
    menu_window:WINDOW,
    status_window:WINDOW,

    // Window height and position
    menu_h:i32, main_h:i32, status_h:i32,
    menu_y:i32, main_y:i32, status_y:i32,


    state:Vec<State>,
    inp:Option<usize>, // Input from user.  A key

    projects:Option<BTreeMap<usize, String>>, // Map project names to menu index
    project:Option<String>, // Current project
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
        nodelay(menu_window, true); // Make calls to wgetch non-blocking
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

            state:vec![State::Stopped],
            inp:None,
            menu_h:menu_h, main_h:main_h, status_h:status_h,
            menu_y:menu_y, main_y:main_y, status_y:status_y,
            projects:None,
            project:None,
        }
    }
    fn current_state(&self) -> State {
        // Assumes there is allays atleast one state
        self.state.iter().last().unwrap().clone()
    }
    // Functions to use ncurses top draw on screen
    fn draw_main_splash(&self) {
        // When there is nothing to see on the main screen...
        let mut x = 0;
        let mut y = self.main_y;
        //werase(self.main_window);
        let l = self.main_y+self.main_h - 3; // FIXME  Why '- 3'?
        let c = COLS() - 1; // FIXME why `- 1`?
        let x = rng::random::<usize>() % c as usize;
        let y = rng::random::<usize>() % l as usize;
        let chars = ["x", ".", "!","@","#","$","%","^","&","*","(",")","_","+","=","-",":",";","<",">","?","/"];
        let ind = rng::random::<usize>() % chars.len();
        let c = chars[ind];
        let r =  mvwprintw(self.main_window, y as i32, x as i32, c);
        if r != 0 {
            panic!("Failed mvwprintw r {}  w {:?} x {} y {} l {} c {}", r, self.main_window, x, y, l, c);
        }
        redraw(self.main_window);
    }

    fn display_project(& self, project:&str) {

        // Display project status

        let x_status = 1; // Start column for status
        let mut y_status = 1; // Status lines
        
        werase(self.main_window);

        // ======================
        // Status
        
        let status = &self.controller.get_status(project);

        // Header
        if wattron(self.main_window, A_UNDERLINE()) != 0 {
            panic!("Failed wattron");
        }
        if mvwprintw(self.main_window, y_status, x_status, "Status") != 0 {
            panic!("Failed mvwprintw");
        }
        if wattroff(self.main_window, A_UNDERLINE()) != 0 {
            panic!("Failed wattroff");
        }
        y_status = y_status+2; // Leave a blank line

        // Project name
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Name: {}", project)) != 0 {
            panic!("Failed mvprint");
        }
        y_status = y_status+1;


        // Population
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Population: {}", status.population)) != 0 {
            panic!("Failed mvprint");
        }
        y_status = y_status+1;

        // Cleared by front end
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Cleared: {:?}", status.cleared)) != 0 {
            panic!("Failed mvprint");
        }
        y_status = y_status+1;

        // Simulation running state
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Running: {:?}", status.running)) != 0 {
            panic!("Failed mvprint");
        }
        y_status = y_status+1;

        // Generation
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Generation: {:?}", status.generation)) != 0 {
            panic!("Failed mvprint");
        }
        y_status = y_status+1;

        // Command
        if mvwprintw(self.main_window, y_status, x_status,
                     &format!("Command: {:?}", status.command)) != 0 {
            panic!("Failed mvprint");
        }
        //  End of status display
        //=======================
        //
        // Analysis display

        if let Some(ref analysis) = status.analysis {

            // A analysis is available

            let x_anal = 30;  // Column for display
            let mut y_anal = 1;  // Each line

            // Heading
            if wattron(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattron");
            }
            if mvwprintw(self.main_window, y_anal, x_anal,
                         format!("Analysis Gen: {}", analysis.generation).as_str()) != 0 {
                panic!("Failed mvwprintw");
            }
            if wattroff(self.main_window, A_UNDERLINE()) != 0 {
                panic!("Failed wattroff");                
            }
            y_anal = y_anal+2; // Leave a blank line

            // Summary of results
            if mvwprintw(self.main_window, y_anal, x_anal,
                         format!("Q: {:.2}%", 100.0 * analysis.correct as f64/analysis.cases as f64).as_str()) != 0 {
                panic!("Failed mvwprintw");
            }
            y_anal = y_anal+1;
            
            if mvwprintw(self.main_window, y_anal, x_anal,
                         format!("Cases {} Incorrect: {} Correct: {} Unclassified: {}",
                                 analysis.cases, analysis.incorrect, analysis.correct,
                                 analysis.cases - analysis.classified).as_str()) != 0 {
                panic!("Failed mvwprintw");
            }
            //y_anal = y_anal+1;
            
        }
        redraw(self.main_window);
    }

    // End of functions to use ncurses top draw on screen
    //---------------------------------------------------

    fn read_projects(&self) -> BTreeMap<usize, String> {
        let mut ret = BTreeMap::new();

        // Get the root directory of the projects
        let proj_dir = &format!("{}Data/", self.root_dir);
        let proj_dir = Path::new(proj_dir);
        //self.status_line(&format!("Current Directory {:?}", env::current_dir()));

        // Get the sub-directories that are projects
        let entries = match fs::read_dir(proj_dir) {
            Ok(v) => v,
            Err(e) => {
                let s = format!("Failed reading {:?} e: {} cd: {:?}", proj_dir.to_str(), e, env::current_dir());
                fe_shut();
                panic!(s);
            }
        };
        let mut idx = 1;  // The menu index for each project
        for e in entries {
            // For ever entry in the directory...
            let p = e.unwrap().path();
            let md = p.metadata().expect("metadata call failed");
            if md.file_type().is_dir() {
                // ...if a sub directories it is a project
                let sp = p.file_name().unwrap().to_str().unwrap().to_string();
                ret.insert(idx, sp);
                idx = idx + 1;
            }
        }
        ret
    }

    // Functions to display user interface.  The interface displayed
    // is dependant on self.state
    fn start_screen(&self) -> bool {
        // Initial screen
        self.make_menu(&vec!["Choose Project", "Display Config"]);
        self.draw_main_splash();
        true
    }

    fn project_screen(&mut self) -> bool {
        // state == GotProject 
        self.make_menu(&vec!["Create", "Refresh Status", "Analyse", "Configuration", "Utilise"]);
        if let Some(ref s) =  self.project {
            self.display_project(s.as_str());
        }
        true
    }

    fn choose_project_screen(&self) -> bool {
        // Display projects to choose with a menu FIXME When there are
        // too many projects to fit this will need a scrolling list
        let x = 1;
        let mut y = 1;

        // Main indow display
        werase(self.main_window);
        let it = &self.projects;
        if let Some(it) = it {
            for (ref k, ref v) in  it.iter() {
                //            let ref v = self.projects.unwrap().get(k).unwrap();
                let str = format!("{} {}", k, v);
                mvwprintw(self.main_window, y, x, &str);
                y = y + 1;
            }
        }
        redraw(self.main_window);

        // Menu.  Assume at least one project
        match &self.projects {
            Some(c) => {
                let c = c.keys().count();
                self.make_menu(&vec![&format!("Enter # 1 -> {} for Project", c)]);
            },
            None => panic!("choose_project_screen called when there are no projects"),
        };
        true
    }
    // End of functions to display user interface.

    fn update_display(&mut self) -> bool{
        let state = self.current_state();
        // eprintln!("update_display {:?} stack size: {}", state, self.state.len());
        match state {
            State::Starting => self.start_screen(),
            State::Refresh => true,
            State::Stopped => true,
            State::DisplayConfig => true,
            State::GotProject => self.project_screen(),
            State::ChooseProject => {
                self.projects = Some(self.read_projects());
                self.choose_project_screen()
            },
        }
    }
    fn state_transition(&mut self) -> bool{
        // Return false to exit programme

        let state = self.current_state();

        // Check for input of 0.  Always means go back a screen/state
        if let Some(key) = self.inp {
            if key == 0 {
                // Going back a state...
                self.inp = None; // Consume
                self.state.pop(); // Going back a state Hack to make
                // the splash screen look good.  It does not clear
                // itself each time draw_main_splash is called (it
                // builds up a picture, random as I write), so needs
                // to be cleared before it is displayed
                if self.current_state() == State::Starting {
                    werase(self.main_window);
                }
                return if self.state.len() == 0 {
                    // No states left
                    false
                }else{
                    // FIXME This is turning into spaghetti code.
                    // Must be a better way to manage this
                    match self.current_state() {
                        // Reset current project
                        State::ChooseProject => self.project = None,
                        _ => (),
                    }
                        
                    true
                }
            }
        }
        
        let ret = match state {
            State::Starting => {

                // If there is some key board input process it
                if let Some(key) = self.inp {
                    self.inp = None; // Consume
                    if let Some(s) = self.handle_key(key, state) {
                        eprintln!("Before push stack size {}", self.state.len());
                        self.state.push(s);
                    }
                }
                true
            },
            State::Refresh => {
                // Pop Refresh off state stack and recurse
                self.state.pop();
                self.state_transition()
            },
            State::Stopped => true,
            State::DisplayConfig => true,
            State::GotProject => {
                if let Some(key) = self.inp {
                    if let Some(s) = self.handle_key(key, state) {
                        self.state.push(s);
                    }
                    self.inp = None; // Consume
                }
                true
            },
            State::ChooseProject => {
                // Get the key pressed....
                if let Some(key) = self.inp {
                    self.inp = None; // Consume
                    if let Some(s) = self.handle_key(key, state) {
                        self.project = Some(self.projects.as_ref().unwrap().get(&key).unwrap().clone());
                        eprintln!("Choosing {:?}", self.project);
                        self.state.push(s);
                    }else{
                        // A key that is unrecognised
                        eprintln!("state_transition called in ChooseProject. Unrecognised key {}", key);
                    }
                }else{
                    panic!("state_transition called in ChooseProject state with no input");
                }
                true
            },
        };
        ret
    }

    // State transition functions.  Called with state

    
fn handle_key(&mut self, key:usize, state:State) -> Option<State>{
    // Handle a keypress.  FIXME `key` i the raw ASCII(?) minus 48.  Works for 0-9
    match state {
            State::Starting => match key {
                1 => {
                    // Choose project
                    Some(State::ChooseProject)
                },
                2 => {
                    // Display config
                    Some(State::DisplayConfig)
                },
                _ => None,
            },
            State::ChooseProject => {
                // See if key is a valid index into self.projects.
                assert!(self.projects.is_some());// Precondition
                if let Some(_) = self.projects.as_ref().unwrap().get(&key){
                    // Got a project
                    Some(State::GotProject)
                }else{
                    // Invalid key
                    None
                }
            },
            State::GotProject => {
                eprintln!("handle_key {} {:?} Start", key, state);
                // Check the input
                if let Some(c) = self.inp {
                    // Got a key
                    eprintln!("handle_key {} {:?} Got key {}", key, state, c);
                    match c {
                        1 => {
                            // Run a simulation.  Start by build a
                            // configuration object.  This is top
                            // level configuration that can be
                            // overridden by project configuration
                            // First load default data.  Either in the
                            // root_dir/.gp_config or hard coded
                            // DefaultConfig
                            let name = self.project.as_ref().unwrap();
                            eprintln!("run {}", name);
                            //let config = self.default_config(name.as_str());

                            // Use the controller to run the simulation.  If it
                            // fails to launch the status is set appropriately
                            let status = match self.controller.run_simulation(name) {
                                Ok(_) => format!("{} started", name),
                                Err(s) => s.to_string(),
                            };
                            eprintln!("runing? {}", name);
                            self.status_line(&status);
                            None
                        },
                        2 => Some(State::Refresh),
                        3 => {
                            // Analyse

                            let name = self.project.as_ref().unwrap();
                            eprintln!("analyse {}", name);

                            // FIXME Use a channel.  This sucks!
                            self.controller.set_command(name, SimulationCommand::Analyse);

                            None
                        },
                        4 => {
                            // Display config
                            None
                        },
                        _ => None,
                    };
                    None
                }else{
                    None
                }
            },
            State::Stopped => None,
            State::DisplayConfig => None,
            State::Refresh => None,
        }
    }
    // End of state transition functions.
    pub fn fe_start(&mut self) {
        // Entry point
        self.state = vec![State::Starting];
        eprintln!("fe_start stack size: {}", self.state.len());
        // Event loop of state machine
        loop {
            //let state = self.state.iter().last().unwrap().clone();
            // Make sure what we display matches the state
            self.update_display();
            self.status_line(&format!("State: {:?}", self.current_state()));
            let inp = wgetch(self.menu_window);
            if inp == ERR {
                // No input available.  Need to sleep
                sleep(Duration::new(0, 50000000)); // Fastest is 2 times a second
            }else{
                let inp = inp  as usize - 48;
                self.inp = Some(inp);
                //eprintln!("Before transition State {:?} stack size: {}", state, self.state.len());
                if self.state_transition() == false{
                    break;
                }
            }
            self.status_line(&format!("State: {:?}", self.current_state()));
        }
        fe_shut();
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

    // fn do_project(& mut self, name:&str) {
    //     // Level two

    //     // Display a project's status and options on
    //     // starting/stopping/resuming/analysing it or using it to
    //     // classify a example
    //     loop {
    //         werase(self.main_window);

    //         let status = self.controller.get_status(name);
    //         let projects:Vec<_> = vec!["Create", "Refresh Status",
    //                                    "Analyse", "Configuration", "Utilise"];

    //         // Display status
    //         // Name:Sting
    //         // cleared:bool,
    //         // running:bool,
    //         // generation:usize,
    //         // path:String,
    //         let x_status = 1; // Start column for status
    //         let mut y_status = 1; // Status lines
    //         if wattron(self.main_window, A_UNDERLINE()) != 0 {
    //             panic!("Failed wattron");
    //         }
    //         if mvwprintw(self.main_window, y_status, x_status, "Status") != 0 {
    //             panic!("Failed mvwprintw");
    //         }
    //         y_status = y_status+2; // Leave a blank line
    //         if wattroff(self.main_window, A_UNDERLINE()) != 0 {
    //             panic!("Failed wattroff");
    //         }

    //         if mvwprintw(self.main_window, y_status, x_status,
    //                      &format!("Name: {}", name)) != 0 {
    //             panic!("Failed mvprint");
    //         }
    //         y_status = y_status+1;
    //         if mvwprintw(self.main_window, y_status, x_status,
    //                      &format!("Cleared: {:?}", status.cleared)) != 0 {
    //             panic!("Failed mvprint");
    //         }
    //         y_status = y_status+1;
    //         if mvwprintw(self.main_window, y_status, x_status,
    //                      &format!("Running: {:?}", status.running)) != 0 {
    //             panic!("Failed mvprint");
    //         }
    //         y_status = y_status+1;
    //         if mvwprintw(self.main_window, y_status, x_status,
    //                      &format!("Generation: {:?}", status.generation)) != 0 {
    //             panic!("Failed mvprint");
    //         }
    //         y_status = y_status+1;
    //         if mvwprintw(self.main_window, y_status, x_status,
    //                      &format!("Path: {}", status.path)) != 0 {
    //             panic!("Failed mvprint");
    //         }
    //         //y_status = y_status+1;

    //         // Get configuration object (if running) to write a
    //         // section on the files in the project's directory
    //         let x_files = 30; // Starting column for files section
    //         let mut y_files = 1;  // Each line...
    //         if wattron(self.main_window, A_UNDERLINE()) != 0 {
    //             panic!("Failed wattron");
    //         }
    //         if mvwprintw(self.main_window, y_files, x_files, "Files") != 0 {
    //             panic!("Failed mvwprintw");
    //         }
    //         if wattroff(self.main_window, A_UNDERLINE()) != 0 {
    //             panic!("Failed wattroff");
    //         }
    //         y_files = y_files+2; // Leave a blank line
    //         let config = match self.controller.get_config(name) {
    //             Some(c) => // Got a handle to a thread running simulation
    //                 c,
    //             None => {
    //                 eprintln!("do_project default config");
    //                 self.default_config(name)
    //             },
    //         };


    //         // Check if the file that describes a population is there
    //         // save_file
    //         let save_file = match config.get_string("save_file") {
    //             Some(f) => f,
    //             None => "<NONE>".to_string(),
    //         };
    //         let save_file_path = format!("{}Data/{}/{}",
    //                                      self.root_dir, name, save_file);
    //         let save_file_exists = Path::new(save_file_path.as_str()).is_file();
    //         let save_file_exists = if save_file_exists {
    //             eprintln!("Existing");
    //             "Exists"
    //         }else{
    //             eprintln!("Not Existing {}", save_file_path);
    //             ""
    //         };

    //         let msg = format!("Trees: {} {}", save_file_exists, save_file);
    //         if mvwprintw(self.main_window, y_files, x_files, msg.as_str()) != 0 {
    //             panic!("Failed mvwprintw");
    //         }

    //         redraw(self.main_window);

    //         let c = self.make_menu(&projects);
    //         // Got a key.

    //         match c {
    //             // 0 allways sends us back
    //             0 => break,

    //             1 => {
    //                 // Run a simulation.  Start by build a
    //                 // configuration object.  This is top level
    //                 // configuration that can be overridden by project
    //                 // configuration First load default data.  Either
    //                 // in the root_dir/.gp_config or hard coded
    //                 // DefaultConfig
    //                 let config = self.default_config(name);

    //                 // Use the controller o run the simulation.  If it
    //                 // fails to launch the status is set appropriately
    //                 let status = match self.controller.run_simulation(&config) {
    //                     Ok(_) => "Ok".to_string(),
    //                     Err(s) => s.to_string(),
    //                 };
    //                 self.status_line(&status);
    //             },
    //             3 => {
    //                 // Analyse
    //             },
    //             4 => {
    //                 // Display config
    //             }

    //             _ => (),
    //         }


    //     }
    // }

    // End of rational ordering of functions....
    //---------------------------------------------------------

    fn default_config(&self, name:&str) -> Config {

        // Get the default configuration file for all simulatuons.  It
        // is in the root of the directory structure in file named
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
                for (k, v) in cfg.data.iter() {
                    config.data.insert(k.to_string(), v.to_string());
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


    fn make_menu(&self, menu_items:&Vec<&str>) {
        werase(self.menu_window);
        let mut x = 1; // Start of menu
        if menu_items.len() > 0 {
            for i in 1..menu_items.len()+1 {
                let s = menu_items.iter().nth(i-1).unwrap();
                if mvwprintw(self.menu_window, 1, x,
                             &format!("{} {}", i, &s)) != 0{
                    panic!("Failed mvwprint");
                }
                x = x + s.len() as i32 + 3;
            }
        }
        if mvwprintw(self.menu_window, 1, x, &format!("{} {}", 0, "Back")) != 0{
            panic!("Failed mvwprint");
        }
        redraw(self.menu_window);
        // let ret = wgetch(self.menu_window) as usize - 48;
        // eprintln!("make_menu Got {} ", ret);
        // ret
    }

    fn status_line(& self, msg:&str) {
        werase(self.status_window);
        if mvwprintw(self.status_window, 1, 1, msg) != 0{
            panic!("Failed mvwprint");
        }
        redraw(self.status_window);
        //eprintln!("status line: {}", msg);
    }

    fn do_analysis_window(&self, _pa:&PopulationAnalysis ){
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

