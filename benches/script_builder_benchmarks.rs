use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neo3::{
	neo_builder::{AccountSigner, ScriptBuilder, Signer, TransactionBuilder},
	neo_clients::HttpProvider,
	neo_protocol::{Account, AccountTrait},
	neo_types::{ContractParameter, OpCode, ScriptHash},
};
use num_bigint::BigInt;
use std::str::FromStr;

fn benchmark_script_builder_simple(c: &mut Criterion) {
	c.bench_function("script_builder_simple", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			builder.push_integer(black_box(BigInt::from(42)));
			builder.push_data(black_box(b"test".to_vec()));
			builder.op_code(&[black_box(OpCode::Add)]);
			let _script = builder.to_bytes();
		});
	});
}

fn benchmark_script_builder_contract_call(c: &mut Criterion) {
	let contract_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();

	c.bench_function("script_builder_contract_call", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			builder
				.contract_call(
					black_box(&contract_hash),
					black_box("transfer"),
					black_box(&[
						ContractParameter::h160(&contract_hash),
						ContractParameter::h160(&contract_hash),
						ContractParameter::integer(100),
						ContractParameter::any(),
					]),
					None,
				)
				.unwrap();
			let _script = builder.to_bytes();
		});
	});
}

fn benchmark_transaction_builder(c: &mut Criterion) {
	let account = Account::create().unwrap();

	c.bench_function("transaction_builder", |b| {
		b.iter(|| {
			let mut builder: TransactionBuilder<HttpProvider> = TransactionBuilder::new();
			builder.set_script(black_box(Some(vec![0x00, 0x01, 0x02])));
			let signer: Signer =
				AccountSigner::called_by_entry_hash160(black_box(account.get_script_hash()))
					.unwrap()
					.into();
			let _ = builder.set_signers(vec![signer]);
			builder.nonce(black_box(12345)).unwrap();
			builder.set_additional_system_fee(black_box(1000000));
			builder.set_additional_network_fee(black_box(500000));
			builder.valid_until_block(black_box(1000000)).unwrap();
			let _tx = builder.build();
		});
	});
}

criterion_group!(
	benches,
	benchmark_script_builder_simple,
	benchmark_script_builder_contract_call,
	benchmark_transaction_builder
);
criterion_main!(benches);
