use crate::errors::CliError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum session duration
    pub max_duration: Duration,
    /// Idle timeout duration
    pub idle_timeout: Duration,
    /// Enable session persistence
    pub persistent: bool,
    /// Maximum concurrent sessions
    pub max_concurrent: usize,
    /// Require re-authentication after timeout
    pub require_reauth: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::hours(24),
            idle_timeout: Duration::minutes(30),
            persistent: false,
            max_concurrent: 5,
            require_reauth: true,
        }
    }
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub permissions: Vec<String>,
    pub is_active: bool,
}

impl Session {
    pub fn new(user_id: &str, config: &SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            created_at: now,
            last_activity: now,
            expires_at: now + config.max_duration,
            metadata: HashMap::new(),
            permissions: Vec::new(),
            is_active: true,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at || !self.is_active
    }

    /// Check if session is idle
    pub fn is_idle(&self, timeout: Duration) -> bool {
        Utc::now() - self.last_activity > timeout
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Invalidate the session
    pub fn invalidate(&mut self) {
        self.is_active = false;
    }
}

/// Session manager for handling user sessions
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    config: SessionConfig,
    auth_callbacks: Arc<RwLock<HashMap<String, Box<dyn Fn(&Session) -> bool + Send + Sync>>>>,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            auth_callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session
    pub fn create_session(&self, user_id: &str) -> Result<Session, CliError> {
        // Check concurrent session limit
        let sessions = self.sessions.read().unwrap();
        let user_sessions: Vec<_> = sessions
            .values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .collect();

        if user_sessions.len() >= self.config.max_concurrent {
            return Err(CliError::Security(format!(
                "Maximum concurrent sessions ({}) reached",
                self.config.max_concurrent
            )));
        }
        drop(sessions);

        // Create new session
        let session = Session::new(user_id, &self.config);
        
        // Store session
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.id.clone(), session.clone());

        // Persist if enabled
        if self.config.persistent {
            self.persist_session(&session)?;
        }

        Ok(session)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Session, CliError> {
        let mut sessions = self.sessions.write().unwrap();
        
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| CliError::Security("Session not found".to_string()))?;

        // Check expiration
        if session.is_expired() {
            session.invalidate();
            return Err(CliError::Security("Session expired".to_string()));
        }

        // Check idle timeout
        if session.is_idle(self.config.idle_timeout) {
            if self.config.require_reauth {
                return Err(CliError::Security("Session idle, re-authentication required".to_string()));
            }
        }

        // Update activity
        session.touch();
        
        Ok(session.clone())
    }

    /// Validate a session
    pub fn validate_session(&self, session_id: &str) -> Result<bool, CliError> {
        match self.get_session(session_id) {
            Ok(session) => {
                // Run custom validation callbacks
                let callbacks = self.auth_callbacks.read().unwrap();
                for (_, callback) in callbacks.iter() {
                    if !callback(&session) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    /// Refresh a session
    pub fn refresh_session(&self, session_id: &str) -> Result<Session, CliError> {
        let mut sessions = self.sessions.write().unwrap();
        
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| CliError::Security("Session not found".to_string()))?;

        if session.is_expired() {
            return Err(CliError::Security("Cannot refresh expired session".to_string()));
        }

        // Extend expiration
        session.expires_at = Utc::now() + self.config.max_duration;
        session.touch();

        if self.config.persistent {
            self.persist_session(session)?;
        }

        Ok(session.clone())
    }

    /// Invalidate a session
    pub fn invalidate_session(&self, session_id: &str) -> Result<(), CliError> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.invalidate();
            
            if self.config.persistent {
                self.remove_persisted_session(session_id)?;
            }
        }

        Ok(())
    }

    /// Invalidate all sessions for a user
    pub fn invalidate_user_sessions(&self, user_id: &str) -> Result<(), CliError> {
        let mut sessions = self.sessions.write().unwrap();
        
        for session in sessions.values_mut() {
            if session.user_id == user_id {
                session.invalidate();
            }
        }

        if self.config.persistent {
            self.remove_user_persisted_sessions(user_id)?;
        }

        Ok(())
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) -> Result<usize, CliError> {
        let mut sessions = self.sessions.write().unwrap();
        let initial_count = sessions.len();

        sessions.retain(|_, session| {
            !session.is_expired() && !session.is_idle(self.config.idle_timeout * 2)
        });

        let removed = initial_count - sessions.len();

        if self.config.persistent && removed > 0 {
            self.cleanup_persisted_sessions()?;
        }

        Ok(removed)
    }

    /// Add an authentication callback
    pub fn add_auth_callback<F>(&self, name: &str, callback: F)
    where
        F: Fn(&Session) -> bool + Send + Sync + 'static,
    {
        let mut callbacks = self.auth_callbacks.write().unwrap();
        callbacks.insert(name.to_string(), Box::new(callback));
    }

    /// Get active sessions count
    pub fn active_sessions_count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.values().filter(|s| s.is_active && !s.is_expired()).count()
    }

    /// Get sessions for a user
    pub fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| s.user_id == user_id && s.is_active && !s.is_expired())
            .cloned()
            .collect()
    }

    // Persistence methods
    fn persist_session(&self, session: &Session) -> Result<(), CliError> {
        let session_file = self.get_session_file_path(&session.id)?;
        let data = serde_json::to_string(session)
            .map_err(|e| CliError::Security(format!("Failed to serialize session: {}", e)))?;
        
        std::fs::write(session_file, data)
            .map_err(|e| CliError::Security(format!("Failed to persist session: {}", e)))?;
        
        Ok(())
    }

    fn remove_persisted_session(&self, session_id: &str) -> Result<(), CliError> {
        let session_file = self.get_session_file_path(session_id)?;
        if session_file.exists() {
            std::fs::remove_file(session_file)
                .map_err(|e| CliError::Security(format!("Failed to remove session file: {}", e)))?;
        }
        Ok(())
    }

    fn remove_user_persisted_sessions(&self, _user_id: &str) -> Result<(), CliError> {
        // Implementation for removing all user sessions from disk
        Ok(())
    }

    fn cleanup_persisted_sessions(&self) -> Result<(), CliError> {
        // Implementation for cleaning up expired sessions from disk
        Ok(())
    }

    fn get_session_file_path(&self, session_id: &str) -> Result<std::path::PathBuf, CliError> {
        let home = dirs::home_dir()
            .ok_or_else(|| CliError::Security("Cannot find home directory".to_string()))?;
        let session_dir = home.join(".neo-cli").join("sessions");
        std::fs::create_dir_all(&session_dir)
            .map_err(|e| CliError::Security(format!("Failed to create session directory: {}", e)))?;
        Ok(session_dir.join(format!("{}.session", session_id)))
    }

    /// Load persisted sessions on startup
    pub fn load_persisted_sessions(&self) -> Result<usize, CliError> {
        let home = dirs::home_dir()
            .ok_or_else(|| CliError::Security("Cannot find home directory".to_string()))?;
        let session_dir = home.join(".neo-cli").join("sessions");
        
        if !session_dir.exists() {
            return Ok(0);
        }

        let mut loaded = 0;
        let mut sessions = self.sessions.write().unwrap();

        for entry in std::fs::read_dir(session_dir)
            .map_err(|e| CliError::Security(format!("Failed to read session directory: {}", e)))?
        {
            let entry = entry.map_err(|e| CliError::Security(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("session") {
                let data = std::fs::read_to_string(&path)
                    .map_err(|e| CliError::Security(format!("Failed to read session file: {}", e)))?;
                
                if let Ok(session) = serde_json::from_str::<Session>(&data) {
                    if !session.is_expired() {
                        sessions.insert(session.id.clone(), session);
                        loaded += 1;
                    } else {
                        // Remove expired session file
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }

        Ok(loaded)
    }
}

/// Session guard for automatic session management
pub struct SessionGuard {
    manager: Arc<SessionManager>,
    session_id: String,
}

impl SessionGuard {
    pub fn new(manager: Arc<SessionManager>, session_id: String) -> Self {
        Self { manager, session_id }
    }

    pub fn session(&self) -> Result<Session, CliError> {
        self.manager.get_session(&self.session_id)
    }

    pub fn is_valid(&self) -> bool {
        self.manager.validate_session(&self.session_id).unwrap_or(false)
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // Touch session on guard drop to update activity
        if let Ok(mut sessions) = self.manager.sessions.write() {
            if let Some(session) = sessions.get_mut(&self.session_id) {
                session.touch();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let manager = SessionManager::new(SessionConfig::default());
        let session = manager.create_session("user123").unwrap();
        
        assert_eq!(session.user_id, "user123");
        assert!(session.is_active);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_validation() {
        let manager = SessionManager::new(SessionConfig::default());
        let session = manager.create_session("user123").unwrap();
        
        assert!(manager.validate_session(&session.id).unwrap());
        
        manager.invalidate_session(&session.id).unwrap();
        assert!(!manager.validate_session(&session.id).unwrap());
    }

    #[test]
    fn test_concurrent_session_limit() {
        let mut config = SessionConfig::default();
        config.max_concurrent = 2;
        let manager = SessionManager::new(config);
        
        let _session1 = manager.create_session("user123").unwrap();
        let _session2 = manager.create_session("user123").unwrap();
        
        let result = manager.create_session("user123");
        assert!(result.is_err());
    }
}