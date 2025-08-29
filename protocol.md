# Protocol.md  
SOL-XMR Atomic Swap (V1 – Stealth-UX)  
Last update: 2025-08-26

---

## 1.  Objective  
Allow a user holding **USDC/SOL on Solana** to swap into **native XMR** (and vice-versa) with a single interaction:  
*“Paste your Monero address → sign one Solana transaction → done.”*  
No temporary wallets, no claim secrets in the UI, no manual sweeping.

---

## 2.  Roles  
- **Alice** – end-user (taker).  
- **Bob** – liquidity maker, headless daemon.  
- **Relayer** – optional off-chain worker that pays SOL gas on behalf of Alice.

---

## 3.  Cryptographic Primitives  
| Primitive | Purpose |
|---|---|
| `H = SHA-256(s)` | Hash-lock used on both chains. |
| Ed25519 Adaptor Sig | Solana PDA unlock reveals `s` to Bob without exposing it on-chain. |
| Monero sub-address | Funds are locked to a one-time address that **still belongs to Alice’s wallet**, so no import step. |

---

## 4.  Direction A – USDC → XMR  
(Alice sells USDC, receives XMR)

### 4.1  Off-chain Setup  
1. Alice enters her **Monero address** `A`.  
2. Backend derives  
   `A_sub = A + H(A || swap_id) * G`  
   which is spendable with Alice’s existing seed.  
3. Bob agrees on amount and sends Alice:  
   - `quote_id`,  
   - `H = hash(s)` (but **not** `s`),  
   - `A_sub` (for confirmation).

### 4.2  On-chain Steps  
| Step | Chain | Action |
|---|---|---|
| 1 | Solana | Alice (or Relayer) calls `lock_usdc` PDA with condition: *“redeemable iff adaptor sig reveals preimage of `H`”*. |
| 2 | Monero | Bob locks exactly `xmr_amount` to `A_sub`. |
| 3 | Monero | After 10-block lock, Bob publishes adaptor signature on Solana, revealing `s`. |
| 4 | Solana | PDA releases USDC to Bob. |
| 5 | Monero | Bob uses `s` to sign the Monero spend key of `A_sub`; Alice already controls the key, so funds appear in her wallet automatically. |

---

## 5.  Direction B – XMR → USDC  
(Alice sells XMR, receives USDC)

Symmetric flow:

1. Alice enters her **Solana address** `S`.  
2. Backend derives adaptor key `P = S + hash(S || swap_id) * G`.  
3. Bob locks USDC on Solana to PDA controlled by `H`.  
4. Alice pays XMR to `A_sub` derived from `P`.  
5. After 10-block lock, Alice publishes adaptor sig on Solana, revealing `s`.  
6. Bob uses `s` to unlock the USDC PDA; USDC lands in `S`.

---

## 6.  Fail-safe Rules  
- **Solana timeout**: 24 h refund to Alice if Bob never locks XMR.  
- **Monero timeout**: 48 h refund to Bob if Alice never reveals `s`.  
- **Relayer repayment**: PDA includes a small fee that compensates the relayer automatically.

---

## 7.  User Interaction Summary  
| Direction | What Alice Does | What She Sees |
|---|---|---|
| USDC → XMR | Connect Solana wallet → paste XMR address → sign one transaction. | “Swap submitted… XMR arrived in wallet after ~10 min.” |
| XMR → USDC | Paste Solana address → pay XMR from any wallet. | “Send XMR to address shown… USDC appears in Solana wallet after ~10 min.” |

No seed phrases, no temporary wallets, no sweeping.

---

## 8.  Future Upgrades  
- **Encrypted order-book (V2)** – match intents under FHE so amounts & identities stay private.  
- **Cross-chain gas abstraction** – relayer batching via meta-tx.  
- **zkFHE coprocessor** – remove final 10-block Monero lock by proving inclusion in FHE.

---

End of document.
