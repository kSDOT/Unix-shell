#![feature(with_options)]
#![feature(seek_convenience)]

#[macro_use]
extern crate lazy_static;

use std::io::{stdin, stdout, Write};
use std::env;
use std::process::Command;
use std::path::Path;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;
use std::io::{BufRead, BufReader, Seek, SeekFrom};


lazy_static!{
    static ref HISTORY_FILE: Mutex<std::fs::File> = Mutex::new(std::fs::File::with_options().read(true)
                                                                                            .write(true)
                                                                                            .append(true)
                                                                                            .create(true)
                                                                                            .open(".history.txt")
                                                                                            .unwrap());
}

macro_rules! history {
    () => {
        *HISTORY_FILE.lock().unwrap()
    };
}

macro_rules! skip_fail {
    ($res:expr, Option<_>) => {
        match $res {
            Some(val) => val,
            None => {
                
                continue;
            }
        }
    };
    ($res:expr, Result<_>) => {
        match $res {
            Ok(val) => val,
            Err(_) => {
                continue;
            }
        }
    };
} 
#[derive(Debug, Clone)]
enum Token{
    Command(String),
    CommandOptionSingle(String),
    CommandOptionCombined(String),
    CommandArguments(String),
    RedirectStream(u8, String),
    Pipe,
    End,
}

enum ReturnCode{
    Stop,
    Continue
}

fn main(){
    while {//for each line of input
        print!("{} >", env::current_dir()//print out new line with current location
                            .unwrap_or("dir".into())
                            .to_string_lossy()
                            );
        stdout().flush().expect("Error flushing to output!");
       
        //read line of input
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        //turn line into token-stream and parse contents
        let result = parse(tokenify(input));
        match result {
            Ok(code) => match code {
                ReturnCode::Continue => true,
                ReturnCode::Stop => false,
            },
            Err(code) => {eprintln!("Error: {}", code); true},
        }
    }{}
}
 
fn tokenify(input: String) -> Vec<Token>{
    //split expressions from each-other
    let expression: Vec<&str> = input.split(';')
                            .map(|s| s.trim())
                            .filter(|slice| !slice.is_empty())
                            .collect();
    expression.iter().for_each(|s| {let _ = writeln!(history!(), "{}",s);});
    history!().flush().ok();//history

    //sides of pipe      
    let pipe_sequences: Vec<Vec<&str>> = expression.into_iter()
                                                 .map(|iter| {iter.split("|").collect()})
                                                 .collect();

    pipe_sequences.iter()//iter on vec of vec of str
                  .map(|slice|{
                           let mut ret:Vec<Token> = slice.iter()//iter on vec of str
                                               .enumerate()
                                               .map(|(index, iter)|
                                                   string_to_token(iter, index == slice.len()-1)                                                                    
                                               )
                                           .flatten().collect();
                           ret.push(Token::End);
                           return ret;
                           }
                       )
                  .flatten()
                  .collect()
}

fn string_to_token(string: &str, last: bool) -> Vec<Token>{
    let mut words = string.split_whitespace().filter(|iter| !iter.is_empty());
    let mut tokens = vec![];
    if let Some(word) = words.next(){
        tokens.push(Token::Command(word.to_owned()));
        //command [args, [args..]] [options, [options..]]
        //first in sequence is command
        while let Some(word) = words.next(){
            let mut iter = word.chars().peekable();
 
            let token = {
                if iter.peek().unwrap() == &'-' {//flags
                    iter.next();
                    if let Some(next_char) = iter.peek(){ //--
                        if next_char== &'-' {
                            iter.next();
                            if iter.peek().is_none() {//empty --
                                continue
                            }
                            else{
                                Token::CommandOptionSingle(iter.collect())
                            }
                        }
                        else {
                            Token::CommandOptionCombined(iter.collect())//-
                        }
                    }
                    else {
                        continue;//empty -
                    }
                }
                else {
                   let taken_first: String = match iter.next(){
                        Some(c) => c.to_owned().to_string(),
                        None => String::new(),
                   };
 
                   let taken_second: String = match iter.next(){
                        Some(c) => c.to_owned().to_string(),
                        None => String::new(),
                   };
                   
                   if taken_first == "<"{
                       if iter.peek().is_none(){
                            iter = match words.next(){
                                Some(item) => item.chars().peekable(),
                                None => continue //end of stream
                            };
                           
                       }
                       Token::RedirectStream(0, taken_second.chars()
                                                            .chain(iter)
                                                            .collect())
                   }
                   else if taken_first == ">" {
                        if iter.peek().is_none(){
                            iter = match words.next(){
                                Some(item) => item.chars().peekable(),
                                None => continue //end of stream
                            };
                        }  
                       
                       Token::RedirectStream(1, taken_second.chars()
                                                            .chain(iter)
                                                            .collect())
                   }
                   else if taken_second == ">" {
                        if iter.peek().is_none(){
                            iter = match words.next(){
                                Some(item) => item.chars().peekable(),
                                None => continue //end of stream
                            };
 
                        }    
                       if let Some(d) = taken_first.chars().next().unwrap().to_digit(10){
                           Token::RedirectStream(d as u8, iter.collect())
                       }
                       else {
                           continue
                       }
                   }
                   else{
                       Token::CommandArguments(taken_first.chars()
                                                       .chain(taken_second.chars())
                                                       .chain(iter)
                                                       .collect())//args
                   }
                }
            };
            tokens.push(token);
           
        }
    }
    if !last{
        tokens.push(Token::Pipe);
    }
   
    tokens
}

fn parse(input: Vec<Token>)-> Result<ReturnCode, Box<dyn std::error::Error>> {
    let mut input = input.iter().enumerate().peekable();
    let mut previous_command_output: Option<std::process::Child> = None; 
    let mut custom_command_output = Vec::<u8>::new();
    let mut command_builder: Option<Command> = None;

    while let Some((_index, item)) = input.next() {
        match item {
            Token::Command(command) => match command.as_str(){
                "cd" => {
                           let dir = match input.peek(){
                                          Some((_, Token::CommandArguments(string))) => {
                                                input.next();//skip next as we captured it in string
                                                &string[..]
                                            },
                                          _ =>"/",
                                       };                              
                          if let Err(e) = env::set_current_dir(&Path::new(&dir)){
                              return Err(Box::from(e))
                          }
                          else {
                              continue
                          }
                        },
                "exit" | "q" => return Ok(ReturnCode::Stop),            
                "help" => print_help( &mut custom_command_output ),
                "history" => {
                    let mut n: usize = 1;
                    if let Some(token) = input.peek().as_mut(){
                        match token{
                            (_, Token::CommandArguments(nr)) => n = skip_fail!(nr.chars().next(), Option<_>).to_digit(10)
                                                                                                            .unwrap_or(1) as usize,
                            _ => (),
                        }
                    }
                    print_history(n, &mut custom_command_output);
                }
                command => command_builder = Some(Command::new(command)),
            },
            Token::CommandArguments(arg) =>  if let Some(command) = command_builder.as_mut(){ command.arg(arg); },
            Token::CommandOptionCombined(arg) => if let Some(command) = command_builder.as_mut(){ command.arg("-".to_owned() + arg);},
            Token::CommandOptionSingle(arg) => if let Some(command) = command_builder.as_mut(){ command.arg("--".to_owned() + arg);},
            Token::Pipe  => {
                              if let Some(mut command) = command_builder.as_mut() {
                                previous_command_output = Some((get_stdio(&mut command, &mut previous_command_output, 
                                                                                        & mut custom_command_output))
                                                                      .stdout(std::process::Stdio::piped())
                                                                      .spawn()?); 
                                if let Some(command) = previous_command_output.as_mut(){
                                    command.wait()?;
                                }
                              }
                            },
            Token::RedirectStream(handle, file_handle_or_name) => {
                let mut file_stream = match file_handle_or_name {
                    handle if handle.starts_with("&") 
                             => unsafe {std::fs::File::from_raw_fd(skip_fail!(handle.chars()
                                                                            .next()
                                                                            .unwrap()
                                                                            .to_digit(10),
                                                                     Option<_> 
                                                                    )as i32
                                                          )},
                    file_name => skip_fail!(std::fs::File::with_options().read(true)
                                                            .write(true)
                                                            .create(true)
                                                            .truncate(false)
                                                            .open(file_name),
                                                            Result<_>
                                            ),
                };
                file_stream.flush()?;
                match handle {
                    0 => if let Some(command) = command_builder.as_mut() {
                        command.stdin(file_stream);
                    },
                    1 => if let Some(command) = command_builder.as_mut() {
                        command.stdout(file_stream);
                    },
                    2 => if let Some(command) = command_builder.as_mut() {
                        command.stderr(file_stream);
                    },
                    _ => continue,

                }
            } ,
            Token::End => {if let Some(mut command) = command_builder.as_mut() {
                                                    get_stdio(&mut command, &mut previous_command_output, &mut custom_command_output)
                                                            .spawn()?.wait()?;
                                                }
                            else   {let _ = std::io::stdout().lock().write_all(custom_command_output.as_slice());}
                                            },
        }
   }
   Ok(ReturnCode::Continue)
}

fn print_help(mut custom_command_output: &mut Vec<u8>){
    let pos = history!().stream_position().unwrap();
    let _ =history!().seek(SeekFrom::Start(0));

    let _ = writeln!(&mut custom_command_output, "Unix shell.
                                        Forwards every command to the system for execution.
                                         Custom commands available:
                                        help -> outputs help
                                        q | exit -> quits terminal
                                        history[ n] -> outputs up to n lines from command history
                                        ");
    
    let _ = history!().seek(SeekFrom::Start(pos));
}
fn print_history(nr_lines: usize, mut custom_command_output: &mut Vec<u8>){
    
    let pos = history!().stream_position().unwrap();
    let _ =history!().seek(SeekFrom::Start(0));

    
    for line in BufReader::new(&history!()).lines().take(nr_lines as usize){
      let _ = writeln!(&mut custom_command_output, "{}", line.unwrap_or("".to_owned()));
    }
    if custom_command_output.is_empty() {
        let _ = writeln!(&mut custom_command_output, "No command in HISTORY");

    }
    let _ = history!().seek(SeekFrom::Start(pos));
}

fn get_stdio<'a>(command_builder: &'a mut std::process::Command, previous_command_output: &mut Option<std::process::Child>,
                custom_command_output: &mut Vec<u8>) 
    -> &'a mut std::process::Command {
        if !custom_command_output.is_empty() {
            let mut temp_file = std::fs::File::with_options().read(true).write(true).create(true).open(".temp.txt").unwrap();
            let _ = temp_file.write_all( custom_command_output);
            let _ = temp_file.seek(SeekFrom::Start(0));
            command_builder.stdin(std::process::Stdio::from(temp_file));

            custom_command_output.clear();
                                          
        }

        else if let Some(command) = previous_command_output.take() {
            command_builder.stdin(std::process::Stdio::from(command.stdout.unwrap()));

        }
  
  command_builder
}