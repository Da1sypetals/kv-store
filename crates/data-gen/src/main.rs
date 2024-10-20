fn main() {
    for i in 133..292 {
        let key = format!("{}", i)
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(".");
        let val = english_numbers::convert_all_fmt(i)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");

        let cmd = format!("$bput bb {}. -s {}", key, val);
        println!("{}", cmd);
    }
}
