use libc::{tcgetattr,tcsetattr,TCSANOW,ICANON,ECHO,STDIN_FILENO};
use std::io::{self,Write};
use libc::{signal, SIGINT, SIG_IGN, SIG_DFL};
use std::env::{current_dir, set_current_dir, var};
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use std::fs;
use std::collections::HashSet;

fn enable_raw_mode() -> libc::termios {
    unsafe {
        let mut termios = std::mem::zeroed();
        tcgetattr(STDIN_FILENO, &mut termios);
        let original = termios;
        termios.c_lflag &= !(ICANON | ECHO); 
        tcsetattr(STDIN_FILENO, TCSANOW, &termios);
        original
    }
}

fn disable_raw_mode(original: libc::termios) {
    unsafe {
        tcsetattr(STDIN_FILENO, TCSANOW, &original);
    }
}

fn read_byte() -> u8 {
    let mut buf = [0u8; 1];
    unsafe { libc::read(STDIN_FILENO, buf.as_mut_ptr() as *mut _, 1); }
    buf[0]
}

fn read_line(history: &Vec<String>) -> String {
    let mut history_index: Option<usize> = None;
    let mut input = String::new();
    let mut cursor:usize = 0;
    let original = enable_raw_mode();
    loop {
        match read_byte() {
            10 => {print!("\r\x1b[K\x1b[36m> \x1b[0m{}", input);break}//enter
            127 => {//character delete
                if cursor > 0 {
                    input.remove(cursor-1);
                    cursor-=1;
                    let rest = &input[cursor..];
                    print!("{}", rest);
                    print!("\x08 \x08");
                    if rest.len() > 0 {print!("\x1b[{}D", rest.len());}
                    print!("\r\x1b[K\x1b[36m> \x1b[0m{}", input);
                    io::stdout().flush().unwrap();
                }
            }
            9 => {//tab
                let (before, last_word) = input.rsplit_once(' ').unwrap_or(("", &input));
                let matches = tab(last_word);
                if matches.len() == 1 {
                    let completed = &matches[0];
                    input = if before.is_empty() {
                        completed.clone()
                    } else {
                        format!("{} {}", before, completed)
                    };
                    print!("\r\x1b[K\x1b[36m> \x1b[0m{}", input);
                    io::stdout().flush().unwrap();
                    cursor = input.len();
                    } else {
                        println!();
                    if matches.len() >= 10 {
                        print!("there are {} matches, are you sure? ", matches.len());
                        io::stdout().flush().unwrap();
                        match read_byte() {
                            121 | 89 => { println!("y"); for m in &matches { print!("{} ", m); } }
                            110 | 78 => { print!("\n\x1b[36m> \x1b[0m{}", input); io::stdout().flush().unwrap(); continue; }
                            _ => println!("y/n only please."),
                        }
                    }
                    for m in &matches { print!("{} ", m); }
                    print!("\n\x1b[36m> \x1b[0m{}", input);
                    io::stdout().flush().unwrap();
                    }
            }
            27 => {
                read_byte();
                match read_byte() {
                    65 => {//up
                        if history.is_empty() { continue; }
                        let new_index = match history_index {
                            None => history.len()-1,
                            Some(i) => if i>0{i-1} else{0}
                        };
                        history_index = Some(new_index);
                        input = history[new_index].clone();
                        cursor = input.len();
                        print!("\r\x1b[K\x1b[36m> \x1b[33m{}\x1b[0m",input);
                        io::stdout().flush().unwrap();
                    }
                    66 => {//down
                        match history_index {
                            None => {}
                            Some(i) => {
                                if i+1 >= history.len() {
                                    history_index = None;
                                    input = String::new();
                                    cursor = 0;
                                } else {
                                    history_index = Some(i+1);
                                    input = history[i+1].clone();
                                    cursor = input.len();
                                }
                                print!("\r\x1b[K\x1b[36m> \x1b[33m{}\x1b[0m", input);
                                io::stdout().flush().unwrap();
                            }
                        }
                    }
                    67 => {//right
                        cursor+=1;
                        print!("\x1b[C");
                            io::stdout().flush().unwrap();
                    }
                    68 => {//left
                        if cursor>0 {
                            cursor-=1;
                            print!("\x1b[D");
                            io::stdout().flush().unwrap();
                        }
                    }
                    _ => {}
                }
            }
            c => {//Everything else
                input.insert(cursor, c as char);
                cursor += 1;
                print!("\r\x1b[K\x1b[36m> \x1b[0m{}", input);
                let rest = &input[cursor..];
                print!("{}", rest);
                let (_,last_word) = input.rsplit_once(' ').unwrap_or(("", &input));
                let matches = tab(last_word);
                if let Some(suggestion) = matches.first() {
                    if suggestion.len() > last_word.len() {
                        let ghost = &suggestion[last_word.len()..];
                        print!("\x1b[2m{}\x1b[0m", ghost);
                        if ghost.len() > 0 {print!("\x1b[{}D", ghost.len());}
                    }
                }
                if rest.len() > 0 {print!("\x1b[{}D", rest.len());}
                io::stdout().flush().unwrap();
            }
        }
    }
    
    disable_raw_mode(original);
    println!();
    input
}
fn main() {
    unsafe { signal(SIGINT, SIG_IGN); }

    let mut last_exit_code:i32 = 0;

    let home = var("HOME").unwrap_or_default();
    let hostname = fs::read_to_string("/etc/hostname").unwrap();
    let username = var("USER").unwrap_or_default();
    
    let history_file = format!("{}/.theshellfiles.hist",home);
    let mut history: Vec<String> = fs::read_to_string(&history_file).unwrap_or_default().lines().map(|x| x.to_string()).collect();
    
    loop {
        let curdir = current_dir().unwrap().to_string_lossy().replace(&home, "~");
        
        println!("\x1b[34m|{}/{}| \x1b[32m{}", hostname.trim(), username, curdir);
        print!("\x1b[36m> \x1b[0m");
            
        io::stdout().flush().unwrap();
        let input = read_line(&history);
        let input = input.trim();
        if input.is_empty() { continue; }
        if input == "x" || input == "exit" {
            fs::write(&history_file, history.join("\n")).unwrap();
            break; }
        if input == "test" {
            let thing = tab("/");
            for h in &thing {
                print!("{}", h);
            }
            
        }
        if history.last().map(|s| s.as_str()) != Some(input) {history.push(input.to_string());}
        let input = &input.replace("$?", &last_exit_code.to_string()).replace("~", &home);
        last_exit_code = execute(tokenize(input));
        if last_exit_code != 0 {
            print!("\x1b[31m[{}] ", last_exit_code);
        }
    }
    
}

fn execute(commands: Vec<Cmd>) -> i32 {
    let mut last_code = 0;
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for cmd in commands.iter() {
        if cmd.cmd == "cd" {
        let home = var("HOME").unwrap_or_default();
        let dir = cmd.args.first().map(|s| s.as_str()).unwrap_or(&home);
        if let Err(e) = set_current_dir(dir) {
            eprintln!("cd: {}", e);
            last_code = 1;
        }
        continue;
        }
        let stdin = match prev_stdout.take() {
            Some(stdout) => Stdio::from(stdout),
            None => Stdio::inherit(),
        };
        let stdout = match cmd.op {
            Op::P => Stdio::piped(),
            _ => Stdio::inherit(),
        };
        let child = unsafe {Command::new(&cmd.cmd)
        .args(&cmd.args)
        .stdin(stdin)
        .stdout(stdout)
        .pre_exec(|| {
            signal(SIGINT, SIG_DFL);
            Ok(())
        })
        .spawn()
        };
        match child {
            Err(e) => {
                eprintln!("{}: {}", cmd.cmd, e);
                last_code = 1;
                continue;
            }
            Ok(mut child) => {
                match cmd.op {
                    Op::P => { prev_stdout = child.stdout.take(); }
                    Op::A => {
                        last_code = child.wait().unwrap().code().unwrap_or(1);
                        if last_code != 0 { return last_code; }
                    }
                    Op::B => { }
                    Op::N => { last_code = child.wait().unwrap().code().unwrap_or(1); }
                }
            }
        }
    }
    last_code
}

fn tab(partial: &str) -> Vec<String> {
    let mut matches = Vec::new();
    if partial.contains('/') {
        let (dir, file_prefix) = partial.rsplit_once('/').unwrap();
        let dir = if dir.is_empty() {"/"} else {dir};
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(file_prefix) {
                    matches.push(format!("{}/{}",dir.trim_end_matches('/'),name));
                }
            }
        }
    } else {
        let paths = var("PATH").unwrap();
        for dir in paths.split(':') {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with(partial) {
                        matches.push(name.to_string());
                    }
                }
            }
        }
    }
    let matches: Vec<String> = matches.into_iter().collect::<HashSet<_>>().into_iter().collect();
    matches
}
enum Op {
    P,
    A,
    N,
    B,
}
struct Cmd {
    cmd: String,
    args: Vec<String>,
    op: Op,
}
fn tokenize(input: &str) -> Vec<Cmd> {
    let mut commands = Vec::new();
    let mut cur_tkn = String::new();
    let mut tkns: Vec<String> = Vec::new();
    let mut qte = false;
    let mut prev_char: char = ' '; 
    
    for c in input.chars() {
        if c == '"' {qte = !qte;} 
        else if c == ' ' && !qte {
            if !cur_tkn.is_empty() {
                tkns.push(cur_tkn.clone());
                cur_tkn = String::new();
            }
        } else if c == '|' && !qte {
            if !cur_tkn.is_empty() {
                tkns.push(cur_tkn);
            }
            commands.push(Cmd {
                cmd: tkns[0].clone(),
                args: tkns[1..].to_vec(),
                op: Op::P,
            });
            tkns = Vec::new();
            cur_tkn = String::new();
        } else if c == '&' && prev_char == '&' {
            if !cur_tkn.is_empty() {
                tkns.push(cur_tkn);
            }
            commands.push(Cmd {
                cmd: tkns[0].clone(),
                args: tkns[1..].to_vec(),
                op: Op::A,
            });
            tkns = Vec::new();
            cur_tkn = String::new();
            prev_char = ' ';
        } else if prev_char == '&' {
            if !cur_tkn.is_empty() {
                tkns.push(cur_tkn.clone());
                cur_tkn = String::new();
            }
            commands.push(Cmd {
                cmd: tkns[0].clone(),
                args: tkns[1..].to_vec(),
                op: Op::B,
            });
            tkns = Vec::new();
            prev_char = ' ';
            cur_tkn.push(c);
        } else if c == '&' {
            prev_char = c;
        } else {cur_tkn.push(c);prev_char = c;}
    }

    if !cur_tkn.is_empty() {
        tkns.push(cur_tkn);
    }
    if !tkns.is_empty() {
        commands.push(Cmd {
            cmd: tkns[0].clone(),
            args: tkns[1..].to_vec(),
            op: Op::N,
            });
    }
    commands
}