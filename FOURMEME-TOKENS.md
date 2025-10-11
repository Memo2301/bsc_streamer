# Four.meme Token Support

## ⚠️ Important: Four.meme Tokens Don't Use PancakeSwap Initially

Your token `0x2a5f6ca36a2931126933c1fb9e333a9ba8154444` (尘埃) has **NO PancakeSwap pairs** because Four.meme tokens trade on a **bonding curve** platform first.

## How Four.meme Works

Four.meme is similar to pump.fun on Solana:

1. **Phase 1: Bonding Curve Trading**
   - Tokens are created and trade on Four.meme's bonding curve
   - Trades happen through Four.meme's contract, NOT PancakeSwap
   - Price increases as more people buy (bonding curve mechanism)

2. **Phase 2: DEX Migration** (if successful)
   - Once market cap threshold is reached (usually ~$69k)
   - Liquidity is migrated to PancakeSwap automatically
   - Token can then be traded on PancakeSwap

## Your Token Status

✅ Token exists: `0x2a5f6ca36a2931126933c1fb9e333a9ba8154444`
- Name: 尘埃 (Chinese: "dust" or "dust mote")
- Symbol: 尘埃
- Decimals: 18
- Total Supply: 1,000,000,000

❌ No PancakeSwap pairs found
- Still in bonding curve phase
- Trading on Four.meme platform

## How to Monitor Four.meme Tokens

### Option 1: Wait for DEX Migration

Once the token graduates to PancakeSwap, our streamer will automatically work:

```bash
# Just wait and check periodically
npm run check-token
```

When pairs appear, the streamer will work normally.

### Option 2: Monitor Four.meme's Bonding Curve Contract (Advanced)

To monitor trades on the bonding curve, we need to:

1. **Find Four.meme's bonding curve contract address**
   ```bash
   # Check the token on BSCScan
   https://bscscan.com/token/0x2a5f6ca36a2931126933c1fb9e333a9ba8154444
   
   # Look at recent transactions
   # The "To" address in buy/sell transactions is the bonding curve contract
   ```

2. **Identify the Swap event signature**
   - Four.meme's bonding curve emits swap/trade events
   - We need the event ABI to decode them

3. **Monitor the bonding curve contract**
   - Similar to monitoring PancakeSwap pairs
   - Listen for trade events on the bonding curve contract

## Quick Check Commands

### Check if token has graduated to PancakeSwap:
```bash
node tools/check-token.js
```

### Check token on BSCScan:
```bash
# Visit:
https://bscscan.com/token/0x2a5f6ca36a2931126933c1fb9e333a9ba8154444
```

### Check token on Four.meme:
```bash
# Visit Four.meme and search for your token
# Look for the trading interface
```

## Finding Four.meme's Bonding Curve Contract

To add bonding curve support, we need to find the contract address:

1. **Method 1: Check Recent Transactions**
   - Go to BSCScan: https://bscscan.com/token/0x2a5f6ca36a2931126933c1fb9e333a9ba8154444
   - Click "Transfers" or "Txns" tab
   - Look at recent buy/sell transactions
   - The "To" address is likely the bonding curve contract

2. **Method 2: Check Token Creation Transaction**
   - Find the token creation tx
   - The contract that created it is likely Four.meme's factory
   - Related contracts handle the bonding curve

3. **Method 3: Ask Community**
   - Check Four.meme's documentation
   - Ask in their Telegram/Discord
   - Look for official contract addresses

## Once You Find the Bonding Curve Contract

Share the address and I can:
1. Add support for monitoring bonding curve trades
2. Decode swap events from the bonding curve
3. Track prices on the bonding curve

Example:
```javascript
// If bonding curve contract is 0xABC123...
export const FOURMEME_BONDING_CURVE = '0xABC123...';

// Then we can listen for trade events on that contract
```

## Recommended Next Steps

1. **Check BSCScan** for your token's transactions
2. **Find the bonding curve contract address** (most frequent "To" address in trades)
3. **Share the contract address** and I'll add support for it
4. **Or wait** for the token to graduate to PancakeSwap (happens automatically at market cap threshold)

## Example: Checking on BSCScan

1. Go to: https://bscscan.com/token/0x2a5f6ca36a2931126933c1fb9e333a9ba8154444
2. Click "Transfers" tab
3. Look for recent trades
4. Note the "To" address - that's likely the bonding curve contract
5. Click on that contract address
6. Check if it has many transactions with your token
7. If yes, that's the bonding curve contract we need to monitor!

---

**Need Help?** Share the bonding curve contract address and I'll add full support for monitoring Four.meme tokens.

