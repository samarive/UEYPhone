use std::io::Write;
use std::io::BufRead;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::BufReader;

fn main() {
    println!("Démarrage du serveur...");
    let listener = TcpListener::bind("localhost:8000")
                                .expect("Impossible d'écouter sur le port 8000");

    println!("Serveur démarré.");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_request(stream);
    }
}

fn handle_request(mut stream: TcpStream) {
    let reader = BufReader::new(&mut stream);
    let request = reader.lines()
                        .map(|x| x.unwrap())
                        .take_while(|x| !x.is_empty())
                        .collect::<Vec::<_>>();
    

    let mut answer = match parse_function(request) {
        Err(e) => format!("HTTP/1.1 400 {:?}", e),
        Ok(f) => {
            if f.arguments[1] == "banane" {
                String::from("HTTP/1.1 200 OK")
            } else {
                String::from("HTTP/1.1 401 Wrong password")
            }
        }
    };

    answer.push_str("\r\nAccess-Control-Allow-Origin: *\r\n");
    stream.write_all(answer.as_bytes()).unwrap();
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