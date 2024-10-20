pub fn gen_complex() -> String {
    let parts: Vec<_> = (1443..1592)
        .map(|re| {
            let im = re + 929;

            let key = format!("{}", re)
                .chars()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(".");
            let val = format!("{}+{}i", re, im);

            format!("$bput bb {}. -s {}", key, val)
        })
        .collect();

    parts.join("\n")
}
