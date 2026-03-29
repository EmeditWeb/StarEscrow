import { SorobanEvent } from "@subql/types-stellar";
import { EscrowCreated, MilestoneApproved } from "../types";

export async function handleEscrowCreated(event: SorobanEvent): Promise<void> {
    // 1. Decode the XDR from the event topics and data
    // In a real Subquery setup, we use generated types from the Contract ABI
    const { payer, freelancer, amount } = event.action.data;

    const record = new EscrowCreated(event.id);
    record.payer = payer.toString();
    record.freelancer = freelancer.toString();
    record.amount = BigInt(amount);
    record.ledger = event.ledger.sequence;
    
    await record.save();
}

export async function handleMilestoneApproved(event: SorobanEvent): Promise<void> {
    const { freelancer, index, amount } = event.action.data;

    const record = new MilestoneApproved(event.id);
    record.freelancer = freelancer.toString();
    record.milestoneIndex = index;
    record.amount = BigInt(amount);
    
    await record.save();
}