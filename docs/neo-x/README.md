# Neo X

## Overview

Neo X is an EVM-compatible chain maintained by Neo, enabling developers to leverage Ethereum compatibility while benefiting from Neo's infrastructure and security. The Neo X module in NeoRust provides interfaces for interacting with this EVM-compatible environment.

## Key Features

- **EVM Compatibility Layer**: Interact with Neo X as an Ethereum-compatible chain
- **Bridge Functionality**: Transfer tokens seamlessly between Neo N3 and Neo X
- **Transaction Support**: Create, sign, and send transactions on Neo X
- **Smart Contract Interaction**: Deploy and interact with EVM smart contracts
- **Web3-Compatible API**: Use familiar Ethereum development patterns

## Components

### Neo X Provider

The Neo X Provider serves as the primary interface for connecting to Neo X nodes:

```rust,no_run
let provider = NeoXProvider::new_http("https://rpc.neoX.io");
let block_number = provider.get_block_number().await?;
```

### Transaction Management

Create, sign, and send transactions on Neo X:

```rust,no_run
let transaction = NeoXTransaction::new()
    .to("0x1234567890123456789012345678901234567890")
    .value(1_000_000_000_000_000_000u128) // 1 ETH in wei
    .gas_price(20_000_000_000u64) // 20 Gwei
    .gas_limit(21_000u64)
    .nonce(provider.get_transaction_count(account.address().to_eth_address(), None).await?)
    .chain_id(provider.get_chain_id().await?)
    .build();

let signed_tx = transaction.sign(account)?;
let txid = provider.send_raw_transaction(&signed_tx).await?;
```

### Smart Contract Interaction

Interact with EVM smart contracts deployed on Neo X:

```rust,no_run
let contract = NeoXContract::new(contract_address, provider.clone());

// Read-only call
let balance = contract.call_read("balanceOf", &[account.address().to_eth_address()]).await?;

// State-changing call
let tx = contract.call_write(
    account,
    "transfer",
    &[recipient, amount.to_string()],
    None,
).await?;
```

### Neo X Bridge

The bridge facilitates token transfers between Neo N3 and Neo X:

```rust,no_run
let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());

// Bridge from Neo N3 to Neo X
let txid = bridge.bridge_to_neox(
    account,
    BridgeToken::Gas,
    amount,
    account.address().to_eth_address(),
).await?;

// Bridge from Neo X to Neo N3
let txid = bridge.bridge_to_neo(
    account,
    BridgeToken::Gas,
    amount,
    account.address(),
).await?;
```

## Integration with Ethereum Tools

Neo X's EVM compatibility enables integration with popular Ethereum development tools:

- **Metamask**: Connect Metamask to Neo X by adding it as a custom network
- **Hardhat/Truffle**: Deploy Solidity contracts to Neo X
- **Web3.js/ethers.js**: Interact with Neo X using JavaScript libraries
- **OpenZeppelin**: Use standard contract implementations

## Considerations

- **Gas Costs**: Neo X uses a gas model similar to Ethereum
- **Cross-Chain Operations**: Bridge operations may take several minutes to complete
- **Asset Representation**: Assets bridged from Neo N3 are represented as ERC-20 tokens on Neo X
- **Security**: Use hardware wallets when possible for high-value operations

## Related Documentation

- [Neo X Tutorial](../src/tutorials/neo-x.md): Step-by-step guide to Neo X integration
- [Bridge Operations](bridge.md): Detailed guide to cross-chain operations
- [EVM Contracts](evm-contracts.md): Working with EVM contracts on Neo X