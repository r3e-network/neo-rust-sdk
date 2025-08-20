//! Unified error handling system for improved developer experience
//! 
//! This module provides a hierarchical error system with context,
//! recovery suggestions, and better error messages.

use thiserror::Error;
use std::fmt;

/// Unified error type for the entire Neo SDK
/// 
/// Provides consistent error handling with context and recovery suggestions.
#[derive(Debug, Error)]
pub enum NeoError {
    /// Network-related errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        recovery: ErrorRecovery,
    },
    
    /// Wallet and account errors
    #[error("Wallet error: {message}")]
    Wallet {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        recovery: ErrorRecovery,
    },
    
    /// Smart contract errors
    #[error("Contract error: {message}")]
    Contract {
        message: String,
        contract: Option<String>,
        method: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        recovery: ErrorRecovery,
    },
    
    /// Transaction errors
    #[error("Transaction failed: {message}")]
    Transaction {
        message: String,
        tx_hash: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        recovery: ErrorRecovery,
    },
    
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        field: Option<String>,
        recovery: ErrorRecovery,
    },
    
    /// Validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: String,
        value: Option<String>,
        recovery: ErrorRecovery,
    },
    
    /// Insufficient funds error
    #[error("Insufficient funds: need {required} but have {available}")]
    InsufficientFunds {
        required: String,
        available: String,
        token: String,
        recovery: ErrorRecovery,
    },
    
    /// Timeout error
    #[error("Operation timed out after {duration:?}")]
    Timeout {
        duration: std::time::Duration,
        operation: String,
        recovery: ErrorRecovery,
    },
    
    /// Rate limit error
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<std::time::Duration>,
        recovery: ErrorRecovery,
    },
    
    /// Generic error with context
    #[error("{message}")]
    Other {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        recovery: ErrorRecovery,
    },
}

/// Error recovery suggestions
#[derive(Debug, Clone)]
pub struct ErrorRecovery {
    /// Suggested actions to recover from the error
    pub suggestions: Vec<String>,
    /// Whether the operation can be retried
    pub retryable: bool,
    /// Recommended retry delay if retryable
    pub retry_after: Option<std::time::Duration>,
    /// Links to documentation
    pub docs: Vec<String>,
}

impl Default for ErrorRecovery {
    fn default() -> Self {
        Self {
            suggestions: vec![],
            retryable: false,
            retry_after: None,
            docs: vec![],
        }
    }
}

impl ErrorRecovery {
    /// Create a new recovery suggestion
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a suggestion
    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
    
    /// Mark as retryable
    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
    
    /// Set retry delay
    pub fn retry_after(mut self, duration: std::time::Duration) -> Self {
        self.retry_after = Some(duration);
        self.retryable = true;
        self
    }
    
    /// Add documentation link
    pub fn doc(mut self, link: impl Into<String>) -> Self {
        self.docs.push(link.into());
        self
    }
}

impl fmt::Display for ErrorRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.suggestions.is_empty() {
            writeln!(f, "\n💡 Suggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "   • {}", suggestion)?;
            }
        }
        
        if self.retryable {
            write!(f, "\n🔄 This operation can be retried")?;
            if let Some(duration) = self.retry_after {
                write!(f, " after {:?}", duration)?;
            }
            writeln!(f)?;
        }
        
        if !self.docs.is_empty() {
            writeln!(f, "\n📚 See documentation:")?;
            for doc in &self.docs {
                writeln!(f, "   • {}", doc)?;
            }
        }
        
        Ok(())
    }
}

/// Result type alias for Neo operations
pub type Result<T> = std::result::Result<T, NeoError>;

/// Builder for creating detailed errors with context
pub struct ErrorBuilder {
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    recovery: ErrorRecovery,
    context: ErrorContext,
}

#[derive(Debug)]
enum ErrorKind {
    Network,
    Wallet,
    Contract,
    Transaction,
    Configuration,
    Validation,
    InsufficientFunds,
    Timeout,
    RateLimit,
    Other,
}

#[derive(Debug, Default)]
struct ErrorContext {
    contract: Option<String>,
    method: Option<String>,
    tx_hash: Option<String>,
    field: Option<String>,
    value: Option<String>,
    required: Option<String>,
    available: Option<String>,
    token: Option<String>,
    duration: Option<std::time::Duration>,
    operation: Option<String>,
}

impl ErrorBuilder {
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Network,
            message: message.into(),
            source: None,
            recovery: ErrorRecovery::default(),
            context: ErrorContext::default(),
        }
    }
    
    /// Create a wallet error
    pub fn wallet(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Wallet,
            message: message.into(),
            source: None,
            recovery: ErrorRecovery::default(),
            context: ErrorContext::default(),
        }
    }
    
    /// Create a contract error
    pub fn contract(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Contract,
            message: message.into(),
            source: None,
            recovery: ErrorRecovery::default(),
            context: ErrorContext::default(),
        }
    }
    
    /// Add source error
    pub fn source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
    
    /// Add contract context
    pub fn with_contract(mut self, contract: impl Into<String>) -> Self {
        self.context.contract = Some(contract.into());
        self
    }
    
    /// Add method context
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.context.method = Some(method.into());
        self
    }
    
    /// Add recovery suggestion
    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.recovery = self.recovery.suggest(suggestion);
        self
    }
    
    /// Mark as retryable
    pub fn retryable(mut self) -> Self {
        self.recovery = self.recovery.retryable(true);
        self
    }
    
    /// Build the error
    pub fn build(self) -> NeoError {
        match self.kind {
            ErrorKind::Network => NeoError::Network {
                message: self.message,
                source: self.source,
                recovery: self.recovery,
            },
            ErrorKind::Wallet => NeoError::Wallet {
                message: self.message,
                source: self.source,
                recovery: self.recovery,
            },
            ErrorKind::Contract => NeoError::Contract {
                message: self.message,
                contract: self.context.contract,
                method: self.context.method,
                source: self.source,
                recovery: self.recovery,
            },
            _ => NeoError::Other {
                message: self.message,
                source: self.source,
                recovery: self.recovery,
            },
        }
    }
}

/// Extension trait for adding context to errors
pub trait ErrorContextExt {
    /// Add context to this error
    fn context(self, message: impl Into<String>) -> NeoError;
    
    /// Add recovery suggestions
    fn recover(self, suggestion: impl Into<String>) -> NeoError;
}

impl<E> ErrorContextExt for E
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context(self, message: impl Into<String>) -> NeoError {
        NeoError::Other {
            message: message.into(),
            source: Some(Box::new(self)),
            recovery: ErrorRecovery::default(),
        }
    }
    
    fn recover(self, suggestion: impl Into<String>) -> NeoError {
        NeoError::Other {
            message: self.to_string(),
            source: Some(Box::new(self)),
            recovery: ErrorRecovery::new().suggest(suggestion),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_builder() {
        let error = ErrorBuilder::network("Connection failed")
            .suggest("Check your internet connection")
            .suggest("Try a different RPC endpoint")
            .retryable()
            .build();
        
        match error {
            NeoError::Network { recovery, .. } => {
                assert_eq!(recovery.suggestions.len(), 2);
                assert!(recovery.retryable);
            }
            _ => panic!("Wrong error type"),
        }
    }
    
    #[test]
    fn test_error_display() {
        let error = NeoError::InsufficientFunds {
            required: "100 GAS".to_string(),
            available: "50 GAS".to_string(),
            token: "GAS".to_string(),
            recovery: ErrorRecovery::new()
                .suggest("Acquire more GAS tokens")
                .suggest("Reduce the transaction amount")
                .doc("https://docs.neo.org/tokens/gas"),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Insufficient funds"));
        assert!(display.contains("need 100 GAS but have 50 GAS"));
    }
}