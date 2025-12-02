use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// The Configuration Vector "Theta"
/// Theta = <K_target, rho, D, power_map_ratio>
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    pub num_constraints: usize, // K_target
    pub redundancy_ratio: f64,  // rho (0.0 to 1.0)
    pub max_depth: usize,       // D
    pub power_map_ratio: f64,   // Probability of generating an S-Box (x^5)
}

/// The Difficulty Scaling Function F(delta) -> Theta
pub fn difficulty_to_config(delta: u32) -> CircuitConfig {
    // 1. Size scales linearly (1k per difficulty level)
    let num_constraints = (delta as usize) * 1000;
    
    // 2. Redundancy drops (Harder to optimize)
    // Starts at 50%, drops to 5% min
    let redundancy_ratio = (0.5 - (delta as f64 * 0.05)).max(0.05);

    // 3. Power Map Ratio (The "Hash-like" quality)
    // As difficulty increases, we add more non-linear S-boxes (x^5)
    // Starts at 5%, caps at 30%
    let power_map_ratio = (0.05 + (delta as f64 * 0.05)).min(0.30);

    // 4. Depth increases
    let max_depth = 10 + (delta as usize * 10);

    CircuitConfig {
        num_constraints,
        redundancy_ratio,
        max_depth,
        power_map_ratio,
    }
}

/// Helper to calculate reducibility (Raw).
/// Returns 1.0 - (optimized / baseline).
/// A negative value implies the reference solver increased the constraint count.
pub fn calculate_reducibility(baseline: f64, optimized: f64) -> f64 {
    1.0 - (optimized / baseline)
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
    code.push_str("    signal input in[5];\n"); 
    code.push_str("    signal output out;\n");

    let mut signals = Vec::new();
    // Initialize pool with inputs
    for i in 0..5 {
        signals.push((format!("in[{}]", i), 0)); 
    }

    // Cache for Redundancy: map "op_signature" -> "variable_name"
    let mut expr_cache: HashMap<String, String> = HashMap::new();

    // 2. Generation Loop
    // We iterate until we hit the target constraint count.
    for i in 0..config.num_constraints {
        let new_var = format!("s_{}", i);
        code.push_str(&format!("    signal {};\n", new_var));

        // Decision Logic
        let rand_val = rng.gen_range(0.0..1.0);

        // CHECK 1: Redundancy (Reuse existing logic)
        if rand_val < config.redundancy_ratio && !expr_cache.is_empty() {
            let keys: Vec<&String> = expr_cache.keys().collect();
            let chosen_sig = keys[rng.gen_range(0..keys.len())];
            
            // Format stored in cache: "OP|OPERAND1|OPERAND2..."
            let parts: Vec<&str> = chosen_sig.split('|').collect();
            
            if parts[0] == "POW5" {
                // To reuse a POW5, we must recreate the unrolled chain
                // Inefficient Reuse: recalculate everything
                let base = parts[1];
                let var_sq = format!("{}_sq", new_var);
                let var_quad = format!("{}_quad", new_var);

                code.push_str(&format!("    signal {};\n", var_sq));
                code.push_str(&format!("    {} <== {} * {};\n", var_sq, base, base));
                
                code.push_str(&format!("    signal {};\n", var_quad));
                code.push_str(&format!("    {} <== {} * {};\n", var_quad, var_sq, var_sq));

                code.push_str(&format!("    {} <== {} * {};\n", new_var, var_quad, base));
            } else {
                // Redundant Arithmetic
                 code.push_str(&format!("    {} <== {} {} {};\n", new_var, parts[1], parts[0], parts[2]));
            }
            
            signals.push((new_var.clone(), 0)); 
        } 
        // CHECK 2: Power Map (Hash-like S-Box x^5)
        else if rand_val < (config.redundancy_ratio + config.power_map_ratio) {
            let idx = rng.gen_range(0..signals.len());
            let (s1, d1) = &signals[idx];

            // FIX: Unroll x^5 into quadratic constraints
            // 1. x^2
            let var_sq = format!("{}_sq", new_var);
            code.push_str(&format!("    signal {};\n", var_sq));
            code.push_str(&format!("    {} <== {} * {};\n", var_sq, s1, s1));
            
            // 2. x^4
            let var_quad = format!("{}_quad", new_var);
            code.push_str(&format!("    signal {};\n", var_quad));
            code.push_str(&format!("    {} <== {} * {};\n", var_quad, var_sq, var_sq));

            // 3. x^5
            code.push_str(&format!("    {} <== {} * {};\n", new_var, var_quad, s1));
            
            let sig = format!("POW5|{}", s1);
            expr_cache.insert(sig, new_var.clone());

            signals.push((new_var, d1 + 3));
        } 
        // CHECK 3: Standard Arithmetic (+ or *)
        else {
            let op = if rng.gen_bool(0.5) { "*" } else { "+" };
            
            let idx1 = rng.gen_range(0..signals.len());
            let idx2 = rng.gen_range(0..signals.len());
            let (s1, d1) = &signals[idx1];
            let (s2, d2) = &signals[idx2];
            
            let new_depth = std::cmp::max(*d1, *d2) + 1;
            
            let (final_s1, final_s2) = if new_depth > config.max_depth {
                ("in[0]", "in[1]")
            } else {
                (s1.as_str(), s2.as_str())
            };

            code.push_str(&format!("    {} <== {} {} {};\n", new_var, final_s1, op, final_s2));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_scaling() {
        let config_1 = difficulty_to_config(1);
        let config_10 = difficulty_to_config(10);
        assert!(config_10.num_constraints > config_1.num_constraints);
        assert!(config_10.max_depth > config_1.max_depth);
        assert!(config_10.redundancy_ratio <= config_1.redundancy_ratio);
        assert!(config_10.power_map_ratio >= config_1.power_map_ratio);
    }

    #[test]
    fn test_deterministic_generation() {
        let config = difficulty_to_config(1);
        let seed = "test_seed";
        let code1 = generate_circom_code(seed, &config);
        let code2 = generate_circom_code(seed, &config);
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_different_seeds_produce_different_code() {
        let config = difficulty_to_config(1);
        let code1 = generate_circom_code("seed_a", &config);
        let code2 = generate_circom_code("seed_b", &config);
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_structure_contains_basics() {
        let config = difficulty_to_config(1);
        let code = generate_circom_code("seed", &config);
        assert!(code.contains("template Challenge"));
        assert!(code.contains("signal input in[5]"));
        assert!(code.contains("signal output out"));
        assert!(code.contains("<=="));
    }

    #[test]
    fn test_reducibility_raw() {
        // Case 1: Optimizer reduces size (Good)
        // Baseline 100, Optimized 80 -> 20% reduction
        assert_eq!(calculate_reducibility(100.0, 80.0), 0.2);

        // Case 2: Optimizer adds overhead (Bad but real)
        // Baseline 100, Optimized 120 -> -20% reduction (Raw)
        assert!((calculate_reducibility(100.0, 120.0) - (-0.2)).abs() < 1e-9);

        // Case 3: No change
        assert_eq!(calculate_reducibility(100.0, 100.0), 0.0);
    }
}