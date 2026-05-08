#[derive(Debug)]
pub struct CommandLineArgs {
    pub chars_per_min: u64,
    pub game_length_sec: u64,
    pub letter_multiple: u64,
    pub symbol_multiple: u64,
}

impl CommandLineArgs {
    const CHARS_PER_MIN_DEF: u64 = 400;
    const CHARS_PER_MIN_MIN: u64 = 100;
    const CHARS_PER_MIN_MAX: u64 = 2000;

    const GAME_LENGTH_SEC_DEF: u64 = 3 * 60;
    const GAME_LENGTH_SEC_MIN: u64 = 30;
    const GAME_LENGTH_SEC_MAX: u64 = 10 * 60;

    const LETTER_MULTIPLE_DEF: u64 = 8;
    const LETTER_MULTIPLE_MIN: u64 = 1;
    const LETTER_MULTIPLE_MAX: u64 = 20;

    const SYMBOL_MULTIPLE_DEF: u64 = 1;
    const SYMBOL_MULTIPLE_MIN: u64 = 1;
    const SYMBOL_MULTIPLE_MAX: u64 = 20;
}

type CLA = CommandLineArgs;

impl Default for CommandLineArgs {
    fn default() -> Self {
        CommandLineArgs {
            chars_per_min: Self::CHARS_PER_MIN_DEF,
            game_length_sec: Self::GAME_LENGTH_SEC_DEF,
            letter_multiple: Self::LETTER_MULTIPLE_DEF,
            symbol_multiple: Self::SYMBOL_MULTIPLE_DEF,
        }
    }
}

pub fn parse_args() -> CommandLineArgs {
    let mut config = CommandLineArgs::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--chars-per-min" | "-c" => {
                if let Some(val) = args.next() {
                    config.chars_per_min = parse_in_range(
                        &val,
                        CLA::CHARS_PER_MIN_MIN,
                        CLA::CHARS_PER_MIN_MAX,
                        "characters per minute",
                        &format!("{}..={}", CLA::CHARS_PER_MIN_MIN, CLA::CHARS_PER_MIN_MAX),
                    );
                }
            }
            "--game-length-sec" | "-g" => {
                if let Some(val) = args.next() {
                    config.game_length_sec = parse_in_range(
                        &val,
                        CLA::GAME_LENGTH_SEC_MIN,
                        CLA::GAME_LENGTH_SEC_MAX,
                        "game length seconds",
                        &format!("{}..={}", CLA::GAME_LENGTH_SEC_MIN, CLA::GAME_LENGTH_SEC_MAX),
                    );
                }
            }
            "--letter-frequency" | "-l" => {
                if let Some(val) = args.next() {
                    config.letter_multiple = parse_in_range(
                        &val,
                        CLA::LETTER_MULTIPLE_MIN,
                        CLA::LETTER_MULTIPLE_MAX,
                        "letter frequency",
                        &format!("{}..={}", CLA::LETTER_MULTIPLE_MIN, CLA::LETTER_MULTIPLE_MAX),
                    );
                }
            }
            "--symbol-frequency" | "-s" => {
                if let Some(val) = args.next() {
                    config.symbol_multiple = parse_in_range(
                        &val,
                        CLA::SYMBOL_MULTIPLE_MIN,
                        CLA::SYMBOL_MULTIPLE_MAX,
                        "symbol frequency",
                        &format!("{}..={}", CLA::LETTER_MULTIPLE_MIN, CLA::LETTER_MULTIPLE_MAX),
                    );
                }
            }
            "--help" | "-h" => {
                CLA::print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {arg}");
                eprintln!("Use --help for usage information.");
                std::process::exit(1);
            }
        }
    }

    config
}

fn parse_in_range<T: std::str::FromStr>(
    val: &str,
    min: T,
    max: T,
    name: &str,
    range_str: &str,
) -> T
where
    T: std::fmt::Display + PartialOrd + Copy,
{
    match val.parse::<T>() {
        Ok(n) if n >= min && n <= max => n,
        Ok(n) => {
            eprintln!(
                "Error: {name} must be between {min} and {max} (got {n})"
            );
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Error: {name} must be a valid integer ({range_str})");
            std::process::exit(1);
        }
    }
}

impl CommandLineArgs{

    fn print_help() {
        println!("QWENCH - Terminal typing game");
        println!();
        println!("Set letter-multiple and symbol-multiple to adjust the ratio of symbols to letters.");
        println!();
        println!("Usage: qwench [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -c, --chars-per-min       Characters per minute              (default: {}, range: {}..={})",
            Self::CHARS_PER_MIN_DEF, Self::CHARS_PER_MIN_MIN, Self::CHARS_PER_MIN_MAX);
        println!("  -g, --game-length-sec     Game length in seconds             (default: {}, range: {}..={})",
            Self::GAME_LENGTH_SEC_DEF, Self::GAME_LENGTH_SEC_MIN, Self::GAME_LENGTH_SEC_MAX);
        println!("  -l, --letter-multiple     Letter frequency                   (default: {}, range: {}..={})",
            Self::LETTER_MULTIPLE_DEF, Self::LETTER_MULTIPLE_MIN, Self::LETTER_MULTIPLE_MAX);
        println!("  -s, --symbol-multiple     Symbol frequency                   (default: {}, range: {}..={})",
            Self::SYMBOL_MULTIPLE_DEF, Self::SYMBOL_MULTIPLE_MIN, Self::SYMBOL_MULTIPLE_MAX);
        println!("  -h, --help                Show this help");
        println!();
        println!("Example:");
        println!("  qwench --chars-per-minute {} --game-length-sec {} --letter-multiple {} --symbol-multiple {}",
            400, 240, 4, 3);
    }
}
