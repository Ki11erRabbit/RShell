mod process;
mod shell;
mod expr;
mod variable;
mod parser;

use std::process::exit;
use shell::Shell;
use std::path::Path;
use std::io::{self,Write};
use process::{run_in_forground,run_in_background};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::env;

fn main() {
    
    let mut shell = Shell::new(Path::new(""));
    let action = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
        unsafe {
            sigaction(Signal::SIGINT, &action).expect("failed to sigaction");
            sigaction(Signal::SIGQUIT, &action).expect("failed to sigaction");
            sigaction(Signal::SIGTSTP, &action).expect("failed to sigaction");
            sigaction(Signal::SIGTTIN, &action).expect("failed to sigaction");
            sigaction(Signal::SIGTTOU, &action).expect("failed to sigaction");
        }


    loop {
        let mut buffer = String::new();
        print!("tsh> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buffer)
            .expect("Failed to read line");

        eval(&buffer,&mut shell);
    }


}


fn eval(cmdline: &str, shell: &mut Shell) {
    let argv: Vec<String>;
    let bg: bool;
    let pair = parser::parseline(&cmdline);
    bg = pair.0;
    argv = pair.1;
    
    if builtin_cmd(&argv) == 1 {
        return;
    }
    let set = parser::parseargs(&argv);

    let cmds = set.0;
    let args = set.1;
    let env = set.2;
    let stdin_redir = set.3;
    let stdout_redir = set.4;

    let job = shell.create_job(cmdline, cmds, args, stdin_redir, stdout_redir);
    
    #[cfg(debug_assertions)]
    println!("Shell: {:?}", shell);

    (*job).borrow_mut().exec(shell);

    if !bg {
        run_in_forground(shell, &job, false);
    }
    else {
        run_in_background(shell, &job, false);
    }

}


fn builtin_cmd(argv: &Vec<String>) -> i32 {
    if argv.len() == 0 {
        return 1;
    } 
    match argv[0].as_str() {
        " " => return 1,
        "" => return 1,
        "quit" => exit(0),
        "exit" => exit(0),
        "cd" => {
            change_dir(argv);
            return 1;
        },
        _ => return 0,
    }
}

pub fn change_dir(argv: &Vec<String>) {
    let path;
    if argv.len() == 1 {
        let key = "HOME";
        match env::var(key) {
            Err(_) => {
                eprintln!("User's home not set!");
                return;
            }
            Ok(val) => {
                path = Path::new(&val);

                match env::set_current_dir(path) {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}",e),
                }

                return;

            }
        }
    }
    else {
        path = Path::new(&argv[2]);
    }

    match env::set_current_dir(path) {
        Ok(_) => (),
        Err(_) => eprintln!("cd: no such file or directory: {}",argv[2]),
    }
}
