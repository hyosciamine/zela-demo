# Zela Custom Procedures — Example Repository

This repository contains example **custom procedures** built on the [Zela](https://zela.io) platform. Each procedure is an independent Rust crate compiled to WebAssembly.

---

## Repository Structure

```
.
├── block_time/       # Procedure: BlockTime
├── hello_world/      # Procedure: HelloWorld
├── priority_fees/    # Procedure: PriorityFees
├── Cargo.toml        
├── Cargo.lock
├── run-procedure.sh  
├── shell.nix         
└── .gitignore
```

> **Only three procedures exist in this repository:** `block_time`, `hello_world`, and `priority_fees`. Calling any other procedure name will result in an error.

---

## Procedures

### 1. `hello_world`

A minimal example demonstrating how to accept input parameters, perform computation, and return a result (or an error).

**Input**

| Field           | Type  | Description              |
|----------------|-------|--------------------------|
| `first_number`  | `i32` | First operand            |
| `second_number` | `i32` | Second operand           |

**Output**

| Field | Type  | Description                   |
|-------|-------|-------------------------------|
| `sum` | `i32` | Sum of the two input numbers  |

**Error case**

If `first_number` is `0`, the procedure returns an error:

```json
{
  "code": 400,
  "message": "Example of an error -- number cannot be 0.",
  "data": null
}
```

---

### 2. `block_time`

Queries the Solana blockchain to retrieve the latest block time and block hash, then compares it against the system clock to measure RPC latency.

**Input**

None — this procedure takes no parameters (pass an empty object `{}`).

**Output**

| Field          | Type     | Description                                                   |
|----------------|----------|---------------------------------------------------------------|
| `block_time`   | `i64`    | Unix timestamp of the latest confirmed block (seconds)        |
| `block_hash`   | `string` | Base58-encoded hash of the latest block                       |
| `system_time`  | `i64`    | System clock timestamp at the start of the call (milliseconds)|
| `time_elapsed` | `i64`    | Total RPC round-trip time in **microseconds**                 |

**Example response**

```json
{
  "block_time": 1712000000,
  "block_hash": "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
  "system_time": 1712000000123,
  "time_elapsed": 340210
}
```

---

### 3. `priority_fees`

Scans one or more Solana blocks and computes the average priority fee paid by non-voting transactions.

The procedure supports two input modes — you must pass **exactly one** of them.

**Input: Latest N blocks**

```json
{
  "block_count": 10
}
```

| Field         | Type     | Description                                |
|---------------|----------|--------------------------------------------|
| `block_count` | `usize`  | Number of most recent confirmed blocks to scan |

**Input: Specific blocks**

```json
{
  "blocks": [295000000, 295000001, 295000005]
}
```

| Field    | Type       | Description                        |
|----------|------------|------------------------------------|
| `blocks` | `[u64]`    | List of specific slot numbers to scan |

**Output**

| Field                            | Type     | Description                                                        |
|----------------------------------|----------|--------------------------------------------------------------------|
| `total_transactions`             | `usize`  | Total number of transactions scanned across all blocks             |
| `vote_transactions`              | `usize`  | Number of transactions skipped because they are voting transactions |
| `latest_block`                   | `u64`    | Slot number of the last processed block                            |
| `average_priority_fee_lamports`  | `u64`    | Average priority fee (in lamports) across all non-voting transactions |

> **Note:** Priority fee = total fee − base fee (5000 lamports). Transactions with a fee below the base fee are skipped with an error log.

---
