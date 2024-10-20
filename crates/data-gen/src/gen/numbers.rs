pub fn gen_numbers() -> String {
    let parts: Vec<_> = (133..292)
        .map(|i| {
            let key = format!("{}", i)
                .chars()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(".");
            let val = english_numbers::convert_all_fmt(i)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-");

            format!("$bput bb {}. -s {}", key, val)
        })
        .collect();

    parts.join("\n")
}
