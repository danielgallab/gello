mod ast;
mod environment;
pub mod interpreter;
pub mod lexer;
pub mod parser;
mod token;

use wasm_bindgen::prelude::*;

/// Run Gello source code and return all output (print statements and errors)
/// as a newline-separated string.
#[wasm_bindgen]
pub fn run_gello(source: &str) -> String {
    let mut interp = interpreter::Interpreter::new();
    let mut results: Vec<String> = Vec::new();

    // Process each line of the source
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lex = lexer::Lexer::new(trimmed);
        match lex.tokenize() {
            Ok(tokens) => {
                let mut parse = parser::Parser::new(tokens);
                match parse.parse() {
                    Ok(statements) => {
                        // Clear output buffer before running to track new output
                        interp.clear_output();

                        if let Err(e) = interp.run(statements) {
                            results.push(format!("ERROR: Runtime error: {}", e));
                        } else {
                            // Collect any print output from this execution
                            results.extend(interp.take_output());
                        }
                    }
                    Err(e) => {
                        results.push(format!("ERROR: Parse error: {}", e));
                    }
                }
            }
            Err(e) => {
                results.push(format!("ERROR: {}", e));
            }
        }
    }

    results.join("\n")
}

// Re-export for use by main.rs
pub use interpreter::Interpreter;
pub use lexer::Lexer;
pub use parser::Parser;
