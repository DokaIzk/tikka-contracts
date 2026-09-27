#!/usr/bin/env python3
"""Deterministically generate ``docs/ERRORS.md`` from ProtocolError."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DOCS = REPO_ROOT / "docs"
CATALOG = REPO_ROOT / "contracts" / "raffle-shared" / "src" / "errors.rs"

def parse_protocol_errors():
    """Parse catalog entries as (catalog code, name, contract ABI code)."""
    content = CATALOG.read_text(encoding="utf-8")
    match = re.search(r"pub enum ProtocolError\s*\{(.*?)\}", content, re.DOTALL)
    if not match:
        print(f"Error: Could not find ProtocolError in {CATALOG}", file=sys.stderr)
        sys.exit(1)

    errors = []
    pattern = re.compile(
        r"^[ \t]*(\w+)[ \t]*=[ \t]*(\d+),"
        r"[ \t]*(?://[ \t]*factory original:[ \t]*(\d+))?[ \t]*$",
        re.MULTILINE,
    )
    for entry in pattern.finditer(match.group(1)):
        catalog_code = int(entry.group(2))
        name = entry.group(1)
        factory_abi_code = entry.group(3)
        if name.startswith("Factory"):
            if factory_abi_code is None:
                print(f"Error: Missing factory ABI code for {name}", file=sys.stderr)
                sys.exit(1)
            errors.append((catalog_code, name[len("Factory"):], int(factory_abi_code)))
        elif catalog_code < 100:
            errors.append((catalog_code, name, catalog_code))

    errors.sort(key=lambda error: error[0])
    return errors


INSTANCE_DESCRIPTIONS = {
    "RaffleNotFound": "The raffle data was not found in storage",
    "RaffleInactive": "The raffle is not in an active state",
    "TicketsSoldOut": "All tickets have been sold",
    "InsufficientFunds": "User does not have enough balance",
    "NotAuthorized": "User is not authorized to perform this action",
    "OracleNotSet": "Oracle address is not configured",
    "RandomnessAlreadyRequested": "Randomness has already been requested",
    "NoRandomnessRequest": "No randomness request found",
    "FallbackTooEarly": "Fallback randomness triggered too early",
    "PrizeNotDeposited": "Prize has not been deposited yet",
    "PrizeAlreadyClaimed": "Prize has already been claimed",
    "PrizeAlreadyDeposited": "Prize deposit was already completed",
    "NotWinner": "Only the winner can claim the prize",
    "ClaimTooEarly": "Cannot claim before cooldown period",
    "InvalidParameters": "Invalid input parameters provided",
    "InvalidQuantity": "Invalid ticket quantity requested",
    "InvalidStatus": "The current raffle status doesn't allow this operation",
    "ContractPaused": "The contract is paused",
    "InvalidStateTransition": "Cannot transition to the requested state",
    "RaffleExpired": "The raffle end time has passed",
    "InsufficientTickets": "Not enough tickets sold to finalize",
    "MultipleTicketsNotAllowed": "User already has a ticket",
    "NoTicketsSold": "No tickets have been purchased",
    "TicketNotFound": "The requested ticket was not found",
    "RaffleEnded": "The raffle has already ended",
    "ArithmeticOverflow": "Arithmetic operation overflow",
    "AlreadyInitialized": "Contract is already initialized",
    "NotInitialized": "Contract has not been initialized",
    "Reentrancy": "Reentrant call detected",
    "TokenTransferFailed": "Token transfer failed",
    "NoActiveTickets": "No active tickets available",
    "DeadlinePassed": "Swap deadline has passed",
    "SlippageExceeded": "Slippage tolerance exceeded",
    "InvalidIndex": "Invalid index provided",
    "MorePrizesThanTickets": "More prizes than tickets",
    "ZeroPrize": "Prize amount is zero",
    "InvalidTokenAddress": "Invalid token address provided",
    "TooManyPrizes": "Exceeds maximum number of prizes",
    "EmergencyTooEarly": "Emergency withdraw too early",
    "InvalidTicketRange": "Invalid ticket range configured",
    "InsufficientAccumulatedFees": "Insufficient accumulated fees",
    "PrizeConfigurationLocked": "Prize configuration is locked",
    "ExceedsMaxTicketsPerTx": "Exceeds max tickets per transaction",
    "ExceedsMaxTicketsPerAddress": "Exceeds the per-address ticket cap",
    "DrawingAlreadyInProgress": "A draw is already in progress",
    "InvalidStatusForDrawingTransition": "Raffle status cannot enter Drawing",
    "DrawingAlreadyComplete": "Randomness was already provided",
    "InvalidEndTime": "Raffle end time is invalid",
    "InvalidAdminAddress": "Admin address is invalid",
    "RandomnessTooEarly": "Randomness fallback is not available yet",
    "CancelTimelockActive": "Cancellation timelock is still active",
    "CancelNotScheduled": "No cancellation is scheduled",
    "OracleNotRegistered": "Caller is not a registered oracle for this raffle",
    "DuplicateOracleSubmission": "This oracle has already submitted its seed",
    "CommitAlreadySubmitted": "A commit has already been submitted for this draw",
}

INSTANCE_MESSAGES = {
    "RaffleNotFound": "Raffle not found",
    "RaffleInactive": "This raffle is not currently active",
    "TicketsSoldOut": "Sorry, all tickets have been sold!",
    "InsufficientFunds": "Insufficient funds to complete this action",
    "NotAuthorized": "You are not authorized to perform this action",
    "OracleNotSet": "Oracle address is not set",
    "RandomnessAlreadyRequested": "Randomness request already in progress",
    "NoRandomnessRequest": "No randomness request found",
    "FallbackTooEarly": "Fallback randomness not available yet",
    "PrizeNotDeposited": "Prize not yet deposited",
    "PrizeAlreadyClaimed": "Prize has already been claimed",
    "PrizeAlreadyDeposited": "Prize has already been deposited",
    "NotWinner": "You are not the winner of this raffle",
    "ClaimTooEarly": "Please wait before claiming your prize",
    "InvalidParameters": "Invalid parameters provided",
    "InvalidQuantity": "Invalid ticket quantity",
    "InvalidStatus": "This action is not allowed in the current raffle state",
    "ContractPaused": "Contract is temporarily paused",
    "InvalidStateTransition": "Cannot change raffle to the requested state",
    "RaffleExpired": "This raffle has ended",
    "InsufficientTickets": "Minimum ticket requirement not met",
    "MultipleTicketsNotAllowed": "Multiple tickets not allowed for this raffle",
    "NoTicketsSold": "No tickets have been sold yet",
    "TicketNotFound": "Ticket not found",
    "RaffleEnded": "This raffle has already ended",
    "ArithmeticOverflow": "Calculation error occurred",
    "AlreadyInitialized": "Contract already initialized",
    "NotInitialized": "Contract not initialized",
    "Reentrancy": "Please try again later",
    "TokenTransferFailed": "Token transfer failed",
    "NoActiveTickets": "No active tickets available",
    "DeadlinePassed": "Swap deadline has passed",
    "SlippageExceeded": "Slippage tolerance exceeded",
    "InvalidIndex": "Invalid index provided",
    "MorePrizesThanTickets": "More prizes than tickets",
    "ZeroPrize": "Prize amount cannot be zero",
    "InvalidTokenAddress": "Invalid token address",
    "TooManyPrizes": "Too many prizes configured",
    "EmergencyTooEarly": "Emergency withdraw not available yet",
    "InvalidTicketRange": "Invalid ticket range",
    "InsufficientAccumulatedFees": "Insufficient accumulated fees",
    "PrizeConfigurationLocked": "Prize configuration is locked",
    "ExceedsMaxTicketsPerTx": "Too many tickets for one transaction",
    "ExceedsMaxTicketsPerAddress": "This address has reached the ticket limit",
    "DrawingAlreadyInProgress": "Drawing already in progress",
    "InvalidStatusForDrawingTransition": "Cannot start drawing in current state",
    "DrawingAlreadyComplete": "Drawing already complete",
    "InvalidEndTime": "Invalid raffle end time",
    "InvalidAdminAddress": "Invalid admin address",
    "RandomnessTooEarly": "Randomness fallback not available yet",
    "CancelTimelockActive": "Cancellation timelock is still active",
    "CancelNotScheduled": "No cancellation is scheduled",
    "OracleNotRegistered": "Caller is not a registered oracle for this raffle",
    "DuplicateOracleSubmission": "This oracle has already submitted its seed",
    "CommitAlreadySubmitted": "This oracle has already committed for this draw",
}

FACTORY_DESCRIPTIONS = {
    "AlreadyInitialized": "Factory is already initialized",
    "NotAuthorized": "User is not the admin",
    "ContractPaused": "Factory is paused",
    "InvalidParameters": "Invalid parameters provided",
    "RaffleNotFound": "Raffle instance not found",
    "AdminTransferPending": "Admin transfer already pending",
    "NoPendingTransfer": "No pending admin transfer",
    "RateLimitExceeded": "Creator rate limit exceeded",
    "NoPendingOp": "No pending admin operation",
    "TimelockNotElapsed": "Timelock has not elapsed",
    "InvalidRaffleId": "Raffle does not belong to this factory",
    "RaffleNotEligible": "Raffle is not eligible for this operation",
    "ArithmeticOverflow": "Arithmetic operation overflow",
    "TreasuryNotSet": "Treasury address is not configured",
    "RecurringNotFound": "Recurring raffle schedule not found",
    "IntervalNotElapsed": "Interval has not elapsed",
    "MaxRoundsReached": "Maximum rounds reached",
    "RecurringInactive": "Recurring raffle is inactive",
    "CreationPaused": "Raffle creation is paused",
    "CallerNotRegisteredRaffle": "Caller is not a raffle registered by this factory",
    "RandomnessSourceTooWeakForPrize": "Randomness source is too weak for the configured prize",
}

FACTORY_MESSAGES = {
    "AlreadyInitialized": "Factory already initialized",
    "NotAuthorized": "You are not the admin",
    "ContractPaused": "Factory is temporarily paused",
    "InvalidParameters": "Invalid parameters provided",
    "RaffleNotFound": "Raffle not found",
    "AdminTransferPending": "Admin transfer already pending",
    "NoPendingTransfer": "No pending admin transfer",
    "RateLimitExceeded": "Rate limit exceeded, try again later",
    "NoPendingOp": "No pending admin operation",
    "TimelockNotElapsed": "Timelock has not elapsed",
    "InvalidRaffleId": "Invalid raffle for this factory",
    "RaffleNotEligible": "Raffle is not eligible",
    "ArithmeticOverflow": "Calculation error occurred",
    "TreasuryNotSet": "Treasury address is not set",
    "RecurringNotFound": "Recurring raffle not found",
    "IntervalNotElapsed": "Interval has not elapsed",
    "MaxRoundsReached": "Maximum rounds reached",
    "RecurringInactive": "Recurring raffle is inactive",
    "CreationPaused": "Raffle creation is paused",
    "CallerNotRegisteredRaffle": "Caller is not a registered raffle",
    "RandomnessSourceTooWeakForPrize": "Randomness source is too weak for this prize",
}


def markdown_table(errors, descriptions, messages):
    lines = [
        "| Code | Error | Description | Frontend Message |",
        "| ---- | ----- | ----------- | ---------------- |",
    ]
    for _, abi_code, name in errors:
        desc = descriptions.get(name, "TODO: Add description")
        msg = messages.get(name, "TODO: Add message")
        lines.append(f"| {abi_code} | `{name}` | {desc} | \"{msg}\" |")
    return "\n".join(lines)


def typescript_mapping(instance_errors):
    lines = [
        "## Error Code Mapping (TypeScript)",
        "",
        "```typescript",
        "const errorMessages: Record<number, string> = {",
    ]
    for _, code, name in instance_errors:
        msg = INSTANCE_MESSAGES.get(name, "TODO: Add message")
        lines.append(f"  {code}: \"{msg}\",")
    lines.append("};")
    lines.append("```")
    return "\n".join(lines)


def main():
    protocol_errors = parse_protocol_errors()
    instance_errors = [error for error in protocol_errors if error[0] < 100]
    factory_errors = [error for error in protocol_errors if 200 <= error[0] <= 299]
    sections = [
        "## Instance Contract Errors\n\n"
        + markdown_table(instance_errors, INSTANCE_DESCRIPTIONS, INSTANCE_MESSAGES),
        "## Factory Contract Errors\n\n"
        + markdown_table(factory_errors, FACTORY_DESCRIPTIONS, FACTORY_MESSAGES),
    ]

    header = """# Error Codes

This document is **auto-generated** from `ProtocolError`. **Do not
edit by hand.** Regenerate whenever error codes or descriptions change:

```bash
python scripts/generate_error_docs.py
```

Sources of truth:
- Canonical catalog: `ProtocolError` in
    [`contracts/raffle-shared/src/errors.rs`](contracts/raffle-shared/src/errors.rs).
- Factory ABI codes remain unchanged; the catalog records them next to each
    namespaced factory code.

---

"""

    docs = (
        header
        + "\n\n---\n\n".join(sections)
        + "\n\n---\n\n"
        + typescript_mapping(instance_errors)
        + "\n"
    )

    out = DOCS / "ERRORS.md"
    out.write_text(docs, encoding="utf-8", newline="\n")
    print(f"Wrote {out.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
