//use data::Data;
//use std::fs::File;
//use std::io::prelude::*;
//use std::sync::mpsc;
use config::Config;
use ncurses::*;
use population::Population;
use population::PopulationStatus;
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, Arc};
use std::thread;

pub struct FrontEnd {
    handles:HashMap<String, (Arc<Mutex<PopulationStatus>>, thread::JoinHandle<()>)>,
    root_dir:String,
}

impl FrontEnd {
    pub fn new () -> FrontEnd {
        FrontEnd{handles:HashMap::new(),
                 root_dir:env::current_dir().unwrap().to_str().unwrap().to_string(),
        }
    }
    fn run_simulation(&mut self, name:&str) -> Result<usize, &str> {

        // Check not already running
        eprintln!("run_simulation");
        let running = match self.handles.entry(name.to_string()) {
            Occupied(o) => {
                let o1 = o.get();
                let ps = &*(o1.0).lock().unwrap();
                if ps.running {
                    eprintln!("Not Running");
                    true
                }else{
                    eprintln!("Not Running");
                    false
                }
            },
            Vacant(_) => {
                eprintln!("Not Running");
                false
            }
        };
        if !running {

            // 

            // Get configuration data
            let cfg_fname = format!("Data/{}/.gp_config", name);
            let config = match Path::new(cfg_fname.as_str()).exists() {
                true => {
                    eprintln!("Got config");
                    // Read data from config
                    Config::new(cfg_fname.as_str())
                },
                false => {
                    // Generate a default config
                    let mut data:HashMap<String, String> = HashMap::new();
                    let mut work_dir = self.root_dir.clone();
                    work_dir.push_str("/Data/");
                    work_dir.push_str(name);
                    work_dir.push('/');
                    data.insert("work_dir".to_string(), work_dir.clone());
                    data.insert("root_dir".to_string(), self.root_dir.clone());
                    data.insert("name".to_string(), name.to_string());
                    data.insert("birthsanddeaths_file".to_string(), format!("{}{}_BnD.txt", work_dir, name).to_string());
                    data.insert("classification_file".to_string(), format!("{}{}_Classes.txt", work_dir, name).to_string());
                    data.insert("copy_prob".to_string(), "50".to_string());
                    data.insert("crossover_percent".to_string(), "50".to_string());
                    data.insert("data_file".to_string(), "data.in".to_string());
                    data.insert("filter".to_string(), "1".to_string());
                    data.insert("generations_file".to_string(), format!("{}{}_Generations.txt", work_dir, name).to_string());
                    data.insert("max_population".to_string(), "10".to_string());
                    data.insert("mode".to_string(), "Create".to_string());
                    data.insert("model_data_file".to_string(), format!("{}{}.txt", work_dir, name).to_string());
                    data.insert("mutate_prob".to_string(), "1".to_string());
                    data.insert("num_generations".to_string(), "40".to_string());
                    data.insert("rescore".to_string(), "0".to_string());
                    data.insert("save_file".to_string(), format!("{}{}_Trees.txt", work_dir, name).to_string());
                    data.insert("seed".to_string(), "11 2 3 120".to_string());
                    data.insert("training_percent".to_string(), "10".to_string());
                    
                    Config{data:data}
                }
            };

            
            // Create the shared memory to monitor and control simulation
            let mut pb = config.get_string("root_dir").unwrap();
            pb.push_str("/Data/");
            pb.push_str(name);
            let bb = Arc::new(Mutex::new(PopulationStatus{running:false, cleared:true, generation:0, path:pb}));
            let h = Population::new_sub_thread(config, bb.clone());
            status_line(&format!("Started thread id {:?}", h.thread().id()));
            self.handles.insert(String::from(name), (bb, h));
            Ok(0)
        }else{
            Err("Running already")
        }
    }
    fn do_project_menu(&mut self, win:WINDOW, project:&String) {
        // Get the root directory of the projects


        let projects:Vec<_> = vec!["Create It".to_string(), "Resume".to_string(),"Run".to_string(),"Configure".to_string()];
        let actions:Vec<_> = projects.iter().zip(0..projects.len()).collect();
        
        loop {
            let c = make_menu(win, &actions, project.as_str());
            status_line(&format!("key: {}",c));
            if c == 0 {
                break;
            }else if c == 1 {
                // Create
                match self.run_simulation(project) {
                    Ok(_) => (), //status_line(&"Set running".to_string()),
                    Err(s) => status_line(&s.to_string()),
                }
            }
        }
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
    fn main_menu(&mut self) -> bool{

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
        let menu_items = vec!(("Choose Project", 1), ("Display Config", 2), ("Quit", 0));
        for i in 0..menu_items.len() {
            let item = format!("{} {}", i, menu_items.iter().nth(i).unwrap().0);
            mvwprintw(win, start_y+i as i32 + 1 as i32, start_x, &item);
        }

        status_line(&"Waiting for menu choice".to_string());
        let c = wgetch(win) - 48;

        let mut ret = true;
        match c {
            0 => self.do_choose_project(win),
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
            if self.main_menu() == false {
                break;
            }
        }
        fe_shut();
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
        eprintln!("make_menu item: {}", item);
        mvwprintw(win, start_y+1+i as i32, start_x, &item);
    }

    // Add quit option
    let y = start_y + menu_items.len() as i32 + 2;
    mvwprintw(win, y, start_x, format!("{} {}", 0, "Back").as_str());
    redraw(win);

    wgetch(win) as usize  - 48 
    //                     'a' 
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

fn display_config() -> bool {
    true
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

