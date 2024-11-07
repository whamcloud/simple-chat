//! This module contains functionalities which are common to both the server and the client.

pub mod messages;

// Extract various parts from the string message
pub fn extract_parts(line: &str) -> (u16, String, String) {
    let lines = line.split(" ").collect::<Vec<&str>>();
    let command = lines[0].trim().to_lowercase();

    let mut data = String::new();
    let mut username = String::new();

    match lines.len() {
        2 => {
            data = line[command.len()..].trim().to_string();
        }
        3.. => {
            username = lines[1].trim().to_lowercase();
            data = line[command.len() + username.len() + 2..]
                .trim()
                .to_string();
        }
        _ => {}
    }

    let command = command
        .strip_prefix("<")
        .unwrap()
        .strip_suffix(">")
        .unwrap();
    // We can trust that command is something we can parse to u16. So we can use unwrap() here safely.
    (command.parse::<u16>().unwrap(), username, data)
}

#[cfg(test)]
mod tests {

    use super::*;

    // Test extraction of message with more than three parts. This should return 3 parts.
    #[test]
    fn test_extract_more_than_three_parts() {
        let input = "<107> testuser Hey this is sample message from a testuser";
        let result = extract_parts(input);
        assert_eq!(result.0, 107);
        assert_eq!(result.1, "testuser");
        assert_eq!(result.2, "Hey this is sample message from a testuser");
    }

    // Test extraction of message with three parts
    #[test]
    fn test_extract_three_parts() {
        let input = "<107> testuser Hey";
        let result = extract_parts(input);
        assert_eq!(result.0, 107);
        assert_eq!(result.1, "testuser");
        assert_eq!(result.2, "Hey");
    }

    // Test extraction of message with two parts
    #[test]
    fn test_extract_two_parts() {
        let input = "<101> testuser";
        let result = extract_parts(input);
        println!("{:?}", &result);
        assert_eq!(result.0, 101);
        assert_eq!(result.1, "");
        assert_eq!(result.2, "testuser");
    }

    // Test extraction of message with two parts
    #[test]
    fn test_extract_one_part() {
        let input = "<103>";
        let result = extract_parts(input);
        println!("{:?}", &result);
        assert_eq!(result.0, 103);
        assert_eq!(result.1, "");
        assert_eq!(result.2, "");
    }
}
