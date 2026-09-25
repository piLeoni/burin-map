use std::process::ExitCode;

fn main() -> ExitCode {
    match burin::cli::run(std::env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("burin: {e}");
            ExitCode::FAILURE
        }
    }
}
