use mas_core::{
    agent_system::{AgentSystem, DelayedHandler, EchoHandler, SinkHandler},
    llm::{CompletionOptions, LlmHandler, LlmProvider, OllamaProvider},
    load_system_from_json, AgentBuilder, Message, MessageHandler, SendResult, SystemConfig,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{info, warn, Level};

/// A custom handler that simulates research work
struct ResearcherHandler;

#[async_trait]
impl MessageHandler for ResearcherHandler {
    async fn handle(&self, message: &Message, agent: &mas_core::Agent) -> Option<String> {
        info!("[{}] Researching: {}", agent.name, message.content);
        tokio::time::sleep(Duration::from_millis(100)).await;
        Some(format!(
            "Research findings for '{}': [simulated data]",
            message.content
        ))
    }
}

/// A custom handler that simulates analysis work
struct AnalystHandler;

#[async_trait]
impl MessageHandler for AnalystHandler {
    async fn handle(&self, message: &Message, agent: &mas_core::Agent) -> Option<String> {
        info!("[{}] Analyzing: {}", agent.name, message.content);
        tokio::time::sleep(Duration::from_millis(150)).await;
        Some(format!(
            "Analysis of '{}': [simulated insights]",
            message.content
        ))
    }
}

/// Parse a CLI argument value (the string after a flag like --config)
fn get_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments first to check for --quiet
    let args: Vec<String> = std::env::args().collect();
    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    // Set log level based on quiet flag
    let log_level = if quiet { Level::WARN } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    info!("Starting Multi-Agent System Demo");

    // Check for --config flag
    let config_path = get_arg_value(&args, "--config").map(PathBuf::from);
    let prompt = get_arg_value(&args, "--prompt");
    let target_agent = get_arg_value(&args, "--to");

    let run_llm_demo = args.iter().any(|a| a == "--llm");
    let dry_run = args.iter().any(|a| a == "--dry-run");

    if let Some(path) = config_path {
        if let Some(prompt_text) = prompt {
            // Run with prompt
            run_prompt(&path, &prompt_text, target_agent.as_deref()).await?;
        } else if dry_run {
            // Just validate
            run_dry_run(&path)?;
        } else {
            // Interactive mode (wait for Ctrl+C)
            run_from_config(&path).await?;
        }
    } else if run_llm_demo {
        run_llm_routing_demo().await?;
    } else {
        run_basic_demo_scenarios().await?;
        print_usage();
    }

    Ok(())
}

fn print_usage() {
    info!("\nUsage:");
    info!("  cargo run -- --config <file.json>                    Load and run system");
    info!("  cargo run -- --config <file.json> --dry-run          Validate config only");
    info!("  cargo run -- --config <file.json> --prompt \"Hello\"   Send a prompt");
    info!("  cargo run -- --config <file.json> --prompt \"Hi\" --to Agent  Send to specific agent");
    info!("  cargo run -- --config <file.json> --prompt \"Hi\" -q   Quiet mode (response only)");
    info!("  cargo run -- --llm                                   Run LLM routing demo");
}

/// Validate config without running (dry-run mode)
fn run_dry_run(path: &PathBuf) -> anyhow::Result<()> {
    info!("Loading configuration from: {}", path.display());
    match mas_core::parse_config_file(path) {
        Ok(config) => {
            info!("Configuration is valid!");
            info!("  Providers: {:?}", config.llm_providers.keys().collect::<Vec<_>>());
            info!("  Agents: {:?}", config.agents.iter().map(|a| &a.name).collect::<Vec<_>>());
            info!("  Global timeout: {}s", config.system.global_timeout_secs);
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Send a prompt to an agent and print the response
async fn run_prompt(path: &PathBuf, prompt: &str, target: Option<&str>) -> anyhow::Result<()> {
    info!("Loading configuration from: {}", path.display());

    // Parse config to find a suitable target agent
    let config = mas_core::parse_config_file(path)?;

    // Determine target agent
    let target_agent = match target {
        Some(name) => {
            // Verify the agent exists
            if !config.agents.iter().any(|a| a.name == name) {
                eprintln!("Error: Agent '{}' not found in config", name);
                eprintln!("Available agents: {:?}", config.agents.iter().map(|a| &a.name).collect::<Vec<_>>());
                std::process::exit(1);
            }
            name.to_string()
        }
        None => {
            // Find a suitable entry point:
            // 1. First, look for an agent named "Coordinator"
            // 2. Then, look for the first routing agent
            // 3. Finally, use the first agent
            config.agents.iter()
                .find(|a| a.name == "Coordinator")
                .or_else(|| config.agents.iter().find(|a| a.handler.routing))
                .or_else(|| config.agents.first())
                .map(|a| a.name.clone())
                .ok_or_else(|| anyhow::anyhow!("No agents found in config"))?
        }
    };

    info!("Target agent: {}", target_agent);

    // Load and instantiate the system
    let system = load_system_from_json(path).await?;

    info!("System loaded. Sending prompt...\n");

    // Create a temporary "User" agent that connects to the target
    let user = AgentBuilder::new("_User")
        .role("CLI User")
        .blocking_connection(&target_agent)
        .build();

    system.register_agent(user, Arc::new(EchoHandler)).await?;

    // Send the prompt
    info!("User: {}", prompt);

    match system.send_message("_User", &target_agent, prompt).await? {
        SendResult::Response(msg) => {
            // Print response prominently
            println!("\n{}", msg.content);
        }
        SendResult::Timeout(err) => {
            eprintln!("\nTimeout: {}", err);
            std::process::exit(1);
        }
        SendResult::Notified => {
            println!("\n(Message sent, no response expected)");
        }
    }

    Ok(())
}

/// Run the agent system from a JSON configuration file (interactive mode)
async fn run_from_config(path: &PathBuf) -> anyhow::Result<()> {
    info!("Loading configuration from: {}", path.display());

    // Load and instantiate the system
    // The system is kept alive by the Arc until we exit
    let _system = load_system_from_json(path).await?;

    info!("System loaded successfully!");
    info!("Agents are running and ready for messages.");

    // Show the system topology
    info!("\nThe system is now running. Agents:");

    if let Ok(config) = mas_core::parse_config_file(path) {
        for agent in &config.agents {
            let routing = if agent.handler.routing { " (routing)" } else { "" };
            let connections: Vec<_> = agent.connections.keys().collect();
            if connections.is_empty() {
                info!("  - {}{}", agent.name, routing);
            } else {
                info!("  - {}{} -> {:?}", agent.name, routing, connections);
            }
        }
    }

    info!("\nTo send a prompt, use: --config {} --prompt \"your message\"", path.display());
    info!("Press Ctrl+C to exit.");

    // Keep the system running
    tokio::signal::ctrl_c().await?;
    info!("\nShutting down...");

    Ok(())
}

/// Run basic demo scenarios with mock handlers
async fn run_basic_demo_scenarios() -> anyhow::Result<()> {
    let system = AgentSystem::new(SystemConfig::with_timeout_secs(5));

    // Build the topology:
    //                     +--Blocking--> Researcher --Blocking--+
    //                     |                                      v
    // User -> Coordinator-+--Blocking--> Analyst <---Blocking---+
    //                     |
    //                     +--Notify----> Logger ----Blocking---> AlertHandler

    let coordinator = AgentBuilder::new("Coordinator")
        .role("Orchestrates research and analysis tasks")
        .system_prompt("You coordinate work between researchers and analysts.")
        .blocking_connection("Researcher")
        .blocking_connection("Analyst")
        .notify_connection("Logger")
        .build();

    let researcher = AgentBuilder::new("Researcher")
        .role("Conducts research on topics")
        .system_prompt("You research topics and provide detailed findings.")
        .blocking_connection("Analyst")
        .build();

    let analyst = AgentBuilder::new("Analyst")
        .role("Analyzes data and provides insights")
        .system_prompt("You analyze data and provide actionable insights.")
        .build();

    let logger = AgentBuilder::new("Logger")
        .role("Logs all activities")
        .system_prompt("You log activities and alert on anomalies.")
        .blocking_connection("AlertHandler")
        .build();

    let alert_handler = AgentBuilder::new("AlertHandler")
        .role("Handles system alerts")
        .system_prompt("You process and respond to system alerts.")
        .build();

    // Register agents with mock handlers
    system.register_agent(coordinator, Arc::new(EchoHandler)).await?;
    system.register_agent(researcher, Arc::new(ResearcherHandler)).await?;
    system.register_agent(analyst, Arc::new(AnalystHandler)).await?;
    system.register_agent(
        logger,
        Arc::new(SinkHandler::new(|msg| {
            info!("[Logger] Received: {} -> {}", msg.from, msg.content);
        })),
    ).await?;
    system.register_agent(alert_handler, Arc::new(EchoHandler)).await?;

    info!("All agents registered. Starting demo scenarios...\n");

    // Demo 1: Simple blocking message
    info!("=== Demo 1: Simple Blocking Message ===");
    match system.send_message("Coordinator", "Researcher", "Research quantum computing").await? {
        SendResult::Response(msg) => info!("Response: {}", msg.content),
        SendResult::Timeout(err) => info!("Timeout: {}", err),
        SendResult::Notified => info!("Notified (unexpected for blocking)"),
    }

    // Demo 2: Notify message (fire-and-forget)
    info!("\n=== Demo 2: Notify Message (Fire-and-Forget) ===");
    match system.send_message("Coordinator", "Logger", "User started a new session").await? {
        SendResult::Response(_) => info!("Response (unexpected for notify)"),
        SendResult::Timeout(_) => info!("Timeout (unexpected for notify)"),
        SendResult::Notified => info!("Message sent to Logger (no wait)"),
    }

    // Demo 3: Parallel messages to multiple agents
    info!("\n=== Demo 3: Parallel Messages to Multiple Agents ===");
    let results = system.broadcast_from_agent("Coordinator", "Analyze market trends").await?;

    for (recipient, result) in results {
        match result {
            Ok(SendResult::Response(msg)) => info!("  {} responded: {}", recipient, msg.content),
            Ok(SendResult::Notified) => info!("  {} notified (fire-and-forget)", recipient),
            Ok(SendResult::Timeout(err)) => info!("  {} timed out: {}", recipient, err),
            Err(e) => info!("  {} error: {}", recipient, e),
        }
    }

    // Demo 4: Verify conversation history
    info!("\n=== Demo 4: Conversation History ===");
    if let Some(messages) = system.get_conversation("Coordinator", "Researcher").await {
        info!("Coordinator <-> Researcher conversation:");
        for msg in messages {
            info!("  [{} -> {}]: {}", msg.from, msg.to, msg.content);
        }
    }

    // Demo 5: Test timeout behavior
    info!("\n=== Demo 5: Timeout Behavior ===");
    let slow_system = AgentSystem::new(SystemConfig::with_timeout_secs(1));

    let sender = AgentBuilder::new("Sender")
        .blocking_connection_with_timeout("SlowAgent", Duration::from_millis(500))
        .build();

    let slow_agent = AgentBuilder::new("SlowAgent").build();

    slow_system.register_agent(sender, Arc::new(EchoHandler)).await?;
    slow_system.register_agent(
        slow_agent,
        Arc::new(DelayedHandler::new(Duration::from_secs(2), "Finally done")),
    ).await?;

    match slow_system.send_message("Sender", "SlowAgent", "Quick question").await? {
        SendResult::Response(msg) => info!("Got response: {}", msg.content),
        SendResult::Timeout(err) => info!("Expected timeout: {}", err),
        SendResult::Notified => info!("Notified (unexpected)"),
    }

    // Demo 6: Connection validation
    info!("\n=== Demo 6: Connection Validation ===");
    match system.send_message("Analyst", "Coordinator", "Hello").await {
        Ok(_) => info!("Message sent (unexpected)"),
        Err(e) => info!("Expected error - no connection: {}", e),
    }

    info!("\n=== Basic Demo Complete ===");

    Ok(())
}

/// Run LLM-powered demo with dynamic routing
async fn run_llm_routing_demo() -> anyhow::Result<()> {
    info!("=== LLM Dynamic Routing Demo ===\n");

    // Auto-detect Ollama and available models
    info!("Detecting Ollama and available models...");
    let provider = match OllamaProvider::detect().await {
        Ok(p) => {
            info!("Using model: {}", p.default_model());
            Arc::new(p)
        }
        Err(e) => {
            warn!("Ollama not available: {}", e);
            warn!("Please ensure Ollama is running: ollama serve");
            warn!("And has at least one model: ollama pull llama3.2");
            return Ok(());
        }
    };

    // Create system with Arc for routing agent registration
    let system = Arc::new(AgentSystem::new(SystemConfig::with_timeout_secs(120)));

    // Build the topology for dynamic routing:
    //
    //                  +--Blocking--> Researcher
    //                  |
    // User -> Coordinator (routing)
    //                  |
    //                  +--Blocking--> Analyst
    //                  |
    //                  +--Notify----> Logger
    //
    // The Coordinator uses LLM to decide whether to:
    // - Answer directly
    // - Forward to Researcher (for research tasks)
    // - Forward to Analyst (for analysis tasks)
    // - Forward to both (for comprehensive requests)

    let coordinator = AgentBuilder::new("Coordinator")
        .role("AI coordinator that routes requests to specialists")
        .system_prompt(
            "You are a coordinator agent. Your job is to understand user requests and decide the best way to handle them.

For research questions (asking about facts, history, how things work), forward to the Researcher.
For analysis tasks (comparing options, evaluating data, making recommendations), forward to the Analyst.
For simple greetings or meta-questions about yourself, respond directly.
For complex requests needing both research and analysis, forward to both agents."
        )
        .blocking_connection("Researcher")
        .blocking_connection("Analyst")
        .notify_connection("Logger")
        .build();

    let researcher = AgentBuilder::new("Researcher")
        .role("Research specialist")
        .system_prompt(
            "You are a research specialist. When asked about a topic, provide factual information and findings. Be thorough but concise (3-5 sentences). Focus on facts and established knowledge."
        )
        .build();

    let analyst = AgentBuilder::new("Analyst")
        .role("Analysis specialist")
        .system_prompt(
            "You are an analysis specialist. When asked to analyze something, provide insights, comparisons, and recommendations. Be concise (3-5 sentences). Focus on evaluation and actionable insights."
        )
        .build();

    let logger = AgentBuilder::new("Logger")
        .role("Activity logger")
        .system_prompt("You log all system activities.")
        .build();

    // Create handlers
    // Coordinator uses routing mode to decide where to forward
    let coordinator_handler = LlmHandler::new(provider.clone())
        .with_routing()
        .with_options(CompletionOptions::new().temperature(0.3).max_tokens(500));

    // Researcher and Analyst use simple mode (direct responses)
    let researcher_handler = LlmHandler::new(provider.clone())
        .with_options(CompletionOptions::new().temperature(0.5).max_tokens(300));

    let analyst_handler = LlmHandler::new(provider.clone())
        .with_options(CompletionOptions::new().temperature(0.5).max_tokens(300));

    // Register agents
    // Coordinator uses routing registration
    AgentSystem::register_routing_agent(
        system.clone(),
        coordinator,
        Arc::new(coordinator_handler),
    ).await?;

    // Other agents use simple registration
    system.register_agent(researcher, Arc::new(researcher_handler)).await?;
    system.register_agent(analyst, Arc::new(analyst_handler)).await?;
    system.register_agent(
        logger,
        Arc::new(SinkHandler::new(|msg| {
            info!("[Logger] {} -> {}: {}", msg.from, msg.to, &msg.content[..msg.content.len().min(50)]);
        })),
    ).await?;

    info!("All agents registered with dynamic routing.\n");

    // Demo scenarios showing dynamic routing
    let scenarios = [
        ("Hello! Who are you?", "Direct response expected"),
        ("What is quantum computing?", "Should forward to Researcher"),
        ("Should I learn Rust or Go for systems programming?", "Should forward to Analyst"),
        ("Research the history of AI and analyze its future impact", "Should forward to both"),
    ];

    // Create a "User" agent that can send to Coordinator
    let user = AgentBuilder::new("User")
        .role("User interface")
        .blocking_connection("Coordinator")
        .build();
    system.register_agent(user, Arc::new(EchoHandler)).await?;

    for (query, description) in scenarios {
        info!("\n=== {} ===", description);
        info!("User: {}", query);

        match system.send_message("User", "Coordinator", query).await? {
            SendResult::Response(msg) => {
                info!("\nCoordinator's response:\n{}", msg.content);
            }
            SendResult::Timeout(err) => {
                warn!("Timeout: {}", err);
            }
            SendResult::Notified => {
                info!("(notified)");
            }
        }

        info!("\n---");

        // Small delay between scenarios
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("\n=== LLM Routing Demo Complete ===");

    Ok(())
}
