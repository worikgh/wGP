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
}

impl FrontEnd {
    pub fn new () -> FrontEnd {

        // The root directory of the process.  FIXME This should be
        // passed in on the command line, optionally
        let root_dir = format!("{}/", env::current_dir().unwrap().to_str().unwrap().to_string());
        
        
        FrontEnd{
            //config:config,
            controller:Controller::new(root_dir.clone()),
            root_dir:root_dir,
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
    
    fn do_project_menu(&mut self, win:WINDOW, name:&str) {

        // For the project named in @param name, offer a menu of
        // things to do with it

        let status = self.controller.get_status(name);
        // Get the root directory of the projects
        let projects:Vec<_> = vec!["Create      ".to_string(), "Status".to_string()];
        let actions:Vec<_> = projects.iter().zip(0..projects.len()).collect();
        
        status_line(&format!("Cleared: {} Runnning: {} Path: {}", status.cleared, status.running, status.path));        
        loop {
            let c = make_menu(win, &actions, name);
            // Got a key.
            
            status_line(&format!("key: {}",c));
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
                    match self.controller.run_simulation(config.copy()) {
                        Ok(_) => (), 
                        Err(s) => status_line(&s.to_string()),
                    }
                },
                
                2 => {
                    self.do_project_status(win, name);
                }
                _ => (),
            }
        }
    }


    fn do_project_status(& mut self, win:WINDOW, name:&str){
        // Display the projects status

        werase(win);

        let status = self.controller.get_status(name);
        // pub cleared:bool,
        // pub running:bool,
        // pub generation:usize,
        // pub path:String, // FIXME This should be a reference
        if mvwprintw(win, 1, 1, &format!("Name: {}", name)) != 0 {
            panic!("Failed mvprint");
        }
        if mvwprintw(win, 2, 1, &format!("Cleared: {:?}", status.cleared)) != 0 {
            panic!("Failed mvprint");
        }
        if mvwprintw(win, 3, 1, &format!("Running: {:?}", status.running)) != 0 {
            panic!("Failed mvprint");
        }
        if mvwprintw(win, 4, 1, &format!("Generation: {:?}", status.generation)) != 0 {
            panic!("Failed mvprint");
        }
        if mvwprintw(win, 5, 1, &format!("Path: {}", status.path)) != 0 {
            panic!("Failed mvprint");
        }
        redraw(win);

        // FIXME  This should be a menu offering refresh or back
        wgetch(win);
    }
    
    fn edit_config(&self, win:WINDOW, config:&Config) -> Config {
        // This is waiting on developing technology to edit data 
        werase(win);
        
        config.copy()
    }
    
    fn do_choose_project(&mut self, win:WINDOW) {

        // Get the root directory of the projects
        let proj_dir = Path::new("./Data/");
        status_line(&format!("Current Directory {:?}", env::current_dir()));

        // Get the sub-directories that are projects
        let mut projects:Vec<String> = Vec::new();
        let entries = match fs::read_dir(proj_dir) {
            Ok(v) => v,
            Err(e) => {
                redraw(win);
                let s = format!("Failed reading {:?} e: {} cd: {:?}", proj_dir.to_str(), e, env::current_dir());
                display_model(win, &s);
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
        let menu_vec:Vec<_> = projects.iter().zip(0..projects.len()).collect();
        
        loop {
            let c = make_menu(win, &menu_vec, "TITLE");
            if c == 0 {
                break;
            }else if c <= menu_vec.len() {
                let project = menu_vec.iter().nth(c-1).unwrap().0;
                status_line(&format!("Choose index {} project {}", c-1, &project));
                self.do_project_menu(win, project);
            }else if c == 0{
                break;
            }else{
                status_line(&format!("Option {} not valid", c));
            }
        }    
    }

    
    fn fe_menu(&mut self) -> bool{

        // Draw the main menu.  Return false if quit is choosen

        /* Get the screen bounds. */
        let (max_x, max_y, x, y) = main_window_dims();

        // Make window FIXME do I have to?
        let win = newwin(max_y, max_x, y, x);    
        redraw(win);
        
        // Top left of main menu
        let start_x = max_x/10 as i32;
        let start_y = max_y/10 as i32;
        
        let msg = "Enter choice:";
        if mvwprintw(win, start_y, start_x, &msg) != 0 {
            panic!("Failed mvprint");
        }

        // The main menu
        let menu_items  = vec!(("Choose Project", 1), ("Display Config", 2), ("Quit", 0));
        for i in 0..menu_items.len() {
            let item = format!("{} {}", i, menu_items.iter().nth(i).unwrap().0);
            mvwprintw(win, start_y+i as i32 + 1 as i32, start_x, &item);
        }

        status_line(&"Waiting for menu choice".to_string());
        let c = wgetch(win) - 48;

        let mut ret = true;
        match c {
            0 => self.do_choose_project(win),
            1 => self.display_config(win, "<PROJECT>"),
            2 => ret = false,
            _ => status_line(&format!("Menu choice {}", c).to_string()),
        };
        destroy_win(win);
        ret
    }
    pub fn fe_start(&mut self) {

        initscr();
        raw();
        start_color();
        cbreak();
        noecho();
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        keypad(stdscr(), true);
        init_pair(1, COLOR_RED, COLOR_BLACK);
        
        
        loop {
            if self.fe_menu() == false {
                break;
            }
        }
        fe_shut();
    }
    
    fn display_config(& mut self, win:WINDOW, name:&str)  {

        let config = self.default_config(name);

        let mut keys = config.data.keys();
        eprintln!("display_config 4");

        let (max_x, max_y) = (getmaxx(win), getmaxy(win));
        // Top left of main menu
        let start_x = max_x/10 as i32;
        let start_y = max_y/10 as i32;
        werase(win);

        for y in 1..config.data.len() {
            if let Some(key) = keys.next() {
                let v = config.data.get(key).unwrap();
                let item = format!("{}:\t{}", key, v);
                eprintln!("y: {} x: {} max_x {}  max_y {} item {}", start_y+(1+y) as i32, start_x, max_x, max_y, item);
                if mvwprintw(win, start_y+1+y as i32, start_x as i32, &item) != 0 {
                    panic!(format!("Failed mvprint: y: {} x: {} max_x {}  max_y {}", start_y+(1+y) as i32, start_x, max_x, max_y));
                }
                redraw(win);
            }
        }
        redraw(win);
        // FIXME  Create a menu here!  0 => to go back
        wgetch(win);
        eprintln!("display_config 5");
    }
}

fn fe_shut(){
    /* Terminate ncurses. */
    endwin();    
}


fn display_model(win:WINDOW, msg:&String) {
    mvwprintw(win, 0,0,msg.as_str());
    redraw(win);
    wgetch(win);
}

fn make_menu(win:WINDOW, menu_items:&Vec<(&String, usize)>, title:&str) -> usize{

    // Return index of item selected.  Not bounded so caller must
    // check

    werase(win);

    let (max_x, max_y) = (COLS(), LINES());
    // Top left of main menu
    let start_x = max_x/10 as i32;
    let start_y = max_y/10 as i32;
    
    mvwprintw(win, start_y, start_x, title);
    mvwprintw(win, start_y+1, start_x, "Enter choice");
    for i in 0..menu_items.len() {
        let item = format!("{} {}", i+1, &menu_items.iter().nth(i).unwrap().0);
        mvwprintw(win, start_y+1+i as i32, start_x, &item);
    }

    // Add quit option
    let y = start_y + menu_items.len() as i32 + 2;
    mvwprintw(win, y, start_x, format!("{} {}", 0, "Back").as_str());
    redraw(win);

    let ret = wgetch(win);
    if ret < 48 {
        0
    }else{
        (ret - 48) as usize
    }
}


fn status_line(msg:&String) {
    let win = newwin(3, COLS(), LINES()-3, 0);    
    box_(win, 0, 0);
    redraw(win);
    let msg = format!("{} {:?}", msg, env::current_dir());
    if mvwprintw(win, 1, 1, &msg) != 0{
        panic!("Failed mvwprint");
    }
    redraw(win);
    //clrtoeol();
}


fn redraw(win:WINDOW) {
    box_(win, 0, 0);
    wrefresh(win);
}    
    
fn main_window_dims() -> (i32, i32, i32, i32) {
    (COLS()-4, LINES()-6, 2, 2)
}


fn destroy_win(win: WINDOW) {
    let ch = ' ' as chtype;
    wborder(win, ch, ch, ch, ch, ch, ch, ch, ch);
    wrefresh(win);
    delwin(win);
}
