# neo-rust-sdk 项目长期笔记

## 环境陷阱
- **PATH 上有独立安装的 cargo 1.95**（`/c/Program Files/Rust stable LLVM 1.95/bin`），挡在 rustup shim 前、无视 rust-toolchain.toml。所有 cargo 命令先 `export PATH="/c/Users/Administrator/.cargo/bin:$PATH"`；仓库 rust-toolchain.toml 固定 channel `1.99.1`（edition 2024）。
- git stash / git fetch 在本机会损坏 `.git/refs`（见用户级记忆），涉及 `.git` 写操作与 cargo 构建（写 target/）一律关沙箱执行。
- **github.com 的 HTTPS git 端点被网络重置**（schannel/openssl 均握手失败，api.github.com 正常）；push/fetch 走 SSH（`git push git@github.com:r3e-network/neo-rust-sdk.git master`，gh 已登录 Jim8y）。push 不会修 remote-tracking 引用，推完手工写 `refs/remotes/origin/master`。
- 远端曾领先本地一个审计修复提交（`0501292b fix: resolve release audit findings`，SGX/account_events/CI），已于 v3.3.2 rebase 合并；再遇到远端有未知提交时，先 fetch（用 SSH URL，只写 FETCH_HEAD）再比对。

## 协议对齐基线（v3.3.2 对齐，v3.3.3 依赖安全发布，2026-11-22）
- 对齐 Neo node v3.11.2：硬分叉枚举到 `HF_Jackalope`(=9)，`LAST_CONFIG_MANAGED_HARDFORK = HF_Huyao`（之后的 Iara/Jackalope 由 Policy.activateHardfork 链上激活）。
- 主网高度：Faun 8,800,000 / Gorgon 12,020,000；测试网：Faun 12,960,000 / Gorgon 17,960,000；Huyao 尚未公布高度，公布后在 `src/neo_config/config.rs` 补一行即可。
- Policy 封装含 `get_hardfork_activation_height` / `activate_hardfork` / `get(set)_temporary_storage_max_ttl`；ContractManagement 含 `get_contract_id`（v3.11.2 新增）。
- RunStats/动态定价、NeoGo 引用计数对齐等属节点 VM 内部，SDK 无 VM，无需实现。

## 代码风格约定
- edition 2024、rust-version 1.99；测试中 `std::env::set_var/remove_var` 必须 unsafe 块 + SAFETY 注释；let-chains 可用。
- 验证基线：clippy --workspace --all-features --all-targets 0 警告；`cargo test --workspace --lib` 734 全过（技术债清理后）。

## 技术债状态（2026-11-22 清理后）
- 已解决：elliptic-curve/k256/p256→0.14、jsonwebtoken→12.0（rsa 0.10）、chrono→time 0.3（全仓）、ed25519-dalek→3、scrypt→0.12、rustix→1.1.6。RUSTSEC-2023-0071 ignore 已移除。
- RUSTSEC-2026-0387 ignore 保留但已收窄：crypto-bigint 0.5.5 仅剩 alloy 2.x（k256 0.13）与 yubihsm 0.42 两条第三方路径；yubihsm main 已迁移待发版，发版后可清理。
- chrono 仍是 serde_with 等的可选依赖留在 Cargo.lock（cargo 标准行为，audit 中为不阻塞 warning，非直接依赖）。
- 技术债清理提交含破坏性 API（neo_fs 字段 OffsetDateTime、YubiWallet=WalletSigner<YubiHsmSigner>、Secp256r1PrivateKey::random 参数类型），下次发布 bump **4.0.0**；commit 尚未推送。
