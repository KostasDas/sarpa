# Simple Argument Parser

[![Crates.io](https://img.shields.io/crates/v/sarpa.svg)](https://crates.io/crates/sarpa)
[![Docs.rs](https://docs.rs/sarpa/badge.svg)](https://docs.rs/sarpa)

A simple and robust command-line argument parsing library for Rust.

This library provides a builder pattern API to define arguments and a parser to process them from the command line. The parsed arguments can be converted to any type that implements `FromStr`.

## Installation

Add this line to your `Cargo.toml` file:

```toml
[dependencies]
sarpa = "0.1.2"
```

### Quick Start

Here is a complete example of how to use the library in src/main.rs:


```rust
use std::env;
use sarpa::{ArgParseError, Parser};

fn main() {
    let mut parser = Parser::new();

    // Define your arguments using the builder pattern
    parser.add_flag("verbose")
        .with_short_name('v')
        .with_help("Enable verbose output");

    parser.add_option("output")
        .with_short_name('o')
        .with_help("Specify an output file")
        .required();

    parser.add_positional("input")
        .with_help("The input file to process");


    let args = env::args();
    let results = parser.parse(args);
    
    match results {
        Ok(parsed_args) => {
            // --- Your main application logic goes here ---

            if parsed_args.flags.contains("verbose") {
                println!("Verbose mode is enabled!");
            }
            
            let output_file = parsed_args.options.get("output").unwrap();
            println!("Output file: {}", output_file);

            // Get and parse a value
            // (Assuming you added another option like --port)
            // let port = parsed_args.get_value_as::<u32>("port")
            //     .unwrap_or(Ok(80)) // Provide a default
            //     .unwrap();
            // println!("Using port: {}", port);
        }
        Err(e) => {

            match e {
                ArgParseError::HelpRequested => {
                    // Print the auto-generated help message
                    println!("{}", parser.generate_help());
                }
                _ => {
                    // Handle any other errors
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
```