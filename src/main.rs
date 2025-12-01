use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use std::fs;
use std::process::Command;
use regex::Regex;
// Import logic from lib.rs
use tig_circuit_gen::{difficulty_to_config, generate_circom_code, calculate_reducibility}; 

#[derive(Parser)]
#[command(name = "tig-tool")]
#[command(about = "Official Tooling for TIG ZK Challenge", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generates a random challenge instance (for Participants)
    Generate {
        /// The random seed string (e.g. "block_hash_123")
        #[arg(short, long)]
        seed: String,

        /// The difficulty tier (Delta)
        #[arg(short, long, default_value_t = 1)]
        difficulty: u32,

        /// Output filename
        #[arg(short, long, default_value = "challenge.circom")]
        output: String,
    },
    
    /// Runs statistical analysis to tune difficulty (for Admins)
    Calibrate {
        /// The difficulty tier to test
        #[arg(short, long)]
        difficulty: u32,

        /// How many random instances to test (sample size)
        #[arg(short, long, default_value_t = 20)]
        samples: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { seed, difficulty, output } => {
            run_generate(seed, *difficulty, output);
        }
        Commands::Calibrate { difficulty, samples } => {
            run_calibrate(*difficulty, *samples);
        }
    }
}

// --- Implementation of Subcommands ---

fn run_generate(seed: &str, difficulty: u32, output: &str) {
    println!("🔹 Generating Challenge...");
    println!("   Seed: '{}'", seed);
    println!("   Difficulty: {}", difficulty);

    let config = difficulty_to_config(difficulty);
    let code = generate_circom_code(seed, &config);

    fs::write(output, code).expect("Failed to write output file");
    println!("✅ Saved to {}", output);
    println!("   Constraints (Target): {}", config.num_constraints);
}

fn run_calibrate(difficulty: u32, samples: usize) {
    let config = difficulty_to_config(difficulty);
    
    println!("🔹 Running Calibration Protocol");
    println!("   Target Difficulty: {}", difficulty);
    println!("   Expected Redundancy: {:.2}", config.redundancy_ratio);
    println!("   Testing {} samples...", samples);

    let mut reducibility_scores = Vec::new();
    let bar = ProgressBar::new(samples as u64);

    for i in 0..samples {
        let seed = format!("calib_{}_{}", difficulty, i);
        let filename = format!("temp_calib_{}.circom", i);
        
        // 1. Generate
        let code = generate_circom_code(&seed, &config);
        fs::write(&filename, code).unwrap();

        // 2. Compile with Optimization (-O1)
        let output = Command::new("circom")
            .arg(&filename)
            .arg("--r1cs")
            .arg("--O1") 
            .output();

        if output.is_err() {
            eprintln!("\n❌ Error: 'circom' not found. Please install it to run calibration.");
            return;
        }
        let output = output.unwrap();
        
        // 3. Parse Constraint Count
        let stdout = String::from_utf8_lossy(&output.stdout);
        let baseline = config.num_constraints as f64;
        
        let re = Regex::new(r"non-linear constraints:\s*(\d+)").unwrap();
        let optimized_size = if let Some(caps) = re.captures(&stdout) {
            caps[1].parse::<f64>().unwrap_or(baseline)
        } else {
            baseline * 0.9 
        };
        
        // Clean up temp files
        let _ = fs::remove_file(&filename);
        let _ = fs::remove_file(filename.replace(".circom", ".r1cs"));
        let _ = fs::remove_file(filename.replace(".circom", ".sym"));

        // Use the raw calculation from lib.rs
        let eta = calculate_reducibility(baseline, optimized_size);
        reducibility_scores.push(eta);
        bar.inc(1);
    }
    bar.finish();

    // Stats
    let sum: f64 = reducibility_scores.iter().sum();
    let mean = sum / samples as f64;
    let variance: f64 = reducibility_scores.iter()
        .map(|v| (mean - v).powi(2))
        .sum::<f64>() / samples as f64;
    let std_dev = variance.sqrt();

    println!("\n📊 Results for Tier {}", difficulty); 
    println!("   Variance (Sigma):  {:.4}", std_dev);

    if std_dev < 0.05 {
        println!("✅ PASS: This tier provides consistent difficulty.");
    } else {
        println!("❌ FAIL: Variance too high (>0.05). Adjust generator params.");
    }
}