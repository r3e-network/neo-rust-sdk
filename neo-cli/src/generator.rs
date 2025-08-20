//! Code generation module for Neo dApp templates
//! 
//! Provides functionality to generate new projects from templates

use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Template {
    pub template: TemplateMetadata,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
}

/// Available project templates
#[derive(Debug)]
pub enum ProjectTemplate {
    BasicDapp,
    Nep17Token,
    NftCollection,
    DefiProtocol,
    OracleConsumer,
}

impl ProjectTemplate {
    /// Get the template file path
    fn template_path(&self) -> &str {
        match self {
            ProjectTemplate::BasicDapp => "templates/basic_dapp.toml",
            ProjectTemplate::Nep17Token => "templates/nep17_token.toml",
            ProjectTemplate::NftCollection => "templates/nft_collection.toml",
            ProjectTemplate::DefiProtocol => "templates/defi_protocol.toml",
            ProjectTemplate::OracleConsumer => "templates/oracle_consumer.toml",
        }
    }
    
    /// Get template display name
    pub fn display_name(&self) -> &str {
        match self {
            ProjectTemplate::BasicDapp => "Basic Neo dApp",
            ProjectTemplate::Nep17Token => "NEP-17 Token",
            ProjectTemplate::NftCollection => "NFT Collection (NEP-11)",
            ProjectTemplate::DefiProtocol => "DeFi Protocol",
            ProjectTemplate::OracleConsumer => "Oracle Consumer",
        }
    }
}

/// Generate a new project from a template
pub fn generate_project(
    template_type: ProjectTemplate,
    project_name: &str,
    target_dir: Option<PathBuf>,
) -> Result<()> {
    println!("🔧 {} project: {}", 
        "Generating".cyan(), 
        project_name.green()
    );
    
    // Determine target directory
    let target = target_dir
        .unwrap_or_else(|| PathBuf::from("."))
        .join(project_name);
    
    // Check if directory already exists
    if target.exists() {
        return Err(anyhow::anyhow!(
            "Directory '{}' already exists", 
            target.display()
        ));
    }
    
    // Load template
    let template = load_template(&template_type)?;
    
    // Create project directory
    fs::create_dir_all(&target)
        .context("Failed to create project directory")?;
    
    // Generate files from template
    for (file_path, content) in &template.files {
        generate_file(&target, file_path, content, project_name)?;
    }
    
    println!("✅ {} created successfully!", 
        format!("Project '{}'", project_name).green().bold()
    );
    
    // Print next steps
    print_next_steps(project_name, &template_type);
    
    Ok(())
}

/// Load a template from file
fn load_template(template_type: &ProjectTemplate) -> Result<Template> {
    let template_path = template_type.template_path();
    
    // For embedded templates, we'll use a match statement
    // In production, these would be loaded from files
    let template_content = match template_type {
        ProjectTemplate::BasicDapp => {
            include_str!("../../templates/basic_dapp.toml")
        }
        ProjectTemplate::Nep17Token => {
            include_str!("../../templates/nep17_token.toml")
        }
        _ => {
            // For templates not yet created, return a default
            return Ok(create_default_template(template_type));
        }
    };
    
    toml::from_str(template_content)
        .context("Failed to parse template")
}

/// Create a default template for types not yet implemented
fn create_default_template(template_type: &ProjectTemplate) -> Template {
    let mut files = HashMap::new();
    
    // Basic structure for all templates
    files.insert(
        "src/main.rs".to_string(),
        format!(
            r#"//! {} Project
            
use neo3::sdk::Neo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    println!("🚀 {} Project");
    
    // Connect to Neo TestNet
    let neo = Neo::testnet().await?;
    println!("✅ Connected to Neo TestNet");
    
    Ok(())
}}
"#,
            template_type.display_name(),
            template_type.display_name()
        ),
    );
    
    files.insert(
        "Cargo.toml".to_string(),
        r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
neo3 = "0.5.0"
tokio = { version = "1.45", features = ["full"] }
"#.to_string(),
    );
    
    files.insert(
        "README.md".to_string(),
        format!("# {{{{project_name}}}}\n\n{} project built with NeoRust SDK.\n", 
            template_type.display_name()
        ),
    );
    
    Template {
        template: TemplateMetadata {
            name: template_type.display_name().to_string(),
            description: format!("{} template", template_type.display_name()),
            version: "1.0.0".to_string(),
            author: "NeoRust Team".to_string(),
        },
        files,
    }
}

/// Generate a single file from template
fn generate_file(
    target_dir: &Path,
    file_path: &str,
    content: &str,
    project_name: &str,
) -> Result<()> {
    let file_path = target_dir.join(file_path);
    
    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create directory")?;
    }
    
    // Replace template variables
    let content = content.replace("{{project_name}}", project_name);
    
    // Write file
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
    
    println!("  📄 Created: {}", 
        file_path.strip_prefix(target_dir)
            .unwrap_or(&file_path)
            .display()
            .to_string()
            .dimmed()
    );
    
    Ok(())
}

/// Print next steps after project generation
fn print_next_steps(project_name: &str, template_type: &ProjectTemplate) {
    println!("\n{}", "Next steps:".cyan().bold());
    println!("  1. cd {}", project_name.green());
    println!("  2. cargo build");
    println!("  3. cargo test");
    
    match template_type {
        ProjectTemplate::Nep17Token => {
            println!("  4. Compile contract: neo3-boa contracts/token.py");
            println!("  5. Deploy contract: neo-cli contract deploy");
        }
        ProjectTemplate::NftCollection => {
            println!("  4. Design your NFT metadata");
            println!("  5. Deploy NFT contract");
        }
        ProjectTemplate::DefiProtocol => {
            println!("  4. Configure liquidity pools");
            println!("  5. Deploy DeFi contracts");
        }
        _ => {
            println!("  4. Configure your application");
            println!("  5. Deploy to Neo blockchain");
        }
    }
    
    println!("\n📚 Documentation: {}", 
        "https://github.com/R3E-Network/NeoRust".blue()
    );
}

/// List available templates
pub fn list_templates() {
    println!("{}", "Available Project Templates:".cyan().bold());
    println!();
    
    let templates = vec![
        (ProjectTemplate::BasicDapp, "General purpose blockchain application"),
        (ProjectTemplate::Nep17Token, "Fungible token following NEP-17 standard"),
        (ProjectTemplate::NftCollection, "Non-fungible token collection (NEP-11)"),
        (ProjectTemplate::DefiProtocol, "Decentralized finance protocol"),
        (ProjectTemplate::OracleConsumer, "Application that consumes oracle data"),
    ];
    
    for (template, description) in templates {
        println!("  {} {}", 
            format!("• {}", template.display_name()).green().bold(),
            format!("- {}", description).dimmed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_project_generation() {
        let temp_dir = TempDir::new().unwrap();
        let project_name = "test_project";
        
        let result = generate_project(
            ProjectTemplate::BasicDapp,
            project_name,
            Some(temp_dir.path().to_path_buf()),
        );
        
        assert!(result.is_ok());
        
        // Check that project directory was created
        let project_dir = temp_dir.path().join(project_name);
        assert!(project_dir.exists());
        
        // Check that main.rs was created
        let main_file = project_dir.join("src").join("main.rs");
        assert!(main_file.exists());
        
        // Check that Cargo.toml was created
        let cargo_file = project_dir.join("Cargo.toml");
        assert!(cargo_file.exists());
    }
    
    #[test]
    fn test_template_variable_replacement() {
        let content = "name = \"{{project_name}}\"";
        let replaced = content.replace("{{project_name}}", "my_project");
        assert_eq!(replaced, "name = \"my_project\"");
    }
}