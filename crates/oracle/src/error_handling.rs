use std::{fmt::Display, ops::ControlFlow, time::Duration};

pub type OracleControlFlow = ControlFlow<(), Option<Duration>>;

/// Sends instructions to control the Oracle main loop flow.
///
/// When continuing, the implementor can opt to define a different duration for the sleep cycle.
pub trait MainLoopFlow {
    fn instruction(&self) -> OracleControlFlow;
}

/// Helper function to convert a slice of items into a string, for use in [`Display`] contexts.
pub fn format_slice(v: &[impl Display]) -> String {
    if v.is_empty() {
        return "[]".to_string();
    }
    let mut result = String::from("[");
    let mut iterator = v.iter().peekable();
    while let Some(value) = iterator.next() {
        let text = value.to_string();
        result.push_str(&text);
        if iterator.peek().is_some() {
            result.push_str(", ")
        } else {
            result.push(']');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_slice_empty() {
        let input: &[char] = &[];
        let expected = "[]";
        let result = format_slice(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn format_slice_single() {
        let input = &['a'];
        let expected = "[a]";
        let result = format_slice(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn format_slice_multiple() {
        let input = "abcd".chars().collect::<Vec<_>>();
        let expected = "[a, b, c, d]";
        let result = format_slice(&input);
        assert_eq!(result, expected);
    }
}
