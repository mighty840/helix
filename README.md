# Helix

DNA-encoded algorithm storage and execution system.

Programs are compiled to an ACGT nucleotide sequence, stored in aqueous/encapsulated DNA, retrieved, and executed on a software virtual machine.

## Architecture

```
.hx source
    │
    ▼ helix-asm (parse + resolve + encode)
raw codons
    │
    ▼ helix-bio (GC/homopolymer/repeat optimization)
bio-safe codons
    │
    ▼ helix-ecc (SYNC + Reed-Solomon + TMR)
DNA string  ──────────────────────────── aqueous/encapsulated storage
    │
    ▼ helix-ecc (decode: TMR vote + RS correction + SYNC resync)
raw codons
    │
    ▼ helix-vm (decode + execute)
result
```

## Crates

| Crate | Purpose |
|-------|---------|
| `helix-isa` | Instruction set, 4-codon encoding, GF tables |
| `helix-ecc` | Reed-Solomon, TMR, SYNC marker pipeline |
| `helix-bio` | Bio-safety linter and synonymous-codon optimizer |
| `helix-asm` | Assembler: `.hx` source → DNA string |
| `helix-vm` | Virtual machine: DNA → execute |
| `helix-sim` | Error-injection simulation and recovery metrics |

## Encoding

| Layer | Detail |
|-------|--------|
| Base alphabet | A C G T (2 bits each) |
| Codon | 3 bases = 6 bits, 64 possible values |
| Instruction | 4 codons = 24 bits (opcode + 3 fields) |
| Registers | R0–R7 (R0 hardwired zero) |
| ISA | 32 opcodes: arithmetic, logic, memory, branch, call/ret |

### Error Correction (4 layers)

1. **Codon degeneracy** — opcodes 0x00–0x0F have synonymous codons
2. **SYNC markers** — 4-codon pattern inserted every 8 instructions for frame sync
3. **Reed-Solomon** — RS(10,8) over GF(2⁶), corrects 1 symbol error per block
4. **TMR** — Triple Modular Redundancy with majority vote

### Bio-Safety Constraints

| Constraint | Limit |
|------------|-------|
| GC content | 40–60% per 100 nt window |
| Homopolymer run | ≤ 4 identical bases |
| Dinucleotide repeat | ≤ 4 repeats |
| Palindrome (hairpin) | ≤ 8 bp |

The bio optimizer swaps synonymous opcode codons to satisfy these constraints.

## Quick Start

```bash
# Build everything
cargo build --workspace -j4

# Assemble a program (outputs DNA string)
cargo run --bin helix-asm -- examples/fibonacci.hx

# Assemble and run (pipe DNA into the VM)
cargo run --bin helix-asm -- examples/fibonacci.hx --no-tmr 2>/dev/null | \
  cargo run --bin helix-vm --

# Simulate DNA storage errors
cargo run --bin helix-sim -- \
  "$(cargo run --bin helix-asm -- examples/gcd.hx --no-tmr 2>/dev/null)" \
  --sub-rate 0.005 --runs 50

# Run model weights demo
cargo run --example weights --manifest-path helix-vm/Cargo.toml
```

## Algorithm Examples

Programs in `examples/` use the Helix assembly language (`.hx`):

| File | Algorithm | Result |
|------|-----------|--------|
| `fibonacci.hx` | Iterative Fibonacci | R1 = F(10) = 55 |
| `gcd.hx` | Euclidean GCD | R1 = GCD(48, 18) = 6 |
| `sum_array.hx` | Array summation | R3 = 150 |
| `bubble_sort.hx` | Bubble sort | arr[0..7] sorted in-place |

### Assembly Language (`.hx`)

```asm
.program "my_program"
.version 1
.memory 256          ; optional: memory words to allocate
.data
arr: .words 1, 2, 3  ; data section (loaded at addr 0)
.text
.entry main          ; entry point label

main:
    IMML R1, 42      ; R1 = 42 (low byte)
    IMMH R1, 0       ; R1 high byte = 0
    HALT
```

**Branches use labels** — the assembler resolves offsets automatically:

```asm
loop:
    CMP  R1, R2
    BGE  done        ; branch to label
    INC  R1, R1
    JMP  loop
done:
    HALT
```

## Model Weights Storage

The `helix-vm/examples/weights.rs` demo shows how neural network weights can be stored as DNA:

- Quantize f32 weights → 6-bit values (0–63, one value per codon)
- Encode as 3 nucleotides per weight
- 20 weights → 60 nucleotides
- MSE from 6-bit quantization: ~0.0001

```bash
cargo run --example weights --manifest-path helix-vm/Cargo.toml
```

## Tests

```bash
# All tests
cargo test --workspace -j4

# Specific crate
cargo test -p helix-vm -j4

# Integration tests only
cargo test --test programs -p helix-vm -j4
```

Test coverage:

| Crate | Unit | Integration | E2E |
|-------|------|-------------|-----|
| helix-isa | encode/decode, GF tables | all-opcode roundtrip | — |
| helix-ecc | RS, TMR, SYNC | pipeline + error correction | — |
| helix-bio | GC, homopolymer, palindrome | constraint scanning | — |
| helix-asm | parser, resolver, emitter | assemble+run programs | — |
| helix-vm | CPU, memory, loader | fibonacci, gcd, sort | — |
| helix-sim | injection, metrics | — | assemble→inject→recover |

## Specification

See [`SPEC.md`](SPEC.md) for the full technical specification.
