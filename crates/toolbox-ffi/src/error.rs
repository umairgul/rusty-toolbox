use std::fmt;

/// A failure with a message ready to show the user.
#[derive(Debug, uniffi::Error)]
pub enum ToolError {
    Failed { message: String },
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::Failed { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for ToolError {}

impl ToolError {
    pub fn new(message: impl fmt::Display) -> Self {
        ToolError::Failed {
            message: message.to_string(),
        }
    }
}
