# Helix — DNA-Encoded Instruction Set Architecture

> **Status (v0.1.0).** This document is the design. Everything in the repository
> runs in software: no sequence has been synthesized, stored or sequenced, and
> storage errors exist only as `helix-sim` injections. Implemented in v0.1.0:
> the ISA and codon mapping (sections 2 to 4), the GC, homopolymer, dinucleotide
> repeat and palindrome checks with synonymous-codon swaps (5.2, partly 5.3),
> SYNC markers, Reed-Solomon over opcode codons and TMR (section 6), the
> assembler and the VM. Planned, not implemented: the strand header, footer,
> CRC and PCR primers (section 7), output formats other than a raw DNA string
> (8.3), the melting-temperature and inverted-repeat checks and NOP spacers
> (5.2, 5.3), the `--profile cell` rules (5.5), and the wet-lab steps of the
> walkthrough (section 13). Known gaps in v0.1.0: the `.data` section is not
> encoded into the DNA; the TMR separator codon `TGA` can occur inside the body;
> SYNC search runs at codon-aligned positions only, so a single insertion or
> deletion is not re-framed.

## 1. Overview

Helix is a system for encoding executable algorithms as synthetic DNA
sequences stored in aqueous solution. It comprises an ISA, an
assembler/compiler toolchain, error correction layers, physical stability
constraints, and a virtual machine (with future FPGA synthesis target).

The goal is to store simple, important algorithms (sorting, searching, sliding
window, etc.) as synthetic DNA in encapsulated aqueous storage, retrieve them
via sequencing, and execute them on a software or hardware decoder.

### 1.1 Design Principles

- **Physical stability** — sequences must be chemically stable in aqueous
  storage; encoding avoids motifs prone to hydrolysis, secondary structures,
  or synthesis/sequencing errors.
- **Error resilience** — DNA synthesis, storage, and sequencing introduce
  errors; the encoding must tolerate substitutions, insertions, and deletions.
- **Simplicity** — a minimal register machine sufficient for fundamental
  algorithms, not a general-purpose application platform.
- **Determinism** — same input always produces the same output; no
  floating-point, no undefined behavior.

### 1.2 Non-Goals

- General-purpose OS or application code
- Self-modifying code (future research direction)
- Direct biological computation (we use DNA as storage, not as a compute medium)
- In-vivo (living cell) storage (future extension; see Section 5.5)

---

## 2. Nucleotide Encoding

### 2.1 Base Mapping

```
Adenine  (A) = 0b00
Cytosine (C) = 0b01
Guanine  (G) = 0b10
Thymine  (T) = 0b11
```

### 2.2 Codons

A codon is a triplet of nucleotides — 6 bits, 64 possible values.

```
Example: GCA = 10 01 00 = 0x24 = 36
```

Codons are the atomic unit of the ISA. All instruction fields are codon-aligned.

### 2.3 Codon Table Partitioning

| Range (decimal) | Count | Purpose                          |
|------------------|-------|----------------------------------|
| 0–31             | 32    | Opcodes                          |
| 32–47            | 16    | Register refs / addressing modes |
| 48–55            | 8     | Control codons (start, stop, sync, ECC) |
| 56–63            | 8     | Reserved / padding               |

The exact codon-to-value mapping is defined in Section 4 (Codon Assignment
Table) and is designed to maximize Hamming distance between frequently-used
codons and to satisfy biological constraints.

---

## 3. Instruction Set Architecture

### 3.1 Registers

| Register | Name | Description                  |
|----------|------|------------------------------|
| R0       | ZERO | Hardwired to 0 (writes ignored) |
| R1       | GP1  | General purpose              |
| R2       | GP2  | General purpose              |
| R3       | GP3  | General purpose              |
| R4       | GP4  | General purpose              |
| R5       | GP5  | General purpose              |
| R6       | GP6  | General purpose              |
| R7       | ACC  | Accumulator / return value   |

Registers are 16-bit unsigned integers. This keeps the machine simple and
the DNA sequences short. 32-bit mode is a future extension.

### 3.2 Instruction Format

Fixed-width: 4 codons = 24 bits per instruction.

```
┌────────────┬────────────┬────────────┬────────────┐
│  Codon 1   │  Codon 2   │  Codon 3   │  Codon 4   │
│  OPCODE    │  DST/FLAGS │  SRC1/IMM  │  SRC2/IMM  │
│  6 bits    │  6 bits    │  6 bits    │  6 bits    │
└────────────┴────────────┴────────────┴────────────┘
 = 12 nucleotides per instruction
```

**Field semantics vary by instruction type:**

#### R-type (register-register)
```
[opcode:6][rd:3 | flags:3][rs1:3 | pad:3][rs2:3 | pad:3]
```

#### I-type (immediate)
```
[opcode:6][rd:3 | flags:3][imm8:8 (spanning codon3 + codon4 partial)][pad:2]
```

Note: The 6-bit codon boundaries are always preserved at the DNA level. The
sub-codon bit fields above describe the *logical* interpretation within each
codon.

#### I-type (alternate encoding for full 8-bit immediate)
```
[opcode:6][rd:3 | mode:3][imm_hi:6][imm_lo:6]
```
Where the 12 bits from codons 3-4 encode a value, but only the low 8 bits are
used as the immediate (top 4 bits = ECC/parity for the immediate value).

### 3.3 Instruction Listing

#### Arithmetic
| Mnemonic | Opcode | Type | Description                      |
|----------|--------|------|----------------------------------|
| ADD      | 0x00   | R    | Rd = Rs1 + Rs2                   |
| SUB      | 0x01   | R    | Rd = Rs1 - Rs2 (wrapping)        |
| MUL      | 0x02   | R    | Rd = (Rs1 * Rs2) & 0xFFFF       |
| DIV      | 0x03   | R    | Rd = Rs1 / Rs2 (trap on div/0)  |
| MOD      | 0x04   | R    | Rd = Rs1 % Rs2 (trap on div/0)  |
| INC      | 0x05   | R    | Rd = Rs1 + 1                     |
| DEC      | 0x06   | R    | Rd = Rs1 - 1                     |

#### Bitwise
| Mnemonic | Opcode | Type | Description                      |
|----------|--------|------|----------------------------------|
| AND      | 0x07   | R    | Rd = Rs1 & Rs2                   |
| OR       | 0x08   | R    | Rd = Rs1 \| Rs2                  |
| XOR      | 0x09   | R    | Rd = Rs1 ^ Rs2                   |
| NOT      | 0x0A   | R    | Rd = ~Rs1                        |
| SHL      | 0x0B   | R    | Rd = Rs1 << Rs2                  |
| SHR      | 0x0C   | R    | Rd = Rs1 >> Rs2 (logical)        |

#### Comparison
| Mnemonic | Opcode | Type | Description                      |
|----------|--------|------|----------------------------------|
| CMP      | 0x0D   | R    | Set flags: Rs1 - Rs2            |

CMP sets internal flags register (zero, negative, carry) used by branch
instructions.

#### Memory
| Mnemonic | Opcode | Type | Description                          |
|----------|--------|------|--------------------------------------|
| LOAD     | 0x0E   | R    | Rd = memory[Rs1]                     |
| STORE    | 0x0F   | R    | memory[Rs1] = Rs2                    |
| PUSH     | 0x10   | R    | SP--; memory[SP] = Rs1               |
| POP      | 0x11   | R    | Rd = memory[SP]; SP++               |

Memory is word-addressed (16-bit words). Address space: 1024 words (2 KB).
Stack pointer (SP) is internal, initialized to top of memory.

#### Immediate
| Mnemonic | Opcode | Type | Description                          |
|----------|--------|------|--------------------------------------|
| IMML     | 0x12   | I    | Rd[7:0] = imm8 (load low byte)      |
| IMMH     | 0x13   | I    | Rd[15:8] = imm8 (load high byte)    |

To load a 16-bit constant, use IMML then IMMH.

#### Control Flow
| Mnemonic | Opcode | Type | Description                          |
|----------|--------|------|--------------------------------------|
| JMP      | 0x14   | I    | PC = PC + sign_extend(imm8)          |
| BEQ      | 0x15   | I    | If zero flag: PC += sign_extend(imm8)|
| BNE      | 0x16   | I    | If !zero flag: PC += sign_extend(imm8)|
| BLT      | 0x17   | I    | If negative flag: PC += sign_extend(imm8)|
| BGE      | 0x18   | I    | If !negative flag: PC += sign_extend(imm8)|
| CALL     | 0x19   | I    | PUSH PC+1; PC += sign_extend(imm8)  |
| RET      | 0x1A   | R    | POP PC                               |

Branch offsets are in instruction units (not nucleotides), relative to current
PC.

#### Special
| Mnemonic | Opcode | Type | Description                          |
|----------|--------|------|--------------------------------------|
| NOP      | 0x1B   | -    | No operation                         |
| HALT     | 0x1C   | -    | Stop execution                       |
| SYNC     | 0x1D   | -    | ECC sync marker (see Section 6)      |
| ECCP     | 0x1E   | -    | ECC parity instruction (see Section 6)|
| TRAP     | 0x1F   | -    | Software trap (error/debug)          |

---

## 4. Codon Assignment Table

The mapping from 64 DNA codons to ISA values is designed with these priorities:

1. **Hamming distance** — codons for different opcodes should differ by >= 2
   nucleotides where possible, so a single-nucleotide substitution is
   detectable.
2. **GC balance** — the codon table should distribute GC-rich and AT-rich
   codons across all instruction types so programs naturally achieve ~50% GC.
3. **Biological avoidance** — the biological start codon (ATG) and stop codons
   (TAA, TAG, TGA) are assigned to control functions (SYNC, delimiters) so
   they never appear unexpectedly in instruction streams.

### 4.1 Reserved Biological Codons

| DNA Codon | Binary  | Assignment          |
|-----------|---------|---------------------|
| ATG       | 001110  | PROGRAM_START marker|
| TAA       | 110000  | PROGRAM_END marker  |
| TAG       | 110010  | BLOCK_END marker    |
| TGA       | 111000  | SECTION_SEPARATOR   |

Design intent: these codons should not appear in instruction opcode positions.
In v0.1.0 only `TGA` is used (as the TMR section separator), and `ATG`
(value 14) is the primary codon of LOAD, so this table is not yet enforced.

### 4.2 Degeneracy (Synonymous Codons)

Like biological DNA, multiple codons may encode the same logical value. This
provides:

- **Error tolerance** — a single-nucleotide substitution in a synonymous codon
  still decodes to the correct value.
- **Bio-safety tuning** — the assembler can choose among synonymous codons to
  fix GC content, avoid restriction sites, or break homopolymer runs.

In v0.1.0 (`helix-isa/src/table.rs`), opcodes 0x00–0x0F each have one
synonymous codon (the opcode value + 32, which changes the first base from A
to G); opcodes 0x10–0x1F have no synonym.

---

## 5. Sequence Stability and Storage

### 5.1 Storage Medium

Helix DNA programs are stored in **cell-free aqueous solution**, not inside
living organisms. This simplifies the system (no concerns about transcription,
translation, or host interference) but introduces its own stability
requirements.

Supported storage methods, in order of durability:

| Method                      | Durability         | Conditions              |
|-----------------------------|--------------------|------------------------|
| Silica-encapsulated DNA     | Thousands of years | Room temperature, dry   |
| Lyophilized (freeze-dried)  | Decades            | Room temperature, dry   |
| TE buffer at -20°C          | Decades            | Freezer                 |
| TE buffer at 4°C            | Years              | Refrigerator            |
| Deionized water at -20°C    | Years              | Freezer                 |
| Deionized water at 4°C      | Months             | Refrigerator            |
| Deionized water at RT       | Weeks–months       | Room temperature        |

**Recommended default:** Silica nanoparticle encapsulation (following the
"DNA of Things" approach by Grass et al.). The DNA is encapsulated in silica
microspheres that protect against hydrolysis, nuclease contamination, and UV
damage. Retrieval is done by dissolving the silica shell in buffered HF or
fluoride solution, then PCR-amplifying the released DNA.

**Budget option:** Lyophilized DNA on filter paper or in microtubes. Stable
for decades if kept dry and dark. Rehydrate in TE buffer before sequencing.

### 5.2 Sequence Constraints

Even without biological concerns, the DNA sequence must satisfy physical and
chemical constraints for reliable synthesis, storage, and sequencing:

| Constraint                        | Threshold               | Reason                          |
|-----------------------------------|-------------------------|---------------------------------|
| GC content per 100nt window       | 40%–60%                 | Synthesis yield, thermal stability |
| Homopolymer runs                  | <= 4 identical bases    | Synthesis/sequencing accuracy   |
| Palindromic sequences             | <= 8bp                  | Prevent hairpin formation       |
| Inverted repeats                  | <= 10bp within 50bp     | Prevent secondary structures    |
| Melting temperature uniformity    | ±5°C across 200nt windows | Even PCR amplification       |
| Dinucleotide repeats              | <= 4 repeats (e.g., ATATATAT) | Polymerase slippage    |

Note: Biological constraints (ORFs, promoters, restriction enzyme sites) are
NOT required for aqueous storage but are available as optional linter rules
via `--profile cell` for future in-vivo storage extensions (see Section 5.5).

### 5.3 Stability Linter Pipeline

```
Assembled DNA
    │
    ├──▶ GC window scan ──▶ swap synonymous codons to rebalance
    ├──▶ Homopolymer scan ──▶ swap synonymous codons to break runs
    ├──▶ Secondary structure scan ──▶ swap synonymous codons
    ├──▶ Dinucleotide repeat scan ──▶ swap synonymous codons
    ├──▶ Tm uniformity check ──▶ adjust GC locally
    │
    └──▶ PASS / FAIL + report
```

If a constraint cannot be satisfied by synonymous codon substitution alone, the
linter inserts NOP instructions (which have multiple synonymous encodings) as
spacers.

### 5.4 Retrieval Pipeline

```
Encapsulated DNA (silica, lyophilized, or in solution)
    │
    ├──▶ Release DNA (dissolve silica shell / rehydrate)
    ├──▶ PCR amplify using Helix-specific primers
    ├──▶ Sequence (Nanopore for speed, Illumina for accuracy)
    ├──▶ Base-call and output raw nucleotide string
    │
    └──▶ Feed into Helix VM decoder
```

### 5.5 Future Extension: In-Vivo Cell Storage

For future work, DNA programs could be integrated into living cells (E. coli,
yeast, etc.) at neutral genomic loci. This would require additional biological
safety constraints:

- Avoid open reading frames (ORFs) in all 6 reading frames
- Avoid promoter motifs (TATA box, Pribnow box, etc.)
- Avoid restriction enzyme recognition sites
- Avoid ribosome binding sites (Shine-Dalgarno sequences)

These rules are implemented in `helix-bio` and activated with
`--profile cell --organism ecoli|yeast|human`.

---

## 6. Error Correction

### 6.1 Error Model

DNA storage errors from synthesis, aqueous/encapsulated storage, and sequencing:

Rates below are design assumptions without a cited source; `helix-sim` uses
whatever rates are passed to it.

| Error Type    | Typical Rate | Description                        |
|---------------|-------------|------------------------------------|
| Substitution  | ~0.1–1%     | One nucleotide replaced by another |
| Deletion      | ~0.01–0.5%  | One nucleotide dropped             |
| Insertion     | ~0.01–0.5%  | One extra nucleotide inserted      |

### 6.2 Layered Error Correction

#### Layer 1: Codon Degeneracy (Hamming-like)

Design intent: synonymous codons are assigned such that single-nucleotide
substitutions within a codon either still decode to the same value (silent
mutation) or decode to an invalid/detectable value.

In v0.1.0 codon values are plain binary, so this holds only partly: for opcodes
0x00–0x0F, a substitution that turns the first base into its synonym's first
base is silent, while most other substitutions produce a different valid opcode
(for example `AAA` ADD becomes `CAA` PUSH).

#### Layer 2: SYNC Markers

A SYNC instruction is inserted every 8 instructions. The SYNC codon pattern is
a fixed, easily recognizable sequence that cannot appear in normal instruction
encodings.

```
SYNC pattern: [SYNC_opcode][0x2A][0x15][0x2A]
              = specific 12-nucleotide motif
```

Purpose: after an insertion or deletion corrupts the reading frame, the decoder
scans forward for the next SYNC marker to re-establish codon alignment. Data
between two SYNCs that can't be decoded is marked as a damaged block.

#### Layer 3: Reed-Solomon Codes

Instructions are grouped into blocks of 8. Each block is followed by 2 parity
instructions (ECCP), computed as Reed-Solomon parity over the 8 data
instructions treated as symbols in GF(2^6).

```
[SYNC][I0][I1][I2][I3][I4][I5][I6][I7][ECCP]
```

In v0.1.0 the Reed-Solomon symbols are the opcode codons (codon 0) of the 8
data instructions, and one ECCP instruction carries both parity symbols in its
last two codons. This corrects 1 wrong opcode codon per block. Operand codons
are not covered by RS; only TMR protects them.

#### Layer 4: Triple Modular Redundancy (TMR)

The entire program is stored as 3 independent copies on the DNA strand,
separated by SECTION_SEPARATOR codons (TGA). The decoder reads all three and
performs majority voting at the instruction level.

```
5'─[PRIMER]─[HEADER]─[COPY_A]─TGA─[COPY_B]─TGA─[COPY_C]─[FOOTER]─[PRIMER]─3'
```

In v0.1.0 the strand is `[COPY_A]─TGA─[COPY_B]─TGA─[COPY_C]` (no primers,
header or footer yet).

For algorithms where strand space is limited, TMR can be reduced to 2 copies
(detection only) or omitted (relying on RS codes alone).

### 6.3 Error Correction Pipeline (Decoder Side)

```
Raw nucleotide sequence
    │
    ├──▶ Primer detection and trimming
    ├──▶ Header parsing (magic, version, block count, checksum)
    ├──▶ TMR alignment (if 3 copies: majority vote per instruction)
    ├──▶ SYNC marker scan and frame alignment
    ├──▶ RS decode per block (correct up to 1 error/block)
    ├──▶ Codon-level validation (check against valid codon set)
    │
    └──▶ Decoded instruction stream (or UNRECOVERABLE error)
```

---

## 7. Program Structure on DNA

> **Planned.** Primers, header and footer are not implemented in v0.1.0. The
> strand is the ECC body only (section 7.3), repeated three times when TMR is on.

### 7.1 Strand Layout

```
5'──[FWD_PRIMER]──[HEADER]──[BODY]──[FOOTER]──[REV_PRIMER]──3'
```

### 7.2 Header (8 codons = 48 nucleotides)

| Field           | Codons | Description                                  |
|-----------------|--------|----------------------------------------------|
| Magic           | 2      | Fixed identifier: marks this as a Helix program |
| Version         | 1      | ISA version (major.minor in 6 bits)          |
| Flags           | 1      | TMR mode, ECC level, word size               |
| Block count     | 1      | Number of instruction blocks                 |
| Data length     | 1      | Total instruction count (excluding ECC/SYNC) |
| Memory required | 1      | Minimum memory words needed                  |
| Header checksum | 1      | XOR of all preceding header codons           |

### 7.3 Body

Sequence of instruction blocks, each structured as:

```
[SYNC][I0][I1][I2][I3][I4][I5][I6][I7][ECCP]
 1     8 instructions                   1 (2 parity symbols)
 = 10 instructions × 4 codons = 40 codons = 120 nucleotides per block
```

A short last block is padded with NOPs to 8 instructions.

If TMR is enabled, the body is repeated 3 times with TGA separators.

### 7.4 Footer (4 codons = 24 nucleotides)

| Field             | Codons | Description                           |
|-------------------|--------|---------------------------------------|
| Program checksum  | 2      | CRC-12 over all body codons           |
| End marker        | 2      | TAA + padding                         |

### 7.5 Primers

Flanking PCR primer regions (20–25 nucleotides each) enable selective
amplification of the program from a mixed DNA pool. Primers are:

- Unique to the Helix system (designed to not match common genomes)
- Enable selective retrieval even if DNA is contaminated with environmental DNA
- Not part of the encoded program

---

## 8. Assembler

### 8.1 Source Language

Helix assembly (`.hx` files) uses a simple line-oriented syntax:

```asm
; Comment
.program "bubble_sort"      ; Program metadata
.version 1                  ; ISA version
.memory 64                  ; Minimum memory words needed

.data                       ; Data section (loaded into memory at addr 0)
    array: .words 5, 3, 8, 1, 4

.text                       ; Code section
.entry main                 ; Entry point label

main:
    IMML  R1, 0             ; R1 = array base address
    IMML  R2, 5             ; R2 = array length
    CALL  bubble_sort
    HALT

bubble_sort:
    SUB   R3, R2, R0        ; i = len
outer:
    IMML  R4, 0             ; j = 0
inner:
    INC   R4, R4            ; j++
    CMP   R4, R3
    BGE   end_inner
    ; load arr[j] and arr[j-1]
    DEC   R5, R4            ; R5 = j-1
    ADD   R5, R1, R5        ; addr(j-1)
    LOAD  R5, R5            ; R5 = arr[j-1]
    ADD   R6, R1, R4        ; addr(j)
    LOAD  R6, R6            ; R6 = arr[j]
    CMP   R5, R6
    BGE   no_swap           ; if arr[j-1] <= arr[j], skip (unsigned)
    JMP   inner             ; (actually BLE, skip swap)
no_swap:
    ; swap arr[j-1] and arr[j]
    ADD   R7, R1, R4
    STORE R7, R5            ; arr[j] = old arr[j-1]
    DEC   R7, R4
    ADD   R7, R1, R7
    STORE R7, R6            ; arr[j-1] = old arr[j]
    JMP   inner
end_inner:
    DEC   R3, R3            ; i--
    IMML  R5, 1
    CMP   R3, R5
    BGE   outer             ; if i > 1, continue
    RET
```

### 8.2 Assembly Pipeline

```
.hx source
    │
    ├──▶ Parse (labels, directives, instructions)
    ├──▶ Resolve labels to offsets
    ├──▶ Encode instructions to binary (24-bit words)
    ├──▶ Group into blocks, insert SYNC markers
    ├──▶ Compute RS parity per block
    ├──▶ Optionally replicate for TMR
    ├──▶ Add header and footer
    ├──▶ Map binary to codon sequences
    ├──▶ Map codons to DNA nucleotides
    ├──▶ Run bio-safety linter (swap synonymous codons as needed)
    ├──▶ Add PCR primers
    │
    └──▶ Output: DNA sequence (.dna file) + assembly listing (.lst)
```

### 8.3 Output Formats

> **Planned.** v0.1.0 writes only the raw DNA string (to stdout or `-o FILE`),
> with a summary on stderr.

| Format  | Extension | Description                              |
|---------|-----------|------------------------------------------|
| DNA     | `.dna`    | Raw nucleotide string (ACGT only)        |
| FASTA   | `.fasta`  | Standard bioinformatics format           |
| Binary  | `.hxb`    | Raw binary (pre-DNA encoding)            |
| Listing | `.lst`    | Human-readable: address, binary, DNA, asm|
| Report  | `.rpt`    | Bio-safety report + ECC summary          |

---

## 9. Virtual Machine

### 9.1 Architecture

The Helix VM decodes and executes DNA sequences in software.

```
┌─────────────────────────────────────────┐
│              Helix VM                    │
│                                         │
│  ┌───────────┐  ┌───────────────────┐   │
│  │  Decoder   │  │  Register File    │   │
│  │ DNA→codon  │  │  R0-R7 (16-bit)  │   │
│  │ codon→inst │  │  PC, SP, FLAGS   │   │
│  └─────┬─────┘  └────────┬──────────┘   │
│        │                  │              │
│  ┌─────▼──────────────────▼──────────┐   │
│  │         Execution Engine           │   │
│  │   ALU  │  Memory  │  Control      │   │
│  └────────┴──────────┴───────────────┘   │
│                                         │
│  ┌───────────────────────────────────┐   │
│  │     Memory (1024 × 16-bit words)  │   │
│  └───────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### 9.2 Execution Lifecycle

1. Load `.dna` file
2. Strip primers
3. Parse header, validate magic and checksum
4. If TMR: separate copies, majority vote
5. For each block: validate SYNC, RS-decode, extract instructions
6. Load data section into memory starting at address 0
7. Set PC to entry point
8. Execute fetch-decode-execute loop until HALT or TRAP

### 9.3 Traps and Errors

| Trap Code | Name              | Cause                         |
|-----------|-------------------|-------------------------------|
| 0x00      | HALT              | Normal termination            |
| 0x01      | DIV_ZERO          | Division or modulo by zero    |
| 0x02      | STACK_OVERFLOW    | SP below address 0            |
| 0x03      | STACK_UNDERFLOW   | SP above initial value        |
| 0x04      | INVALID_ADDRESS   | Memory access out of bounds   |
| 0x05      | INVALID_OPCODE    | Unrecognized instruction      |
| 0x06      | ECC_FAILURE       | Unrecoverable data corruption |

### 9.4 Execution Limits

To prevent infinite loops (important for untrusted retrieved sequences):

- **Max instructions**: configurable, default 1,000,000
- **Max memory**: 1024 words (fixed)
- **Max stack depth**: 256 words

---

## 10. Error Simulation

The `helix-sim` tool injects realistic DNA storage errors into `.dna` files for
testing the decoder's error correction.

### 10.1 Error Injection Modes

| Mode          | Description                                          |
|---------------|------------------------------------------------------|
| Substitution  | Replace random nucleotides (configurable rate)       |
| Deletion      | Remove random nucleotides                            |
| Insertion     | Insert random nucleotides at random positions        |
| Composite     | All three at configurable independent rates          |
| Burst         | Concentrated errors in a random region (simulates    |
|               | localized DNA damage)                                |

### 10.2 Metrics

After decoding an error-injected sequence, the simulator reports:

- Instructions correctly recovered vs. corrupted
- Blocks recovered by RS vs. blocks recovered by TMR vs. unrecoverable
- SYNC re-alignment events
- Overall program integrity (pass/fail)

---

## 11. Crate Structure

```
helix/
├── Cargo.toml              (workspace)
├── SPEC.md                 (this file)
├── helix-isa/              ISA definitions, codon tables, encoding/decoding
│   └── src/
│       ├── lib.rs
│       ├── codon.rs        Codon type, nucleotide mapping
│       ├── instruction.rs  Instruction types, opcode enum
│       ├── encode.rs       Instruction → codon sequence
│       ├── decode.rs       Codon sequence → instruction
│       └── table.rs        Codon assignment table generation
├── helix-asm/              Assembler: .hx source → .dna output
│   └── src/
│       ├── lib.rs
│       ├── parser.rs       Assembly language parser
│       ├── resolver.rs     Label resolution, offset calculation
│       ├── emitter.rs      Binary emission, block layout
│       └── main.rs         CLI: helix-asm input.hx -o output.dna
├── helix-bio/              Sequence stability linter and optimizer
│   └── src/
│       ├── lib.rs
│       ├── gc.rs           GC content analysis and rebalancing
│       ├── homopolymer.rs  Homopolymer run detection
│       ├── repeats.rs      Dinucleotide repeat detection
│       ├── structure.rs    Secondary structure (palindrome/hairpin) detection
│       ├── cell_safety.rs  Optional in-vivo constraints (ORFs, promoters, RE sites)
│       └── optimizer.rs    Synonymous codon substitution engine
├── helix-ecc/              Error correction encoding and decoding
│   └── src/
│       ├── lib.rs
│       ├── reed_solomon.rs RS codes over GF(2^6)
│       ├── sync.rs         SYNC marker generation and scanning
│       ├── tmr.rs          Triple modular redundancy
│       └── pipeline.rs     Full ECC encode/decode pipeline
├── helix-vm/               Virtual machine: execute DNA programs
│   └── src/
│       ├── lib.rs
│       ├── loader.rs       DNA file loading, primer stripping, header parsing
│       ├── cpu.rs          Register file, ALU, control logic
│       ├── memory.rs       Memory subsystem
│       └── main.rs         CLI: helix-vm program.dna [--input data] [--trace]
└── helix-sim/              Error simulation and testing
    └── src/
        ├── lib.rs
        ├── inject.rs       Error injection engine
        ├── metrics.rs      Recovery metrics and reporting
        └── main.rs         CLI: helix-sim program.dna --sub-rate 0.01 ...
```

---

## 12. Future Directions

- **FPGA synthesis** — translate the VM into Verilog/VHDL for hardware
  execution on an FPGA. The fixed-width instruction format and simple register
  machine map cleanly to a pipelined hardware design.
- **32-bit word size** — extend registers and memory to 32 bits for more
  complex algorithms.
- **In-vivo cell storage** — integrate programs into living cells with
  organism-specific bio-safety profiles (E. coli, S. cerevisiae, human cell
  lines) including promoter, RE site, and ORF avoidance.
- **Self-modifying code** — explore controlled self-modification where
  "protein" outputs can influence which "gene" blocks execute next.
- **DNA synthesis integration** — direct output to synthesis vendor formats
  (Twist, IDT) with vendor-specific constraint checking.
- **Compression** — Huffman or arithmetic coding of instruction streams before
  codon mapping, to reduce strand length.
- **Standard library** — a collection of pre-assembled `.dna` files for common
  algorithms (sorting, searching, hashing, basic crypto).

---

## 13. Example: Full Pipeline Walkthrough

### Input: bubble_sort.hx
(See Section 8.1 for source)

### Step 1: Assemble (v0.1.0, real output)
```
$ helix-asm examples/gcd.hx -o gcd.dna
[ASM] program: gcd
[ASM] instructions: 9
[ASM] DNA length: 726 nt
[ASM] bio swaps: 5, violations remaining: 8
```

726 nt = 2 blocks (one padded) × 40 codons × 3 nt, three TMR copies plus two
`TGA` separators.

### Step 2: Execute (v0.1.0, real output)
```
$ helix-vm gcd.dna
[LOAD] 16 instructions, entry=0
[DONE] 23 instructions executed
[REGS] R1=6 R2=0 R3=0 R4=0 R5=0 R6=0 R7=0
[EXIT] OK
```

### Step 3: Simulated error test (v0.1.0, real output)
```
$ helix-sim gcd.dna
Program: 16 instructions, 726 nt
Running 1 simulation(s)...
=== Simulation Report ===
Original:  726 nt
Corrupted: 725 nt
Errors:    1 subs, 1 dels, 0 ins (total: 2)
Recovery:  0/16 instructions (0.0%)
ECC ok:    false
Output match: false
```

With the default seed (42) this run includes one deletion, which v0.1.0 cannot
re-frame (see the status note at the top). "Recovery" and "Output match"
compare decoded instructions with the originals; the program is not executed.

### Planned steps (not done)
Export to a vendor format (FASTA), synthesis, storage (silica encapsulation,
lyophilized, or TE buffer at -20°C), release and PCR amplification with Helix
primers, sequencing, and decoding of the base-called reads. None of these
steps has been carried out.
