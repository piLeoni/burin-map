use std::process::ExitCode;

fn main() -> ExitCode {
    match burin_map::cli::run(std::env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let red = anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::AnsiColor::Red.into()));
            anstream::eprintln!("\n{red}error{red:#} {e}");
            ExitCode::FAILURE
        }
    }
}
