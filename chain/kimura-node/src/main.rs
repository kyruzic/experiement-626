use clap::{Parser, Subcommand};
use kimura_node::{Node, NodeConfig, NodeServices};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "kimura-node")]
#[command(about = "Kimura blockchain node")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run as leader node
    #[arg(long, global = true)]
    leader: bool,

    /// Database path
    #[arg(long, global = true, default_value = "./data")]
    db_path: PathBuf,

    /// Network listen address
    #[arg(long, global = true, default_value = "/ip4/0.0.0.0/tcp/0")]
    listen_addr: String,

    /// Leader address (required for peer mode)
    #[arg(long, global = true)]
    leader_addr: Option<String>,

    /// Block production interval in seconds (leader only)
    #[arg(long, global = true, default_value = "5")]
    block_interval_secs: u64,

    /// Log level
    #[arg(long, global = true, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the node (default command)
    Run,
    /// Submit a message to the blockchain
    Submit {
        /// Message sender identifier
        #[arg(short, long)]
        sender: String,
        /// Message content
        #[arg(short, long)]
        content: String,
    },
    /// Query blockchain state
    Query {
        /// Query type
        #[arg(value_enum)]
        query_type: QueryType,
        /// Block height (for block queries)
        #[arg(short, long)]
        height: Option<u64>,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum QueryType {
    /// Get current chain height
    Height,
    /// Get current chain hash
    Hash,
    /// Get latest block
    Latest,
    /// Get specific block by height
    Block,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = match cli.log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .compact()
        .init();

    match &cli.command {
        Some(Commands::Submit { sender, content }) => {
            submit_message(&cli, sender.clone(), content.clone()).await
        }
        Some(Commands::Query { query_type, height }) => {
            query_blockchain(&cli, query_type.clone(), *height).await
        }
        _ => run_node(cli).await,
    }
}

async fn run_node(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let config = create_config(&cli);

    info!("Starting Kimura blockchain node...");
    info!("Mode: {}", if config.is_leader { "LEADER" } else { "PEER" });
    info!("Database path: {:?}", config.db_path);
    info!("Listen address: {}", config.listen_addr);

    if let Err(e) = config.validate() {
        error!("Configuration error: {}", e);
        std::process::exit(1);
    }

    let node = match Node::new(config) {
        Ok(node) => node,
        Err(e) => {
            error!("Failed to create node: {}", e);
            std::process::exit(1);
        }
    };

    info!("Node initialized, starting main loop...");

    if let Err(e) = node.run().await {
        error!("Node error: {}", e);
        std::process::exit(1);
    }

    info!("Kimura node stopped");
    Ok(())
}

async fn submit_message(
    cli: &Cli,
    sender: String,
    content: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Submitting message...");

    let config = create_config(cli);
    let services = NodeServices::new(&config)?;

    let message = services.submit_message(sender, content)?;

    println!("Message submitted successfully!");
    println!("Message ID: {}", hex::encode(message.id));
    println!("Timestamp: {}", message.timestamp);

    Ok(())
}

async fn query_blockchain(
    cli: &Cli,
    query_type: QueryType,
    height: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = create_config(cli);
    let services = NodeServices::new(&config)?;

    match query_type {
        QueryType::Height => {
            let height = services.get_current_height()?;
            println!("Current chain height: {}", height);
        }
        QueryType::Hash => {
            let hash = services.get_current_hash()?;
            match hash {
                Some(h) => println!("Current chain hash: {}", hex::encode(&h[..16])),
                None => println!("No chain hash found (chain may be empty)"),
            }
        }
        QueryType::Latest => {
            let block = services.get_latest_block()?;
            match block {
                Some(b) => {
                    println!("Latest block:");
                    println!("  Height: {}", b.header.height);
                    println!("  Timestamp: {}", b.header.timestamp);
                    println!("  Prev Hash: {}...", hex::encode(&b.header.prev_hash[..8]));
                    println!("  Message Count: {}", b.message_ids.len());
                }
                None => println!("No blocks found (chain is empty)"),
            }
        }
        QueryType::Block => {
            let h = height.unwrap_or(0);
            let block = services.get_block(h)?;
            match block {
                Some(b) => {
                    println!("Block at height {}:", h);
                    println!("  Timestamp: {}", b.header.timestamp);
                    println!("  Prev Hash: {}...", hex::encode(&b.header.prev_hash[..8]));
                    println!("  Message Count: {}", b.message_ids.len());
                }
                None => println!("Block {} not found", h),
            }
        }
    }

    Ok(())
}

fn create_config(cli: &Cli) -> NodeConfig {
    NodeConfig {
        is_leader: cli.leader,
        db_path: cli.db_path.clone(),
        listen_addr: cli.listen_addr.clone(),
        leader_addr: cli.leader_addr.clone(),
        block_interval_secs: cli.block_interval_secs,
        log_level: cli.log_level.clone(),
    }
}
