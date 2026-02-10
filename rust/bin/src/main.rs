use std::env;

use core::ServiceConfig;

const VERSION: &str = "0.0.1";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // Handle --version / --help early.
    for arg in &args {
        if arg == "--version" || arg == "-V" {
            println!("openerp {}", VERSION);
            return;
        }
        if arg == "--help" || arg == "-h" {
            print_usage();
            return;
        }
    }

    let config = ServiceConfig::from_args(&args);

    println!("OpenERP v{}", VERSION);
    println!("  listen:     {}", config.listen);
    if let Some(ref dir) = config.data_dir {
        println!("  data-dir:   {}", dir.display());
    }
    println!("  db:         {}", config.resolve_db_path().display());
    println!("  sqlite:     {}", config.resolve_sqlite_path().display());
    println!("  search-dir: {}", config.resolve_search_dir().display());
    println!("  blob-dir:   {}", config.resolve_blob_dir().display());
    println!("  tsdb-dir:   {}", config.resolve_tsdb_dir().display());
    println!();
    println!("No modules registered yet. Exiting.");
}

fn print_usage() {
    println!("openerp {}", VERSION);
    println!();
    println!("USAGE:");
    println!("    openerp [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --data-dir=PATH     Static configuration directory");
    println!("    --db=PATH           redb database path");
    println!("    --sqlite=PATH       SQLite database path");
    println!("    --search-dir=PATH   Tantivy search index directory");
    println!("    --blob-dir=PATH     Blob storage directory");
    println!("    --tsdb-dir=PATH     TSDB WAL/archive directory");
    println!("    --listen=ADDR       HTTP listen address (default: 0.0.0.0:8080)");
    println!("    --version, -V       Print version");
    println!("    --help, -h          Print this help");
}
