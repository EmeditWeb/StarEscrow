# Soroban Event Indexing Integration Guide

This document provides the schema and integration steps for indexing `StarEscrow` contract events using **Mercury**.

## 1. Event Schema Documentation

The `StarEscrow` contract emits events for every state-changing operation. All events use the contract ID as the primary filter and a `Symbol` as the first topic.

| Event Topic (Symbol)  | Data Payload (Tuple)                                                                    | Triggering Action                       |
| :-------------------- | :-------------------------------------------------------------------------------------- | :-------------------------------------- |
| `escrow_created`      | `(payer: Address, freelancer: Address, total_amount: i128, milestones: Vec<Milestone>)` | Escrow initialization                   |
| `milestone_submitted` | `(freelancer: Address, idx: u32, description: String)`                                  | Work submitted for review               |
| `milestone_approved`  | `(freelancer: Address, idx: u32, description: String, amount: i128)`                    | Payment released for milestone          |
| `payment_released`    | `(freelancer: Address, amount: i128)`                                                   | Full payment/Recurring release finality |
| `escrow_cancelled`    | `(payer: Address, amount: i128)`                                                        | Escrow terminated by payer              |
| `escrow_expired`      | `(payer: Address, amount: i128)`                                                        | Funds reclaimed after deadline          |
| `yield_deposited`     | `(protocol: Address, amount: i128)`                                                     | Funds moved to yield strategy           |

## 2. Mercury Integration Guide

Mercury provides a specialized indexer for Soroban. To integrate StarEscrow:

### Step 1: Subscribe to Contract

Register your contract ID with Mercury to start ingesting historical and real-time events.

- **Endpoint:** `https://api.mercurydata.app/subscribe`
- **Body:**

```json
{
  "contract_id": "YOUR_CONTRACT_ID",
  "label": "StarEscrow_Main_Instance"
}
```

## Configure the Schema

Mercury maps XDR events into a queryable relational format. Use the following mapping for the escrow_created event:

- Topic 1: Symbol(escrow_created)
- Data[0]: Address (Payer)
- Data[1]: Address (Freelancer)
- Data[2]: i128 (Amount)

## Example Queries (GraphQL)

Fetch all Approved Milestones for a Freelancer

```graphql
query GetFreelancerEarnings($address: String!) {
  event_data(
    where: {
      topic1: { _eq: "milestone_approved" }
      data_address: { _eq: $address }
    }
  ) {
    ledger
    data_amount
    data_description
    transaction_hash
  }
}
```

# Monitor Escrow Creations (Real-time)

```graphql
subscription {
  event_data(where: { topic1: { _eq: "escrow_created" } }) {
    data_payer
    data_freelancer
    data_total_amount
  }
}
```
