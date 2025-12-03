# TIG Circuit Generator

**TIG Circuit Generator** is a deterministic procedural generation tool designed for the **TIG ZK Challenge**. It produces random, valid, and tunable R1CS circuit instances (in Circom format) to serve as unoptimized benchmarks for competitors.

This tool ensures **Isotropic Difficulty**, meaning that all challenges within a specific "Difficulty Tier" are statistically guaranteed to offer the same level of optimization potential, preventing "lucky seeds."

## 📦 Project Structure

```text
tig-circuit-gen/
├── Cargo.toml         # Rust dependencies (clap, rand, sha2, etc.)
├── README.md          # Documentation
└── src/
    ├── lib.rs         # Core Logic
    │   ├── Generator: Procedural DAG generation with power maps
    │   └── Configuration: Maps Difficulty (δ) -> Parameters (θ = ⟨K, ρ, D, P_map⟩)
    └── main.rs        # CLI Entry Point
        ├── generate: Subcommand for participants
        └── calibrate: Subcommand for admins (variance control)
````

## 🚀 Prerequisites

To run the full pipeline, you need the following installed:

1.  **Rust Toolchain** (to build this generator):
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
    ```
2.  **Circom Compiler** (to compile the output to R1CS):
      * [Installation Guide](https://docs.circom.io/getting-started/installation/)
3.  **SnarkJS** (Optional: for inspection and witness testing):
    ```bash
    npm install -g snarkjs
    ```

## 🛠️ Installation & Building

1.  Clone this repository.
2.  Build the optimized binary:
    ```bash
    cargo build --release
    ```
3.  The executable will be available at:
    ```bash
    ./target/release/tig-circuit-gen
    ```

*(Optional) You can alias this to `tig-tool` for easier use:*

```bash
alias tig-tool="./target/release/tig-circuit-gen"
```

## 📖 Usage Guide

The tool has two main modes: **Generation** (for users) and **Calibration** (for admins).

### 1\. Generating a Challenge

Generates a random `.circom` circuit based on a seed and difficulty level.

```bash
# Syntax: generate --seed <STRING> --difficulty <INT>
./target/release/tig-circuit-gen generate --seed "round_1_seed" --difficulty 2
```

  * **--seed**: A string (e.g., block hash). The same seed always produces the same file.
  * **--difficulty**: The tier level (scales size and complexity).
  * **--output**: (Optional) Filename, defaults to `challenge.circom`.

### 2\. Calibrating Difficulty (Admin Only)

Runs a statistical **"Variance Control"** protocol to ensure **Isotropic Difficulty** - that all challenges within a tier offer equivalent optimization potential, preventing "lucky seeds."

```bash
# Syntax: calibrate --difficulty <INT> --samples <INT>
./target/release/tig-circuit-gen calibrate --difficulty 2 --samples 50
```

#### What Calibration Does:

1. **Generates N random instances** with identical difficulty parameters
2. **Compiles each with `circom --O1`** (reference optimization level)
3. **Measures "reducibility"** = `1 - (optimized_size / baseline_size)` for each instance
4. **Calculates statistics**: Mean reducibility and standard deviation across all samples
5. **Pass/Fail verdict**: Standard deviation must be `< 0.05` for fair competition

#### Interpreting Results:

```bash
📊 Results for Tier 3
   Variance (Sigma):  0.0423    # Consistency measure

✅ PASS: This tier provides consistent difficulty.
```

- **Variance (Sigma)**: Consistency across seeds (lower = more fair)
- **PASS**: σ < 0.05 means all participants face equivalent challenges
- **FAIL**: σ ≥ 0.05 means some seeds are "luckier" than others

**Note**: The calibration output no longer displays "Mean Reducibility" as this metric can be misleading when the reference solver adds overhead.

#### If Calibration Fails:

Edit the scaling parameters in `src/lib.rs:17-34`, rebuild, and re-calibrate:

```bash
# After editing difficulty_to_config()
cargo build --release
./target/release/tig-circuit-gen calibrate --difficulty 3 --samples 100
```

## 🔗 The Full Pipeline (Generator → Circom → R1CS)

To generate the official **Baseline Circuit ($C^0$)** for the challenge, follow this exact pipeline.

### Step 1: Generate the Instance

Use the Rust tool to create the procedural source code.

```bash
./target/release/tig-circuit-gen generate --seed "official_seed_v1" --difficulty 5 --output challenge.circom
```

### Step 2: Compile to R1CS (The Baseline)

Compile the circuit using `circom`.
**CRITICAL:** You must use the `--O0` flag to disable compiler optimizations. This ensures the baseline retains the procedural inefficiencies for competitors to solve.

```bash
circom challenge.circom --r1cs --wasm --sym --O0
```

  * `--r1cs`: Outputs `challenge.r1cs` (The Constraint System).
  * `--wasm`: Outputs witness generation code.
  * `--sym`: Outputs debugging symbols.
  * `--O0`: **Disables optimization (Required for $C^0$).**

### Step 3: Inspect the Constraints (Optional)

Use `snarkjs` to verify the size and structure of the generated R1CS.

```bash
# Print general info (Constraint count, etc.)
snarkjs r1cs info challenge.r1cs

# Print the actual constraint equations
snarkjs r1cs print challenge.r1cs
```

## 🔬 Power Maps: Hash-like Algebraic Complexity

One of the key features of this generator is the inclusion of **Power Maps** (S-Box operations), which inject algebraic complexity similar to cryptographic hash functions.

### Why Power Maps?

Real-world ZK circuits (especially those involving cryptographic operations) contain non-linear operations that are difficult to optimize. The x^5 S-Box is used in ZK-friendly hash functions like **Poseidon** over BN254 curves.

### Implementation Details

When a power map is generated, the system creates an **x^5** operation by unrolling it into R1CS-compatible quadratic constraints:

```circom
// For an input signal x, generate x^5:
signal x_sq;
x_sq <== x * x;           // x²

signal x_quad;
x_quad <== x_sq * x_sq;   // x⁴

signal x_pow5;
x_pow5 <== x_quad * x;    // x⁵
```

This unrolling:
1. Creates 3 intermediate signals per power map
2. Generates 3 quadratic constraints (R1CS compatible)
3. Introduces algebraic dependencies that resist simplification
4. Mimics realistic bottlenecks found in production ZK circuits

The probability of injecting power maps increases with difficulty, from 5% at tier 1 to 30% at tier 5+, ensuring higher tiers contain more cryptographic-like complexity.

## 🧠 Tuning Difficulty Tiers

To adjust the hardness of the tiers, edit `src/lib.rs` function `difficulty_to_config`.

```rust
pub fn difficulty_to_config(delta: u32) -> CircuitConfig {
    // 1. Size scales linearly (1k per difficulty level)
    let num_constraints = (delta as usize) * 1000;

    // 2. Redundancy drops (Harder to optimize)
    let redundancy_ratio = (0.5 - (delta as f64 * 0.05)).max(0.05);

    // 3. Power Map Ratio (The "Hash-like" quality)
    let power_map_ratio = (0.05 + (delta as f64 * 0.05)).min(0.30);

    // 4. Depth increases
    let max_depth = 10 + (delta as usize * 10);

    // ...
}
```

After editing, rebuild with `cargo build --release` and run `calibrate` to verify your new settings.

## 🎯 Workflow

```bash
# 1. Generate the official challenge instance
./target/release/tig-circuit-gen generate \
    --seed "official_round_1_seed" \
    --difficulty 5 \
    --output official_challenge.circom

# 2. Create the baseline R1CS (what participants must beat)
circom official_challenge.circom --r1cs --wasm --sym --O0

# 3. Verify tier fairness with statistical analysis
./target/release/tig-circuit-gen calibrate --difficulty 5 --samples 100

# 4. Inspect baseline statistics
snarkjs r1cs info official_challenge.r1cs
# Example output: "5000 constraints" = baseline participants must beat
```

## 📊 Understanding Difficulty Parameters

The generator uses four key parameters that scale with difficulty:

| Parameter | Formula | Effect |
|-----------|---------|---------|
| **Constraints** | `δ × 1000` | Linear size scaling |
| **Redundancy** | `max(0.5 - δ×0.05, 0.05)` | Optimization potential (duplicate operations) |
| **Power Maps** | `min(0.05 + δ×0.05, 0.30)` | Non-linear S-Box density (x^5 operations) |
| **Depth** | `10 + δ×10` | Memory pressure (computation chain length) |

### What are Power Maps?

Power Maps inject **algebraic S-Box operations** (x^5) similar to those found in ZK-friendly hash functions like Poseidon. These operations:
- Are unrolled into chains of quadratic constraints (x², x⁴, x⁵)
- Create non-linear bottlenecks resistant to optimization
- Mimic realistic cryptographic circuit patterns

As difficulty increases, more power maps are injected, making the circuits algebraically more complex and harder to simplify.

### Examples:
- **Difficulty 1**: 1K constraints, 45% redundancy, 10% power maps, depth 20
- **Difficulty 5**: 5K constraints, 25% redundancy, 30% power maps, depth 60
- **Difficulty 10**: 10K constraints, 5% redundancy, 30% power maps, depth 110

## ✅ Validation Results

All difficulty tiers have been tested and validated for consistent optimization potential:

```bash
# Comprehensive tier validation test
for i in {1..10}; do 
    ./target/release/tig-circuit-gen calibrate --difficulty $i --samples 50
done
```

**Results Summary:**

| Tier | Constraints | Redundancy | Power Maps | Depth | Variance (σ) | Status |
|------|-------------|------------|------------|-------|--------------|---------|
| 1 | 1,000 | 0.45 | 0.10 | 20 | < 0.05 | ✅ PASS |
| 2 | 2,000 | 0.40 | 0.15 | 30 | < 0.05 | ✅ PASS |
| 3 | 3,000 | 0.35 | 0.20 | 40 | < 0.05 | ✅ PASS |
| 4 | 4,000 | 0.30 | 0.25 | 50 | < 0.05 | ✅ PASS |
| 5 | 5,000 | 0.25 | 0.30 | 60 | < 0.05 | ✅ PASS |
| 6 | 6,000 | 0.20 | 0.30 | 70 | < 0.05 | ✅ PASS |
| 7 | 7,000 | 0.15 | 0.30 | 80 | < 0.05 | ✅ PASS |
| 8 | 8,000 | 0.10 | 0.30 | 90 | < 0.05 | ✅ PASS |
| 9 | 9,000 | 0.05 | 0.30 | 100 | < 0.05 | ✅ PASS |
| 10 | 10,000 | 0.05 | 0.30 | 110 | < 0.05 | ✅ PASS |

**Note**: Validation results show that all tiers maintain isotropic difficulty with consistent variance. The specific Mean Reducibility values from previous versions are no longer displayed as the new power map implementation changes the optimization landscape.

**All tiers achieve σ < 0.05**, ensuring **Isotropic Difficulty** across the entire difficulty range. This validates that:

- **Fair Competition**: No participant gets "lucky" with easier-to-optimize seeds
- **Predictable Scaling**: Higher tiers are consistently more challenging
- **Statistical Reliability**: The generator produces stable optimization targets

### Reproducing Validation:

```bash
# Quick validation of a single tier
./target/release/tig-circuit-gen calibrate --difficulty 5 --samples 50

# Full validation suite (takes ~2 minutes)
for i in {1..10}; do 
    echo "Testing Tier $i..."
    ./target/release/tig-circuit-gen calibrate --difficulty $i --samples 50
    echo "---"
done
```
## About

**TIG Circuit Generator** was developed by Luca at [CryptoEconLab](https://cryptoeconlab.com/) for [The Innovation Game](https://tig.foundation/) ZK Challenge, as part of the Challenge Owners collaboration between TIG and CEL.
