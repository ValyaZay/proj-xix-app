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
2. Test cases: 
   - deposit/withdraw invariants:
     - &#x2705; sum of users deposit shares == bank total deposit shares
     - &#x2705; bank_token_account.amount >= bank_state.total_deposits
   - &#x2705; double withdraw attempt;
   - rounding edge cases;
   - zero deposit / zero share cases;
   - &#x2705; repeated deposit -> withdraw cycles.
3. Stateful fuzzing - scenarios:
   1. &#x2705; init_bank -> init_user -> for _ 0..100 { deposit -> check bank and user state -> check invariants -> roll slot and blockhash } 

   2. &#x2705; init_bank -> for _ 0..100 { init_user -> 
                                            deposit -> check bank and user state -> check invariants -> roll slot and blockhash;
                                            withdraw -> check bank and user state -> check invariants -> roll slot and blockhash;
                                          }
   
   3. &#x2705; init_bank -> for _ 0..100 { init_user ->
                                            deposit -> check bank and user state -> check invariants -> roll slot and blockhash;
                                            withdraw -> check bank and user state -> check invariants -> roll slot and blockhash;
                                            withdraw -> check bank and user state -> check invariants -> roll slot and blockhash;
                                          }
   4. init_bank -> for _ 0..100 { randomly choose actions among:
                                      init_user -> roll slot and blockhash -> set timestamp -> record event -> roll step -> check bank and user state -> check invariants;
                                      random user (among inited) deposits -> roll slot and blockhash -> set timestamp -> record event -> roll step -> check bank and user state -> check invariants;
                                      random user (among inited) withdraws -> roll slot and blockhash -> set timestamp -> record event -> roll step -> check bank and user state -> check invariants;
                                }
4. Replay events from the recorded events and compare the bank and users state:
   1. &#x2705;  state replay events for statefull fuzz scenario 3;
   2. logic replay events for statefull fuzz scenario 3;
   3. state replay events for statefull fuzz scenario 4;
   4. logic replay events for statefull fuzz scenario 4;

**!!! The jsonl files that expose a bug are saved in replay/fixtures directory for further investigation and logic replay**

### 4. &#x2757; POSTPONED &#x2757; Integrate to frontend
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
Every emitted event is recorded, followed by a BankSnapshot and a UserSnapshot events recorded for the debugging purpose during the events replay campaign, e.g.:
```
DepositEvent
BankSnapshot

DepositEvent
BankSnapshot

WithdrawEvent
BankSnapshot

CloseUserEvent
BankSnapshot
```


&#x1F331; &#x1F331; &#x1F331; &#x1F331; &#x1F331;

## Step 2 (DRAFT)
AMM integration

### 1. Goal
Make simple and correct `swap` mechanics 
Integrate AMM to the protocol logic
Add BTC bank
Update the user's state accordingly - there will be 2 assets already

### 2. Implemented features
1. Swap
2. Swap_preview

### 3. Testing
1. Tools: 
   1. LiteSVM.
   2. Surfpool.
2. Test cases: 
- `swap` invariants:
  - TODO
- deposit → swap → withdraw
- withdraw → deposit → swap
- failed swap + recovery states

### 4. Observation
1. Use the unified event schema:
```
{
   "step": 1,
   "seed": 1234,
   "type": "swap",
   "tx_id": "...", 
   "timestamp": 123456,
   "user": "...",
   "data": {
      "from_asset": "USDC",
      "to_asset": "BTC",
      "amount_in": 1000,
      "amount_out": 0.025
   }
}
```

2. Using unified event stream generated by a file-based event indexer, analyze:
- swap impact on vault liquidity
- slippage patterns
- sequence effects (deposit → swap → withdraw)
- invariant violations across flows

3. Make use of event replay

&#x1F331; &#x1F331; &#x1F331; &#x1F331; &#x1F331;

# Overall protocol shape
1. protocol logic (vault + AMM)
2. correctness layer (LiteSVM + fuzz)
3. observability layer (JSONL indexer)
