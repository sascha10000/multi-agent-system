use multi_agent_backend::{Agent, AgentSystem};

#[tokio::main]
async fn main() {
    println!("Multi-Agent Backend System");
    println!("==========================\n");

    // Create agent system
    let mut system = AgentSystem::new();

    // Create agents
    let researcher = Agent::new(
        "Researcher".to_string(),
        "You are a researcher agent. Your task is to gather and analyze information.".to_string(),
    );

    let analyst = Agent::new(
        "Analyst".to_string(),
        "You are an analyst agent. Your task is to process data and provide insights.".to_string(),
    );

    let coordinator = Agent::new(
        "Coordinator".to_string(),
        "You are a coordinator agent. Your task is to manage and organize tasks between agents."
            .to_string(),
    );

    // Add agents to system
    system.add_agent(researcher).unwrap();
    system.add_agent(analyst).unwrap();
    system.add_agent(coordinator).unwrap();

    // Connect agents
    system.connect_agents("Researcher", "Analyst").unwrap();
    system.connect_agents("Analyst", "Coordinator").unwrap();

    // Create a session for all agents
    println!("Creating session 'main_session' for all agents...");
    system.create_session("main_session".to_string()).unwrap();
    println!("Active session: {:?}\n", system.get_active_session());

    // Demonstrate communication
    println!("Agent connections:");
    for agent in system.list_agents() {
        println!("  {} -> {:?}", agent.name, agent.get_connections());
    }

    println!("\nAttempting message passing:");

    // Valid message (connected agents)
    match system.send_message(
        "Researcher",
        "Analyst",
        "Here's my research data".to_string(),
    ) {
        Ok(msg) => println!(
            "  ✓ Message sent from {} to {}: {}",
            msg.from, msg.to, msg.content
        ),
        Err(e) => println!("  ✗ Error: {}", e),
    }

    // Invalid message (not connected)
    match system.send_message("Researcher", "Coordinator", "Hello".to_string()) {
        Ok(msg) => println!(
            "  ✓ Message sent from {} to {}: {}",
            msg.from, msg.to, msg.content
        ),
        Err(e) => println!("  ✗ Error: {}", e),
    }

    // Wait for async tasks to process messages
    println!("\nWaiting for message processing...");

    // Give tasks a moment to process messages
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Wait for tasks to complete (this also removes the session to signal exit)
    match system.wait_for_session_tasks("main_session").await {
        Ok(_) => println!("Session processing complete!"),
        Err(e) => println!("Warning: {}", e),
    }

    println!("\nDemo complete!");
}
