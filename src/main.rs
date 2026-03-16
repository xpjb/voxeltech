use interestingtech::app;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let single = args
        .iter()
        .position(|a| a == "--single")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u8>().ok())
        .filter(|&n| n < 10);
    app::run(single);
}
