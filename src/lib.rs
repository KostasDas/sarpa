use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum ArgParseError {
    UnknownArgument(String),
    MissingValueForOption(String),
    OptionInMiddleOfGroup(String),
    HelpRequested,
    MissingRequiredArgument(String),
}

struct Argument {
    pub arg_type: ArgumentKind,
    pub short_name: Option<char>,
    pub long_name: String,
    pub help: String,
    pub required: bool
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

impl ParsedArgs {
    pub fn get_value_as<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        let option = self.options.get(name)?;
        Some(option.parse::<T>())
    }
}

pub struct Parser {
    definitions: Vec<Argument>
}

pub struct ArgumentBuilder<'a> {
    parser: &'a mut Parser
}

impl<'a> ArgumentBuilder<'a> {
    pub fn with_help(self, help_text: &str) -> Self {
        if let Some(arg) = self.parser.definitions.last_mut() {
            arg.help = help_text.to_string();
        }
        self
    }
    pub fn with_short_name(self, short_name: char) -> Self {
        if let Some(arg) = self.parser.definitions.last_mut() {
            arg.short_name = Some(short_name);
        }
        self
    }
    pub fn required(self) -> Self {
        if let Some(arg) = self.parser.definitions.last_mut() {
            arg.required = true;
        }
        self
    }
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
        arg_type: ArgumentKind,
    ) {
        self.definitions.push(Argument {
            arg_type,
            short_name: None,
            long_name: long_name.to_string(),
            help: "".to_string(),
            required: false
        })
    }

    /// Defines a flag argument (e.g., --verbose, -v).
    pub fn add_flag(&mut self, long_name: &str) -> ArgumentBuilder {
        self.add(long_name, ArgumentKind::Flag);
        ArgumentBuilder{parser: self}
    }

    /// Defines an option argument that takes a value (e.g., --output <file>, -o <file>).
    pub fn add_option(&mut self, long_name: &str) -> ArgumentBuilder {
        self.add(long_name, ArgumentKind::Option);
        ArgumentBuilder{parser: self}
    }

    /// Defines a positional argument.
    pub fn add_positional(&mut self, long_name: &str) -> ArgumentBuilder {
        self.add(long_name, ArgumentKind::Positional);
        ArgumentBuilder{parser: self}
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
                                        return Err(ArgParseError::OptionInMiddleOfGroup(def.long_name.clone()));
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

        // validate any required parameters
        self.validate_args(&results)?;
        Ok(results)
    }

    fn validate_args(&self, results: &ParsedArgs) -> Result<(), ArgParseError> {
        for def in &self.definitions {
            if def.required {
                let was_provided = match def.arg_type {
                    ArgumentKind::Flag => results.flags.contains(&def.long_name),
                    ArgumentKind::Option => results.options.contains_key(&def.long_name),
                    ArgumentKind::Positional => !results.positional.is_empty(),
                };

                if !was_provided {
                    return Err(ArgParseError::MissingRequiredArgument(def.long_name.clone()));
                }
            }
        }
        Ok(())
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
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("increases the verbosity");
        let result = parser.parse(to_args(vec!["program", "--verbose"])).unwrap();
        assert!(result.flags.contains("verbose"));
    }

    #[test]
    fn test_long_option() {
        let mut parser = Parser::new();
        parser.add_option("output")
            .with_short_name('o')
            .with_help("where the output should be stored");
        let result = parser.parse(to_args(vec!["program", "--output", "file.txt"])).unwrap();
        assert_eq!(result.options.get("output"), Some(&"file.txt".to_string()));
    }

    #[test]
    fn test_short_combined_flags() {
        let mut parser = Parser::new();
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("increased the verbosity");
        parser.add_flag("all")
            .with_short_name('a')
            .with_help("foobar");
        let result = parser.parse(to_args(vec!["program", "-av"])).unwrap();
        assert!(result.flags.contains("verbose"));
        assert!(result.flags.contains("all"));
    }

    #[test]
    fn test_short_option_with_value() {
        let mut parser = Parser::new();
        parser.add_option("output")
            .with_short_name('o')
            .with_help("foobaz");
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("foobarbaz");
        let result = parser.parse(to_args(vec!["program", "-vo", "file.txt"])).unwrap();
        assert!(result.flags.contains("verbose"));
        assert_eq!(result.options.get("output"), Some(&"file.txt".to_string()));
    }

    #[test]
    fn test_positional_argument() {
        let mut parser = Parser::new();
        parser.add_positional("input_file")
            .with_help("positional");
        let result = parser.parse(to_args(vec!["program", "data.csv"])).unwrap();
        assert_eq!(result.positional, vec!["data.csv"]);
    }

    #[test]
    fn test_mixed_arguments() {
        let mut parser = Parser::new();
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("verbose city");
        parser.add_option("output")
            .with_short_name('o')
            .with_help("output city");
        parser.add_positional("input")
            .with_help("positional city");
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
        parser.add_option("output")
            .with_short_name('o')
            .with_help("test");
        let result = parser.parse(to_args(vec!["program", "--output"]));
        assert!(matches!(result, Err(ArgParseError::MissingValueForOption(_))));
    }

    #[test]
    fn test_err_option_in_middle_of_group() {
        let mut parser = Parser::new();
        parser.add_option("output")
            .with_short_name('o')
            .with_help("test");
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("test2");
        let result = parser.parse(to_args(vec!["program", "-ov", "file.txt"]));
        assert!(matches!(result, Err(ArgParseError::OptionInMiddleOfGroup(_))));
    }

    #[test]
    fn test_help_flag_returns_help_requested_error() {
        let mut parser = Parser::new();
        parser.add_flag("verbose")
            .with_short_name('v')
            .with_help("Enable verbose output.");

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
        parser.add_flag("all")
            .with_short_name('a')
            .with_help("List all items.");
        parser.add_option("output")
            .with_short_name('o')
            .with_help("Specify output file.");
        parser.add_positional("input")
            .with_help("The input file to process.");

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

    #[test]
    fn test_required_option_is_present() {
        let mut parser = Parser::new();
        parser.add_option("output").required();
        let result = parser.parse(to_args(vec!["program", "--output", "file.txt"]));
        assert!(result.is_ok());
    }

    #[test]
    fn test_err_missing_required_option() {
        let mut parser = Parser::new();
        parser.add_option("output").required();
        let result = parser.parse(to_args(vec!["program"]));
        assert!(matches!(result, Err(ArgParseError::MissingRequiredArgument(_))));
    }

    #[test]
    fn test_get_value_as() {
        
        let mut options = HashMap::new();
        options.insert("port".to_string(), "8080".to_string());
        options.insert("rate".to_string(), "hello".to_string());

        let parsed_args = ParsedArgs {
            flags: HashSet::new(),
            options,
            positional: vec![],
        };

        // 2. Test Success case: Valid key and valid parse
        let port = parsed_args.get_value_as::<u32>("port").unwrap().unwrap();
        assert_eq!(port, 8080);

        // 3. Test Failure case: Valid key but invalid parse
        let rate_result = parsed_args.get_value_as::<f64>("rate");
        // We expect Some(Err(...))
        assert!(rate_result.is_some());
        assert!(rate_result.unwrap().is_err());

        // 4. Test Absence case: Key does not exist
        let missing_result = parsed_args.get_value_as::<i32>("missing");
        // We expect None
        assert!(missing_result.is_none());
    }
}
