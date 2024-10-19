// utilities parsing a command
use regex::Regex;

use super::errors::ParseResult;

pub fn split_by_pipe_and_space(cmd: &str) -> Vec<Vec<String>> {
    // 1. 将字符串按照pipe character `|` 分割，存储在Vec<String>
    let parts: Vec<String> = cmd.split('|').map(|s| s.trim().to_string()).collect();

    // 2. 将vec中的每个字符串去除头尾空格之后，按照空格分割，得到Vec<Vec<String>>
    let result: Vec<Vec<String>> = parts
        .iter()
        .map(|part| part.split_whitespace().map(|s| s.to_string()).collect())
        .collect();

    result
}

pub fn split_by_whitespace(input: &str) -> Vec<String> {
    input.split_whitespace().map(|s| s.to_string()).collect()
}

pub fn split_first_two_spaces(input: &str) -> Vec<String> {
    // 定义正则表达式，匹配前两个连续的空格
    let re = Regex::new(r"\s+").unwrap();

    // 使用正则表达式分割字符串
    let mut splits = re.splitn(input, 3);

    // 收集分割结果
    let mut result = Vec::new();
    if let Some(first) = splits.next() {
        result.push(first.to_string());
    }
    if let Some(second) = splits.next() {
        result.push(second.to_string());
    }
    if let Some(rest) = splits.next() {
        result.push(rest.to_string());
    }

    result
}

pub fn split_first_space(input: &str) -> Vec<String> {
    // 定义正则表达式，匹配前两个连续的空格
    let re = Regex::new(r"\s+").unwrap();

    // 使用正则表达式分割字符串
    let mut splits = re.splitn(input, 2);

    // 收集分割结果
    let mut result = Vec::new();

    if let Some(first) = splits.next() {
        result.push(first.to_string());
    }
    if let Some(rest) = splits.next() {
        result.push(rest.to_string());
    }

    result
}

pub fn enforce_batch_identifier(ident: &str) -> ParseResult<()> {
    // 创建一个正则表达式，匹配大小写字母和数字
    let re = Regex::new(r"^[a-zA-Z0-9]+$").unwrap();

    // 使用正则表达式匹配字符串
    if re.is_match(ident) {
        Ok(())
    } else {
        Err(super::errors::ParseError::InvalidIdentifier {
            ident: ident.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::split_first_space;

    #[test]
    fn test_split() {
        let a = "  sHello     world  ".trim();
        let res = split_first_space(a);
        dbg!(res);

        let a = "  sHello    ".trim();
        let res = split_first_space(a);
        dbg!(res);
    }
}
