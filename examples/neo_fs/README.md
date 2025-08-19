# NeoFS Examples

These examples show how to use the NeoFS functionality in NeoRust.

## Available Examples

1. **Basic Usage**: Shows how to create containers, upload/download objects, and manage access control.
2. **Multipart Upload**: Demonstrates how to upload large files in multiple parts.

## Running the Examples

> **Important Note**: The NeoFS module requires gRPC implementation which is planned for a future release.
> These examples demonstrate the API design and usage patterns that will be available once
> the gRPC backend is integrated. The current implementation returns NotImplemented errors
> as documented placeholders.

To run an example:

```bash
# Make sure you have a wallet file available
cargo run --example neo_fs_basic_usage

# Or for the multipart upload example
cargo run --example neo_fs_multipart_upload
```

You'll need to have a wallet file available and may need to adjust the paths in the examples to point to your specific wallet file.

## Example Output

Since NeoFS requires gRPC implementation (planned for future release), you'll see output like:

```
Creating container...
Error creating container: Not implemented: create_container: This method requires gRPC implementation
```

This is expected behavior until the full gRPC implementation is added.

## Requirements

- NeoRust SDK
- A Neo wallet with funds (for uploading to mainnet/testnet)
- Network connectivity to NeoFS nodes
