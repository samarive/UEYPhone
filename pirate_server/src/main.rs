use std::io::Write;
use std::io::BufRead;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::BufReader;
use oracle::{Connection, Error};
use rand::{distributions::Alphanumeric, Rng};
use std::time::{Duration, Instant};

fn main() {
    println!("Démarrage du serveur...");
    
    let database = Connection::connect("pirate_phone", "bananes", "")
                            .expect("Impossible de se connecter à la BDD");

    let listener = TcpListener::bind("localhost:8000")
                                .expect("Impossible d'écouter sur le port 8000");

    let mut tokens = Vec::<Token>::new();

    println!("Serveur démarré.");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_request(stream, &database, &mut tokens);
    }
}

fn handle_request(mut stream: TcpStream, database: &Connection, tokens: &mut Vec::<Token>) {
    let reader = BufReader::new(&mut stream);
    let request = reader.lines()
                        .map(|x| x.unwrap())
                        .take_while(|x| !x.is_empty())
                        .collect::<Vec::<_>>();
    

    let mut answer = match parse_function(request) {
        Err(e) => format!("HTTP/1.1 400 {:?}", e),
        Ok(f) => {
            match f.name.as_str() {
                "connect" => {
                    if f.arguments.len() != 2 {
                        String::from("HTTP/1.1 400 Wrong nmbr of arguments")
                    } else if login(&f.arguments[0], &f.arguments[1]) {
                        tokens.push(Token::new());
                        println!("Token created : {}", tokens.last().unwrap().value());
                        format!("HTTP/1.1 200 {}", tokens.last().unwrap().value())                        
                    } else {
                        String::from("HTTP/1.1 401 Wrong password")
                    }
                },
                "call_later" => {
                    if f.arguments.len() == 2 && is_token_valid(&f.arguments[1], &tokens) {
                        let mut num = String::new();
                        let mut should_skip = 0;
                        for i in 0..f.arguments[0].len() {
                            if should_skip != 0{
                                should_skip -= 1;
                                continue;
                            }
                            if &f.arguments[0][i..i+1] == "%" {
                                should_skip = 2;
                                num.push(' ');
                            } else {
                                num.push_str(&f.arguments[0][i..i+1]);
                            }
                        }

                        let r = database.execute(
                            &format!(
                                "update Contact set to_call=1 where numero = '{}'",
                                num
                            ),
                            &[]
                        );

                        if let Err(e) = r { 
                            println!("Error while resetting to_call : {e:#?}.");
                        } 

                        database.commit().expect("Error while commiting");

                        String::from("HTTP/1.1 200 OK")
                    } else if f.arguments.len() == 2 {
                        String::from("HTTP/1.1 401 Invalid token")
                    } else {
                        String::from("HTTP/1.1 400 Wrong number of arguments")
                    }
                }
                "get_call_info" => {
                    if f.arguments.len() == 1 && is_token_valid(&f.arguments[0], &tokens) {
                        let sql = "select nom, prenom, numero from Contact where numero = get_call_info";
                        
                        match database.query(sql, &[]) {
                            Ok(rows) => match rows.last() {
                                Some(row) => {
                                    let row = row.unwrap(); 
                                    let nom: String = row.get("nom").unwrap();
                                    let prenom: String = row.get("prenom").unwrap();
                                    let numero: String = row.get("numero").unwrap();

                                    if let Err(e) =  database.execute(&format!("call touch_call_info('{numero}')"), &[]) {
                                        println!("Error while touching call info : {e:#?}.");
                                    }
                                    if let Err(_) = database.commit() {
                                        println!("Error while committing.");
                                    }

                                    format!("HTTP/1.1 200 {nom},{prenom},{numero},")
                                },
                                None => {
                                    println!("No row selected.");
                                    String::from("HTTP/1.1 200 All clear")
                                }
                            },
                            Err(e) => {
                                println!("Error in query : {e:#?}");
                                String::from("HTTP/1.1 200 All clear")
                            }
                        }
                    } else if f.arguments.len() == 1 {
                        String::from("HTTP/1.1 401 Invalid token")
                    } else {
                        String::from("HTTP/1.1 400 Wrong number of arguments")
                    }
                },
                _ => {
                    String::from("HTTP/1.1 400 Unknown function")
                }
            }
        }
    };

    answer.push_str("\r\nAccess-Control-Allow-Origin: *\r\n");
    stream.write_all(answer.as_bytes()).unwrap();
}

fn login(username: &str, password: &str) -> bool {
    password == "alba" 
}

fn parse_function(request: Vec::<String>) -> Result<FunctionCall, FunctionParsingError> {
    let mut state = FunctionParserState::FunctionName;
    
    let mut result = FunctionCall {
        name: String::new(),
        arguments: Vec::<String>::new()
    };

    let mut buffer = String::new();

    let request = request[0].chars()
                            .skip_while(|x| *x != '/')
                            .skip(1)
                            .take_while(|x| *x != 'H')
                            .collect::<String>();
    for c in request.chars() {
        if c == ' ' {continue;}
        let next_state = state.transition(FunctionParserTransition::from(c));

        if next_state == state {
            buffer.push(c);
        } else {
            match state {
                FunctionName => result.name = buffer.clone(),
                Argument(_) => result.arguments.push(buffer.clone()),
                _ => {},
            }
            buffer.clear();
        }
        state = next_state;
    }
    
    match state {
        FunctionParserState::End => Ok(result),
        FunctionParserState::Error(e) => Err(e),
        _ => Err(FunctionParsingError::UnexpectedEOL)
    }
}

fn is_token_valid(token: &String, l: &Vec::<Token>) -> bool {
    l.iter().map(|x| x.value()).collect::<Vec<String>>().contains(token)
}

struct Token {
    value: String,
    creation: Instant
}
impl Token {

    const VALID_FOR: u64 = 30;

    fn new() -> Self {
        Self {
            value: rand::thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(16)
                            .map(char::from)
                            .collect(),
            creation: Instant::now()
        }
    }

    fn is_valid(&self) {
        self.creation.elapsed().as_secs() / 60 < Self::VALID_FOR;
    }

    fn value(&self) -> String {
        self.value.clone()
    }
}

struct FunctionCall {
    name: String,
    arguments: Vec::<String>
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum FunctionParsingError {
    UnexpectedCharacter (char),
    CharacterAfterEnd,
    UnexpectedEOL
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum FunctionParserState {
    FunctionName,
    Argument(usize),
    End,
    Error (FunctionParsingError)
}
use FunctionParserState::*;
enum FunctionParserTransition {
    Character,
    OpenParenthesis,
    Comma,
    ClosedParenthesis
}
use FunctionParserTransition::*;

impl FunctionParserState {
    fn transition(self, transition: FunctionParserTransition) -> Self {
        match self {
            FunctionName => match transition {
                Character => self,
                OpenParenthesis => Argument(0),
                Comma => Error(FunctionParsingError::UnexpectedCharacter(',')),
                ClosedParenthesis => Error(FunctionParsingError::UnexpectedCharacter(')'))
            },
            Argument(n) => match transition {
                Character => self,
                OpenParenthesis => Error(FunctionParsingError::UnexpectedCharacter('(')),
                Comma => Argument(n + 1),
                ClosedParenthesis => End  
            },
            End => Error(FunctionParsingError::CharacterAfterEnd),
            Error(e) => Error(e)
        }
    }
}

impl FunctionParserTransition {
    fn from(c: char) -> Self {
        match c {
            '(' => Self::OpenParenthesis,
            ')' => Self::ClosedParenthesis,
            ',' => Self::Comma,
            _ => Self::Character
        }
    }
}