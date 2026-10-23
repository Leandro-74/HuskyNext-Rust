use std::process::ExitCode;

use huskynext::config;
use huskynext::device::{self, DeviceCandidate};
use huskynext::protocol::{
    CONNECTION_TEST_HEX, DEFAULT_SPEED, Effect, KeyboardState, MAX_BRIGHTNESS, Rgb,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("huskynext {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|arg| arg == "--list") {
        return list_devices();
    }

    if let Some(index) = args.iter().position(|arg| arg == "--test") {
        let color = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str)
            .unwrap_or(CONNECTION_TEST_HEX);
        return send_test(color);
    }

    if !args.is_empty() {
        eprintln!("Argumento desconhecido: {}", args.join(" "));
        eprintln!();
        print_help();
        return ExitCode::from(2);
    }

    match huskynext::app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Falha ao abrir a interface: {error}");
            ExitCode::FAILURE
        }
    }
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

    let saved = config::load().config.device();
    let candidate = saved
        .and_then(|saved| device::find_candidate(&candidates, saved))
        .or_else(|| choose_default_candidate(&candidates));

    let Some(candidate) = candidate else {
        eprintln!(
            "Nenhum dispositivo adequado encontrado. Use --list para inspecionar os dispositivos HID."
        );
        return ExitCode::FAILURE;
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

fn choose_default_candidate(candidates: &[DeviceCandidate]) -> Option<&DeviceCandidate> {
    candidates
        .iter()
        .find(|candidate| {
            candidate.vendor_specific
                && (candidate
                    .manufacturer
                    .as_deref()
                    .is_some_and(|value| value.to_lowercase().contains("sonix"))
                    || candidate
                        .product
                        .as_deref()
                        .is_some_and(|value| value.to_lowercase().contains("sled")))
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|candidate| candidate.vendor_specific)
        })
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
