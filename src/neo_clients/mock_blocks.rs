#![allow(dead_code)]

use primitive_types::{H160, H256};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct MockBlocks;

impl MockBlocks {
	fn unix_timestamp_ms() -> u64 {
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards")
			.as_millis() as u64
	}
	pub(super) fn random_uint256() -> H256 {
		let mut rng = rand::thread_rng();
		let mut data = [0u8; 32];
		rng.fill(&mut data);
		H256::from(data)
	}

	pub(super) fn random_uint160() -> H160 {
		let mut rng = rand::thread_rng();
		let mut data = [0u8; 20];
		rng.fill(&mut data);
		H160::from(data)
	}

	// pub fn create_contract_state() -> ContractState {
	// 	ContractState {
	// 		id: 1,
	// 		nef: Self::create_nef_file(),
	// 		update_counter: 0,
	// 		hash: Self::random_uint160(),
	// 		manifest: Self::create_contract_manifest(),
	// 	}
	// }

	// pub fn create_invalid_transaction(transaction_type: InvalidTransactionType) -> Transaction {
	// 	let mut tx = Self::create_transaction(Self::random_uint160());
	//
	// 	match transaction_type {
	// 		InvalidTransactionType::InsufficientBalance => {
	// 			tx.sys_fee = i64::MAX;
	// 		},
	// 		InvalidTransactionType::InvalidSignature => {
	// 			tx.witnesses[0].invocation =
	// 				InvocationScript::from_serialized_script(vec![0xFF; 64]);
	// 		},
	// 		InvalidTransactionType::InvalidScript => {
	// 			tx.script = vec![0xFF];
	// 		},
	// 		InvalidTransactionType::Oversized => {
	// 			tx.script = vec![0; 65536]; // Assuming max size is less than this
	// 		},
	// 		InvalidTransactionType::Expired => {
	// 			tx.valid_until_block = 0;
	// 		},
	// 		// InvalidTransactionType::Conflicting => {
	// 		//     // This would depend on your conflict rules
	// 		//     tx.attributes.push(TransactionAttribute::Conflicts(Self::random_uint256()));
	// 		// },
	// 	}
	//
	// 	tx
	// }
}

pub(super) enum InvalidTransactionType {
	InsufficientBalance,
	InvalidSignature,
	InvalidScript,
	Oversized,
	Expired,
	// Conflicting,
}
