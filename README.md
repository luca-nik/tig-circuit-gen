TIG Circuit GeneratorTIG Circuit Generator is a deterministic procedural generation tool designed for the TIG ZK Challenge. It produces random, valid, and tunable R1CS circuit instances (in Circom format) to serve as unoptimized benchmarks for competitors.This tool ensures Isotropic Difficulty, meaning that all challenges within a specific "Difficulty Tier" are statistically guaranteed to offer the same level of optimization potential, preventing "lucky seeds."📦 Project Structuretig-circuit-gen/
├── Cargo.toml         # Rust dependencies (clap, rand, sha2, etc.)
├── README.md          # Documentation
└── src/
    ├── lib.rs         # Core Logic
    │   ├── Generator: Procedural DAG generation logic
    │   └── Configuration: Maps Difficulty (Delta) -> Parameters (Theta)
    └── main.rs        # CLI Entry Point
        ├── generate: Subcommand for participants
        └── calibrate: Subcommand for admins (variance control)
🚀 PrerequisitesTo run the full pipeline, you need the following installed:Rust Toolchain (to build this generator):curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
Circom Compiler (to compile the output to R1CS):Installation GuideSnarkJS (Optional: for inspection and witness testing):npm install -g snarkjs
🛠️ Installation & BuildingClone this repository.Build the optimized binary:cargo build --release
The executable will be available at:./target/release/tig-circuit-gen
(Optional) You can alias this to tig-tool for easier use:alias tig-tool="./target/release/tig-circuit-gen"
📖 Usage GuideThe tool has two main modes: Generation (for users) and Calibration (for admins).1. Generating a ChallengeGenerates a random .circom circuit based on a seed and difficulty level.# Syntax: generate --seed <STRING> --difficulty <INT>
./target/release/tig-circuit-gen generate --seed "round_1_seed" --difficulty 2
--seed: A string (e.g., block hash). The same seed always produces the same file.--difficulty: The tier level (scales size and complexity).--output: (Optional) Filename, defaults to challenge.circom.2. Calibrating Difficulty (Admin Only)Runs a statistical "Variance Control" protocol. It generates multiple random instances for a given tier, attempts to optimize them using circom --O1, and ensures the difficulty is consistent (Low Variance).# Syntax: calibrate --difficulty <INT> --samples <INT>
./target/release/tig-circuit-gen calibrate --difficulty 2 --samples 50
Output: Prints Mean Reducibility (how optimizable the tier is) and Variance (stability).Goal: Standard Deviation should be < 0.05.🔗 The Full Pipeline (Generator → Circom → R1CS)To generate the official Baseline Circuit ($C^0$) for the challenge, follow this exact pipeline.Step 1: Generate the InstanceUse the Rust tool to create the procedural source code../target/release/tig-circuit-gen generate --seed "official_seed_v1" --difficulty 5 --output challenge.circom
Step 2: Compile to R1CS (The Baseline)Compile the circuit using circom.CRITICAL: You must use the --O0 flag to disable compiler optimizations. This ensures the baseline retains the procedural inefficiencies for competitors to solve.circom challenge.circom --r1cs --wasm --sym --O0
--r1cs: Outputs challenge.r1cs (The Constraint System).--wasm: Outputs witness generation code.--sym: Outputs debugging symbols.--O0: Disables optimization (Required for $C^0$).Step 3: Inspect the Constraints (Optional)Use snarkjs to verify the size and structure of the generated R1CS.# Print general info (Constraint count, etc.)
snarkjs r1cs info challenge.r1cs

# Print the actual constraint equations
snarkjs r1cs print challenge.r1cs
🧠 Tuning Difficulty TiersTo adjust the hardness of the tiers, edit src/lib.rs function difficulty_to_config.pub fn difficulty_to_config(delta: u32) -> CircuitConfig {
    // Example: Linear scaling of constraints
    let num_constraints = (delta as usize) * 1000;
    
    // Example: Reducing redundancy makes optimization harder
    let redundancy_ratio = (0.5 - (delta as f64 * 0.02)).max(0.05);
    
    // ...
}
