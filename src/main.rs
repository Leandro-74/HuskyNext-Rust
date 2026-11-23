use std::process::ExitCode;

use huskynext::config;
use huskynext::device;
use huskynext::protocol::{
    CONNECTION_TEST_HEX, DEFAULT_SPEED, Effect, KeyboardState, MAX_BRIGHTNESS, Rgb,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_args(&args) {
        Ok(Command::Help) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("huskynext {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::List) => list_devices(),
        Ok(Command::Test(color)) => send_test(color.as_deref().unwrap_or(CONNECTION_TEST_HEX)),
        Ok(Command::Gui) => match huskynext::app::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Falha ao abrir a interface: {error}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("{message}");
            eprintln!();
            print_help();
            ExitCode::from(2)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Gui,
    List,
    Test(Option<String>),
    Help,
    Version,
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Command::Help);
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        return Ok(Command::Version);
    }

    if args.iter().any(|arg| arg == "--list") {
        return Ok(Command::List);
    }

    if let Some(index) = args.iter().position(|arg| arg == "--test") {
        let color = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .cloned();
        return Ok(Command::Test(color));
    }

    if !args.is_empty() {
        return Err(format!("Argumento desconhecido: {}", args.join(" ")));
    }

    Ok(Command::Gui)
}

fn list_devices() -> ExitCode {
    match device::discover() {
        Ok(candidates) => {
            if candidates.is_empty() {
                println!("Nenhum dispositivo HID encontrado.");
            } else {
                for (index, candidate) in candidates.iter().enumerate() {
                    println!(
                        "[{}] {} | {} | {} | {}",
                        index + 1,
                        candidate.display_name(),
                        candidate.ids(),
                        if candidate.vendor_specific {
                            "vendor-specific"
                        } else {
                            "outro HID"
                        },
                        candidate.path
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Falha ao enumerar dispositivos: {error}");
            ExitCode::FAILURE
        }
    }
}

fn send_test(color: &str) -> ExitCode {
    let color = match Rgb::from_hex(color) {
        Ok(color) => color,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let candidates = match device::discover() {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!("Falha ao enumerar dispositivos: {error}");
            return ExitCode::FAILURE;
        }
    };

    let loaded = config::load();
    if let Some(warning) = &loaded.warning {
        eprintln!("{warning}");
    }
    let saved = loaded.config.device();

    let candidate = match device::select_for_test(&candidates, saved) {
        Ok(candidate) => candidate,
        Err(error) => {
            eprintln!("{error}");
            if let Some(hint) = error.user_hint() {
                eprintln!();
                eprintln!("{hint}");
            }
            return ExitCode::FAILURE;
        }
    };

    let state = KeyboardState {
        effect: Effect::Static,
        brightness: MAX_BRIGHTNESS,
        speed: DEFAULT_SPEED,
        color,
    };

    match device::write_to_candidate(candidate, &state.report()) {
        Ok(sent) => {
            println!(
                "{sent} bytes enviados para {} ({}) com cor {}.",
                candidate.display_name(),
                candidate.ids(),
                color.to_hex()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            if let Some(hint) = error.user_hint() {
                eprintln!();
                eprintln!("{hint}");
            }
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "HuskyNext {version}

Uso:
  huskynext                 Abre a interface gráfica iced
  huskynext --list          Lista os dispositivos HID detectados
  huskynext --test [HEX]    Envia cor estática a 100% (padrão 8835E9)
  huskynext --version       Mostra a versão
  huskynext --help          Mostra esta ajuda

Exemplos:
  huskynext --test 00FF88
  huskynext --list

Sem --test, o dispositivo salvo em ~/.huskynext/config.json é usado.
A configuração e o protocolo não são oficiais e podem variar por firmware.",
        version = env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_args_without_arguments_opens_gui() {
        assert_eq!(parse_args(&[]), Ok(Command::Gui));
    }

    #[test]
    fn parse_args_help_wins_over_everything() {
        assert_eq!(parse_args(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse_args(&args(&["-h"])), Ok(Command::Help));
        assert_eq!(
            parse_args(&args(&["--version", "--test", "FF0000", "--help"])),
            Ok(Command::Help)
        );
    }

    #[test]
    fn parse_args_version_wins_over_list_and_test() {
        assert_eq!(parse_args(&args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse_args(&args(&["-V"])), Ok(Command::Version));
        assert_eq!(
            parse_args(&args(&["--list", "--test", "FF0000", "--version"])),
            Ok(Command::Version)
        );
    }

    #[test]
    fn parse_args_list_wins_over_test() {
        assert_eq!(
            parse_args(&args(&["--test", "FF0000", "--list"])),
            Ok(Command::List)
        );
    }

    #[test]
    fn parse_args_test_accepts_optional_hex_color() {
        assert_eq!(
            parse_args(&args(&["--test", "00FF88"])),
            Ok(Command::Test(Some("00FF88".to_string())))
        );
        assert_eq!(parse_args(&args(&["--test"])), Ok(Command::Test(None)));
        assert_eq!(
            parse_args(&args(&["--test", "-x"])),
            Ok(Command::Test(None))
        );
    }

    #[test]
    fn parse_args_rejects_unknown_arguments() {
        assert_eq!(
            parse_args(&args(&["--bogus"])),
            Err("Argumento desconhecido: --bogus".to_string())
        );
    }

    #[test]
    fn parse_args_ignores_unknown_arguments_after_test() {
        assert_eq!(
            parse_args(&args(&["--test", "00FF88", "--bogus"])),
            Ok(Command::Test(Some("00FF88".to_string())))
        );
    }
}
