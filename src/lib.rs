use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// The Configuration Vector "Theta" from the proposal.
/// Theta = <K_target, rho, D>
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    pub num_constraints: usize, // K_target
    pub redundancy_ratio: f64,  // rho (0.0 to 1.0)
    pub max_depth: usize,       // D
}

/// The Difficulty Scaling Function F(delta) -> Theta
/// Adjust these math formulas to tune your tiers!
pub fn difficulty_to_config(delta: u32) -> CircuitConfig {
    // Example scaling logic:
    // Size scales linearly: 1 -> 1,000 constraints
    let num_constraints = (delta as usize) * 1000;
    
    // Redundancy drops as difficulty rises (Harder to optimize)
    // Starts at 50%, drops to 5% min
    let redundancy_ratio = (0.5 - (delta as f64 * 0.02)).max(0.05);

    // Depth increases with difficulty
    let max_depth = 10 + (delta as usize * 5);

    CircuitConfig {
        num_constraints,
        redundancy_ratio,
        max_depth,
    }
}

/// The Generator G(s, theta)
pub fn generate_circom_code(seed: &str, config: &CircuitConfig) -> String {
    // 1. Deterministic Seeding
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut rng = ChaCha20Rng::from_seed(result.into());

    // Header
    let mut code = String::new();
    code.push_str("pragma circom 2.0.0;\n\n");
    code.push_str("template Challenge() {\n");
    code.push_str("    signal input in[5];\n"); // 5 random seeds
    code.push_str("    signal output out;\n");

    let mut signals = Vec::new();
    // Initialize pool with inputs (depth 0)
    for i in 0..5 {
        signals.push((format!("in[{}]", i), 0)); 
    }

    // Cache for Redundancy: map "op_signature" -> "variable_name"
    let mut expr_cache: HashMap<String, String> = HashMap::new();

    // 2. Generation Loop
    for i in 0..config.num_constraints {
        let new_var = format!("s_{}", i);
        code.push_str(&format!("    signal {};\n", new_var));

        // Decision: Redundant or New?
        let is_redundant = rng.gen_bool(config.redundancy_ratio) && !expr_cache.is_empty();

        if is_redundant {
            // Pick an existing expression to repeat (inefficient!)
            let keys: Vec<&String> = expr_cache.keys().collect();
            let chosen_sig = keys[rng.gen_range(0..keys.len())];
            
            // Re-compute the same expression into a new variable
            let parts: Vec<&str> = chosen_sig.split('|').collect();
            code.push_str(&format!("    {} <== {} {} {};\n", new_var, parts[1], parts[0], parts[2]));
            
            signals.push((new_var.clone(), 0)); 
        } else {
            // Generate Fresh Logic
            let op = if rng.gen_bool(0.5) { "*" } else { "+" };
            
            // Select operands, preferring recent ones to create chains (Depth)
            let idx1 = rng.gen_range(0..signals.len());
            let idx2 = rng.gen_range(0..signals.len());
            let (s1, d1) = &signals[idx1];
            let (s2, d2) = &signals[idx2];
            
            let new_depth = std::cmp::max(*d1, *d2) + 1;
            
            // If depth exceeded, reset to inputs to break the chain
            let (final_s1, final_s2) = if new_depth > config.max_depth {
                ("in[0]", "in[1]")
            } else {
                (s1.as_str(), s2.as_str())
            };

            let line = format!("    {} <== {} {} {};\n", new_var, final_s1, op, final_s2);
            code.push_str(&line);

            // Cache it
            let sig = format!("{}|{}|{}", op, final_s1, final_s2);
            expr_cache.insert(sig, new_var.clone());
            
            signals.push((new_var, new_depth));
        }
    }

    // Output wiring
    code.push_str(&format!("    out <== s_{};\n", config.num_constraints - 1));
    code.push_str("}\n");
    code.push_str("component main = Challenge();\n");

    code
}