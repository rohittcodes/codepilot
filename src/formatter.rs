use serde_json::Value;
use regex::Regex;

pub struct ResponseFormatter;

impl ResponseFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Formats response text to be more user-friendly for AI application display
    pub fn format_response(&self, response: &str) -> String {
        let cleaned = self.clean_markdown(response);
        let formatted = self.format_json_blocks(&cleaned);
        let final_text = self.format_code_blocks(&formatted);
        self.wrap_and_format(&final_text)
    }

    /// Remove markdown formatting and convert to plain text
    fn clean_markdown(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove markdown headers (### ## #)
        let header_regex = Regex::new(r"^#{1,6}\s*(.*)$").unwrap();
        result = header_regex.replace_all(&result, "$1").to_string();

        // Remove bold/italic markers (**text** or *text*)
        let bold_regex = Regex::new(r"\*\*([^*]+)\*\*").unwrap();
        result = bold_regex.replace_all(&result, "$1").to_string();
        
        let italic_regex = Regex::new(r"\*([^*]+)\*").unwrap();
        result = italic_regex.replace_all(&result, "$1").to_string();

        // Remove inline code markers (`code`)
        let inline_code_regex = Regex::new(r"`([^`]+)`").unwrap();
        result = inline_code_regex.replace_all(&result, "$1").to_string();

        // Remove code block markers (```language or ```)
        let code_block_regex = Regex::new(r"```\w*\n?").unwrap();
        result = code_block_regex.replace_all(&result, "").to_string();

        result
    }

    /// Format JSON blocks to be more readable
    fn format_json_blocks(&self, text: &str) -> String {
        // Try to parse and pretty-print JSON objects
        let json_regex = Regex::new(r"(\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})").unwrap();
        
        json_regex.replace_all(text, |caps: &regex::Captures| {
            let json_str = &caps[1];
            match serde_json::from_str::<Value>(json_str) {
                Ok(value) => {
                    self.format_json_value(&value, 0)
                }
                Err(_) => json_str.to_string()
            }
        }).to_string()
    }

    /// Recursively format JSON values with proper indentation
    fn format_json_value(&self, value: &Value, indent_level: usize) -> String {
        let indent = "  ".repeat(indent_level);
        let next_indent = "  ".repeat(indent_level + 1);

        match value {
            Value::Object(map) => {
                if map.is_empty() {
                    return "{}".to_string();
                }

                let mut result = String::new();
                for (i, (key, val)) in map.iter().enumerate() {
                    if i == 0 {
                        result.push_str(&format!("{}: {}", key, self.format_json_value(val, indent_level + 1)));
                    } else {
                        result.push_str(&format!("\n{}{}: {}", next_indent, key, self.format_json_value(val, indent_level + 1)));
                    }
                }
                result
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    return "[]".to_string();
                }
                
                let items: Vec<String> = arr.iter()
                    .map(|v| self.format_json_value(v, indent_level))
                    .collect();
                
                if items.len() <= 3 && items.iter().all(|s| s.len() <= 20) {
                    format!("[{}]", items.join(", "))
                } else {
                    format!("[\n{}{}]", 
                        items.iter()
                            .map(|s| format!("{}{}", next_indent, s))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        indent
                    )
                }
            }
            Value::String(s) => {
                if s.len() > 50 {
                    format!("\"{}...\"", &s[..47])
                } else {
                    format!("\"{}\"", s)
                }
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
        }
    }

    /// Format code blocks with proper indentation
    fn format_code_blocks(&self, text: &str) -> String {
        // Add proper spacing around code-like content
        let mut result = text.to_string();
        
        // Add spacing around colons and equal signs for better readability
        result = result.replace(":", ": ");
        result = result.replace("  :", ": "); // Fix double spaces
        result = result.replace("= ", " = ");
        
        result
    }

    /// Wrap text and add proper formatting for AI app display
    fn wrap_and_format(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut formatted_lines = Vec::new();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                formatted_lines.push("".to_string());
                continue;
            }

            // Handle long lines by wrapping them
            if trimmed.len() > 80 {
                let wrapped = self.wrap_line(trimmed, 80);
                formatted_lines.extend(wrapped);
            } else {
                formatted_lines.push(trimmed.to_string());
            }
        }
        
        // Remove excessive empty lines
        let mut result = Vec::new();
        let mut prev_empty = false;
        
        for line in formatted_lines {
            if line.trim().is_empty() {
                if !prev_empty {
                    result.push(line);
                    prev_empty = true;
                }
            } else {
                result.push(line);
                prev_empty = false;
            }
        }
        
        result.join("\n")
    }

    /// Wrap a single line to specified width
    fn wrap_line(&self, line: &str, width: usize) -> Vec<String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut wrapped = Vec::new();
        let mut current_line = String::new();
        
        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + word.len() + 1 <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                wrapped.push(current_line.clone());
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            wrapped.push(current_line);
        }
        
        wrapped
    }

    /// Format different types of responses based on their content
    pub fn format_agent_response(&self, agent_name: &str, response: &str) -> String {
        let formatted = self.format_response(response);
        
        // Add agent-specific formatting
        let header = match agent_name {
            "linear" => "Linear Agent:",
            "github" => "GitHub Agent:",
            "supabase" => "Supabase Agent:",
            "orchestrator" => "Orchestrator:",
            _ => "Agent:",
        };
        
        format!("{}\n{}", header, formatted)
    }

    /// Format error messages
    pub fn format_error(&self, error: &str) -> String {
        format!("Error: {}", self.format_response(error))
    }

    /// Format success messages
    pub fn format_success(&self, message: &str) -> String {
        format!("Success: {}", self.format_response(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatting() {
        let formatter = ResponseFormatter::new();
        let json_text = r#"{"name":"test","value":123,"nested":{"key":"value"}}"#;
        let formatted = formatter.format_response(json_text);
        assert!(formatted.contains("name: \"test\""));
    }

    #[test]
    fn test_markdown_cleaning() {
        let formatter = ResponseFormatter::new();
        let markdown = "## Header\n**bold text** and *italic* with `code`";
        let cleaned = formatter.clean_markdown(markdown);
        assert!(!cleaned.contains("##"));
        assert!(!cleaned.contains("**"));
        assert!(!cleaned.contains("`"));
    }
}
