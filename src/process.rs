use nix::unistd::Pid;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid,WaitPidFlag,WaitStatus};
use std::borrow::BorrowMut;
use std::process::{self,Command, Stdio, Child};
use std::os::unix::process::CommandExt;
use crate::shell::Shell;
use std::fs::{File,OpenOptions};
use std::cell::RefCell;
use std::rc::Rc;
use std::hash::{Hasher,Hash};

fn kill_process_group(pgid: Pid, signal: Signal) -> Result<(), nix::errno::Errno>  {
    kill(Pid::from_raw(-pgid.as_raw()),signal)
}


#[derive(Debug,PartialEq,Clone,Copy,Hash)]
pub enum ProcessStatus {
    Running,
    Exited(i32),
    Stopped,
    Undef
}

#[derive(Debug,PartialEq,Clone,Hash)]
pub enum Redirection {
    Normal,
    Pipe,
    File((String,bool)),
    Redir(String)
}

#[derive(Debug)]
pub struct Process {
    pub cmd: String,
    pub args: Vec<String>,
    pub stdin_redir: Redirection,
    pub stdout_redir: Redirection,
    pub status: ProcessStatus,
    pub process: Option<Child>,
}

impl Process {
    pub fn new(cmd: String, args: Vec<String>, stdin_redir: Redirection, stdout_redir: Redirection) -> Process {
        Process {
        cmd: cmd,
        args: args,
        stdin_redir: stdin_redir,
        stdout_redir: stdout_redir,
        status: ProcessStatus::Undef,
        process: None
        }
    }

    #[inline]
    pub fn pid(&self) -> Pid {
        Pid::from_raw(self.process.as_ref().expect("Process not set yet").id().try_into().unwrap())
    }

    pub fn set_process(&mut self, process: Child) {
        self.process = Some(process);
    }
}

impl Hash for Process {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cmd.hash(state);
        self.args.hash(state);
        self.stdin_redir.hash(state);
        self.stdout_redir.hash(state);
        self.stdout_redir.hash(state);
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Process) -> bool {
        self.cmd == other.cmd && self.args == other.args && self.stdin_redir == other.stdout_redir
    }
    
    fn ne(&self, other: &Process) -> bool {
        self.cmd != other.cmd && self.args != other.args && self.stdin_redir != other.stdout_redir
    }
}

#[derive(Debug)]
pub struct Job {
    id: u32,
    pub state: ProcessStatus,
    pipeline: String,
    pub processes: Vec<Process>,
    pgid: Pid,

}

impl Job {
    pub fn new(id: u32, pipeline: &str) -> Job {
        Job {
            id,
            state: ProcessStatus::Undef,
            pipeline: pipeline.to_string(),
            processes: Vec::new(),
            pgid: Pid::from_raw(0),
        }
    }

    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn pgid(&self) -> Pid {
        self.pgid
    }

    #[inline]
    pub fn pipeline(&self) ->&str {
        self.pipeline.as_str()
    }

    #[inline]
    pub fn completed(&self) -> bool {
        match self.state {
            ProcessStatus::Exited(_) => return true,
            _ => return false,
        }
    }
    
    #[inline]
    pub fn stopped(&self) -> bool {
        match self.state {
            ProcessStatus::Stopped => return true,
            _ => return false,
        }
    }

    pub fn add_process(&mut self, process: Process) {
        self.processes.push(process);
    }
    

    pub fn update_process_state(&mut self, pid: Pid, state: ProcessStatus) {
        for process in self.processes.iter_mut() {
            if process.pid() == pid {
                process.status = state;
                break;
            }
        }
        
        #[cfg(debug_assertions)]
        println!("state {:?}\n",state);
        
        match &state {
            ProcessStatus::Stopped => {
                if self.processes.iter().all(|process| matches!(process.status,ProcessStatus::Stopped )) {
                    self.state = state;
                }

            },
            ProcessStatus::Exited(_) => {
                if self.processes.iter().all(|process| matches!(process.status,ProcessStatus::Exited(_))) {
                    self.state = state;
                }
                
            },
            _ => return,
        }



/*        match &state {
            ProcessStatus::Exited(_) | ProcessStatus::Stopped => {
                for process in self.processes.iter_mut() {
                    match process.status {
                        ProcessStatus::Exited(_) => continue,
                        ProcessStatus::Stopped => continue,
                        _ => return,
                    }
                }
            },
            _ => return,
        }*/

        //self.state = state;

        #[cfg(debug_assertions)]
        println!("job state {:?}\n",self.state);
    }


    pub fn exec(&mut self, shell: &mut Shell) {
       
        let mut group_id = 0;
        for i in 0..self.processes.len() {
            let mut command = &mut Command::new(self.processes[i].cmd.as_str());
            if shell.interactive() {
                command = command.process_group(group_id);
            }
            command = command.args(self.processes[i].args.as_slice());

            match &self.processes[i].stdout_redir {
                Redirection::Pipe => {
                    command = command.stdout(Stdio::piped());
                },
                Redirection::File((file_name,append)) => {
                    let file;
                    if *append {
                        file = OpenOptions::new()
                            .write(true)
                            .append(*append)
                            .open(file_name.as_str())
                            .expect("Bad file path");
                    }
                    else {
                        file = File::create(file_name).expect("Bad file path");
                    }

                    command = command.stdout(file);
                },
                _ => (),
            }

            match &self.processes[i].stdin_redir {
                Redirection::Pipe => {
                    command = command.stdin(self.processes[i-1].process.as_mut().unwrap().stdout.take().unwrap()); 
                },
                Redirection::File((file_name,_)) => {
                    match File::open(file_name.as_str()) {
                        Ok(file) => command = command.stdin(file),
                        Err(_) => {
                            eprintln!("Bad file path");
                        }
                    }
                },
                _ => (),
            }

            match command.spawn() {
                Ok(proc) => {
                    self.processes[i].set_process(proc);
                    self.processes[i].status = ProcessStatus::Running;
                },
                Err(_) => {
                    eprintln!("{}: Command not found", self.processes[i].cmd);
                }
            }
            
            if i == 0 && shell.interactive() {
                self.pgid = self.processes[0].pid();
                group_id = self.processes[0].process.as_ref().expect("Child not yet initialized").id().try_into().unwrap();
            }
        }
        self.state = ProcessStatus::Running;
    }
    
}

impl Drop for Job {
    fn drop(&mut self) {
        //println!("[{}] ({}) Done: {}",self.id,self.pgid,self.pipeline);
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Job) -> bool {
        self.id == other.id
    }

    fn ne(&self, other: &Job) -> bool {
        self.id != other.id
    }

}

impl Hash for Job {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.pipeline.hash(state);
        self.processes.hash(state);
        self.pgid.hash(state);
    }

}

pub fn continue_job(shell: &mut Shell, job: &Rc<RefCell<Job>>, background: bool) {
    

    for proc in (**job).borrow_mut().processes.iter_mut() {
        match proc.status {
            ProcessStatus::Stopped => {
                proc.status = ProcessStatus::Running;
            },
            _ => {},
        }
    }
    (**job).borrow_mut().state = ProcessStatus::Running;

    if background {
        run_in_background(shell, job, true);
    }
    else {
        run_in_background(shell, job, true)
    }
}

pub fn run_in_forground(shell: &mut Shell, job: &Rc<RefCell<Job>>, sigcont: bool) -> ProcessStatus {
    shell.remove_background_job(job);


    if sigcont {
        kill_process_group(job.borrow().pgid(), Signal::SIGCONT).expect("failed to kill(SIGCONT)");
        
    }
    
    let status = wait_for_job(shell, job);

    status    
}

pub fn run_in_background(shell: &mut Shell, job: &Rc<RefCell<Job>>, sigcont: bool) {
    
    shell.add_background_job(job);

    if sigcont {
        kill_process_group(job.borrow().pgid(), Signal::SIGCONT).expect("failed to kill(SIGCONT)");
    }
}

pub fn delete_job(shell: &mut Shell, job: &Rc<RefCell<Job>>) {
    #[cfg(debug_assertions)]
    println!("Delete Job");

    if shell.remove_background_job(job) {
        println!("[{}] ({}) Done: {}",job.borrow().id(),job.borrow().pgid(),job.borrow().pipeline());
    }
    
    #[cfg(debug_assertions)]
    println!("shell: {:?}\n",shell);

    shell.remove_job(job.borrow().id()).expect("Job not in jobs Map");
    
}


pub fn wait_for_job(shell: &mut Shell, job: &Rc<RefCell<Job>>) -> ProcessStatus {
    
    loop {
        //let mut job = job.borrow_mut();
        if job.borrow().completed() || job.borrow().stopped() {
            break;
        }

        wait_for_process(job,true);
    }

    let state: ProcessStatus = job.borrow().state;

    match state {
        ProcessStatus::Exited(_) => {
            delete_job(shell, &job);
            return state;
        }
        ProcessStatus::Stopped => {
            println!("Job [{}] ({}) stopped {}",job.borrow().id(),job.borrow().pgid, job.borrow().pipeline());
            return state;
        },
        _ => unreachable!(),
    }
}

pub fn wait_for_process(job: &Rc<RefCell<Job>>, block: bool) -> Option<Pid> {
    let options = if block {
        WaitPidFlag::WUNTRACED
    }
    else {
        WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG
    };

    let result = waitpid(None,Some(options));

    let (pid, state) = match result {
        Ok(WaitStatus::Exited(pid, status)) => {
            (pid,ProcessStatus::Exited(status))
        },
        Ok(WaitStatus::Signaled(pid, signal, _)) => {
            (pid, ProcessStatus::Exited(-1))
        }
        Ok(WaitStatus::Stopped(pid,signal)) => {
            (pid, ProcessStatus::Stopped)
        },
        Err(nix::errno::Errno::ECHILD) | Ok(WaitStatus::StillAlive) => {
            return None;
        }
        status => {
            panic!("Unexpected waitpid event: {:?}",status);
        }
    };
    let job = &**job;
    let mut job = job.borrow_mut();
    job.update_process_state(pid, state);

    Some(pid)
}
