use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub enum ArgParseError {
    UnknownArgument(String),
    MissingValueForOption(String),
    OptionInMiddleOfGroup(String),
    HelpRequested,
}

struct Argument {
    pub arg_type: ArgumentKind,
    pub short_name: Option<char>,
    pub long_name: String,
    pub help: String
}

enum ArgumentKind {
    Flag,
    Option,
    Positional
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub flags: HashSet<String>,
    pub options: HashMap<String, String>,
    pub positional: Vec<String>
}

pub struct Parser {
    definitions: Vec<Argument>
}


impl Parser {
    pub fn new() -> Self {
        Parser {
            definitions: Vec::new()
        }
    }
    
    fn add(
        &mut self,
        long_name: &str,
        short_name: Option<char>,
        arg_type: ArgumentKind,
        help: &str,
    ) {
        self.definitions.push(Argument {
            arg_type,
            short_name,
            long_name: long_name.to_string(),
            help: help.to_string()
        })
    }

    /// Defines a flag argument (e.g., --verbose, -v).
    pub fn add_flag(&mut self, long_name: &str, short_name: char, help: &str) {
        self.add(long_name, Some(short_name), ArgumentKind::Flag, help);
    }

    /// Defines an option argument that takes a value (e.g., --output <file>, -o <file>).
    pub fn add_option(&mut self, long_name: &str, short_name: char, help: &str) {
        self.add(long_name, Some(short_name), ArgumentKind::Option, help);
    }

    /// Defines a positional argument.
    pub fn add_positional(&mut self, name: &str, help: &str) {
        self.add(name, None, ArgumentKind::Positional, help);
    }
    
    pub fn generate_help(&self) -> String {
        let mut help = String::from("Usage: [PROGRAM_NAME] [OPTIONS] [ARGUMENTS]\n");
        help.push_str("\nOptions:\n");

        for def in &self.definitions {
            if let ArgumentKind::Flag | ArgumentKind::Option = def.arg_type {
                let short = def.short_name.map_or_else(
                    || "    ".to_string(),
                    |s| format!("-{}, ", s)
                );
                help.push_str(&format!("  {}{:<20} {}\n", short, def.long_name, def.help))
            }
        }
        
        help.push_str("\nArguments:\n");
        for def in &self.definitions {
            if let ArgumentKind::Positional = def.arg_type {
                help.push_str(&format!("  {:<22} {}\n", def.long_name, def.help))
            }
        }
        
        help
    }
    
    
    
    fn parse<T: Iterator<Item = String>>(&self, mut args: T) -> Result<ParsedArgs, ArgParseError> {
        let mut results = ParsedArgs {
            flags: HashSet::new(),
            options: HashMap::new(),
            positional: vec![]
        };

        args.next(); // skip program name

        while let Some(arg) = args.next() {
            if arg == "--help" || arg == "-h" {
                return Err(ArgParseError::HelpRequested);
            }
            if let Some(arg_without_prefix) = arg.strip_prefix("--") {
                let argument_def = self.definitions
                    .iter()
                    .find(|x| {
                        x.long_name == arg_without_prefix
                    });
                match argument_def {
                    None => {
                        return Err(ArgParseError::UnknownArgument(String::from(arg_without_prefix)))
                    }
                    Some(def) => {
                        match def.arg_type {
                            ArgumentKind::Flag => {
                                results.flags.insert(def.long_name.clone());
                            }
                            ArgumentKind::Option => {
                                Self::extract_option(&mut args, &mut results, def)?
                            }
                            ArgumentKind::Positional => {
                                unreachable!(
                                    "Positional argument definitions should not be matched against prefixed arguments."
                                );
                            }
                        }
                    }
                }
                

            } else if let Some(arg_without_prefix) = arg.strip_prefix("-") {
                let count = arg_without_prefix.chars().count();
                for (i, char) in arg_without_prefix.chars().enumerate() {
                    let argument_def = self.definitions
                        .iter()
                        .find(|x| {x.short_name == Some(char)});

                    match argument_def {
                        None => {
                            return Err(ArgParseError::UnknownArgument(char.to_string()))
                        }
                        Some(def) => {
                            match def.arg_type {
                                ArgumentKind::Flag => {
                                    results.flags.insert(def.long_name.clone());
                                }
                                ArgumentKind::Option => {
                                    if i == count - 1 {
                                        Self::extract_option(&mut args, &mut results, def)?;
                                    } else {
                                        Err(ArgParseError::OptionInMiddleOfGroup(def.long_name.clone()))?;
                                    }
                                    break; // last option in the group, exit the loop.
                                }
                                ArgumentKind::Positional => {
                                    unreachable!(
                                        "Positional argument definitions should not be matched against prefixed arguments."
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                results.positional.push(arg)
            }
        }

        Ok(results)
    }

    fn extract_option<T: Iterator<Item=String>>(args: &mut T, results: &mut ParsedArgs, x: &Argument) -> Result<(), ArgParseError> {
        match args.next() {
            Some(value) => {
                results.options.insert(x.long_name.clone(), value);
                Ok(())
            }
            None => {
                Err(ArgParseError::MissingValueForOption(x.long_name.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to make creating argument lists for tests easier.
    fn to_args(args: Vec<&str>) -> impl Iterator<Item = String> {
        args.into_iter().map(|s| s.to_string())
    }

    #[test]
    fn test_long_flag() {
        let mut parser = Parser::new();
        parser.add_flag("verbose", 'v', "increases the verbosity");
        let result = parser.parse(to_args(vec!["program", "--verbose"])).unwrap();
        assert!(result.flags.contains("verbose"));
    }

    #[test]
    fn test_long_option() {
        let mut parser = Parser::new();
        parser.add_option("output", 'o', "where the output should be stored");
        let result = parser.parse(to_args(vec!["program", "--output", "file.txt"])).unwrap();
        assert_eq!(result.options.get("output"), Some(&"file.txt".to_string()));
    }

    #[test]
    fn test_short_combined_flags() {
        let mut parser = Parser::new();
        parser.add_flag("verbose", 'v', "increased the verbosity");
        parser.add_flag("all", 'a', "foobar");
        let result = parser.parse(to_args(vec!["program", "-av"])).unwrap();
        assert!(result.flags.contains("verbose"));
        assert!(result.flags.contains("all"));
    }

    #[test]
    fn test_short_option_with_value() {
        let mut parser = Parser::new();
        parser.add_option("output", 'o', "foobaz");
        parser.add_flag("verbose", 'v', "foobarbaz");
        let result = parser.parse(to_args(vec!["program", "-vo", "file.txt"])).unwrap();
        assert!(result.flags.contains("verbose"));
        assert_eq!(result.options.get("output"), Some(&"file.txt".to_string()));
    }

    #[test]
    fn test_positional_argument() {
        let mut parser = Parser::new();
        parser.add_positional("input_file", "positional");
        let result = parser.parse(to_args(vec!["program", "data.csv"])).unwrap();
        assert_eq!(result.positional, vec!["data.csv"]);
    }

    #[test]
    fn test_mixed_arguments() {
        let mut parser = Parser::new();
        parser.add_flag("verbose", 'v', "verbose city");
        parser.add_option("output", 'o', "output city");
        parser.add_positional("input", "positional city");
        let result = parser.parse(to_args(vec!["program", "-v", "the_input.txt", "--output", "out.log"])).unwrap();
        assert!(result.flags.contains("verbose"));
        assert_eq!(result.positional, vec!["the_input.txt"]);
        assert_eq!(result.options.get("output"), Some(&"out.log".to_string()));
    }

    #[test]
    fn test_err_unknown_long_argument() {
        let parser = Parser::new();
        let result = parser.parse(to_args(vec!["program", "--unknown"]));
        assert!(matches!(result, Err(ArgParseError::UnknownArgument(_))));
    }

    #[test]
    fn test_err_unknown_short_argument() {
        let parser = Parser::new();
        let result = parser.parse(to_args(vec!["program", "-x"]));
        assert!(matches!(result, Err(ArgParseError::UnknownArgument(_))));
    }

    #[test]
    fn test_err_missing_value_for_option() {
        let mut parser = Parser::new();
        parser.add_option("output", 'o', "test");
        let result = parser.parse(to_args(vec!["program", "--output"]));
        assert!(matches!(result, Err(ArgParseError::MissingValueForOption(_))));
    }

    #[test]
    fn test_err_option_in_middle_of_group() {
        let mut parser = Parser::new();
        parser.add_option("output", 'o', "test");
        parser.add_flag("verbose", 'v', "test2");
        let result = parser.parse(to_args(vec!["program", "-ov", "file.txt"]));
        assert!(matches!(result, Err(ArgParseError::OptionInMiddleOfGroup(_))));
    }

    #[test]
    fn test_help_flag_returns_help_requested_error() {
        let mut parser = Parser::new();
        parser.add_flag("verbose", 'v', "Enable verbose output.");

        // Test long form --help
        let long_args = to_args(vec!["program", "--help"]);
        let long_result = parser.parse(long_args);
        assert_eq!(long_result.unwrap_err(), ArgParseError::HelpRequested);

        // Test short form -h
        let short_args = to_args(vec!["program", "-h"]);
        let short_result = parser.parse(short_args);
        assert_eq!(short_result.unwrap_err(), ArgParseError::HelpRequested);
    }

    #[test]
    fn test_generate_help_message_formatting() {
        let mut parser = Parser::new();
        parser.add_flag("all", 'a', "List all items.");
        parser.add_option("output", 'o', "Specify output file.");
        parser.add_positional("input", "The input file to process.");

        let help_text = parser.generate_help();

        // Use a raw string literal r#"..."# for easy multi-line comparison
        let expected_text = r#"Usage: [PROGRAM_NAME] [OPTIONS] [ARGUMENTS]

Options:
  -a, all                  List all items.
  -o, output               Specify output file.

Arguments:
  input                  The input file to process.
"#;

        assert_eq!(help_text, expected_text);
    }
}
