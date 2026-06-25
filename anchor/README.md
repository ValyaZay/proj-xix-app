# Bank Program
This is an MVP developed gradually in steps to achieve complete testing and internalize Solana program development roadmap.

***!!! Scope expansion disguised as ambition should be avoided by all means !!!***

## Step 1
Implement USDC Bank only.

### 1. Goal
Make it simple and correct, testable, deployable, frontend-integratable.

### 2. Implemented Features
1. &#x2705; Initialize bank. 
2. &#x2705; Initialize user. 
3. &#x2705; Close user.
4. &#x2705; Deposit USDC. 
5. &#x2705; Issue shares. There is no SPL token representation for shares, they are tracked internally per user and globally per bank. 
6. &#x2705; Withdraw USDC proportional to user shares / total shares (no external yield source in Step 1).

**Not meant to be implemented**:
1. No yield strategies.
2. No lending integration.
3. No fancy logic.

### 3. Testing
1. Tools: 
   1. &#x2705; LiteSVM.
   2. Surfpool.
2. Test cases: 
   - deposit/withdraw invariants:
     - &#x2705; sum of users deposit shares == bank total deposit shares
     - &#x2705; bank_token_account.amount >= bank_state.total_deposits
   - &#x2705; double withdraw attempt;
   - rounding edge cases;
   - zero deposit / zero share cases;
   - &#x2705; repeated deposit -> withdraw cycles.
3. Stateful fuzzing - scenarios:
   1. &#x2705; init_bank -> init_user -> for _ 0..100 { deposit -> record event -> check bank and user state -> check invariants -> roll slot and blockhash } 

   2. &#x2705; init_bank -> for _ 0..100 { init_user -> 
                                  deposit -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                  withdraw -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                }
   
   3. init_bank -> for _ 0..100 { init_user ->
                                  deposit -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                  withdraw -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                  withdraw -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                }
   4. init_bank -> for _ 0..100 { randomly choose actions among:
                                      init_user -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                      deposit -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                      withdraw -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                      close_user -> record event -> check bank and user state -> check invariants -> roll slot and blockhash;
                                }

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

### &#x2705; 5. Integrate file-based event indexer (a lightweight mock indexer inside test harness)
This is an off-chain post-processing layer in the test codebase. 
It processes transaction results after execution and:
1. parses a particular event,
3. serializes it,
4. appends them to a JSONL file.
**This simulates a simplified blockchain indexer for local development and UI testing purposes**
