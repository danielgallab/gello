use std::io::{self, Write};

use gello::{Interpreter, Lexer, Parser};

fn main() {
    println!("Gello 0.1.0");

    let mut input = String::new();
    let mut interpreter = Interpreter::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("Error reading input");
            continue;
        }

        let line = input.trim();

        if line.is_empty() {
            continue;
        }

        if line == "exit" {
            break;
        }

        let lexer = Lexer::new(line);
        match lexer.tokenize() {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                match parser.parse() {
                    Ok(statements) => {
                        // Clear output buffer before running
                        interpreter.clear_output();

                        if let Err(e) = interpreter.run(statements) {
                            eprintln!("Runtime error: {}", e);
                        } else {
                            // Print any output from print statements
                            for line in interpreter.take_output() {
                                println!("{}", line);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Parse error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
