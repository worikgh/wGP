use config::Config;
use config_default::ConfigDefault;
use ncurses::*;
//use population::PopulationAnalysis;
use population::Population;
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
    Invalid, // Returned from FrontEnd::current_state if there is no state
}
pub struct FrontEnd {

    // The directory structure of simulations is constant belo this
    // root directory
    root_dir:String,

    main_window:WINDOW,
    menu_window:WINDOW,
    status_window:WINDOW,

    // Window height and position
    #[allow(dead_code)]
    menu_h:i32, main_h:i32, //status_h:i32,
    //menu_y:i32,
    main_y:i32, //status_y:i32,


    state:Vec<State>,
    inp:Option<usize>, // Input from user.  A key

    // Map project names to menu index.  Initialised by reading the
    // contents of Data/ Each subdirectory is a project.  FIXME Write
    // a process to check that a directory has everything it needs
    projects:Option<BTreeMap<usize, String>>, 

    project:Option<String>, // Current project

    // The set of all programme trees
    population:Population, 
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
            root_dir:root_dir.clone(),

            main_window:main_window,
            menu_window:menu_window,
            status_window:status_window,

            state:vec![State::Stopped],
            inp:None,
            menu_h:menu_h, main_h:main_h, //status_h:status_h,
            //menu_y:menu_y,
            main_y:main_y, //status_y:status_y,
            projects:None,
            project:None,

            population:Population::new(&ConfigDefault::population(root_dir.as_str())),
        }
    }
    fn current_state(&self) -> State {
        // Assumes there is allays atleast one state.
        //FIXME On exit this assumption fails
        if self.state.len() > 0 {
            self.state.iter().last().unwrap().clone()
        }else{
            State::Invalid
        }
    }
    // Functions to use ncurses top draw on screen
    fn draw_main_splash(&self) {
        // When there is nothing to see on the main screen...

        let l = self.main_y+self.main_h - 4; // FIXME  Why '- 4'? (-3 if no kanji)
        let c = COLS() - 2; // FIXME why `- 2`? (-1 if no kanji)
        let x = rng::random::<usize>() % c as usize;
        let y = rng::random::<usize>() % l as usize;
        let chars = ["大","赛","列","表","页","@","#","$","%","^","&","*","(",")","_","+","=","-",":",";","<",">","?","/"];
        let ind = rng::random::<usize>() % chars.len();
        let c = chars[ind];
        let r =  mvwprintw(self.main_window, y as i32, x as i32, c);
        if r != 0 {
            panic!("Failed mvwprintw r {}  w {:?} x {} y {} l {} c {}", r, self.main_window, x, y, l, c);
        }
        redraw(self.main_window);
    }


    fn display_project(&mut self, project:&str) {

        // Display project status

        let x_status = 1; // Start column for status
        let mut y_status = 1; // Status lines
        
        werase(self.main_window);

        // ======================
        // Status
        
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



        match self.population.status(project) {
            Err(err) =>
            // Simulation does not exist, yet, so cannot display status or analysis
            {
                if mvwprintw(self.main_window, y_status, x_status,
                             &format!("{}", err)) != 0 {
                    panic!("Failed mvprint");
                }},
            Ok(status) => {

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


                //  End of status display
                //=======================
                //y_anal = y_anal+1;

                // Analysis display
                let x_anal = 30;  // Column for display
                let mut y_anal = 1;  // Each line

                match status.analysis {
                    None => {
                        if wattron(self.main_window, A_UNDERLINE()) != 0 {
                            panic!("Failed wattron");
                        }
                        if mvwprintw(self.main_window, y_anal, x_anal, "No Analysis") != 0 {
                            panic!("Failed mvwprintw");
                        }
                        if wattroff(self.main_window, A_UNDERLINE()) != 0 {
                            panic!("Failed wattroff");                
                        }
                    },
                    Some(pa) => {
                        // A analysis ready

                        // Heading
                        if wattron(self.main_window, A_UNDERLINE()) != 0 {
                            panic!("Failed wattron");
                        }
                        if mvwprintw(self.main_window, y_anal, x_anal,
                                     format!("Analysis Gen: {}", pa.generation).as_str()) != 0 {
                            panic!("Failed mvwprintw");
                        }
                        if wattroff(self.main_window, A_UNDERLINE()) != 0 {
                            panic!("Failed wattroff");                
                        }
                        y_anal = y_anal+2; // Leave a blank line

                        // Summary of results
                        if mvwprintw(self.main_window, y_anal, x_anal,
                                     format!("Q: {:.2}%", 100.0 * pa.correct as f64/pa.cases as f64).as_str()) != 0 {
                            panic!("Failed mvwprintw");
                        }
                        y_anal = y_anal+1;
                        
                        if mvwprintw(self.main_window, y_anal, x_anal,
                                     format!("Cases {} Incorrect: {} Correct: {} Unclassified: {}",
                                             pa.cases, pa.incorrect, pa.correct,
                                             pa.cases - pa.classified).as_str()) != 0 {
                            panic!("Failed mvwprintw");
                        }

                    },
                };
            },
        };
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
        self.make_menu(&vec!["Create", "Analyse", "Refresh (Delete forest)", "Utilise"]);
        if let Some(ref p) = self.project.clone() {
            self.display_project(p.as_str());
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
        match state {
            State::Invalid => panic!("State: {:?}",state),
            State::Starting => self.start_screen(),
            State::Stopped => true,
            State::DisplayConfig => true,
            State::GotProject => self.project_screen(),
            State::ChooseProject => {
                self.projects = Some(self.read_projects());
                self.choose_project_screen()
            },
        }
    }

    fn state_transition(& mut self) -> bool{
        // Return false to exit programme

        let state = self.current_state();

        // Check for input of 0.  Always means go back a screen/state
        if let Some(key) = self.inp {
            if key == 0 {
                // Going back a state...
                self.inp = None; // Consume

                self.state.pop(); // Going back a state

                return if self.state.len() == 0 {
                    // No states left
                    false
                }else{

                    // FIXME This is turning into spaghetti code.
                    // Must be a better way to manage this
                    if  self.current_state() == State::ChooseProject {
                        // Reset current project
                        self.project = None;
                    }
                    
                    // Hack to make the splash screen look good.  It does
                    // not clear itself each time draw_main_splash is
                    // called (it builds up a picture, random as I write),
                    // so needs to be cleared before it is displayed
                    if self.current_state() == State::Starting {
                        werase(self.main_window);
                    }

                    true
                }
            }
        }
        
        // Key is not 0
        let ret = match state {
            State::Invalid => panic!("State: {:?}",state),
            State::Starting => {

                // If there is some key board input process it
                if let Some(key) = self.inp {
                    if let Some(s) = self.handle_key(key, state) {
                        self.state.push(s);
                    }
                    self.inp = None; // Consume
                }
                true
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
                    if let Some(s) = self.handle_key(key, state) {
                        self.project = Some(self.projects.as_ref().unwrap().get(&key).unwrap().clone());
                        self.state.push(s);
                    }else{
                        // A key that is unrecognised
                        eprintln!("state_transition called in ChooseProject. Unrecognised key {}", key);
                    }
                    self.inp = None; // Consume
                }else{
                    panic!("state_transition called in ChooseProject state with no input");
                }
                true
            },
        };
        ret
    }

    // From current state and key that is pressed do the action for
    // this state and determine the next state
    fn handle_key(& mut self, key:usize, state:State) -> Option<State>{
        // Handle a keypress.  FIXME `key` i the raw ASCII(?) minus 48.  Works for 0-9
        match state {
            State::Invalid => panic!("State: {:?}",state),
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
                match key {
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
                        let config = self.default_config(name.as_str());

                        // Create the simulation.  
                        match self.population.create(name, &config) {
                            Err(err) => self.status_line(err.as_str()),
                            Ok(_) => {
                                // Run the simulation.  If it
                                // fails to launch the status is
                                // set appropriately
                                let status = match self.population.start(name) {
                                    Ok(_) => format!("{} started", name),
                                    Err(s) => s.to_string(),
                                };
                                self.status_line(status.as_str());
                            },
                        }
                        None
                    },
                    2 => {
                        // Do a Analysis
                        match self.project {
                            None => self.status_line(&format!("No project")),// FIXME Panic?
                            Some(ref p) => self.population.analyse(p),
                        };
                        None
                    },
                    3 => {
                        // Delete 
                        match self.project {
                            None => self.status_line("No project to delete"),
                            Some(ref p) => {
                                match self.population.delete(p.as_str()){
                                    Ok(_) => (),
                                    Err(err) => eprintln!("Failed to delete project: {}  Err: {}", p, err),
                                };
                            },
                        };
                        self.project = None;
                        None
                    },
                    _ => None,
                }
            },
            State::Stopped => None,
            State::DisplayConfig => None,
        }
    }
    // End of state transition functions.

    pub fn fe_start(& mut self) {
        // Entry point
        self.state = vec![State::Starting];
        // Event loop of state machine
        loop {

            // Make sure what we display matches the state
            self.update_display();
            let inp = wgetch(self.menu_window);

            if inp == ERR {
                // No input available.  Need to sleep
                sleep(Duration::new(0, 50000000)); // Fastest is 2 times a second
                continue;
            }else{

                if inp > 47 && inp < 58 {
                    // FIXME This is apalling! But currently only
                    // numbers are used as input
                    let inp = inp  as usize - 48;
                    self.inp = Some(inp);
                    if self.state_transition() == false{
                        break;
                    }
                }else{
                    eprintln!("Invalid input code: {}", inp);
                }
            }
        }
        fe_shut();
    }


    #[allow(dead_code)]
    fn do_display_config(&self, name:&str)  {

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




    // End of rational ordering of functions....
    //---------------------------------------------------------

    fn default_config(&self, name:&str) -> Config {

        // Get the default configuration file for all simulatuons.  It
        // is in the root of the directory structure in file named
        // ".gp_config".  Or not.  If not use a hard coded default
        let mut config =
            match File::open(format!("{}.gp_config", self.root_dir)){
                Ok(f) => Config::new_file(f),
                Err(_) => ConfigDefault::population(self.root_dir.as_str()),
            };
        // Check if there is a specific configuration for this project
        let project_config =
            match File::open(format!("{}/Data/{}/.gp_config", self.root_dir, name)){
                Ok(f) => Config::new_file(f),
                Err(_) => ConfigDefault::project(name),
            };
        for (k, v) in project_config.data.iter() {
            config.data.insert(k.to_string(), v.to_string());
        }
        config
    }

    // fn edit_config(&self, config:&Config) -> Config {
    //     // This is waiting on developing technology to edit data

    // }



    #[allow(dead_code)]
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
    }

    fn status_line(& self, msg:&str) {
        werase(self.status_window);
        if mvwprintw(self.status_window, 1, 1, msg) != 0{
            panic!("Failed mvwprint");
        }
        redraw(self.status_window);
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

