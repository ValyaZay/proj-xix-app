# Bank Program
This is an MVP developed gradually in steps to achieve complete testing and internalize Solana program development roadmap.
!!! Scope expansion disguised as ambition should be avoided by all means !!!

## Step 1
Implement USDC Bank only.

### 1. Goal
Make it simple and correct, testable, deployable, frontend-integratable.

### 2. Implemented Features
1. Deposit USDC.
2. Issue shares. There is no SPL token representation for shares, they are tracked internally per user and globally per bank.
3. Withdraw USDC proportional to user shares / total shares (no external yield source in Step 1).

**Not meant to be implemented**:
1. No yield strategies.
2. No lending integration.
3. No fancy logic.

### 3. Testing
1. Tools: liteSVM, surfpool.
2. Test cases: 
   - deposit/withdraw invariants:
     - total users shares == bank total shares
     - bank_token_account.amount >= bank_state.total_deposits
   - double withdraw attempt;
   - rounding edge cases;
   - zero deposit / zero share cases;
   - repeated deposit / withdraw cycles.

### 4. Integrate to frontend
1. Generate TypeScript client with Codama.
2. Add simple React UI:
   - deposit USDC
   - withdraw USDC
   - display: 
     - user shares balance (on-chain state);
     - bank total assets (on-chain state);
     - bank total shares (on-chain state); 
     - current redeemable USDC based on shares (computed off-chain from vault state + shares);
     - deposit history (optional, reconstructed from indexed transaction logs/events);

### 5. Integrate file-based event indexer (a lightweight mock indexer inside test harness)
This is an off-chain post-processing layer in the test codebase. 
It processes transaction results after execution and:
1. reads transaction logs (including emitted events),
2. extracts meaningful events,
3. converts them into structured data,
4. appends them to a JSONL file.
**This simulates a simplified blockchain indexer for local development and UI testing purposes**
