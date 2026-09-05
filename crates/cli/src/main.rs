fn main() {
    match audiorouter_cli::run(std::env::args().skip(1)) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("error: {error:?}");
            std::process::exit(2);
        }
    }
}
