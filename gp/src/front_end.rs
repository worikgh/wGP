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
    
    fn default_config(&self, name:&str) -> Config {

        // Get the default configuration file for all simulatuons.  It
        // is iin the root of the directory structure in file named
        // ".gp_config".  Or not.  If not use a hard coded default
        let mut config = match File::open(format!("{}.gp_config", self.root_dir)){
            Ok(f) => Config::new_file(f),
            Err(_) => ConfigDefault::new(name),
        };

        // Update the configuration file.  The root_dir may not be
        // accurate as the file may have been moved, and the "name" is
        // particular to a simulation
        config.data.insert("root_dir".to_string(), self.root_dir.clone());
        config.data.insert("name".to_string(), name.to_string());
        config
    }
    
    fn do_project(& mut self, name:&str) {

        loop {
            werase(self.main_window);

            let status = self.controller.get_status(name);
            let projects:Vec<_> = vec!["Create", "Refresh Status"];        
            // pub cleared:bool,
            // pub running:bool,
            // pub generation:usize,
            // pub path:String, // FIXME This should be a reference
            if mvwprintw(self.main_window, 1, 1, &format!("Name: {}", name)) != 0 {
                panic!("Failed mvprint");
            }
            if mvwprintw(self.main_window, 2, 1, &format!("Cleared: {:?}", status.cleared)) != 0 {
                panic!("Failed mvprint");
            }
            if mvwprintw(self.main_window, 3, 1, &format!("Running: {:?}", status.running)) != 0 {
                panic!("Failed mvprint");
            }
            if mvwprintw(self.main_window, 4, 1, &format!("Generation: {:?}", status.generation)) != 0 {
                panic!("Failed mvprint");
            }
            if mvwprintw(self.main_window, 5, 1, &format!("Path: {}", status.path)) != 0 {
                panic!("Failed mvprint");
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
                    let status = match self.controller.run_simulation(config.copy()) {
                        Ok(_) => "Ok".to_string(), 
                        Err(s) => s.to_string(),
                    };
                    self.status_line(&status);
                },
                
                _ => (),
            }
            
            
        }
    }


    // fn edit_config(&self, config:&Config) -> Config {
    //     // This is waiting on developing technology to edit data 
        
    // }

    fn do_choose_object(&mut self){
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
            let p = e.unwrap().path();
            let md = p.metadata().expect("metadata call failed");
            if md.file_type().is_dir() {
                // Projects are in sub directories
                let sp = p.file_name().unwrap().to_str().unwrap().to_string();
                projects.push(sp);
            }
        }

        // Display the projects and wait for user to select one
        let menu_vec:Vec<_> = projects.iter().zip(1..(1+projects.len())).map(|x| format!("{} {}", x.0, x.1)).collect();
        
        loop {
            self.fe_main(&menu_vec);
            let c = self.make_menu(&vec!["Enter choice", "Display Config", "Quit"] );
            if c == 0 {
                break;
            }else if c <= menu_vec.len() {
                let project = menu_vec.iter().nth(c-1).unwrap().clone();
                self.status_line(&format!("Choose index {} project {}", c-1, &project));
                self.do_project(&project);
            }else if c == 0{
                break;
            }else{
                self.status_line(&format!("Option {} not valid", c));
            }
        }    
        
    }


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

    pub fn fe_start(&mut self) {
        loop {
            let c = self.make_menu(&vec!["Choose Object", "Display Config"] );
            match c {
                0 => break,
                1 => self.do_choose_object(),
                2 => self.do_display_config("<PROJECT NAME>"),
                _ => panic!(),
            }
        }
        fe_shut();
    }
    
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

    fn make_menu(&self, menu_items:&Vec<&str>) -> usize {
        werase(self.menu_window);
        //redraw(self.status_window);
        // eprintln!("make_menu");
        let mut x = 1; // Start of menu
        // werase(self.menu_window);
        for i in 1..menu_items.len()+1 {
            let s = menu_items.iter().nth(i-1).unwrap();
            if mvwprintw(self.menu_window, 1, x, &format!("{} {}", i, &s)) != 0{
                panic!("Failed mvwprint");
            }
            eprintln!("make_menu {}", format!("{} {}", x, s));
            x = x + s.len() as i32 + 3;
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

}

fn fe_shut(){
    /* Terminate ncurses. */
    endwin();    
}







fn redraw(win:WINDOW) {
    box_(win, 0, 0);
    wrefresh(win);
}    
    
