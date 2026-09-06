fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("mcp")
        && args.get(1).map(String::as_str) == Some("serve")
    {
        if let Err(error) = audiorouter_cli::run_mcp_stdio(&args[2..]) {
            eprintln!("error: {error:?}");
            std::process::exit(2);
        }
        return;
    }
    match audiorouter_cli::run(args) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("error: {error:?}");
            std::process::exit(2);
        }
    }
}
