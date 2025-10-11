# Automatic DEX Migration Detection

## Overview

The BSC Token Streamer now automatically detects when a Four.meme bonding curve token migrates to PancakeSwap and seamlessly switches from bonding curve monitoring to DEX monitoring.

## How It Works

### 1. Initial Detection

When you start monitoring a token, the streamer checks if it's currently on the Four.meme bonding curve:

```bash
🔍 Checking Four.meme bonding curve for token...
  ✅ Found Four.meme bonding curve activity

🎯 Token is on Four.meme bonding curve!
📡 Monitoring bonding curve trades...
🔄 Watching for PairCreated event to auto-switch to DEX
```

### 2. Dual Event Monitoring

While monitoring bonding curve trades, the streamer simultaneously:
- **Captures all bonding curve trades** in real-time (Transfer events)
- **Watches PancakeSwap Factory** for PairCreated events (event-driven)
- **Maintains full price tracking** during bonding curve phase

### 3. Migration Detection (Event-Driven)

When the token migrates to PancakeSwap, the Factory emits a PairCreated event:

```bash
🎉 MIGRATION DETECTED! PairCreated event received!
🔄 Switching from bonding curve to DEX monitoring...

📡 Now monitoring 1 DEX pair(s)

  ✅ Listening to WBNB pair: 0x1234...5678

✨ DEX monitoring is now active!
```

### 4. Seamless Transition

After migration:
- **Bonding curve monitoring stops automatically**
- **DEX monitoring starts immediately**
- **Price tracking continues** with DEX data
- **No manual intervention required**

## Benefits

### ✅ Never Miss Price Updates

Your application continues receiving price updates even after the token migrates from Four.meme to PancakeSwap.

### ✅ Zero Downtime

The transition is seamless - you don't need to restart the streamer or reconfigure anything.

### ✅ Instant Detection

Migration is detected immediately when the PairCreated event is emitted by PancakeSwap Factory. No polling, no delays!

### ✅ Resource Efficient

Event-driven detection uses minimal resources. Only a single WebSocket filter watching for PairCreated events - no periodic RPC calls.

## Example Output

### Before Migration (Bonding Curve)

```bash
🔴 SELL 尘埃 ➡️ [Four.meme Bonding Curve]
   Amount: 2,865,937.474682 尘埃
   For: 0.000123 BNB
   Price: 0.000000000043 BNB
   Bonding Curve: 0x5c952063...550762b
   Tx: https://bscscan.com/tx/0x8a40d34...
   Time: 2025-10-09T11:36:11+00:00
```

### Migration Event (Instant!)

```bash
🎉 MIGRATION DETECTED! PairCreated event received!
🔄 Switching from bonding curve to DEX monitoring...

📡 Now monitoring 1 DEX pair(s)

  ✅ Listening to WBNB pair: 0x1234...5678

✨ DEX monitoring is now active!
```

### After Migration (DEX)

```bash
🟢 BUY 尘埃 from WBNB ➡️ [PancakeSwap V2]
   Amount: 1,234,567.89 尘埃
   For: 0.05 WBNB
   Price: 0.000000040521 WBNB
   Change: +5.23% 🚀
   Session High: 0.000000042100 WBNB
   Session Low: 0.000000038900 WBNB
   Swaps: 145
   Tx: https://bscscan.com/tx/0x9b51e45...
   Time: 2025-10-09T11:45:33+00:00
```

## Technical Details

### Event-Driven Detection

The streamer watches the PancakeSwap V2 Factory contract for `PairCreated` events:

```solidity
event PairCreated(
    address indexed token0,
    address indexed token1,
    address pair,
    uint
);
```

### How Migration is Detected

1. WebSocket filter monitors PancakeSwap Factory (0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73)
2. When a `PairCreated` event is emitted with our token address
3. Instant detection - typically within 1-3 seconds of migration transaction
4. Streamer immediately switches to DEX monitoring

### Performance Impact

- **Network:** Single WebSocket filter (event-driven, no polling)
- **CPU:** Negligible (only processes relevant events)
- **Memory:** Minimal (~few KB for channel)
- **Latency:** Near-instant (1-3 seconds from migration transaction)

## Configuration

No configuration required! The feature works automatically and is completely event-driven.

The PairCreated event filter is set up automatically when monitoring bonding curve tokens.

## Troubleshooting

### Migration not detected?

- **Check WebSocket connection:** Ensure your BSC WebSocket endpoint is working
- **Check Factory filter:** Verify the streamer is watching PancakeSwap Factory events
- **Check token address:** Ensure the token address is correct
- **Check migration transaction:** Verify the pair was actually created on PancakeSwap V2

### Still seeing bonding curve trades after migration?

- **Detection is instant:** Event-based detection happens within 1-3 seconds
- **Check console:** Look for "MIGRATION DETECTED! PairCreated event received!"
- **Restart streamer:** If issue persists, restart the application

## Use Cases

### 1. Trading Bots

Your bot can now monitor a token from launch on Four.meme through its migration to PancakeSwap without any manual intervention.

### 2. Price Tracking Services

Maintain continuous price data across both bonding curve and DEX phases.

### 3. Alert Systems

Get notified about trades whether they happen on the bonding curve or DEX.

### 4. Analytics Platforms

Collect complete trading data from token launch through DEX listing.

## Future Enhancements

Potential improvements for future versions:

- [ ] Support for other DEX factories (SushiSwap, Biswap, ApeSwap, etc.)
- [ ] Migration event callbacks for custom actions
- [ ] Emit migration events to external systems (webhooks, message queues)
- [ ] Track migration metadata (pair address, initial liquidity, timestamp)

---

🦀 Built with Rust for performance and reliability!

