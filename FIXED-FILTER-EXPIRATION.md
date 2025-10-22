# Fixed: BSC Streamer Filter Expiration Errors

## 🐛 Problem

The BSC streamer was experiencing "filter not found" errors every 7 seconds:

```
ERROR make_request{method="eth_getFilterChanges"}: error=(code: -32000, message: filter not found, data: None)
```

### Root Cause

The streamer was using `.watch()` method which relies on:
1. **`eth_newFilter`** - Creates a filter on the RPC node
2. **`eth_getFilterChanges`** - Polls the filter every 7 seconds for new events

**Issue**: These filters expire quickly on RPC nodes (typically 5-10 minutes, but sometimes immediately). When the polling interval is too slow or the node is aggressive about filter cleanup, filters get removed before the next poll, causing the errors.

## ✅ Solution

Switched from polling-based filters (`.watch()`) to proper WebSocket subscriptions (`.subscribe_logs()`).

### What Changed

**Before (Polling Approach):**
```rust
// Old approach using eth_newFilter + eth_getFilterChanges
match parser.provider.watch(&filter).await {
    Ok(watcher) => {
        let mut stream = watcher.stream();
        while let Some(log) = stream.next().await {
            // Process log
        }
    }
}
```

**After (WebSocket Subscription):**
```rust
// New approach using eth_subscribe
match parser.provider.subscribe_logs(&filter).await {
    Ok(mut stream) => {
        while let Some(log) = stream.next().await {
            // Process log
        }
    }
}
```

### Technical Details

**WebSocket Subscriptions (`eth_subscribe`)**:
- ✅ Native push-based event streaming
- ✅ No filter expiration (subscription persists until explicitly unsubscribed)
- ✅ Lower latency (events pushed immediately)
- ✅ More efficient (no polling overhead)

**Polling Filters (`eth_newFilter`)**:
- ❌ Filter expiration issues
- ❌ Higher latency (depends on polling interval)
- ❌ More RPC calls (continuous polling)
- ❌ Can miss events if filter expires between polls

### Files Modified

1. **`bsc_streamer/src/core/streamer.rs`**:
   - Replaced all `.watch()` calls with `.subscribe_logs()`
   - Added `M::Provider: ethers::providers::PubsubClient` trait bound
   - Updated imports

2. **`bsc_streamer/src/lib.rs`**:
   - Added `PubsubClient` trait bound to `StreamerRunner` implementation

3. **`bsc_streamer/src/multi_token_streamer.rs`**:
   - Added `PubsubClient` trait bound to `add_token` method

### Affected Components

All event subscriptions now use WebSocket subscriptions:
- ✅ DEX swap events (PancakeSwap V2/V3)
- ✅ Bonding curve transfers (Four.meme)
- ✅ PairCreated events (migration detection)
- ✅ Post-migration DEX monitoring

## 🎯 Benefits

1. **No More Filter Errors**: Eliminates "filter not found" errors completely
2. **Lower Latency**: Events are pushed immediately instead of waiting for next poll
3. **More Reliable**: Subscriptions don't expire like filters do
4. **Better Performance**: No polling overhead, reduced RPC calls
5. **Cleaner Logs**: No more ERROR messages cluttering the output

## ⚠️ Requirements

The streamer now **requires** WebSocket provider with `PubsubClient` support:
- ✅ `Provider<Ws>` - Supported
- ❌ `Provider<Http>` - Not supported (no WebSocket)

This is already the case in your codebase, as you're using `Provider::<Ws>::connect()` for BSC connections.

## 📝 Notes

- The change is backward compatible for existing WebSocket usage
- HTTP providers would need to stay with `.watch()` but you're not using HTTP for BSC streaming
- WebSocket subscriptions are the recommended approach for real-time event monitoring in Ethereum/BSC applications

