use crate::process::Redirection;
use std::env;



pub fn parseline(cmdline: &str) -> (bool,Vec<String>) {
    let mut argv: Vec<String> = Vec::new();
    let bg: bool;
    let mut append: bool = false;
    let mut array = cmdline.to_string(); 
    /*if array.contains("\n") {
        array.pop();
    }*/ 
    //array.push(' ');
    let result = cmdline.rfind("&");
    if result != None && cmdline.get(result.unwrap()-1..=result.unwrap()) != Some("&&"){ 
        bg = true;
    }
    else {
        bg = false;
    }

    while array.len() != 0 {
        //println!("{}",array);
        //println!("array len: {}", array.len());
        match array.get(..1).unwrap() {
            "'" => {
                        let mut temp: String = array.drain(..1).collect();
                        //println!("{}!", array);
                        let temp2: String = array.drain(..array.find('\'').unwrap()+1).collect();
                        temp += &temp2;

                        argv.push(temp);
                   },
            " " => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "|" => {
                        //println!("Space");
                        match array.get(..2) {
                            Some("||") => {
                                argv.push(array.drain(..2).collect());
                            },
                            _ => {
                                argv.push(array.drain(..1).collect());
                            },
                        }
                   },
            "<" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            ">" => {
                        //println!("Space");
                        match array.get(..2) {
                            Some(">>") => {
                                append = true;
                                argv.push(array.drain(..2).collect());
                            },
                            _ => {
                                argv.push(array.drain(..1).collect());
                            },
                        }
                   },
            "=" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "&" => {
                        //println!("Space");
                        match array.get(..2) {
                            Some("&&") => {
                                argv.push(array.drain(..2).collect());
                            },
                            _ => {
                                array.drain(..1);
                            },
                        }
                   },
            "\n" => {
                        //println!("Space");
                        array.drain(..1);
                   },
            _ => {
                        //println!("Default");
                        argv.push(array.drain(0..array.find(|c: char| c == '>' || c == '|' || c == '<' || c == ' ' || c == '=' || c == '\n').unwrap()).collect());

                 } 
        }

        //println!("{:?}",argv);

    }


    return (bg,argv);
}

pub fn parseargs(argv: &Vec<String>) -> (Vec<String>,Vec<Vec<String>>,Vec<(String,String)>,Vec<Redirection>,Vec<Redirection>) {
    let mut cmds: Vec<String> = Vec::new();
    let mut args: Vec<Vec<String>> = Vec::new();
    let mut env: Vec<(String,String)> = Vec::new();
    let mut stdin_redir: Vec<Redirection> = Vec::new();
    let mut stdout_redir: Vec<Redirection> = Vec::new();

    let mut curr_cmd = 0;
    cmds.push("".to_string());
    args.push(Vec::new());
    stdin_redir.push(Redirection::Normal);
    stdout_redir.push(Redirection::Normal);

    let mut skip = false;
    for i in 0..argv.len() {
        match argv[i].as_str() {
            "|" => {
                    stdout_redir[curr_cmd] = Redirection::Pipe;
                    stdin_redir.push(Redirection::Pipe);
                    stdout_redir.push(Redirection::Normal);
                    cmds.push("".to_string());
                    args.push(Vec::new());
                    curr_cmd += 1;
                },
            "<" => {
                    //stdin_redir[curr_cmd] = args[curr_cmd].len();
                    stdin_redir[curr_cmd] = Redirection::File((argv[i+2].clone(),false));
                    skip = true;
                },
            ">" => {
                    //stdout_redir[curr_cmd] = args[curr_cmd].len();
                    stdout_redir[curr_cmd] = Redirection::File((argv[i+2].clone(),false));
                    skip = true;
                },
            ">>" => {
                    //stdout_redir[curr_cmd] = args[curr_cmd].len();
                    stdout_redir[curr_cmd] = Redirection::File((argv[i + 2].clone(), false));
                    skip = true;
                },
            "=" => {
                    skip = true;
                },
            " " => {
                    
                },
            "&&" => {
                    stdout_redir[curr_cmd] = Redirection::Normal;
                    stdin_redir.push(Redirection::Normal);
                    stdout_redir.push(Redirection::Normal);
                    stdin_redir.push(Redirection::Normal);
                    stdout_redir.push(Redirection::Normal);
                    cmds.push("&&".to_string());
                    args.push(Vec::new());
                    cmds.push("".to_string());
                    args.push(Vec::new());
                    curr_cmd += 2;
                },
            "||" => {
                    stdout_redir[curr_cmd] = Redirection::Normal;
                    stdin_redir.push(Redirection::Normal);
                    stdout_redir.push(Redirection::Normal);
                    stdin_redir.push(Redirection::Normal);
                    stdout_redir.push(Redirection::Normal);
                    cmds.push("&&".to_string());
                    args.push(Vec::new());
                    cmds.push("".to_string());
                    args.push(Vec::new());
                    curr_cmd += 2;
                },
            _ => {
                    if skip {
                        skip = false;
                        continue;
                    }

                    if i + 2 < argv.len() && argv[i+1].as_str() == "=" {
                        let val;
                        if argv[i+2].contains("'") {
                            val = argv[i+2].clone().drain(1..argv[i].len()-1).collect();
                        }
                        else {
                            val = argv[i+2].clone();
                        }
                        env.push((argv[i].clone(),val));
                        skip = true;
                        continue;
                    }

                    if cmds[curr_cmd].as_str() == "" {
                        let cmd;

                        if argv[i].get(..1) == Some("$") {
                            match env::var(argv[i].clone().drain(1..).collect::<String>()) {
                                Ok(val) => {
                                    if val.contains(" ") {
                                        let mut var:Vec<&str> = val.split(" ").collect();
                                        cmd = var[0].to_string();
                                        var.remove(0);
                                        for arg in var.iter() {
                                            args[curr_cmd].push(arg.to_string());
                                        } 
                                    }
                                    else {
                                        cmd = val;
                                    }
                                }
                                Err(_) =>  cmd = argv[i].as_str().to_string(),
                                    

                                //cmd = argv[i].as_str().to_string(),
                            }
                        }
                        else {
                            cmd = argv[i].as_str().to_string();
                        }

                            
                        
                        cmds[curr_cmd] = cmd;
                    }
                    else {
                        if argv[i].get(..1) == Some("$") {

                            match env::var(argv[i].clone().drain(1..).collect::<String>()) {
                                Ok(val) => {
                                    if val.contains(" ") {
                                        let var:Vec<&str> = val.split(" ").collect();
                                        for arg in var.iter() {
                                            args[curr_cmd].push(arg.to_string());
                                        } 
                                    }
                                    else {
                                        args[curr_cmd].push(val);
                                    }
                                }
                                Err(_) =>  args[curr_cmd].push(argv[i].as_str().to_string()),
                                    
                            }//args[curr_cmd].push(argv[i].as_str().to_string()),
                            

                        }
                        else {
                            args[curr_cmd].push(argv[i].as_str().to_string());
                        }
                    }
                }
        } 

    }
    

    return (cmds,args,env,stdin_redir,stdout_redir);
}
