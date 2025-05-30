# Soroban Timelock Contract

## Project Structure

This repository uses the recommended structure for a Soroban project:
```text
.
├── contracts
│   └── sorobon-timelock
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
├── Cargo.lock
└── README.md
```


This project implements a "Claimable Balance with Timelock" smart contract on the Soroban platform. It allows a depositor to lock a certain amount of a specified token, making it claimable by a predefined set of claimants only after (or before) a specific time condition is met.

This project was developed as part of the Stellar Bootcamp by [Rise In](https://www.risein.com/).

## Features

*   **Token Deposit:** Users can deposit a specific token (SRC-20 compatible) into the contract.
*   **Claimant Whitelist:** Deposits can only be claimed by addresses specified in a claimant list during deposit.
*   **Time-Bound Claims:** Claims are subject to a time condition:
    *   `Before`: The balance can only be claimed *before* a specified Unix timestamp.
    *   `After`: The balance can only be claimed *after* a specified Unix timestamp.
*   **Single Use (Current Design):** The contract, as currently designed, is intended for a single deposit and a single successful claim. Once claimed, the balance entry is removed.
*   **Authorization:** Both deposit and claim operations require authorization from the respective addresses (`from` for deposit, `claimant` for claim).
*   **Limited Claimants:** A maximum of 10 claimants can be specified for a single claimable balance to prevent excessive storage use.

## Contract Interface

The contract exposes the following public functions:

### `deposit`

Deposits tokens into the contract and sets up the claimable balance.

*   **Parameters:**
    *   `from: Address` - The address depositing the tokens and authorizing the transaction.
    *   `token: Address` - The contract address of the token being deposited.
    *   `amount: i128` - The amount of tokens to deposit.
    *   `claimants: Vec<Address>` - A vector of addresses eligible to claim the balance (max 10).
    *   `time_bound: TimeBound` - A struct specifying the time condition for the claim.
        *   `kind: TimeBoundKind` - Enum (`Before` or `After`).
        *   `timestamp: u64` - The Unix timestamp for the time condition.
*   **Events:** (If you add events, list them here)
*   **Panics:**
    *   If `claimants.len() > 10`.
    *   If the contract has already been initialized (i.e., a deposit has already been made).
    *   If `from` has not authorized the call.

### `claim`

Allows an authorized claimant to claim the deposited tokens if the time condition is met.

*   **Parameters:**
    *   `claimant: Address` - The address attempting to claim the balance and authorizing the transaction.
*   **Events:** (If you add events, list them here)
*   **Panics:**
    *   If `claimant` has not authorized the call.
    *   If no balance is available to claim (or it has already been claimed).
    *   If the `time_bound` predicate is not fulfilled.
    *   If the `claimant` is not in the list of allowed claimants.

## Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
*   [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup)

## Building the Contract

To build the contract into a Wasm file:

```bash
soroban contract build