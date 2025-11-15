use multi_agent_backend::{Agent, AgentSystem};

fn main() {
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
        "You are a coordinator agent. Your task is to manage and organize tasks between agents.".to_string(),
    );

    // Add agents to system
    system.add_agent(researcher).unwrap();
    system.add_agent(analyst).unwrap();
    system.add_agent(coordinator).unwrap();

    // Connect agents
    system.connect_agents("Researcher", "Analyst").unwrap();
    system.connect_agents("Analyst", "Coordinator").unwrap();

    // Demonstrate communication
    println!("Agent connections:");
    for agent in system.list_agents() {
        println!("  {} -> {:?}", agent.name, agent.get_connections());
    }

    println!("\nAttempting message passing:");

    // Valid message (connected agents)
    match system.send_message("Researcher", "Analyst", "Here's my research data".to_string()) {
        Ok(msg) => println!("  ✓ Message sent from {} to {}: {}", msg.from, msg.to, msg.content),
        Err(e) => println!("  ✗ Error: {}", e),
    }

    // Invalid message (not connected)
    match system.send_message("Researcher", "Coordinator", "Hello".to_string()) {
        Ok(msg) => println!("  ✓ Message sent from {} to {}: {}", msg.from, msg.to, msg.content),
        Err(e) => println!("  ✗ Error: {}", e),
    }
}
