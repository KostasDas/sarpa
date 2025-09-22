use std::collections::{HashMap, HashSet};
use std::str::Chars;

// At the top of your file or in a new module
#[derive(Debug)] // This lets us print the error for debugging
pub enum ArgParseError {
    UnknownArgument(String),
    MissingValueForOption(String),
}

struct Argument {
    pub arg_type: ArgumentKind,
    pub short_name: Option<char>,
    pub long_name: String
}

enum ArgumentKind {
    Flag,
    Option,
    Positional
}

pub struct ParsedArgs {
    pub flags: HashSet<String>,
    pub options: HashMap<String, String>,
    pub positional: Vec<String>
}

pub struct Parser {
    definitions: Vec<Argument>
}


impl Parser {
    fn parse<T: Iterator<Item = String>>(&self, mut args: T) -> Result<ParsedArgs, ArgParseError> {
        let mut results = ParsedArgs {
            flags: HashSet::new(),
            options: HashMap::new(),
            positional: vec![]
        };

        args.next(); // skip program name

        while let Some(arg) = args.next() {
            if let Some(arg_without_prefix) = arg.strip_prefix("--") {
                let argument = self.definitions
                    .iter()
                    .find(|x| {
                        x.long_name == arg_without_prefix
                    });
                match argument {
                    None => {
                        return Err(ArgParseError::UnknownArgument(String::from(arg_without_prefix)))
                    }
                    Some(x) => {
                        match x.arg_type {
                            ArgumentKind::Flag => {
                                results.flags.insert(x.long_name.clone());
                            }
                            ArgumentKind::Option => {
                                match args.next() {
                                    Some(value) => {
                                        results.options.insert(x.long_name.clone(), value);
                                    }
                                    None => {
                                        return Err(ArgParseError::MissingValueForOption(x.long_name.clone()));
                                    }
                                }
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
                //todo iterate through all chars instead of this. See gemini last discussion.
                // some programs unify the args -> ls -l -a -t to ls -lat. So you need to lookup each char
                let argument = self.definitions
                    .iter()
                    .find(|x| {
                        let arg_as_char : Option<char> = arg_without_prefix.chars().last();
                    });

            } else {
                results.positional.push(arg)
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
