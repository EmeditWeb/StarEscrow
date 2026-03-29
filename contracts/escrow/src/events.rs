use soroban_sdk::{symbol_short, Address, Env, String, Vec};
use crate::storage::Milestone;

pub fn contract_paused(env: Env, admin: Address) {
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("paused")),
        admin
    );
}

pub fn contract_unpaused(env: Env, admin: Address) {
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("unpaused")),
        admin
    );
}

pub fn escrow_created(env: Env, payer: Address, freelancer: Address, amount: i128, milestones: Vec<Milestone>) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("created")),
        (payer, freelancer, amount, milestones)
    );
}

pub fn yield_deposited(env: Env, protocol: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("yield"), symbol_short!("deposit")),
        (protocol, amount)
    );
}

pub fn milestone_submitted(env: Env, freelancer: Address, index: u32, description: String) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("submitted")),
        (freelancer, index, description)
    );
}

pub fn milestone_approved(env: Env, freelancer: Address, index: u32, description: String, amount: i128) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("approved")),
        (freelancer, index, description, amount)
    );
}

pub fn payment_released(env: Env, freelancer: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("released")),
        (freelancer, amount)
    );
}

pub fn dispute_raised(env: Env, caller: Address) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("disputed")),
        caller
    );
}

pub fn dispute_resolved(env: Env, release_to: Address) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("resolved")),
        release_to
    );
}

pub fn recurring_released(env: Env, freelancer: Address, amount: i128, count: u32) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("recurring")),
        (freelancer, amount, count)
    );
}

pub fn escrow_cancelled(env: Env, payer: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("cancelled")),
        (payer, amount)
    );
}

pub fn escrow_expired(env: Env, payer: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("escrow"), symbol_short!("expired")),
        (payer, amount)
    );
}

pub fn freelancer_transferred(env: Env, old: Address, new: Address) {
    env.events().publish(
        (symbol_short!("freelanc"), symbol_short!("transfer")),
        (old, new)
    );
}

pub fn payer_transferred(env: Env, old: Address, new: Address) {
    env.events().publish(
        (symbol_short!("payer"), symbol_short!("transfer")),
        (old, new)
    );
}

pub fn deadline_extended(env: Env, old: u64, new: u64) {
    env.events().publish(
        (symbol_short!("deadline"), symbol_short!("extended")),
        (old, new)
    );
}

pub fn milestone_updated(env: Env, old: String, new: String) {
    env.events().publish(
        (symbol_short!("mileston"), symbol_short!("updated")),
        (old, new)
    );
}
