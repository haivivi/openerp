use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod parser;
mod ir;
mod rust_server;
mod rust_client;

#[derive(Parser)]
#[command(name = "openerp-codegen")]
#[command(about = "Generate REST API code from .api schema files")]
struct Args {
    /// Input .api schema file
    #[arg(short, long)]
    input: PathBuf,
    
    /// Output directory
    #[arg(short, long)]
    output: PathBuf,
    
    /// Target language (rust-server, rust-client, typescript-client, go-client)
    #[arg(short, long, default_value = "rust-server")]
    target: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("📖 Reading schema: {}", args.input.display());
    let input = std::fs::read_to_string(&args.input)?;
    
    println!("🔍 Parsing...");
    let schema = parser::parse(&input)?;
    
    println!("🎨 Generating {} code...", args.target);
    let code = match args.target.as_str() {
        "rust-server" => rust_server::generate(&schema)?,
        "rust-client" => rust_client::generate(&schema)?,
        _ => anyhow::bail!("Unsupported target: {}", args.target),
    };
    
    println!("💾 Writing to: {}", args.output.display());
    std::fs::create_dir_all(&args.output)?;
    std::fs::write(args.output.join("lib.rs"), code)?;
    
    println!("✅ Done!");
    Ok(())
}
