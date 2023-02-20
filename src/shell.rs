use crate::process::{Job,Redirection,Process};
use crate::variable::Variables;
use std::collections::{HashMap,HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use nix::unistd::{getpid,Pid};
use std::path::Path;
use std::env;
use core::hash::{Hasher, Hash};


#[derive(Debug)]
struct JobWrapper(Rc<RefCell<Job>>);

impl Hash for JobWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let job = self.0.borrow();
        let job = &*job;
        job.hash(state);
    }

}

impl PartialEq for JobWrapper {
    fn eq(&self, other: &Self) -> bool {
        let job1 = self.0.borrow();
        let job1 = &*job1;
        let job2 = other.0.borrow();
        let job2 = &*job2;

        PartialEq::eq(job1,job2)
    }
}

impl Eq for JobWrapper {}




#[derive(Debug)]
pub struct Shell {
    pub pgid: Pid,
    script_name: String,
    interactive: bool,
    history: Vec<String>,//vec for now
    last_status: i32,
    last_job: Option<Rc<Job>>,
    global_values: Variables,
    local_values: Vec<Variables>,
    aliases: HashMap<String,(String,Vec<String>)>,
    jobs: HashMap<u32, Rc<RefCell<Job>>>,
    bg_jobs: HashSet<JobWrapper>,
    next_job_id: u32,
}


impl Shell {
    pub fn new(history_path: &Path) -> Shell {
        Shell {
            pgid: getpid(),
            script_name: "".to_owned(),
            interactive: false,
            history: Vec::new(),//for now
            last_status: 0,
            last_job: None,
            global_values: Variables::new(),
            local_values: Vec::new(),
            aliases: HashMap::new(),
            jobs: HashMap::new(),
            bg_jobs: HashSet::new(),
            next_job_id: 1,
        }
    }

    pub fn find_next_job_id(&mut self) {
        
    }

    pub fn set_interactive(&mut self,interactive: bool) {
        self.interactive = interactive;
    }

    #[inline]
    pub fn interactive(&self) -> bool {
        self.interactive
    }

    #[inline]
    pub fn add_background_job(&mut self,job: &Rc<RefCell<Job>>) {
        self.bg_jobs.insert(JobWrapper(job.clone()));
    }

    pub fn remove_background_job(&mut self, job: &Rc<RefCell<Job>>) -> bool {
        self.bg_jobs.remove(&JobWrapper(job.clone()))
    }


    pub fn export(self, key: &str, value: &str) {
        env::set_var(key,value);
    }

    pub fn set_alias(&mut self, key: &str, value: (String,Vec<String>)) {
        self.aliases.insert(key.to_owned(), value);
    }   

    pub fn lookup_alias(&self, alias: &str) -> Option<(String,Vec<String>)> {
        self.aliases.get(&alias.to_string()).cloned()
    } 

    pub fn remove_job(&mut self, job_id: u32) -> Option<Rc<RefCell<Job>>> {
        #[cfg(debug_assertions)]
        println!("shell jobs: {:?}\n",self.jobs);
        self.jobs.remove(&job_id)
    }

    pub fn create_job(&mut self, cmdline: &str, cmds: Vec<String>, args: Vec<Vec<String>>,stdin_redir: Vec<Redirection>, stdout_redir: Vec<Redirection>) -> Rc<RefCell<Job>> {
        let job = Rc::new(RefCell::new(Job::new(self.next_job_id,cmdline)));

        self.jobs.insert(self.next_job_id,job.clone());
        self.next_job_id += 1;
        
        for i in 0..cmds.len() {
            job.borrow_mut().add_process(Process::new(cmds[i].clone(),args[i].clone(),stdin_redir[i].clone(),stdout_redir[i].clone()))
        }


        job
    }

    
}
